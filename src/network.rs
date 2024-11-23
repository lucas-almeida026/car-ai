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

	pub fn feed_forward(&mut self, inputs: Vec<f64>) -> Vec<f64> {
		let mut outputs = self.levels[0].feed_forward(inputs);
		for i in 1..self.levels.len() {
			outputs = self.levels[i].feed_forward(outputs);
		}
		outputs
	}
}

pub struct Level {
	pub inputs: Vec<f64>,
	pub outputs: Vec<f64>,
	pub biases: Vec<f64>,
	pub weights: Vec<Vec<f64>>,
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
				self.weights[j][i] = rand::random::<f64>() * 2.0 - 1.0;
			}
		}

		for i in 0..self.biases.len() {
			self.biases[i] = rand::random::<f64>() * 2.0 - 1.0;
		}
	}

	pub fn feed_forward(&mut self, inputs: Vec<f64>) -> Vec<f64>{
		assert_eq!(self.inputs.len(), inputs.len());
		for i in 0..self.inputs.len() {
			self.inputs[i] = inputs[i];
		}

		for i in 0..self.outputs.len() {
			let mut sum = 0.0;
			for j in 0..self.inputs.len() {
				sum += self.inputs[j] * self.weights[i][j];
			}

			if sum > self.biases[i] {
				self.outputs[i] = 1.0;
			} else {
				self.outputs[i] = 0.0;
			}
		}
		self.outputs.clone()
	}
}