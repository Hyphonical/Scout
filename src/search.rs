// Search - Semantic image search using embeddings

use std::path::{Path, PathBuf};

use crate::embedder::TextEncoder;
use crate::embedding::{combine_embeddings, cosine_similarity};
use crate::logger::{log, Level};
use crate::processor::VisionEncoder;
use crate::sidecar::{current_version, iter_sidecars, ImageSidecar};

pub struct SearchResult {
	pub path: PathBuf,
	pub score: f32,
}

pub struct SearchQuery<'a> {
	pub text: Option<&'a str>,
	pub image: Option<&'a Path>,
	pub weight: f32, // 0.0 = image only, 1.0 = text only
}

impl<'a> SearchQuery<'a> {
	pub fn text_only(query: &'a str) -> Self {
		Self { text: Some(query), image: None, weight: 1.0 }
	}

	pub fn image_only(path: &'a Path) -> Self {
		Self { text: None, image: Some(path), weight: 0.0 }
	}

	pub fn combined(text: &'a str, image: &'a Path, weight: f32) -> Self {
		Self { text: Some(text), image: Some(image), weight }
	}
}

pub fn search_with_query(
	root: &Path,
	query: SearchQuery,
	min_score: f32,
	exclude_path: Option<&Path>,
	recursive: bool,
) -> Vec<SearchResult> {
	// Build query embedding based on what's provided
	let mut text_embedding: Option<Vec<f32>> = None;
	let mut image_embedding: Option<Vec<f32>> = None;

	// Process image first if provided (larger model, load/unload separately)
	if let Some(image_path) = query.image {
		log(Level::Debug, &format!("Encoding reference image: {}", image_path.display()));
		match VisionEncoder::new() {
			Ok(encoder) => {
				match encoder.process_image(image_path) {
					Ok(result) => {
						image_embedding = Some(result.embedding);
						log(Level::Debug, "Reference image encoded successfully");
					}
					Err(e) => {
						log(Level::Error, &format!("Failed to process reference image: {:?}", e));
					}
				}
			}
			Err(e) => {
				log(Level::Error, &format!("Failed to load vision model: {:?}", e));
			}
		}
		// VisionEncoder dropped here, freeing memory before loading text model
	}

	// Process text if provided
	if let Some(text) = query.text {
		log(Level::Debug, &format!("Encoding text query: {}", text));
		match TextEncoder::new() {
			Ok(encoder) => {
				match encoder.embed(text) {
					Ok(emb) => {
						text_embedding = Some(emb);
						log(Level::Debug, "Text query encoded successfully");
					}
					Err(e) => {
						log(Level::Error, &format!("Failed to embed text: {:?}", e));
					}
				}
			}
			Err(e) => {
				log(Level::Error, &format!("Failed to load text model: {:?}", e));
			}
		}
		// TextEncoder dropped here
	}

	// Combine embeddings with weight
	let query_embedding = match combine_embeddings(
		text_embedding.as_deref(),
		image_embedding.as_deref(),
		query.weight,
	) {
		Some(emb) => emb,
		None => {
			log(Level::Error, "No valid embedding could be generated");
			return Vec::new();
		}
	};

	// Canonicalize exclude path for comparison
	let exclude_canonical = exclude_path.and_then(|p| p.canonicalize().ok());

	// Search through sidecars
	let mut results = Vec::new();
	let mut outdated_count = 0;

	for (sidecar_path, base_dir) in iter_sidecars(root, recursive) {
		let Ok(sidecar) = ImageSidecar::load(&sidecar_path) else { continue };

		if !sidecar.is_current_version() {
			outdated_count += 1;
		}

		// Reconstruct full path from base_dir + filename
		let source_path = base_dir.join(&sidecar.filename);

		// Skip the reference image if exclude_path is set
		if let Some(ref exclude) = exclude_canonical {
			if let Ok(canonical) = source_path.canonicalize() {
				if &canonical == exclude {
					continue;
				}
			}
		}

		let score = cosine_similarity(&query_embedding, &sidecar.embedding);

		if score >= min_score {
			results.push(SearchResult { path: source_path, score });
		}
	}

	if outdated_count > 0 {
		log(
			Level::Warning,
			&format!(
				"{} sidecars were created with an older version. Run 'scout scan -f' to upgrade to v{}",
				outdated_count,
				current_version()
			),
		);
	}

	results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
	results
}