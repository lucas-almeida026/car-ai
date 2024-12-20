use crate::fns::lerpf32;
use serde::{de::Error, Deserialize, Serialize};
use serde_json::Result;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferBindingType, BufferDescriptor,
    CommandEncoderDescriptor, ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor,
    PipelineCompilationOptions, PipelineLayout, PipelineLayoutDescriptor, ShaderModuleDescriptor,
    ShaderSource, ShaderStages,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct NeuralNetwork {
    pub levels: Vec<Level>,
}
impl NeuralNetwork {
    pub fn new(neuron_count: &[u32]) -> Self {
        let mut levels: Vec<Level> = Vec::with_capacity(neuron_count.len());
        for i in 0..neuron_count.len() - 1 {
            levels.push(Level::new(neuron_count[i], neuron_count[i + 1]));
        }
        Self { levels }
    }

    pub fn randomize(&mut self) {
        for level in self.levels.iter_mut() {
            level.randomize();
        }
    }

    pub fn feed_forward(&mut self, inputs: &Vec<f32>) -> &Vec<f32> {
		let mut outputs = self.levels[0].feed_forward(inputs);
        for i in 1..self.levels.len() {
			outputs = self.levels[i].feed_forward(&outputs);
			// let (used, remaining) = self.levels.split_at_mut(i);
			// if i == 1 {
			// 	used.last_mut().unwrap().feed_forward(inputs);
			// } else {
			// 	remaining.first_mut().unwrap().feed_forward(&used.last().unwrap().outputs);
			// }
        }
		&self.levels.last().unwrap().outputs
    }

    pub async fn gpu_feed_forward<'a>(
        &mut self,
        inputs: &Vec<f32>,
        gpu_handler_factory: &mut GpuHandlerFactory<'a>,
    ) -> Vec<f32> {
        let mut outputs = self.levels[0]
            .gpu_feed_forward(inputs, gpu_handler_factory)
            .await;
        for i in 1..self.levels.len() {
            outputs = self.levels[i]
                .gpu_feed_forward(&outputs, gpu_handler_factory)
                .await;
        }
        outputs
    }

    pub fn prune(&mut self, base: &NeuralNetwork, t: f32) {
        for (x, level) in self.levels.iter_mut().enumerate() {
            for i in 0..level.biases.len() {
                level.biases[i] = lerpf32(level.biases[i], base.levels[x].biases[i], t);
            }

            for i in 0..level.weights.len() {
                for j in 0..level.weights[i].len() {
                    level.weights[i][j] =
                        lerpf32(level.weights[i][j], base.levels[x].weights[i][j], t);
                }
            }
        }
    }

    pub fn save_as_file(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string(&self).unwrap();
        std::fs::write(path, json).unwrap();
        Ok(())
    }

    pub fn load_from_file(path: &str) -> Result<NeuralNetwork> {
        let json = std::fs::read_to_string(path);
        if let Ok(json) = json {
            return Ok(serde_json::from_str(&json).unwrap());
        }
        Err(Error::custom("File not found"))
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Level {
    pub inputs: Vec<f32>,
    pub outputs: Vec<f32>,
    pub biases: Vec<f32>,
    pub weights: Vec<Vec<f32>>,
}

impl Level {
    pub fn new(input_count: u32, output_count: u32) -> Self {
        Self {
            inputs: vec![0.0; input_count as usize],
            outputs: vec![0.0; output_count as usize],
            biases: vec![0.0; output_count as usize],
            weights: vec![vec![0.0; input_count as usize]; output_count as usize],
        }
    }

    pub fn randomize(&mut self) {
        for i in 0..self.inputs.len() {
            for j in 0..self.outputs.len() {
                self.weights[j][i] = rand::random::<f32>() * 2.0 - 1.0;
            }
        }

        for i in 0..self.biases.len() {
            self.biases[i] = rand::random::<f32>() * 2.0 - 1.0;
        }
    }

    pub fn feed_forward(&mut self, inputs: &Vec<f32>) -> Vec<f32> {
        assert_eq!(self.inputs.len(), inputs.len());
        for i in 0..self.inputs.len() {
            self.inputs[i] = inputs[i];
        }

        for i in 0..self.outputs.len() {
            let mut sum = 0.0;
            for j in 0..self.inputs.len() {
                sum += self.inputs[j] * self.weights[i][j];
            }

            sum += self.biases[i];

            self.outputs[i] = sum.tanh();
        }

		self.outputs.clone()
    }

    pub async fn gpu_feed_forward<'a>(
        &mut self,
        inputs: &Vec<f32>,
        gpu_handler_factory: &mut GpuHandlerFactory<'a>,
    ) -> Vec<f32> {
        let flat_weights: Vec<f32> = self.weights.clone().into_iter().flatten().collect();
        let matrix_buffer = MatrixBuffer::new(
            gpu_handler_factory.device,
            &inputs,
            &flat_weights,
            &self.biases,
            &self.outputs,
        );
        let mut gpu_handler = gpu_handler_factory.create_handler(matrix_buffer);
        gpu_handler.dispatch();
        let outputs = gpu_handler.read_staging_buffer().await;
        self.outputs = outputs.clone();
        outputs
    }
}

