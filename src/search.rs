//! Semantic image search functionality
//!
//! Performs similarity search across indexed images using text queries,
//! reference images, or weighted combinations of both.

use std::path::Path;
#[cfg(feature = "video")]
use std::path::PathBuf;

use crate::logger::{log, Level};
use crate::models::ModelManager;
use crate::sidecar::{current_version, iter_sidecars, Sidecar};
use crate::types::{CombineWeight, Embedding, SearchMatch};

#[cfg(feature = "video")]
use crate::types::MediaType;

/// Search query variants supporting different search modes
pub enum SearchQuery<'a> {
	Text(&'a str),
	Image(&'a Path),
	Combined { text: &'a str, image: &'a Path, weight: CombineWeight },
}

impl<'a> SearchQuery<'a> {
	/// Builds the query embedding using appropriate model(s)
	fn build_embedding(&self, models: &mut ModelManager) -> Option<Embedding> {
		match self {
			SearchQuery::Text(text) => {
				log(Level::Debug, &format!("Encoding query: {}", text));
				models.encode_text(text).ok()
			}
			SearchQuery::Image(path) => {
				log(Level::Debug, &format!("Encoding reference: {}", path.display()));
				models.encode_image(path).ok().map(|(emb, _)| emb)
			}
			SearchQuery::Combined { text, image, weight } => {
				log(Level::Debug, &format!("Encoding combined query (weight: {:.2})", weight.value()));

				let text_emb = models.encode_text(text).ok();
				let image_emb = models.encode_image(image).ok().map(|(emb, _)| emb);

				Embedding::combine(
					text_emb.as_ref(),
					image_emb.as_ref(),
					weight.value(),
				)
			}
		}
	}
}

/// Searches indexed images for semantic matches
///
/// # Arguments
/// * `root` - Root directory containing indexed images
/// * `query` - Search query (text, image, or combined)
/// * `min_score` - Minimum similarity threshold [0.0, 1.0]
/// * `exclude_path` - Optional path to exclude from results (e.g., reference image)
/// * `recursive` - Whether to search subdirectories
///
/// Returns matches sorted by descending similarity score
pub fn search(
	root: &Path,
	query: SearchQuery,
	min_score: f32,
	exclude_path: Option<&Path>,
	recursive: bool,
) -> Vec<SearchMatch> {
	let mut models = ModelManager::new();

	let query_emb = match query.build_embedding(&mut models) {
		Some(emb) => emb,
		None => {
			log(Level::Error, "Failed to generate query embedding");
			return Vec::new();
		}
	};

	let exclude_canonical = exclude_path.and_then(|p| p.canonicalize().ok());
	let mut results = Vec::new();
	#[cfg(feature = "video")]
	let mut video_best: std::collections::HashMap<PathBuf, (f32, f64)> = std::collections::HashMap::new();
	let mut outdated = 0;

	for (sidecar_path, base_dir) in iter_sidecars(root, recursive) {
		let Ok(sidecar) = Sidecar::load_auto(&sidecar_path) else { continue };

		if !sidecar.is_current_version() {
			outdated += 1;
		}

		let source_path = base_dir.join(sidecar.filename());

		if let Some(ref exclude) = exclude_canonical {
			if let Ok(canonical) = source_path.canonicalize() {
				if &canonical == exclude {
					continue;
				}
			}
		}

		match sidecar {
			Sidecar::Image(img) => {
				let score = query_emb.similarity(&img.embedding());
				if score >= min_score {
					results.push(SearchMatch::new(source_path, score));
				}
			}
			#[cfg(feature = "video")]
			Sidecar::Video(vid) => {
				// Find best matching frame in video
				let mut best_score = 0.0;
				let mut best_timestamp = 0.0;
				
				for (timestamp, frame_emb) in vid.frames() {
					let score = query_emb.similarity(&frame_emb);
					if score > best_score {
						best_score = score;
						best_timestamp = timestamp;
					}
				}

				if best_score >= min_score {
					// Track only the best match for each video
					let canonical = source_path.canonicalize().unwrap_or_else(|_| source_path.clone());
					video_best.entry(canonical)
						.and_modify(|(s, ts)| {
							if best_score > *s {
								*s = best_score;
								*ts = best_timestamp;
							}
						})
						.or_insert((best_score, best_timestamp));
				}
			}
		}
	}

	// Add video results (one per video)
	#[cfg(feature = "video")]
	for (path, (score, timestamp)) in video_best {
		results.push(SearchMatch::new_video(path, score, timestamp));
	}

	if outdated > 0 {
		log(
			Level::Warning,
			&format!(
				"{} outdated sidecars found. Run 'scout scan -f' to upgrade to v{}",
				outdated, current_version()
			),
		);
	}

	results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
	results
}
