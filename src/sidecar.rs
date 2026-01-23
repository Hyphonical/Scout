// Sidecar - JSON metadata storage for processed images
//
// Hash-based storage layout: .scout/ab/abcdef123456.json
// Uses absolute paths for reliable lookups regardless of working directory.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::config::{HASH_BUFFER_SIZE, INPUT_SIZE, MAX_TAGS, SIDECAR_DIR};

fn clean_path(path: &str) -> String {
	path.strip_prefix(r"\\?\").unwrap_or(path).to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageSidecar {
	pub scout_version: String,
	pub source_image: String,
	pub image_hash: String,
	pub processed_at: DateTime<Utc>,
	pub model: ModelInfo,
	pub parameters: ProcessingParameters,
	pub tags: Vec<TagEntry>,
	pub stats: TagStats,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
	pub name: String,
	pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessingParameters {
	pub threshold: f32,
	pub max_tags: usize,
	pub input_size: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagEntry {
	pub id: usize,
	pub name: String,
	pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagStats {
	pub total_tags: usize,
	pub processing_ms: u64,
}

impl ImageSidecar {
	pub fn new(source: &Path, hash: String, tags: Vec<TagEntry>, threshold: f32, ms: u64) -> Self {
		Self {
			scout_version: env!("CARGO_PKG_VERSION").to_string(),
			source_image: clean_path(&source.to_string_lossy()),
			image_hash: hash.clone(),
			processed_at: Utc::now(),
			model: ModelInfo { name: "onnx-tagger".to_string(), version: "1.0".to_string() },
			parameters: ProcessingParameters { threshold, max_tags: MAX_TAGS, input_size: INPUT_SIZE },
			tags: tags.into_iter().take(MAX_TAGS).collect(),
			stats: TagStats { total_tags: 0, processing_ms: ms },
			embedding: None,
		}
	}

	pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
		self.embedding = Some(embedding);
		self
	}

	pub fn save(&self, path: &Path) -> Result<()> {
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent).context("Create sidecar dir")?;
		}
		let json = serde_json::to_string_pretty(self).context("Serialize")?;
		fs::write(path, json).context("Write sidecar")
	}

	#[allow(dead_code)]
	pub fn load(path: &Path) -> Result<Self> {
		let json = fs::read_to_string(path).context("Read sidecar")?;
		serde_json::from_str(&json).context("Parse sidecar")
	}
}

/// Computes sidecar path using hash-based layout: .scout/ab/abcdef123456.json
/// This prevents directory bloat and makes lookups independent of source path.
pub fn get_sidecar_path(hash: &str, root: &Path) -> PathBuf {
	let prefix = if hash.len() >= 2 { &hash[..2] } else { hash };
	root.join(SIDECAR_DIR).join(prefix).join(format!("{}.json", hash))
}

/// Computes a fast FNV-1a hash of the first 64KB of a file.
pub fn compute_file_hash(path: &Path) -> Result<String> {
	let mut file = fs::File::open(path).context("Open for hash")?;
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

/// Finds sidecar by hash in the .scout directory.
pub fn find_sidecar_by_hash(hash: &str, root: &Path) -> Option<PathBuf> {
	let path = get_sidecar_path(hash, root);
	path.exists().then_some(path)
}

/// Iterates all sidecar files in .scout directory.
pub fn iter_sidecars(root: &Path) -> impl Iterator<Item = PathBuf> {
	let scout_dir = root.join(SIDECAR_DIR);
	walkdir::WalkDir::new(&scout_dir)
		.into_iter()
		.filter_map(|e| e.ok())
		.filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
		.filter(|e| e.path().is_file())
		.map(|e| e.path().to_path_buf())
}
