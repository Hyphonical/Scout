//! HDBSCAN clustering for image embeddings

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use hdbscan::{Hdbscan, HdbscanHyperParams};
use rayon::prelude::*;

use crate::core::{Cluster, ClusterDatabase, ClusterParams, Embedding};
use crate::storage::Sidecar;
use crate::ui;

/// Clusters embeddings using HDBSCAN algorithm
pub fn cluster_embeddings(
	sidecars: Vec<(PathBuf, Sidecar)>,
	params: ClusterParams,
) -> Result<ClusterDatabase> {
	if sidecars.is_empty() {
		anyhow::bail!("No embeddings found to cluster");
	}

	ui::info(&format!("Clustering {} images", sidecars.len()));

	// Extract embeddings and build lookup maps
	let mut embeddings_2d: Vec<Vec<f32>> = Vec::with_capacity(sidecars.len());
	let mut hash_to_idx: HashMap<String, usize> = HashMap::new();
	let mut idx_to_hash: Vec<String> = Vec::with_capacity(sidecars.len());

	for (idx, (_, sidecar)) in sidecars.iter().enumerate() {
		let hash = sidecar.hash().to_string();
		let embedding = sidecar.primary_embedding();

		embeddings_2d.push(embedding.0.clone());
		hash_to_idx.insert(hash.clone(), idx);
		idx_to_hash.push(hash);
	}

	// Configure HDBSCAN
	let hyper_params = match params.min_samples {
		Some(min_samples) => HdbscanHyperParams::builder()
			.min_cluster_size(params.min_cluster_size)
			.min_samples(min_samples)
			.build(),
		None => HdbscanHyperParams::builder()
			.min_cluster_size(params.min_cluster_size)
			.build(),
	};

	// Run clustering
	let clusterer = Hdbscan::new(&embeddings_2d, hyper_params);
	let labels = clusterer.cluster().context("HDBSCAN clustering failed")?;

	// Process results
	let mut cluster_map: HashMap<i32, Vec<String>> = HashMap::new();
	let mut noise_hashes: Vec<String> = Vec::new();

	for (idx, &label) in labels.iter().enumerate() {
		let hash = &idx_to_hash[idx];
		if label == -1 {
			noise_hashes.push(hash.clone());
		} else {
			cluster_map
				.entry(label)
				.or_default()
				.push(hash.clone());
		}
	}

	// Build clusters with representatives and cohesion scores
	let clusters: Vec<Cluster> = cluster_map
		.into_par_iter()
		.map(|(cluster_id, hashes)| {
			let representative = find_representative(&hashes, &sidecars, &hash_to_idx);
			let cohesion = compute_cohesion(&hashes, &sidecars, &hash_to_idx);

			Cluster {
				id: cluster_id as usize,
				image_hashes: hashes,
				representative_hash: representative,
				cohesion,
			}
		})
		.collect();

	// Sort clusters by size (largest first) and re-assign IDs
	let mut clusters = clusters;
	clusters.sort_by(|a, b| b.image_hashes.len().cmp(&a.image_hashes.len()));

	for (new_id, cluster) in clusters.iter_mut().enumerate() {
		cluster.id = new_id;
	}

	Ok(ClusterDatabase {
		version: env!("CARGO_PKG_VERSION").to_string(),
		timestamp: chrono::Utc::now().to_rfc3339(),
		params,
		clusters,
		noise: noise_hashes,
		total_images: sidecars.len(),
	})
}

/// Find the most representative image in a cluster (closest to centroid)
fn find_representative(
	hashes: &[String],
	sidecars: &[(PathBuf, Sidecar)],
	hash_to_idx: &HashMap<String, usize>,
) -> String {
	let embeddings: Vec<Embedding> = hashes
		.iter()
		.filter_map(|h| hash_to_idx.get(h).map(|&idx| sidecars[idx].1.primary_embedding()))
		.collect();

	if embeddings.is_empty() {
		return hashes.first().cloned().unwrap_or_default();
	}

	let centroid = compute_centroid(&embeddings);

	hashes
		.iter()
		.max_by(|a, b| {
			let sim_a = hash_to_idx
				.get(*a)
				.map(|&idx| centroid.similarity(&sidecars[idx].1.primary_embedding()))
				.unwrap_or(0.0);
			let sim_b = hash_to_idx
				.get(*b)
				.map(|&idx| centroid.similarity(&sidecars[idx].1.primary_embedding()))
				.unwrap_or(0.0);
			sim_a.partial_cmp(&sim_b).unwrap_or(std::cmp::Ordering::Equal)
		})
		.cloned()
		.unwrap_or_else(|| hashes[0].clone())
}

/// Compute average pairwise similarity within cluster
fn compute_cohesion(
	hashes: &[String],
	sidecars: &[(PathBuf, Sidecar)],
	hash_to_idx: &HashMap<String, usize>,
) -> f32 {
	if hashes.len() < 2 {
		return 1.0;
	}

	let embeddings: Vec<Embedding> = hashes
		.iter()
		.filter_map(|h| hash_to_idx.get(h).map(|&idx| sidecars[idx].1.primary_embedding()))
		.collect();

	let mut total_similarity = 0.0;
	let mut count = 0;

	for i in 0..embeddings.len() {
		for j in (i + 1)..embeddings.len() {
			total_similarity += embeddings[i].similarity(&embeddings[j]);
			count += 1;
		}
	}

	if count > 0 {
		total_similarity / count as f32
	} else {
		1.0
	}
}

/// Compute centroid (mean) of embeddings
fn compute_centroid(embeddings: &[Embedding]) -> Embedding {
	if embeddings.is_empty() {
		return Embedding::raw(vec![0.0; 1024]);
	}

	let dim = embeddings[0].0.len();
	let mut centroid = vec![0.0; dim];

	for emb in embeddings {
		for (i, &val) in emb.0.iter().enumerate() {
			centroid[i] += val;
		}
	}

	let n = embeddings.len() as f32;
	for val in &mut centroid {
		*val /= n;
	}

	Embedding::raw(centroid).normalize()
}
