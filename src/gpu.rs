pub async fn init_wgpu() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("Failed to find a GPU adapter");
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .expect("Failed to create device");
    (device, queue)
}

pub fn generate_feed_forward_shader(neuron_count: &[u32]) -> String {
	assert!(neuron_count.len() > 1, "There mustt be at least one layer in the network");
	let mut shader = String::new();

	shader.push_str("@group(0) @binding(0) var<storage, read> inputs: array<f32>;\n");
    shader.push_str("@group(0) @binding(1) var<storage, read> weights: array<f32>;\n");
    shader.push_str("@group(0) @binding(2) var<storage, read> biases: array<f32>;\n");
    shader.push_str("@group(0) @binding(3) var<storage, write> outputs: array<f32>;\n\n");

    shader.push_str("@compute @workgroup_size(64)\n");
    shader.push_str("fn main(@builtin(global_invocation_id) id: vec3<u32>) {\n");
    shader.push_str("    let idx = id.x;\n");
	// Generate shader code for each layer
    let mut offset_weights = 0;
    let mut offset_biases = 0;
    let mut input_size = neuron_count[0];

    for (i, &output_size) in neuron_count[1..].iter().enumerate() {
        shader.push_str(&format!(
            "    if (idx < {}) {{\n",
            output_size
        ));
        shader.push_str("        var sum: f32 = 0.0;\n");
        shader.push_str("        for (var j: u32 = 0; j < INPUT_SIZE; j++) {\n");
        shader.push_str(&format!(
            "            sum += inputs[j] * weights[{} + idx * INPUT_SIZE + j];\n",
            offset_weights
        ));
        shader.push_str("        }\n");
        shader.push_str(&format!(
            "        outputs[idx] = select(0.0, 1.0, sum > biases[{} + idx]);\n",
            offset_biases
        ));
        shader.push_str("    }\n");

        // Update offsets and input size for the next layer
        offset_weights += input_size * output_size;
        offset_biases += output_size;
        input_size = output_size;
    }

    shader.push_str("}\n");
    shader.replace("INPUT_SIZE", &neuron_count[0].to_string())
}

pub fn generate_feed_forward_shader_2(neuron_count: &[u32]) -> String {
	assert!(neuron_count.len() > 1, "There must be at least one layer in the network");

    let mut shader = String::new();

    shader.push_str("@group(0) @binding(0) var<storage, read> inputs: array<f32>;\n");
    shader.push_str("@group(0) @binding(1) var<storage, read> weights: array<f32>;\n");
    shader.push_str("@group(0) @binding(2) var<storage, read> biases: array<f32>;\n");
    shader.push_str("@group(0) @binding(3) var<storage, write> outputs: array<f32>;\n\n");

    shader.push_str("@compute @workgroup_size(64)\n");
    shader.push_str("fn main(@builtin(global_invocation_id) id: vec3<u32>) {\n");
    shader.push_str("    let idx = id.x;\n");

    let mut offset_weights = 0;
    let mut offset_biases = 0;

    for (i, (&input_size, &output_size)) in neuron_count.iter().zip(neuron_count.iter().skip(1)).enumerate() {
        shader.push_str(&format!(
            "    if (idx < {}) {{\n",
            output_size
        ));
        shader.push_str("        var sum: f32 = 0.0;\n");
        shader.push_str(&format!(
            "        for (var j: u32 = 0; j < {}; j++) {{\n",
            input_size
        ));
        shader.push_str(&format!(
            "            sum += inputs[j] * weights[{} + idx * {} + j];\n",
            offset_weights, input_size
        ));
        shader.push_str("        }\n");
        shader.push_str(&format!(
            "        outputs[idx] = select(0.0, 1.0, sum > biases[{} + idx]);\n",
            offset_biases
        ));
        shader.push_str("    }\n");

        shader.push_str("    barrier();\n");
        shader.push_str("    inputs = outputs;\n"); // Pass outputs to inputs for the next layer

        offset_weights += input_size * output_size;
        offset_biases += output_size;
    }

    shader.push_str("}\n");

    shader
}