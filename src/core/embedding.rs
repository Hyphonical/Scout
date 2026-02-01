//! Normalized embedding vectors for semantic similarity

#[derive(Debug, Clone)]
pub struct Embedding(Vec<f32>);

impl Embedding {
	/// Create normalized embedding from raw data
	pub fn new(data: Vec<f32>) -> Self {
		Self(normalize(&data))
	}

	/// Create from pre-normalized data (deserialization)
	pub fn raw(data: Vec<f32>) -> Self {
		Self(data)
	}

	/// Get raw vector
	pub fn as_slice(&self) -> &[f32] {
		&self.0
	}

	/// Cosine similarity [0.0, 1.0]
	pub fn similarity(&self, other: &Self) -> f32 {
		self.0.iter().zip(other.0.iter()).map(|(a, b)| a * b).sum()
	}

	/// Weighted combination of two embeddings
	pub fn blend(a: &Self, b: &Self, weight_a: f32) -> Self {
		let weight_b = 1.0 - weight_a;
		let combined: Vec<f32> =
			a.0.iter()
				.zip(b.0.iter())
				.map(|(av, bv)| av * weight_a + bv * weight_b)
				.collect();
		Self::new(combined)
	}
}

fn normalize(v: &[f32]) -> Vec<f32> {
	let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
	if norm > 0.0 {
		v.iter().map(|x| x / norm).collect()
	} else {
		v.to_vec()
	}
}
