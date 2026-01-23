// Stats - Aggregate tag statistics across all indexed images

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::sidecar::{iter_sidecars, ImageSidecar};

pub struct TagStats {
	pub name: String,
	pub count: usize,
	pub avg_confidence: f32,
}

pub struct StatsResult {
	pub total_images: usize,
	pub total_tags: usize,
	pub unique_tags: usize,
	pub top_tags: Vec<TagStats>,
}

/// Calculates tag statistics across all indexed images.
pub fn calculate_stats(root: &Path, limit: usize) -> StatsResult {
	let mut tag_counts: HashMap<String, (usize, f32)> = HashMap::new();
	let mut total_images = 0;
	let mut total_tags = 0;

	for path in iter_sidecars(root) {
		let Ok(content) = fs::read_to_string(&path) else { continue };
		let Ok(sidecar) = serde_json::from_str::<ImageSidecar>(&content) else { continue };

		total_images += 1;
		total_tags += sidecar.tags.len();

		for tag in sidecar.tags {
			let entry = tag_counts.entry(tag.name).or_insert((0, 0.0));
			entry.0 += 1;
			entry.1 += tag.confidence;
		}
	}

	let unique_tags = tag_counts.len();

	let mut sorted: Vec<_> = tag_counts.into_iter()
		.map(|(name, (count, sum))| TagStats {
			name,
			count,
			avg_confidence: sum / count as f32,
		})
		.collect();

	sorted.sort_by(|a, b| b.count.cmp(&a.count));
	sorted.truncate(limit);

	StatsResult {
		total_images,
		total_tags,
		unique_tags,
		top_tags: sorted,
	}
}
