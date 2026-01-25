// Search - Semantic image search using SigLIP2 embeddings

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::SIDECAR_DIR;
use crate::embedder::{cosine_similarity, TextEncoder};
use crate::sidecar::{iter_sidecars, ImageSidecar};
use crate::logger::{log, Level};

pub struct SearchResult {
	pub path: PathBuf,
	pub score: f32,
}

pub fn search_images(root: &Path, query: &str, min_score: f32) -> Vec<SearchResult> {
	let scout_dir = root.join(SIDECAR_DIR);
	if !scout_dir.exists() {
		return Vec::new();
	}

	let encoder = match TextEncoder::new() {
			Ok(e) => e,
			Err(e) => {
				log(Level::Error, &format!("Failed to load text model: {:?}", e));
				return Vec::new();
			}
		};

	let query_embedding = match encoder.embed(query) {
			Ok(e) => e,
			Err(e) => {
				log(Level::Error, &format!("Failed to embed query: {:?}", e));
				return Vec::new();
			}
		};

	let mut results = Vec::new();

	for sidecar_path in iter_sidecars(root) {
		let Ok(content) = fs::read_to_string(&sidecar_path) else { continue };
		let Ok(sidecar) = serde_json::from_str::<ImageSidecar>(&content) else { continue };

		// For normalized embeddings, cosine similarity is just the dot product
		// Returns values in range [-1, 1], where 1 = identical, 0 = orthogonal, -1 = opposite
		let score = cosine_similarity(&query_embedding, &sidecar.embedding);

		// Only keep results above the minimum score threshold
		// Note: min_score should typically be set between 0.0-1.0 for best results
		if score >= min_score {
			results.push(SearchResult {
				path: PathBuf::from(&sidecar.source),
				score,
			});
		}
	}

	results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
	results	
}