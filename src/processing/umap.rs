//! UMAP dimensionality reduction for faster clustering

use anyhow::Result;
use ndarray::Array2;
use rayon::prelude::*;

use crate::core::Embedding;
use crate::ui;

/// Reduce embeddings from 1024D to lower dimensions using UMAP
pub fn reduce_embeddings(
	embeddings: &[Embedding],
	n_components: usize,
	n_neighbors: usize,
) -> Result<Vec<Vec<f32>>> {
	let n_samples = embeddings.len();
	let n_features = embeddings[0].0.len();

	ui::info(&format!(
		"Reducing {}D to {}D using UMAP",
		n_features, n_components
	));
	ui::debug(&format!("UMAP neighbors: {}", n_neighbors));

	// Convert embeddings to ndarray format (umap-rs uses f32)
	let mut data = Array2::<f32>::zeros((n_samples, n_features));
	for (i, emb) in embeddings.iter().enumerate() {
		for (j, &val) in emb.0.iter().enumerate() {
			data[[i, j]] = val;
		}
	}

	// Compute K-nearest neighbors
	ui::debug("Computing K-nearest neighbors...");
	let (knn_indices, knn_distances) = compute_knn(embeddings, n_neighbors)?;

	// Convert to ndarray format (umap-rs uses u32 for indices)
	let mut knn_indices_array = Array2::<u32>::zeros((n_samples, n_neighbors));
	let mut knn_dists_array = Array2::<f32>::zeros((n_samples, n_neighbors));

	for i in 0..n_samples {
		for j in 0..n_neighbors {
			knn_indices_array[[i, j]] = knn_indices[i][j] as u32;
			knn_dists_array[[i, j]] = knn_distances[i][j];
		}
	}

	// Initialize embedding with random values
	let init = initialize_embedding(n_samples, n_components);

	// Configure and run UMAP
	ui::debug("Running UMAP optimization...");

	let config = umap_rs::UmapConfig {
		n_components,
		graph: umap_rs::GraphParams {
			n_neighbors,
			..Default::default()
		},
		..Default::default()
	};

	let umap = umap_rs::Umap::new(config);

	let fitted_model = umap.fit(
		data.view(),
		knn_indices_array.view(),
		knn_dists_array.view(),
		init.view(),
	);

	let embedding = fitted_model.embedding();

	// Convert back to Vec<Vec<f32>>
	let result: Vec<Vec<f32>> = (0..n_samples)
		.map(|i| (0..n_components).map(|j| embedding[[i, j]]).collect())
		.collect();

	ui::success("UMAP reduction complete");

	Ok(result)
}

type KnnResult = (Vec<Vec<usize>>, Vec<Vec<f32>>);

/// Compute K-nearest neighbors using brute force (accurate for high dimensions)
fn compute_knn(embeddings: &[Embedding], k: usize) -> Result<KnnResult> {
	let n_samples = embeddings.len();

	ui::debug(&format!(
		"Computing KNN (brute force) for {} samples, k={}",
		n_samples, k
	));

	// Parallel computation of nearest neighbors
	let results: Vec<(Vec<usize>, Vec<f32>)> = (0..n_samples)
		.into_par_iter()
		.map(|i| {
			// Compute distances to all other points
			let mut distances: Vec<(usize, f32)> = (0..n_samples)
				.filter(|&j| i != j)
				.map(|j| (j, embeddings[i].distance(&embeddings[j])))
				.collect();

			// Sort by distance and take k nearest
			distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
			distances.truncate(k);

			let indices: Vec<usize> = distances.iter().map(|(idx, _)| *idx).collect();
			let dists: Vec<f32> = distances.iter().map(|(_, d)| *d).collect();

			(indices, dists)
		})
		.collect();

	let knn_indices = results.iter().map(|(idx, _)| idx.clone()).collect();
	let knn_distances = results.iter().map(|(_, dist)| dist.clone()).collect();

	Ok((knn_indices, knn_distances))
}

/// Initialize embedding with random values in range [-10, 10]
fn initialize_embedding(n_samples: usize, n_components: usize) -> Array2<f32> {
	use rand::Rng;
	let mut rng = rand::rng();

	let mut init = Array2::<f32>::zeros((n_samples, n_components));
	for i in 0..n_samples {
		for j in 0..n_components {
			init[[i, j]] = rng.random_range(-10.0f32..10.0f32);
		}
	}

	init
}
