// Sidecar - JSON metadata storage for processed images
//
// Hash-based storage layout: .scout/ab/abcdef123456.json

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::config::{HASH_BUFFER_SIZE, SIDECAR_DIR};

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageSidecar {
	pub version: String,
	pub source: String,
	pub hash: String,
	pub processed: DateTime<Utc>,
	pub embedding: Vec<f32>,
	pub processing_ms: u64,
}

impl ImageSidecar {
	pub fn new(source: &Path, hash: String, embedding: Vec<f32>, processing_ms: u64) -> Self {
		// Strip the Windows verbatim prefix if present
		let source_str = source.to_string_lossy()
			.trim_start_matches(r"\\?\")
			.to_string();

		Self {
			version: env!("CARGO_PKG_VERSION").to_string(),
			source: source_str,
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
		let json = serde_json::to_string(self).context("Serialize")?;
		fs::write(path, json).context("Write sidecar")
	}
}

/// Computes sidecar path: .scout/ab/abcdef123456.json
pub fn get_sidecar_path(hash: &str, root: &Path) -> PathBuf {
	let prefix = if hash.len() >= 2 { &hash[..2] } else { hash };
	root.join(SIDECAR_DIR).join(prefix).join(format!("{}.json", hash))
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

/// Finds sidecar by hash.
pub fn find_sidecar_by_hash(hash: &str, root: &Path) -> Option<PathBuf> {
	let path = get_sidecar_path(hash, root);
	path.exists().then_some(path)
}

/// Iterates all sidecar files.
pub fn iter_sidecars(root: &Path) -> impl Iterator<Item = PathBuf> {
	walkdir::WalkDir::new(root.join(SIDECAR_DIR))
		.into_iter()
		.filter_map(|e| e.ok())
		.filter(|e| e.path().extension().is_some_and(|x| x == "json") && e.path().is_file())
		.map(|e| e.path().to_path_buf())
}
