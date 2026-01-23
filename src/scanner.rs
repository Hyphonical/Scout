// Scanner - Discovers image files from paths, globs, and directories

use anyhow::{Context, Result};
use glob::glob;
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
	fn new() -> Self {
		Self { images: Vec::new(), skipped: Vec::new(), errors: Vec::new() }
	}

	pub fn total_found(&self) -> usize {
		self.images.len() + self.skipped.len()
	}
}

/// Scans all input paths and returns discovered images.
/// Uses hash-based sidecar lookup for skip detection.
pub fn scan_inputs(inputs: &[PathBuf], recursive: bool, force: bool) -> Result<ScanResult> {
	let root = find_common_root(inputs)?;
	let mut result = ScanResult::new();
	let mut seen = HashSet::new();

	for input in inputs {
		let input_str = input.to_string_lossy();

		if input_str.contains('*') || input_str.contains('?') {
			match glob(&input_str) {
				Ok(paths) => {
					for path in paths.flatten() {
						collect_image(&path, &root, recursive, force, &mut result, &mut seen);
					}
				}
				Err(e) => result.errors.push(format!("Glob '{}': {}", input_str, e)),
			}
		} else if input.exists() {
			collect_image(input, &root, recursive, force, &mut result, &mut seen);
		} else {
			result.errors.push(format!("Not found: {}", input.display()));
		}
	}

	Ok(result)
}

fn collect_image(path: &Path, root: &Path, recursive: bool, force: bool, result: &mut ScanResult, seen: &mut HashSet<PathBuf>) {
	if path.is_file() {
		add_if_image(path, root, force, result, seen);
	} else if path.is_dir() {
		let walker = if recursive { WalkDir::new(path) } else { WalkDir::new(path).max_depth(1) };

		for entry in walker.into_iter().filter_map(|e| e.ok()) {
			let p = entry.path();
			if !is_scout_path(p) && p.is_file() {
				add_if_image(p, root, force, result, seen);
			}
		}
	}
}

fn add_if_image(path: &Path, root: &Path, force: bool, result: &mut ScanResult, seen: &mut HashSet<PathBuf>) {
	if !is_image(path) { return; }

	let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
	if seen.contains(&canonical) { return; }
	seen.insert(canonical.clone());

	// Compute hash for sidecar lookup
	let hash = match compute_file_hash(&canonical) {
		Ok(h) => h,
		Err(_) => return,
	};

	// Check if sidecar already exists using hash-based path
	if !force && find_sidecar_by_hash(&hash, root).is_some() {
		result.skipped.push(canonical);
		return;
	}

	let sidecar_path = get_sidecar_path(&hash, root);
	result.images.push(ImageEntry { path: canonical, sidecar_path });
}

fn is_image(path: &Path) -> bool {
	path.extension()
		.and_then(|e| e.to_str())
		.map(|ext| IMAGE_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)))
		.unwrap_or(false)
}

fn is_scout_path(path: &Path) -> bool {
	path.components().any(|c| c.as_os_str() == SIDECAR_DIR)
}

fn find_common_root(paths: &[PathBuf]) -> Result<PathBuf> {
	if paths.is_empty() {
		return std::env::current_dir().context("No current directory");
	}

	let first = &paths[0];
	let first_dir = if first.is_dir() { first.clone() } else {
		first.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
	};

	Ok(first_dir.canonicalize().unwrap_or(first_dir))
}
