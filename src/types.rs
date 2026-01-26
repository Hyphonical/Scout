// Types - Core domain types for type safety

use std::path::PathBuf;

/// 16-character hex hash of image file
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageHash(pub String);

impl ImageHash {
	pub fn as_str(&self) -> &str {
		&self.0
	}

	pub fn short(&self) -> &str {
		&self.0[..8]
	}
}

impl std::fmt::Display for ImageHash {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

/// Normalized embedding vector (unit length)
#[derive(Debug, Clone)]
pub struct Embedding(pub Vec<f32>);

impl Embedding {
	pub fn new(data: Vec<f32>) -> Self {
		Self(normalize(&data))
	}

	pub fn raw(data: Vec<f32>) -> Self {
		Self(data)
	}

	pub fn similarity(&self, other: &Self) -> f32 {
		self.0.iter().zip(other.0.iter()).map(|(a, b)| a * b).sum()
	}

	pub fn combine(text: Option<&Self>, image: Option<&Self>, text_weight: f32) -> Option<Self> {
		match (text, image) {
			(Some(t), Some(i)) => {
				let image_weight = 1.0 - text_weight;
				let combined: Vec<f32> = t.0.iter()
					.zip(i.0.iter())
					.map(|(tv, iv)| tv * text_weight + iv * image_weight)
					.collect();
				Some(Self::new(combined))
			}
			(Some(t), None) => Some(t.clone()),
			(None, Some(i)) => Some(i.clone()),
			(None, None) => None,
		}
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

/// Weight for combining text and image embeddings [0.0, 1.0]
#[derive(Debug, Clone, Copy)]
pub struct CombineWeight(f32);

impl CombineWeight {
	pub fn new(w: f32) -> Result<Self, String> {
		if (0.0..=1.0).contains(&w) {
			Ok(Self(w))
		} else {
			Err(format!("weight must be [0.0, 1.0], got {}", w))
		}
	}

	pub fn value(&self) -> f32 {
		self.0
	}
}

/// Result from searching indexed images
#[derive(Debug)]
pub struct SearchMatch {
	pub path: PathBuf,
	pub score: f32,
}

impl SearchMatch {
	pub fn new(path: PathBuf, score: f32) -> Self {
		Self { path, score }
	}
}
