//! Cluster data structures for HDBSCAN clustering

use serde::{Deserialize, Serialize};

/// Represents a single cluster of visually similar media
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
	/// Cluster ID (assigned after sorting by size)
	pub id: usize,
	/// File hashes of media in this cluster
	pub image_hashes: Vec<String>,
	/// Hash of the most representative file (closest to centroid)
	pub representative_hash: String,
	/// Average similarity within cluster (0.0-1.0)
	pub cohesion: f32,
}

/// Complete clustering result for a directory
#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterDatabase {
	/// Scout version that created this
	pub version: String,
	/// When clustering was performed
	pub timestamp: String,
	/// Parameters used
	pub params: ClusterParams,
	/// All discovered clusters
	pub clusters: Vec<Cluster>,
	/// File hashes marked as noise (don't belong to any cluster)
	pub noise: Vec<String>,
	/// Total images processed
	pub total_images: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterParams {
	pub min_cluster_size: usize,
	pub min_samples: Option<usize>,
}

impl ClusterDatabase {
	pub fn noise_percent(&self) -> f32 {
		if self.total_images == 0 {
			0.0
		} else {
			(self.noise.len() as f32 / self.total_images as f32) * 100.0
		}
	}
}
