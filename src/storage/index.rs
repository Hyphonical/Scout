//! Sidecar discovery and scanning

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::SIDECAR_DIR;
use crate::core::{FileHash, MediaType};
use crate::storage::Sidecar;

/// Find sidecar for a specific hash
pub fn find(media_dir: &Path, hash: &FileHash) -> Option<PathBuf> {
	let path = super::sidecar::build_path(media_dir, hash);
	if path.exists() {
		Some(path)
	} else {
		None
	}
}

/// Scan directory for all sidecars
pub fn scan(root: &Path, recursive: bool) -> Vec<(PathBuf, PathBuf)> {
	let mut results = Vec::new();
	scan_recursive(root, root, recursive, &mut results);
	results
}

/// Load all sidecars from a directory (for clustering)
pub fn load_all_sidecars(dir: &Path, recursive: bool) -> Vec<(PathBuf, Sidecar)> {
	let sidecar_paths = scan(dir, recursive);
	let mut sidecars = Vec::with_capacity(sidecar_paths.len());

	for (sidecar_path, _media_dir) in sidecar_paths {
		if let Ok(sidecar) = super::sidecar::load(&sidecar_path) {
			sidecars.push((sidecar_path, sidecar));
		}
	}

	sidecars
}

fn scan_recursive(
	current: &Path,
	root: &Path,
	recursive: bool,
	results: &mut Vec<(PathBuf, PathBuf)>,
) {
	let Ok(entries) = fs::read_dir(current) else {
		return;
	};

	for entry in entries.filter_map(|e| e.ok()) {
		let path = entry.path();

		if path.is_dir() {
			if path.file_name() == Some(std::ffi::OsStr::new(SIDECAR_DIR)) {
				// Found .scout directory - scan for sidecars
				let media_dir = path.parent().unwrap_or(root).to_path_buf();
				scan_sidecar_dir(&path, &media_dir, results);
			} else if recursive {
				scan_recursive(&path, root, recursive, results);
			}
		}
	}
}

fn scan_sidecar_dir(scout_dir: &Path, media_dir: &Path, results: &mut Vec<(PathBuf, PathBuf)>) {
	let Ok(entries) = fs::read_dir(scout_dir) else {
		return;
	};

	for entry in entries.filter_map(|e| e.ok()) {
		let path = entry.path();
		if path.extension().and_then(|s| s.to_str()) == Some("msgpack") {
			results.push((path, media_dir.to_path_buf()));
		}
	}
}

/// Find the actual file by hash in the given directory
pub fn find_file_by_hash(media_dir: &Path, hash: &str) -> Option<PathBuf> {
	let Ok(entries) = fs::read_dir(media_dir) else {
		return None;
	};

	for entry in entries.filter_map(|e| e.ok()) {
		let path = entry.path();
		
		// Skip directories and non-media files
		if path.is_dir() || MediaType::detect(&path).is_none() {
			continue;
		}

		// Compute hash and compare
		if let Ok(file_hash) = FileHash::compute(&path) {
			if file_hash.as_str() == hash {
				return Some(path);
			}
		}
	}

	None
}
