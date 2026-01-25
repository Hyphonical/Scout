// Scanner - Discovers image files in directories

use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::{IMAGE_EXTENSIONS, SIDECAR_DIR};
use crate::sidecar::{compute_file_hash, find_sidecar_by_hash, get_sidecar_path};

pub struct ScanResult {
	pub images: Vec<ImageEntry>,
	pub skipped: Vec<PathBuf>,
	pub errors: Vec<String>,
}

pub struct ImageEntry {
	pub path: PathBuf,
	pub sidecar_path: PathBuf,
}

impl ScanResult {
	pub fn total(&self) -> usize {
		self.images.len() + self.skipped.len()
	}
}

pub fn scan_directory(directory: &Path, recursive: bool, force: bool) -> Result<ScanResult> {
	let root = directory.canonicalize().unwrap_or_else(|_| directory.to_path_buf());
	let mut result = ScanResult { images: Vec::new(), skipped: Vec::new(), errors: Vec::new() };
	let mut seen = HashSet::new();

	let walker = if recursive {
		WalkDir::new(&root)
	} else {
		WalkDir::new(&root).max_depth(1)
	};

	for entry in walker.into_iter().filter_map(|e| e.ok()) {
		let path = entry.path();
		if is_scout_path(path) || !path.is_file() || !is_image(path) {
			continue;
		}

		let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
		if seen.contains(&canonical) {
			continue;
		}
		seen.insert(canonical.clone());

		let hash = match compute_file_hash(&canonical) {
			Ok(h) => h,
			Err(e) => {
				result.errors.push(format!("{}: {}", canonical.display(), e));
				continue;
			}
		};

		if !force && find_sidecar_by_hash(&hash, &root).is_some() {
			result.skipped.push(canonical);
			continue;
		}

		result.images.push(ImageEntry {
			path: canonical,
			sidecar_path: get_sidecar_path(&hash, &root),
		});
	}

	Ok(result)
}

fn is_image(path: &Path) -> bool {
	path.extension()
		.and_then(|e| e.to_str())
		.is_some_and(|ext| IMAGE_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)))
}

fn is_scout_path(path: &Path) -> bool {
	path.components().any(|c| c.as_os_str() == SIDECAR_DIR)
}
