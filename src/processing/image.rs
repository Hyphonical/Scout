//! Image processing and encoding

use anyhow::{Context, Result};
use std::path::Path;

use crate::core::Embedding;
use crate::models::Models;

/// Load and encode image file
pub fn encode(models: &mut Models, path: &Path) -> Result<Embedding> {
	crate::ui::debug(&format!("Encoding image: {}", path.display()));
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
