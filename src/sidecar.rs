// Sidecar - MessagePack metadata storage for processed images
//
// Per-directory storage: each folder has its own .scout/ with relative filenames only.
// This makes directories portable and cross-platform.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::config::{HASH_BUFFER_SIZE, SIDECAR_DIR, SIDECAR_EXT};

/// Returns the current program version.
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
	pub fn new(filename: &str, hash: String, embedding: Vec<f32>, processing_ms: u64) -> Self {
		Self {
			version: env!("CARGO_PKG_VERSION").to_string(),
			filename: filename.to_string(),
			hash,
			processed: Utc::now(),
			embedding,
			processing_ms,
		}
	}

	pub fn save(&self, path: &Path) -> Result<()> {
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent).context("Create sidecar dir")?;
		}
		let bytes = rmp_serde::to_vec(self).context("Serialize")?;
		fs::write(path, bytes).context("Write sidecar")
	}

	pub fn load(path: &Path) -> Result<Self> {
		let bytes = fs::read(path).context("Read sidecar")?;
		rmp_serde::from_slice(&bytes).context("Deserialize")
	}

	/// Checks if this sidecar was created with the current program version.
	pub fn is_current_version(&self) -> bool {
		self.version == current_version()
	}
}

/// Computes sidecar path: <image_dir>/.scout/<hash>.msgpack
pub fn get_sidecar_path(hash: &str, image_dir: &Path) -> PathBuf {
	image_dir.join(SIDECAR_DIR).join(format!("{}.{}", hash, SIDECAR_EXT))
}

/// Computes FNV-1a hash of first 64KB.
pub fn compute_file_hash(path: &Path) -> Result<String> {
	let mut file = fs::File::open(path).context("Open")?;
	let mut buf = vec![0u8; HASH_BUFFER_SIZE];
	let n = file.read(&mut buf)?;
	buf.truncate(n);

	let mut hash: u64 = 0xcbf29ce484222325;
	for byte in &buf {
		hash ^= *byte as u64;
		hash = hash.wrapping_mul(0x100000001b3);
	}
	Ok(format!("{:016x}", hash))
}

/// Finds sidecar by hash in the image's directory.
pub fn find_sidecar_by_hash(hash: &str, image_dir: &Path) -> Option<PathBuf> {
	let path = get_sidecar_path(hash, image_dir);
	path.exists().then_some(path)
}

/// Iterates all sidecar files, returning (sidecar_path, base_directory).
/// The base_directory is the directory containing the .scout folder.
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
				.filter(|e| e.path().extension().is_some_and(|x| x == SIDECAR_EXT) && e.path().is_file())
				.map(move |e| (e.path().to_path_buf(), base_dir.clone()))
		})
}
