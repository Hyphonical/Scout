//! # Image Processing
//!
//! Load and encode images, with format detection fallback.
//! Handles mismatched extensions and corrupted files gracefully.

use anyhow::{Context, Result};
use std::path::Path;

use crate::core::Embedding;
use crate::models::Models;

/// Load and encode image file
pub fn encode(models: &mut Models, path: &Path) -> Result<Embedding> {
	crate::ui::debug(&format!("Encoding image: {}", path.display()));

	// Try to open with default extension-based detection first
	if let Ok(img) = image::open(path) {
		return models.encode_image(&img);
	}

	// If that fails, check actual file format and try again
	if let Ok(bytes) = std::fs::read(path) {
		if let Ok(detected_format) = image::guess_format(&bytes) {
			let extension = path
				.extension()
				.and_then(|ext| ext.to_str())
				.unwrap_or("unknown")
				.to_lowercase();

			let detected_ext = format!("{:?}", detected_format).to_lowercase();

			if !extension.is_empty() && !detected_ext.contains(&extension) {
				let link = crate::ui::path_link(path, 60);
				crate::ui::warn(&format!(
					"File extension does not match content: {} (expected {:?}, detected {:?})",
					link, extension, detected_format
				));

				// Try to decode with the detected format
				if let Ok(img) = image::load_from_memory_with_format(&bytes, detected_format) {
					return models.encode_image(&img);
				}
			}
		}
	}

	// If all else fails, return the original error
	let img = image::open(path).with_context(|| {
		format!(
			"Failed to open image. File may be corrupted or in an unsupported format: {}",
			path.display()
		)
	})?;
	models.encode_image(&img)
}

/// Encode a DynamicImage (for video frames)
pub fn encode_image(models: &mut Models, img: &image::DynamicImage) -> Result<Embedding> {
	models.encode_image(img)
}
