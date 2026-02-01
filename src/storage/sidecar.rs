//! Sidecar file format and I/O

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{SIDECAR_DIR, SIDECAR_EXT};
use crate::core::{Embedding, FileHash};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageSidecar {
	version: String,
	filename: String,
	hash: String,
	embedding: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoSidecar {
	version: String,
	filename: String,
	hash: String,
	frames: Vec<VideoFrame>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoFrame {
	pub timestamp: f64,
	pub embedding: Vec<f32>,
}

#[derive(Debug)]
pub enum Sidecar {
	Image(ImageSidecar),
	Video(VideoSidecar),
}

impl ImageSidecar {
	pub fn new(filename: String, hash: FileHash, embedding: Embedding) -> Self {
		Self {
			version: VERSION.to_string(),
			filename,
			hash: hash.as_str().to_string(),
			embedding: embedding.as_slice().to_vec(),
		}
	}
	
	pub fn embedding(&self) -> Embedding {
		Embedding::raw(self.embedding.clone())
	}
	
	pub fn filename(&self) -> &str {
		&self.filename
	}
	
	pub fn is_current_version(&self) -> bool {
		self.version == VERSION
	}
}

impl VideoSidecar {
	pub fn new(filename: String, hash: FileHash, frames: Vec<(f64, Embedding)>) -> Self {
		Self {
			version: VERSION.to_string(),
			filename,
			hash: hash.as_str().to_string(),
			frames: frames.into_iter().map(|(ts, emb)| VideoFrame {
				timestamp: ts,
				embedding: emb.as_slice().to_vec(),
			}).collect(),
		}
	}
	
	pub fn frames(&self) -> Vec<(f64, Embedding)> {
		self.frames.iter()
			.map(|f| (f.timestamp, Embedding::raw(f.embedding.clone())))
			.collect()
	}
	
	pub fn filename(&self) -> &str {
		&self.filename
	}
	
	pub fn is_current_version(&self) -> bool {
		self.version == VERSION
	}
}

impl Sidecar {
	pub fn filename(&self) -> &str {
		match self {
			Sidecar::Image(img) => img.filename(),
			Sidecar::Video(vid) => vid.filename(),
		}
	}
	
	pub fn is_current_version(&self) -> bool {
		match self {
			Sidecar::Image(img) => img.is_current_version(),
			Sidecar::Video(vid) => vid.is_current_version(),
		}
	}
}

/// Save image sidecar
pub fn save_image(sidecar: &ImageSidecar, media_dir: &Path, hash: &FileHash) -> Result<()> {
	let path = build_path(media_dir, hash);
	ensure_dir(&path)?;
	let bytes = rmp_serde::to_vec(sidecar).context("Serialize failed")?;
	fs::write(&path, bytes).context("Write failed")?;
	Ok(())
}

/// Save video sidecar
pub fn save_video(sidecar: &VideoSidecar, media_dir: &Path, hash: &FileHash) -> Result<()> {
	let path = build_path(media_dir, hash);
	ensure_dir(&path)?;
	let bytes = rmp_serde::to_vec(sidecar).context("Serialize failed")?;
	fs::write(&path, bytes).context("Write failed")?;
	Ok(())
}

/// Load sidecar (auto-detect type)
pub fn load(path: &Path) -> Result<Sidecar> {
	let bytes = fs::read(path).context("Read failed")?;
	
	// Try video first
	if let Ok(video) = rmp_serde::from_slice::<VideoSidecar>(&bytes) {
		return Ok(Sidecar::Video(video));
	}
	
	// Fall back to image
	let image = rmp_serde::from_slice::<ImageSidecar>(&bytes)
		.context("Deserialize failed")?;
	Ok(Sidecar::Image(image))
}

pub fn build_path(media_dir: &Path, hash: &FileHash) -> PathBuf {
	media_dir.join(SIDECAR_DIR).join(format!("{}.{}", hash.as_str(), SIDECAR_EXT))
}

fn ensure_dir(path: &Path) -> Result<()> {
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent).context("Failed to create .scout directory")?;
	}
	Ok(())
}
