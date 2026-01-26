// Search - Semantic image search

use std::path::Path;

use crate::logger::{log, Level};
use crate::models::ModelManager;
use crate::sidecar::{current_version, iter_sidecars, ImageSidecar};
use crate::types::{CombineWeight, Embedding, SearchMatch};

pub enum SearchQuery<'a> {
	Text(&'a str),
	Image(&'a Path),
	Combined { text: &'a str, image: &'a Path, weight: CombineWeight },
}

impl<'a> SearchQuery<'a> {
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
	let mut outdated = 0;

	for (sidecar_path, base_dir) in iter_sidecars(root, recursive) {
		let Ok(sidecar) = ImageSidecar::load(&sidecar_path) else { continue };

		if !sidecar.is_current_version() {
			outdated += 1;
		}

		let source_path = base_dir.join(&sidecar.filename);

		if let Some(ref exclude) = exclude_canonical {
			if let Ok(canonical) = source_path.canonicalize() {
				if &canonical == exclude {
					continue;
				}
			}
		}

		let score = query_emb.similarity(&sidecar.embedding());

		if score >= min_score {
			results.push(SearchMatch::new(source_path, score));
		}
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
