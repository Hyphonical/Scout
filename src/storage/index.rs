//! Sidecar discovery and scanning

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use walkdir::WalkDir;

use crate::config::SIDECAR_DIR;
use crate::core::{FileHash, MediaType};
use crate::storage::Sidecar;

pub fn find(media_dir: &Path, hash: &FileHash) -> Option<PathBuf> {
	let path = super::sidecar::build_path(media_dir, hash);
	if path.exists() {
		Some(path)
	} else {
		None
	}
}

pub fn scan(root: &Path, recursive: bool) -> Vec<(PathBuf, PathBuf)> {
	let mut results = Vec::new();
	scan_recursive(root, root, recursive, &mut results);
	results
}

pub fn load_all_sidecars(dir: &Path, recursive: bool) -> Vec<(PathBuf, Sidecar)> {
	let sidecar_paths = scan(dir, recursive);

	if sidecar_paths.is_empty() {
		return Vec::new();
	}

	crate::ui::debug("Building file hash cache...");
	let cache_start = std::time::Instant::now();
	let hash_cache = build_hash_cache(dir, recursive);
	let cache_duration = cache_start.elapsed();
	crate::ui::debug(&format!(
		"Built hash cache ({} files) in {:.2}s",
		hash_cache.len(),
		cache_duration.as_secs_f32()
	));

	let mut results = Vec::with_capacity(sidecar_paths.len());

	for (sidecar_path, _media_dir) in sidecar_paths {
		if let Ok(sidecar) = super::sidecar::load(&sidecar_path) {
			let hash = sidecar.hash();

			if let Some(media_path) = hash_cache.get(hash) {
				results.push((media_path.clone(), sidecar));
			}
		}
	}

	results
}

fn build_hash_cache(dir: &Path, recursive: bool) -> HashMap<String, PathBuf> {
	let walker = if recursive {
		WalkDir::new(dir)
	} else {
		WalkDir::new(dir).max_depth(1)
	};

	let media_files: Vec<PathBuf> = walker
		.into_iter()
		.filter_map(|e| e.ok())
		.filter(|e: &walkdir::DirEntry| e.file_type().is_file())
		.map(|e: walkdir::DirEntry| e.path().to_path_buf())
		.filter(|p: &PathBuf| MediaType::detect(p).is_some())
		.collect();

	media_files
		.par_iter()
		.filter_map(|path| {
			FileHash::compute(path)
				.ok()
				.map(|hash| (hash.as_str().to_string(), path.clone()))
		})
		.collect()
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

pub fn find_file_by_hash(media_dir: &Path, hash: &str) -> Option<PathBuf> {
	let Ok(entries) = fs::read_dir(media_dir) else {
		return None;
	};

	for entry in entries.filter_map(|e| e.ok()) {
		let path = entry.path();

		if path.is_dir() || MediaType::detect(&path).is_none() {
			continue;
		}

		if let Ok(file_hash) = FileHash::compute(&path) {
			if file_hash.as_str() == hash {
				return Some(path);
			}
		}
	}

	None
}
