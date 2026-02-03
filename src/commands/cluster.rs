//! Cluster command - group images by visual similarity

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use colored::*;
use serde::{Deserialize, Serialize};

use crate::config::{CLUSTERS_FILE, SIDECAR_DIR};
use crate::core::{ClusterDatabase, ClusterParams};
use crate::processing::cluster::cluster_embeddings;
use crate::storage::index;
use crate::ui;

#[derive(Debug, Serialize, Deserialize)]
struct ClusterExport {
	timestamp: String,
	total_images: usize,
	clusters: Vec<ClusterInfo>,
	noise: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClusterInfo {
	id: usize,
	size: usize,
	cohesion: f32,
	representative: String,
	members: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
	dir: &Path,
	recursive: bool,
	force: bool,
	min_cluster_size: usize,
	min_samples: Option<usize>,
	use_umap: bool,
	preview_count: usize,
	export: Option<&Path>,
) -> Result<()> {
	let clusters_path = dir.join(SIDECAR_DIR).join(CLUSTERS_FILE);

	let start = Instant::now();

	ui::debug(&format!(
		"Starting clustering: dir={}, recursive={}, force={}",
		dir.display(),
		recursive,
		force
	));

	// Check for cached clusters
	if !force {
		if let Some(cached_db) = load_cached_clusters(&clusters_path) {
			ui::debug(&format!(
				"Found cached clusters: {} clusters, {} images",
				cached_db.clusters.len(),
				cached_db.total_images
			));

			// Load sidecars to build hash-to-path lookup
			ui::debug("Loading sidecars for cached cluster display...");
			let (_, hash_to_path) = index::load_all_sidecars(dir, recursive);

			// Handle --export flag
			if let Some(export_path) = export {
				return export_clusters(&cached_db, &hash_to_path, export_path);
			}

			ui::success("Using cached clusters");
			print_clusters(&cached_db, &hash_to_path, preview_count);
			ui::debug(&format!(
				"{}",
				format!("Clustered at: {}", cached_db.timestamp).dimmed()
			));

			ui::debug(&format!(
				"{}",
				"Run with --force to recluster".dimmed()
			));
			return Ok(());
		}
	} else {
		ui::debug("Force flag set, skipping cache check");
	}

	// Load sidecars
	ui::info(&format!(
		"Loading embeddings from {}",
		ui::path_link(dir, 40)
	));

	let (sidecars, hash_to_path) = index::load_all_sidecars(dir, recursive);

	if sidecars.is_empty() {
		ui::warn("No embeddings found. Run 'scout scan' first");
		return Ok(());
	}

	ui::success(&format!("Loaded {} embeddings", sidecars.len()));

	// Log embedding statistics
	if let Some((_, first_sidecar)) = sidecars.first() {
		let emb = first_sidecar.primary_embedding();
		ui::debug(&format!("Embedding dimension: {}D", emb.0.len()));
	}

	let params = ClusterParams {
		min_cluster_size,
		min_samples,
	};

	let cluster_db = cluster_embeddings(sidecars, params, use_umap)?;

	// Log clustering results
	ui::debug(&format!(
		"Found {} clusters and {} noise points",
		cluster_db.clusters.len(),
		cluster_db.noise.len()
	));
	if !cluster_db.clusters.is_empty() {
		let sizes: Vec<usize> = cluster_db
			.clusters
			.iter()
			.map(|c| c.image_hashes.len())
			.collect();
		let avg_size = sizes.iter().sum::<usize>() as f32 / sizes.len() as f32;
		let max_size = sizes.iter().max().unwrap_or(&0);
		let min_size = sizes.iter().min().unwrap_or(&0);
		ui::debug(&format!(
			"Cluster sizes: min={}, max={}, avg={:.1}",
			min_size, max_size, avg_size
		));

		let cohesions: Vec<f32> = cluster_db.clusters.iter().map(|c| c.cohesion).collect();
		let avg_cohesion = cohesions.iter().sum::<f32>() / cohesions.len() as f32;
		ui::debug(&format!(
			"Average cluster cohesion: {:.1}%",
			avg_cohesion * 100.0
		));
	}

	let duration = start.elapsed();

	// Always save clusters
	save_clusters(dir, &cluster_db)?;

	// Handle --export flag
	if let Some(export_path) = export {
		return export_clusters(&cluster_db, &hash_to_path, export_path);
	}

	// Print results
	print_clusters(&cluster_db, &hash_to_path, preview_count);
	eprintln!(
		"\n{}",
		format!("Completed in {:.1}s", duration.as_secs_f32()).dimmed()
	);

	Ok(())
}

fn load_cached_clusters(path: &Path) -> Option<ClusterDatabase> {
	if !path.exists() {
		return None;
	}

	let bytes = fs::read(path).ok()?;
	rmp_serde::from_slice(&bytes).ok()
}

fn print_clusters(
	db: &ClusterDatabase,
	hash_to_path: &HashMap<String, PathBuf>,
	preview_count: usize,
) {
	ui::success(&format!(
		"{} clusters, {} images, {} noise ({:.1}%)",
		db.clusters.len(),
		db.total_images,
		db.noise.len(),
		db.noise_percent()
	));

	for cluster in &db.clusters {
		eprintln!(
			"\n{} {} ({} images, {:.1}% cohesion)",
			"Cluster".bright_white(),
			cluster.id.to_string().bright_cyan(),
			cluster.image_hashes.len(),
			cluster.cohesion * 100.0
		);

		// Show representative
		if let Some(repr_path) = hash_to_path.get(&cluster.representative_hash) {
			eprintln!(
				"  {}: {}",
				"Representative".dimmed(),
				ui::path_link(repr_path, 60).bright_white()
			);
		}

		// Show preview images/videos
		for (i, hash) in cluster.image_hashes.iter().take(preview_count).enumerate() {
			if let Some(path) = hash_to_path.get(hash) {
				eprintln!(
					"  {} {}",
					format!("[{}]", i + 1).dimmed(),
					ui::path_link(path, 60)
				);
			}
		}

		if cluster.image_hashes.len() > preview_count {
			eprintln!(
				"  {}",
				format!(
					"... and {} more",
					cluster.image_hashes.len() - preview_count
				)
				.dimmed()
			);
		}
	}

	if !db.noise.is_empty() {
		eprintln!("\n{} ({} images)", "Noise".bright_yellow(), db.noise.len());
		for hash in db.noise.iter().take(10) {
			if let Some(path) = hash_to_path.get(hash) {
				eprintln!("  {}", ui::path_link(path, 60));
			}
		}
		if db.noise.len() > 10 {
			eprintln!(
				"  {}",
				format!("... and {} more", db.noise.len() - 10).dimmed()
			);
		}
	}
}

fn save_clusters(dir: &Path, db: &ClusterDatabase) -> Result<()> {
	let scout_dir = dir.join(SIDECAR_DIR);
	fs::create_dir_all(&scout_dir)?;

	let clusters_path = scout_dir.join(CLUSTERS_FILE);
	let bytes = rmp_serde::to_vec(db).context("Failed to serialize clusters")?;
	fs::write(&clusters_path, bytes).context("Failed to write clusters file")?;

	ui::success(&format!("Saved clusters to {}", clusters_path.display()));
	Ok(())
}

fn export_clusters(
	db: &ClusterDatabase,
	hash_to_path: &HashMap<String, PathBuf>,
	export_path: &Path,
) -> Result<()> {
	let clusters_info: Vec<ClusterInfo> = db
		.clusters
		.iter()
		.map(|cluster| {
			let representative = hash_to_path
				.get(&cluster.representative_hash)
				.map(|p| p.to_string_lossy().to_string())
				.unwrap_or_else(|| cluster.representative_hash.clone());

			let members: Vec<String> = cluster
				.image_hashes
				.iter()
				.filter_map(|hash| {
					hash_to_path
						.get(hash)
						.map(|p| p.to_string_lossy().to_string())
				})
				.collect();

			ClusterInfo {
				id: cluster.id,
				size: cluster.image_hashes.len(),
				cohesion: cluster.cohesion,
				representative,
				members,
			}
		})
		.collect();

	let noise: Vec<String> = db
		.noise
		.iter()
		.filter_map(|hash| {
			hash_to_path
				.get(hash)
				.map(|p| p.to_string_lossy().to_string())
		})
		.collect();

	let export_data = ClusterExport {
		timestamp: db.timestamp.clone(),
		total_images: db.total_images,
		clusters: clusters_info,
		noise,
	};

	let json = serde_json::to_string_pretty(&export_data)?;

	if export_path.to_str() == Some("-") || export_path.as_os_str().is_empty() {
		// Output to stdout
		println!("{}", json);
	} else {
		// Write to file
		std::fs::write(export_path, json)?;
		ui::success(&format!("Exported to {}", export_path.display()));
	}

	Ok(())
}
