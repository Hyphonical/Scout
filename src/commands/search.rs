//! Search command - find similar media

use anyhow::{anyhow, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::config::NEGATIVE_WEIGHT;
use crate::core::Embedding;
use crate::models::Models;
use crate::storage;
use crate::ui;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
	pub path: String,
	pub score: f32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timestamp: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchExport {
	query: String,
	results: Vec<Match>,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
	query_text: Option<&str>,
	query_image: Option<&Path>,
	weight: f32,
	negative: Option<&str>,
	dir: &Path,
	recursive: bool,
	limit: usize,
	min_score: f32,
	open_first: bool,
	include_ref: bool,
	exclude_videos: bool,
	paths_only: bool,
	export: Option<&Path>,
) -> Result<()> {
	let search_start = std::time::Instant::now();

	// Build query embedding
	let mut models = Models::new()?;

	let query_emb = match (query_text, query_image) {
		(Some(text), None) => {
			ui::info(&format!("Searching for: \"{}\"", text));
			models.encode_text(text)?
		}
		(None, Some(img_path)) => {
			ui::info(&format!("Searching by image: {}", img_path.display()));
			let img = image::open(img_path)?;
			models.encode_image(&img)?
		}
		(Some(text), Some(img_path)) => {
			let filename = img_path
				.file_name()
				.and_then(|n| n.to_str())
				.unwrap_or("image");
			ui::info(&format!(
				"Combined search: \"{}\" + {} (weight: {:.2})",
				text, filename, weight
			));
			let text_emb = models.encode_text(text)?;
			let img = image::open(img_path)?;
			let img_emb = models.encode_image(&img)?;
			Embedding::blend(&text_emb, &img_emb, weight)
		}
		(None, None) => {
			return Err(anyhow!("Must provide either query text or --image"));
		}
	};

	// Build negative embedding if provided
	let negative_emb = if let Some(neg) = negative {
		ui::debug(&format!("Negative prompt: \"{}\"", neg));
		Some(models.encode_text(neg)?)
	} else {
		None
	};

	ui::info(&format!(
		"Loading embeddings from {}",
		ui::path_link(dir, 40)
	));
	let (sidecars, hash_cache) = storage::load_all_sidecars(dir, recursive);

	if sidecars.is_empty() {
		ui::warn("No indexed images found. Run 'scout scan' first.");
		return Ok(());
	}

	ui::success(&format!("Loaded {} embeddings", sidecars.len()));

	let mut matches = Vec::new();

	for (_path, sidecar) in sidecars {
		let hash = sidecar.hash().to_string();

		match sidecar {
			storage::Sidecar::Image(img) => {
				let mut score = query_emb.similarity(&img.embedding());

				if let Some(ref neg_emb) = negative_emb {
					let neg_score = neg_emb.similarity(&img.embedding());
					score -= neg_score * NEGATIVE_WEIGHT;
				}

				if score >= min_score {
					if let Some(image_path) = hash_cache.get(&hash) {
						matches.push(Match {
							path: image_path.to_string_lossy().to_string(),
							score,
							timestamp: None,
							hash: Some(hash.clone()),
						});
					}
				}
			}
			storage::Sidecar::Video(vid) => {
				if exclude_videos {
					continue;
				}

				// Find best frame
				let mut best_score = 0.0;
				let mut best_timestamp = 0.0;

				for (timestamp, frame_emb) in vid.frames() {
					let mut score = query_emb.similarity(&frame_emb);

					if let Some(ref neg_emb) = negative_emb {
						let neg_score = neg_emb.similarity(&frame_emb);
						score -= neg_score * NEGATIVE_WEIGHT;
					}

					if score > best_score {
						best_score = score;
						best_timestamp = timestamp;
					}
				}

				if best_score >= min_score {
					if let Some(video_path) = hash_cache.get(&hash) {
						matches.push(Match {
							path: video_path.to_string_lossy().to_string(),
							score: best_score, // Clamp back to 0.0 for display
							timestamp: Some(best_timestamp),
							hash: Some(hash.clone()),
						});
					}
				}
			}
		}
	}

	// Filter out reference image if not including it
	if !include_ref {
		if let Some(ref_path) = query_image {
			if let Ok(canonical_ref) = ref_path.canonicalize() {
				let canonical_ref_str = canonical_ref.to_string_lossy().to_string();
				matches.retain(|m| {
					if let Ok(canonical_match) = Path::new(&m.path).canonicalize() {
						canonical_match.to_string_lossy() != canonical_ref_str
					} else {
						true
					}
				});
			}
		}
	}

	matches.sort_by(|a, b| {
		b.score
			.partial_cmp(&a.score)
			.unwrap_or(std::cmp::Ordering::Equal)
	});
	matches.truncate(limit);

	if matches.is_empty() {
		ui::warn("No matches found");
		return Ok(());
	}

	// Build query string for export
	let query_string = match (query_text, query_image) {
		(Some(text), None) => text.to_string(),
		(None, Some(img_path)) => format!("image:{}", img_path.display()),
		(Some(text), Some(img_path)) => format!("{} + image:{}", text, img_path.display()),
		(None, None) => String::new(),
	};

	// Handle --export flag
	if let Some(export_path) = export {
		let export_data = SearchExport {
			query: query_string,
			results: matches.clone(),
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
		return Ok(());
	}

	// Handle --paths flag
	if paths_only {
		// Output paths to stdout, all logging already went to stderr
		for m in &matches {
			println!("{}", m.path);
		}
		return Ok(());
	}

	// Normal interactive output
	ui::header("Results");

	for (i, m) in matches.iter().enumerate() {
		let path = Path::new(&m.path);

		let link = ui::log::path_link(path, 60);
		let percentage = (m.score * 100.0).round() as u32;

		let location_str = if let Some(ts) = m.timestamp {
			format!(
				" @ {}",
				crate::processing::video::format_timestamp(ts).bright_yellow()
			)
		} else {
			String::new()
		};

		println!(
			"{}. {}{} {} {}",
			format!("{:2}", i + 1).bright_blue().bold(),
			link.bright_white(),
			location_str.dimmed(),
			format!("{}%", percentage).dimmed(),
			if m.score > 0.8 { "🔥" } else { "" }
		);
	}

	let search_duration = search_start.elapsed().as_millis() as f32;

	println!();

	// Low score warning
	if !matches.is_empty() && matches[0].score < 0.10 {
		ui::warn("Top result has low similarity (<10%)");
		println!();
		println!("  {} Try these techniques:", "💡".bright_blue().bold());
		println!("     • Add more descriptive details");
		println!("     • Use full sentences: \"Woman with red hair sitting on bench\"");
		println!("     • Prefix with \"Image of...\" or \"Photo of...\"");
		println!("     • Add a reference image with --image and low --weight");
		println!();
	}

	ui::success(&format!(
		"Found {} matches in {:.0}ms",
		matches.len(),
		search_duration
	));

	if open_first && !matches.is_empty() {
		if let Err(e) = open::that(&matches[0].path) {
			ui::warn(&format!("Failed to open: {}", e));
		}
	}

	Ok(())
}