pub struct GpuHandlerFactory<'a> {
    pub device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    shader: wgpu::ShaderModule,
    pub bind_group_layout: BindGroupLayout,
    pub pipeline_layout: PipelineLayout,
}

pub struct GpuHandler<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    shader: &'a wgpu::ShaderModule,
    matrix_buffer: MatrixBuffer,
    bind_group: BindGroup,
    pipeline: ComputePipeline,
}

impl<'a> GpuHandler<'a> {
    pub fn new(factory: &'a mut GpuHandlerFactory, matrix_buffer: MatrixBuffer) -> Self {
        let bind_group = factory.device.create_bind_group(&BindGroupDescriptor {
            label: Some("feed forward bind group"),
            layout: &factory.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: matrix_buffer.input_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: matrix_buffer.weight_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: matrix_buffer.bias_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: matrix_buffer.output_buffer.as_entire_binding(),
                },
            ],
        });
        let pipeline = factory
            .device
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("feed forward pipeline"),
                layout: Some(&factory.pipeline_layout),
                module: &factory.shader,
                entry_point: Some("main"),
                compilation_options: PipelineCompilationOptions::default(),
                cache: None,
            });

        Self {
            device: &factory.device,
            queue: &factory.queue,
            shader: &factory.shader,
            matrix_buffer,
            bind_group,
            pipeline,
        }
    }

    pub fn dispatch(&mut self) {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("feed forward encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("feed forward compute pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);
            compute_pass.dispatch_workgroups(64, 1, 1);
        }

        encoder.copy_buffer_to_buffer(
            &self.matrix_buffer.output_buffer,
            0,
            &self.matrix_buffer.staging_buffer,
            0,
            (64 * std::mem::size_of::<f32>()) as u64,
        );

        let cmd = encoder.finish();
        self.queue.submit(Some(cmd));
    }

    pub async fn read_staging_buffer(&self) -> Vec<f32> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let buffer_slice = self.matrix_buffer.staging_buffer.slice(..);

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        self.device.poll(wgpu::Maintain::Wait);
        receiver
            .await
            .expect("Failed to retrieve results")
            .expect("Failed to map buffer");

        let data = buffer_slice.get_mapped_range();
        let results: Vec<f32> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        self.matrix_buffer.staging_buffer.unmap();

        results.iter().map(|x| *x as f32).collect()
    }
}

impl<'a> GpuHandlerFactory<'a> {
    pub fn new(device: &'a wgpu::Device, queue: &'a wgpu::Queue) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("feed forward shader"),
            source: ShaderSource::Wgsl(Self::shader_mmul_64x64().into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("feed forward bind group layout"),
            entries: &[
                //inputs
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                //weights
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                //biases
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                //outputs
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("feed forward pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        Self {
            device,
            queue,
            shader,
            bind_group_layout,
            pipeline_layout,
        }
    }

    pub fn create_handler(&mut self, buffer: MatrixBuffer) -> GpuHandler {
        GpuHandler::new(self, buffer)
    }

    fn shader_mmul_64x64() -> String {
        let mut shader = r"
@group(0) @binding(0) var<storage, read> inputs: array<f32>;        // Input array (length 64)
@group(0) @binding(1) var<storage, read> weights: array<f32>; // Weights array (length 64x64)
@group(0) @binding(2) var<storage, read> biases: array<f32>;        // Bias array (length 64)
@group(0) @binding(3) var<storage, read_write> outputs: array<f32>;      // Output array (length 64)

// Entry point for the compute shader
@compute @workgroup_size(64) // Process 64 outputs in parallel

fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x; // Index for output neuron

    if (i < 64u) { // Check bounds
        var sum: f32 = 0.0;

        // Perform dot product of inputs and weights for this output neuron
        for (var j: u32 = 0u; j < 64u; j = j + 1u) {
            sum = sum + inputs[j] * weights[i * 64u + j];
        }

        // Add bias
        sum = sum + biases[i];

        // Apply activation function (tanh)
        outputs[i] = tanh(sum);
    }
}";
        shader.to_string()
    }
}

struct MatrixBuffer {
    input_buffer: wgpu::Buffer,
    weight_buffer: wgpu::Buffer,
    bias_buffer: wgpu::Buffer,
    output_buffer: wgpu::Buffer,
    staging_buffer: wgpu::Buffer,
}

impl MatrixBuffer {
    pub fn new(
        device: &wgpu::Device,
        input: &Vec<f32>,
        weights: &Vec<f32>,
        biases: &Vec<f32>,
        outputs: &Vec<f32>,
    ) -> Self {
        Self {
            input_buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: Some("input buffer"),
                contents: bytemuck::cast_slice(input),
                usage: wgpu::BufferUsages::STORAGE,
            }),
            weight_buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: Some("weight buffer"),
                contents: bytemuck::cast_slice(weights),
                usage: wgpu::BufferUsages::STORAGE,
            }),
            bias_buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: Some("bias buffer"),
                contents: bytemuck::cast_slice(biases),
                usage: wgpu::BufferUsages::STORAGE,
            }),
            output_buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: Some("output buffer"),
                contents: bytemuck::cast_slice(outputs),
                usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
            }),
            staging_buffer: device.create_buffer(&BufferDescriptor {
                label: Some("staging buffer"),
                size: outputs.len() as u64 * std::mem::size_of::<f32>() as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
        }
    }
}
