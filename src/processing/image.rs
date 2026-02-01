//! Image processing and encoding

use anyhow::{Context, Result};
use std::path::Path;

use crate::core::Embedding;
use crate::models::Models;

/// Load and encode image file
pub fn encode(models: &mut Models, path: &Path) -> Result<Embedding> {
	let img = image::open(path).context("Failed to open image")?;
	models.encode_image(&img)
}

/// Encode a DynamicImage (for video frames)
pub fn encode_image(models: &mut Models, img: &image::DynamicImage) -> Result<Embedding> {
	models.encode_image(img)
}