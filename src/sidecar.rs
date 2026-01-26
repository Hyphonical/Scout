// Sidecar - MessagePack metadata storage

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{SIDECAR_DIR, SIDECAR_EXT};
use crate::types::{Embedding, ImageHash};

pub fn compute_file_hash(path: &Path) -> Result<ImageHash> {
	use std::fs::File;
	use std::io::Read;

	const HASH_BUFFER: usize = 65536;

	let mut file = File::open(path).context("Failed to open for hashing")?;
	let mut buf = vec![0u8; HASH_BUFFER];
	let n = file.read(&mut buf)?;
	buf.truncate(n);

	let mut hash: u64 = 0xcbf29ce484222325;
	for byte in &buf {
		hash ^= *byte as u64;
		hash = hash.wrapping_mul(0x100000001b3);
	}

	Ok(ImageHash(format!("{:016x}", hash)))
}

pub fn current_version() -> &'static str {
	env!("CARGO_PKG_VERSION")
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageSidecar {
	pub version: String,
	pub filename: String,
	pub hash: String,
	pub processed: DateTime<Utc>,
	pub embedding: Vec<f32>,
	pub processing_ms: u64,
}

impl ImageSidecar {
	pub fn new(filename: &str, hash: ImageHash, embedding: Embedding, processing_ms: u64) -> Self {
		Self {
			version: current_version().to_string(),
			filename: filename.to_string(),
			hash: hash.0,
			processed: Utc::now(),
			embedding: embedding.0,
			processing_ms,
		}
	}

	pub fn save(&self, path: &Path) -> Result<()> {
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent).context("Failed to create sidecar directory")?;
		}
		let bytes = rmp_serde::to_vec(self).context("Failed to serialize sidecar")?;
		fs::write(path, bytes).context("Failed to write sidecar")?;
		Ok(())
	}

	pub fn load(path: &Path) -> Result<Self> {
		let bytes = fs::read(path).context("Failed to read sidecar")?;
		rmp_serde::from_slice(&bytes).context("Failed to deserialize sidecar")
	}

	pub fn is_current_version(&self) -> bool {
		self.version == current_version()
	}

	pub fn embedding(&self) -> Embedding {
		Embedding::raw(self.embedding.clone())
	}
}

pub fn sidecar_path(hash: &ImageHash, image_dir: &Path) -> PathBuf {
	image_dir.join(SIDECAR_DIR).join(format!("{}.{}", hash.as_str(), SIDECAR_EXT))
}

pub fn find_sidecar(hash: &ImageHash, image_dir: &Path) -> Option<PathBuf> {
	let path = sidecar_path(hash, image_dir);
	path.exists().then_some(path)
}

pub fn iter_sidecars(root: &Path, recursive: bool) -> impl Iterator<Item = (PathBuf, PathBuf)> {
	let walker = if recursive {
		walkdir::WalkDir::new(root)
	} else {
		walkdir::WalkDir::new(root).max_depth(1)
	};

	walker
		.into_iter()
		.filter_map(|e| e.ok())
		.filter(|e| e.file_type().is_dir() && e.file_name() == SIDECAR_DIR)
		.flat_map(|scout_dir| {
			let base_dir = scout_dir.path().parent().unwrap_or(scout_dir.path()).to_path_buf();
			walkdir::WalkDir::new(scout_dir.path())
				.max_depth(1)
				.into_iter()
				.filter_map(|e| e.ok())
				.filter(|e| {
					e.path().extension().is_some_and(|x| x == SIDECAR_EXT) && e.path().is_file()
				})
				.map(move |e| (e.path().to_path_buf(), base_dir.clone()))
		})
}
