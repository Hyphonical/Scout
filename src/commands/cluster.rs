//! Cluster command - group images by visual similarity

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use colored::*;

use crate::cli::OutputFormat;
use crate::core::{ClusterDatabase, ClusterParams};
use crate::processing::cluster::cluster_embeddings;
use crate::storage::index;
use crate::ui;

pub fn run(
	dir: &Path,
	recursive: bool,
	min_cluster_size: usize,
	min_samples: Option<usize>,
	output_format: OutputFormat,
	save: bool,
) -> Result<()> {
	ui::info(&format!("Loading embeddings from {}", ui::path_link(dir, 40)));

	let sidecars = index::load_all_sidecars(dir, recursive);

	if sidecars.is_empty() {
		ui::warn("No embeddings found. Run 'scout scan' first");
		return Ok(());
	}

	ui::success(&format!("Loaded {} embeddings", sidecars.len()));

	// Build hash-to-path lookup using find_file_by_hash
	let hash_to_path: HashMap<String, PathBuf> = sidecars
		.iter()
		.filter_map(|(sidecar_path, sidecar)| {
			let media_dir = sidecar_path.parent()?.parent()?;
			let hash = sidecar.hash().to_string();
			index::find_file_by_hash(media_dir, &hash).map(|path| (hash, path))
		})
		.collect();

	let params = ClusterParams {
		min_cluster_size,
		min_samples,
	};

	let start = Instant::now();
	let cluster_db = cluster_embeddings(sidecars, params)?;
	let duration = start.elapsed();

	match output_format {
		OutputFormat::Human => {
			print_human_output(&cluster_db, &hash_to_path);
			println!(
				"\n{}",
				format!("Completed in {:.1}s", duration.as_secs_f32()).dimmed()
			);
		}
		OutputFormat::Json => print_json_output(&cluster_db),
		OutputFormat::Csv => print_csv_output(&cluster_db, &hash_to_path),
	}

	if save {
		save_clusters(dir, &cluster_db)?;
	}

	Ok(())
}

fn print_human_output(db: &ClusterDatabase, hash_to_path: &HashMap<String, PathBuf>) {
	ui::success(&format!(
		"{} clusters, {} images, {} noise ({:.1}%)",
		db.clusters.len(),
		db.total_images,
		db.noise.len(),
		db.noise_percent()
	));

	for cluster in &db.clusters {
		println!(
			"\n{} {} ({} images, {:.1}% cohesion)",
			"Cluster".bright_white(),
			cluster.id.to_string().bright_cyan(),
			cluster.image_hashes.len(),
			cluster.cohesion * 100.0
		);

		// Show representative
		if let Some(repr_path) = hash_to_path.get(&cluster.representative_hash) {
			println!(
				"  {}: {}",
				"Representative".dimmed(),
				ui::path_link(repr_path, 60).bright_white()
			);
		}

		// Show first 5 images
		for (i, hash) in cluster.image_hashes.iter().take(5).enumerate() {
			if let Some(path) = hash_to_path.get(hash) {
				println!(
					"  {} {}",
					format!("[{}]", i + 1).dimmed(),
					ui::path_link(path, 60)
				);
			}
		}

		if cluster.image_hashes.len() > 5 {
			println!(
				"  {}",
				format!("... and {} more", cluster.image_hashes.len() - 5).dimmed()
			);
		}
	}

	if !db.noise.is_empty() {
		println!(
			"\n{} ({} images)",
			"Noise".bright_yellow(),
			db.noise.len()
		);
		for hash in db.noise.iter().take(10) {
			if let Some(path) = hash_to_path.get(hash) {
				println!("  {}", ui::path_link(path, 60));
			}
		}
		if db.noise.len() > 10 {
			println!("  {}", format!("... and {} more", db.noise.len() - 10).dimmed());
		}
	}
}

fn print_json_output(db: &ClusterDatabase) {
	let json = serde_json::to_string_pretty(db).expect("Failed to serialize");
	println!("{}", json);
}

fn print_csv_output(db: &ClusterDatabase, hash_to_path: &HashMap<String, PathBuf>) {
	println!("image_path,cluster_id,is_representative,is_noise");

	for cluster in &db.clusters {
		for hash in &cluster.image_hashes {
			if let Some(path) = hash_to_path.get(hash) {
				let is_repr = hash == &cluster.representative_hash;
				println!(
					"{},{},{},false",
					path.display(),
					cluster.id,
					is_repr
				);
			}
		}
	}

	for hash in &db.noise {
		if let Some(path) = hash_to_path.get(hash) {
			println!("{},-1,false,true", path.display());
		}
	}
}

fn save_clusters(dir: &Path, db: &ClusterDatabase) -> Result<()> {
	let scout_dir = dir.join(".scout");
	fs::create_dir_all(&scout_dir)?;

	let clusters_path = scout_dir.join("clusters.msgpack");
	let bytes = rmp_serde::to_vec(db).context("Failed to serialize clusters")?;
	fs::write(&clusters_path, bytes).context("Failed to write clusters file")?;

	ui::success(&format!("Saved clusters to {}", clusters_path.display()));
	Ok(())
}
