//! Core domain types with strong type safety
//!
//! This module defines the fundamental types used throughout Scout:
//! - `ImageHash`: Content-based file identification
//! - `Embedding`: Normalized vector representations for semantic similarity
//! - `CombineWeight`: Type-safe weight parameter for hybrid search
//! - `SearchMatch`: Search result with relevance score
//! - `MediaType`: Distinguishes images from videos

use std::path::PathBuf;

/// Type of media being processed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
	Image,
	Video,
}

/// Content-based hash identifier for media files (16-character hex string)
///
/// Uses FNV-1a hash of the first 64KB of file content for efficient
/// deduplication and change detection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageHash(pub String);

impl ImageHash {
	/// Returns the full hash string
	pub fn as_str(&self) -> &str {
		&self.0
	}

	/// Returns first 8 characters for logging/display
	pub fn short(&self) -> &str {
		&self.0[..8]
	}
}

impl std::fmt::Display for ImageHash {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

/// Normalized embedding vector for semantic similarity comparison
///
/// Embeddings are automatically normalized to unit length during construction
/// to enable cosine similarity via dot product.
#[derive(Debug, Clone)]
pub struct Embedding(pub Vec<f32>);

impl Embedding {
	/// Creates a new embedding with automatic normalization
	pub fn new(data: Vec<f32>) -> Self {
		Self(normalize(&data))
	}

	/// Creates an embedding from pre-normalized data (for deserialization)
	pub fn raw(data: Vec<f32>) -> Self {
		Self(data)
	}

	/// Computes cosine similarity with another embedding [0.0, 1.0]
	pub fn similarity(&self, other: &Self) -> f32 {
		self.0.iter().zip(other.0.iter()).map(|(a, b)| a * b).sum()
	}

	/// Combines text and image embeddings with weighted average
	///
	/// # Arguments
	/// * `text` - Optional text embedding
	/// * `image` - Optional image embedding
	/// * `text_weight` - Weight for text component [0.0, 1.0]
	///
	/// Returns `None` if both inputs are `None`, otherwise returns the weighted combination
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

/// Normalizes a vector to unit length
fn normalize(v: &[f32]) -> Vec<f32> {
	let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
	if norm > 0.0 {
		v.iter().map(|x| x / norm).collect()
	} else {
		v.to_vec()
	}
}

/// Type-safe weight parameter for combining text and image embeddings
///
/// Ensures weight is always in valid range [0.0, 1.0] where:
/// - 0.0 = 100% image, 0% text
/// - 0.5 = 50% image, 50% text
/// - 1.0 = 0% image, 100% text
#[derive(Debug, Clone, Copy)]
pub struct CombineWeight(f32);

impl CombineWeight {
	/// Creates a new weight, returning error if out of range
	pub fn new(w: f32) -> Result<Self, String> {
		if (0.0..=1.0).contains(&w) {
			Ok(Self(w))
		} else {
			Err(format!("weight must be [0.0, 1.0], got {}", w))
		}
	}

	/// Returns the weight value
	pub fn value(&self) -> f32 {
		self.0
	}
}

/// Search result containing path and relevance score
#[derive(Debug)]
pub struct SearchMatch {
	pub path: PathBuf,
	pub score: f32,
	#[cfg(feature = "video")]
	pub timestamp: Option<f64>, // For videos: timestamp in seconds
	#[cfg(feature = "video")]
	pub media_type: MediaType,
}

impl SearchMatch {
	pub fn new(path: PathBuf, score: f32) -> Self {
		Self {
			path,
			score,
			#[cfg(feature = "video")]
			timestamp: None,
			#[cfg(feature = "video")]
			media_type: MediaType::Image,
		}
	}

	#[cfg(feature = "video")]
	pub fn new_video(path: PathBuf, score: f32, timestamp: f64) -> Self {
		Self { path, score, timestamp: Some(timestamp), media_type: MediaType::Video }
	}
}
