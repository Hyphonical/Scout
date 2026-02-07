//! # Directory Scanning
//!
//! Discover and filter media files with parallel hashing.
//! Respects .scoutignore and handles resolution/size limits.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::config::SIDECAR_DIR;
use crate::core::{FileHash, MediaType};
use crate::ui;

fn load_scoutignore(dir: &Path) -> Vec<String> {
	let ignore_path = dir.join(".scoutignore");
	if !ignore_path.exists() {
		return Vec::new();
	}

	let Ok(file) = File::open(&ignore_path) else {
		return Vec::new();
	};

	BufReader::new(file)
		.lines()
		.map_while(Result::ok)
		.filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
		.collect()
}

fn is_ignored(path: &Path, patterns: &[String]) -> bool {
	let path_str = path.to_string_lossy().to_lowercase();
	patterns
		.iter()
		.any(|pattern| path_str.contains(&pattern.to_lowercase()))
}

#[derive(Clone)]
pub struct MediaFile {
	pub path: PathBuf,
	pub filename: String,
	pub hash: FileHash,
	#[allow(dead_code)]
	pub media_type: MediaType,
}

pub struct ScanResult {
	pub to_process: Vec<MediaFile>,
	pub already_indexed: usize,
	pub outdated: usize,
	pub filtered: usize,
}

/// Scan directory for media files
pub fn scan_directory(
	root: &Path,
	recursive: bool,
	force: bool,
	min_resolution: Option<u32>,
	max_size_mb: Option<u64>,
) -> ScanResult {
	// 1. Discovery Phase (Sequential, fast IO)
	ui::debug("Scanning directory structure...");
	let candidates = discover_files(root, recursive);
	ui::debug(&format!("Found {} candidate files", candidates.len()));

	// 2. Processing Phase (Parallel, CPU intensive)
	ui::debug("Processing files (metadata & hashing)...");

	let already_indexed = std::sync::atomic::AtomicUsize::new(0);
	let outdated = std::sync::atomic::AtomicUsize::new(0);
	let filtered = std::sync::atomic::AtomicUsize::new(0);

	let to_process: Vec<MediaFile> = candidates
		.into_par_iter()
		.filter_map(|path| {
			// Filters (Size/Resolution)
			if let Some(max_mb) = max_size_mb {
				if let Ok(metadata) = fs::metadata(&path) {
					let size_mb = metadata.len() / 1024 / 1024;
					if size_mb > max_mb {
						filtered.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
						return None;
					}
				}
			}

			if let Some(min_res) = min_resolution {
				if let Ok(img) = image::image_dimensions(&path) {
					let (width, height) = img;
					if width.min(height) < min_res {
						filtered.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
						return None;
					}
				}
			}

			// Hashing
			let Ok(hash) = FileHash::compute(&path) else {
				ui::warn(&format!("Failed to hash: {}", path.display()));
				return None;
			};

			// Existence Check
			if !force {
				let media_dir = path.parent().unwrap_or(&path);
				if let Some(sidecar_path) = crate::storage::find(media_dir, &hash) {
					if let Ok(sidecar) = crate::storage::load(&sidecar_path) {
						if sidecar.is_current_version() {
							already_indexed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
							return None;
						} else {
							outdated.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
						}
					}
				}
			}

			let filename = path
				.file_name()
				.and_then(|n| n.to_str())
				.unwrap_or("unknown")
				.to_string();

			let media_type = MediaType::detect(&path)?;

			Some(MediaFile {
				path,
				filename,
				hash,
				media_type,
			})
		})
		.collect();

	ScanResult {
		to_process,
		already_indexed: already_indexed.load(std::sync::atomic::Ordering::Relaxed),
		outdated: outdated.load(std::sync::atomic::Ordering::Relaxed),
		filtered: filtered.load(std::sync::atomic::Ordering::Relaxed),
	}
}

fn discover_files(root: &Path, recursive: bool) -> Vec<PathBuf> {
	let mut files = Vec::new();
	let mut seen = HashSet::new();
	discover_recursive(root, recursive, &mut files, &mut seen);
	files
}

fn discover_recursive(
	current: &Path,
	recursive: bool,
	files: &mut Vec<PathBuf>,
	seen: &mut HashSet<PathBuf>,
) {
	let ignore_patterns = load_scoutignore(current);

	let Ok(entries) = fs::read_dir(current) else {
		return;
	};

	for entry in entries.filter_map(|e| e.ok()) {
		let path = entry.path();

		// Check basic ignore rules
		if path.file_name() == Some(std::ffi::OsStr::new(SIDECAR_DIR)) {
			continue;
		}

		if !ignore_patterns.is_empty() && is_ignored(&path, &ignore_patterns) {
			continue;
		}

		if path.is_dir() {
			if recursive {
				discover_recursive(&path, recursive, files, seen);
			}
		} else if MediaType::detect(&path).is_some() {
			if let Ok(canonical) = path.canonicalize() {
				if seen.insert(canonical.clone()) {
					files.push(canonical);
				}
			}
		}
	}
}
