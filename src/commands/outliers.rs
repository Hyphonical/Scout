//! # Outliers Command
//!
//! Find statistically unusual media in the embedding space using
//! Local Outlier Factor (LOF) algorithm.

use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use colored::*;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::core::Embedding;
use crate::storage;
use crate::ui;

#[derive(Debug, Serialize, Deserialize)]
struct OutlierExport {
	total_analyzed: usize,
	outliers: Vec<OutlierInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OutlierInfo {
	path: String,
	score: f32,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
	dir: &Path,
	recursive: bool,
	limit: usize,
	neighbors: usize,
	export: Option<&Path>,
) -> Result<()> {
	let start = Instant::now();

	ui::info(&format!(
		"Analyzing embeddings from {}",
		ui::path_link(dir, 40)
	));

	let (sidecars, hash_cache) = storage::load_all_sidecars(dir, recursive);

	if sidecars.is_empty() {
		ui::warn("No indexed media found. Run 'scout scan' first.");
		return Ok(());
	}

	if sidecars.len() < neighbors + 1 {
		ui::warn(&format!(
			"Not enough media ({}) for outlier detection. Need at least {} files.",
			sidecars.len(),
			neighbors + 1
		));
		return Ok(());
	}

	ui::success(&format!("Loaded {} embeddings", sidecars.len()));
	ui::debug(&format!("Using k={} neighbors for LOF", neighbors));

	// Extract embeddings with their hashes
	let items: Vec<(String, Embedding)> = sidecars
		.iter()
		.map(|(_, s)| (s.hash().to_string(), s.primary_embedding()))
		.collect();

	// Compute LOF scores
	ui::debug("Computing Local Outlier Factor scores...");
	let scores = compute_lof_scores(&items, neighbors);

	// Sort by score (higher = more anomalous)
	let mut scored: Vec<(String, f32)> = items
		.iter()
		.zip(scores.iter())
		.map(|((hash, _), &score)| (hash.clone(), score))
		.collect();

	scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

	// Take top outliers
	let outliers: Vec<(String, f32)> = scored.into_iter().take(limit).collect();

	let duration = start.elapsed();

	// Handle --export flag
	if let Some(export_path) = export {
		let export_data = OutlierExport {
			total_analyzed: items.len(),
			outliers: outliers
				.iter()
				.filter_map(|(hash, score)| {
					hash_cache.get(hash).map(|p| OutlierInfo {
						path: p.to_string_lossy().to_string(),
						score: *score,
					})
				})
				.collect(),
		};

		let json = serde_json::to_string_pretty(&export_data)?;
		if export_path.to_str() == Some("-") || export_path.as_os_str().is_empty() {
			println!("{}", json);
		} else {
			std::fs::write(export_path, json)?;
			ui::success(&format!("Exported to {}", export_path.display()));
		}
		return Ok(());
	}

	// Print results
	ui::header("Outliers");

	// Calculate min/max scores for gradient
	let min_lof = outliers.iter().map(|(_, score)| *score).min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(1.0);
	let max_lof = outliers.iter().map(|(_, score)| *score).max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(5.0);

	for (i, (hash, score)) in outliers.iter().enumerate() {
		if let Some(path) = hash_cache.get(hash) {
			let link = ui::path_link(path, 60);
			let colored_score = ui::log::color_gradient(*score, min_lof, max_lof, false);

			println!(
				"{}. {} LOF: {}",
				format!("{:2}", i + 1).bright_blue().bold(),
				link.bright_white(),
				colored_score
			);
		}
	}

	println!();
	ui::success(&format!(
		"Found {} outliers in {:.1}s",
		outliers.len(),
		duration.as_secs_f32()
	));
	ui::debug("Higher LOF scores indicate more unusual media");

	Ok(())
}

/// Compute Local Outlier Factor scores for all items.
/// Higher scores indicate more anomalous points (> 1.0 = outlier).
fn compute_lof_scores(items: &[(String, Embedding)], k: usize) -> Vec<f32> {
	let embeddings: Vec<&Embedding> = items.iter().map(|(_, e)| e).collect();

	// Compute k-distance and neighbors for each point
	let neighborhoods: Vec<(Vec<usize>, f32)> = embeddings
		.par_iter()
		.enumerate()
		.map(|(i, emb)| {
			let mut distances: Vec<(usize, f32)> = embeddings
				.iter()
				.enumerate()
				.filter(|(j, _)| *j != i)
				.map(|(j, other)| (j, 1.0 - emb.similarity(other))) // cosine distance
				.collect();

			distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
			distances.truncate(k);

			let k_distance = distances.last().map(|(_, d)| *d).unwrap_or(0.0);
			let neighbors: Vec<usize> = distances.iter().map(|(j, _)| *j).collect();

			(neighbors, k_distance)
		})
		.collect();

	// Compute reachability distances and LRD for each point
	let lrds: Vec<f32> = neighborhoods
		.par_iter()
		.enumerate()
		.map(|(i, (neighbors, _))| {
			if neighbors.is_empty() {
				return 1.0;
			}

			let sum_reach_dist: f32 = neighbors
				.iter()
				.map(|&j| {
					let dist_ij = 1.0 - embeddings[i].similarity(embeddings[j]);
					let k_dist_j = neighborhoods[j].1;
					dist_ij.max(k_dist_j) // reachability distance
				})
				.sum();

			if sum_reach_dist > 0.0 {
				neighbors.len() as f32 / sum_reach_dist
			} else {
				1.0
			}
		})
		.collect();

	// Compute LOF for each point
	neighborhoods
		.par_iter()
		.enumerate()
		.map(|(i, (neighbors, _))| {
			if neighbors.is_empty() || lrds[i] == 0.0 {
				return 1.0;
			}

			let sum_lrd_ratio: f32 = neighbors.iter().map(|&j| lrds[j] / lrds[i]).sum();

			sum_lrd_ratio / neighbors.len() as f32
		})
		.collect()
}
