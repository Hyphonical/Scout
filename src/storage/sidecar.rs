//! Sidecar file format and I/O

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{SIDECAR_DIR, SIDECAR_EXT};
use crate::core::{Embedding, FileHash};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Serialize, Deserialize)]
pub struct Sidecar {
	version: String,
	filename: String,
	hash: String,
	embedding: Vec<f32>,
}

impl Sidecar {
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

/// Save sidecar to disk
pub fn save(sidecar: &Sidecar, media_dir: &Path, hash: &FileHash) -> Result<()> {
	let sidecar_path = build_path(media_dir, hash);

	if let Some(parent) = sidecar_path.parent() {
		fs::create_dir_all(parent).context("Failed to create .scout directory")?;
	}

	let bytes = rmp_serde::to_vec(sidecar).context("Failed to serialize sidecar")?;
	fs::write(&sidecar_path, bytes).context("Failed to write sidecar")?;

	Ok(())
}

/// Load sidecar from disk
pub fn load(sidecar_path: &Path) -> Result<Sidecar> {
	let bytes = fs::read(sidecar_path).context("Failed to read sidecar")?;
	rmp_serde::from_slice(&bytes).context("Failed to deserialize sidecar")
}

/// Build sidecar path from hash
pub fn build_path(media_dir: &Path, hash: &FileHash) -> PathBuf {
	media_dir.join(SIDECAR_DIR).join(format!("{}.{}", hash.as_str(), SIDECAR_EXT))
}