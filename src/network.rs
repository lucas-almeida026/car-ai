use crate::fns::lerpf64;
use serde::{de::Error, Deserialize, Serialize};
use serde_json::{Result};

#[derive(Serialize, Deserialize)]
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

	pub fn prune(&mut self, base: &NeuralNetwork, t: f64) {
		for (x, level) in self.levels.iter_mut().enumerate() {
			for i in 0..level.biases.len() {
				level.biases[i] = lerpf64(level.biases[i], base.levels[x].biases[i], t);
			}

			for i in 0..level.weights.len() {
				for j in 0..level.weights[i].len() {
					level.weights[i][j] = lerpf64(level.weights[i][j], base.levels[x].weights[i][j], t);
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

#[derive(Serialize, Deserialize)]
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