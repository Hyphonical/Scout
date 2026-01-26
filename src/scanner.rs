// Scanner - Discovers image files in directories with advanced filtering

use anyhow::Result;
use image::ImageReader;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::cli::ScanFilters;
use crate::config::{IMAGE_EXTENSIONS, SIDECAR_DIR};
use crate::logger::{log, Level};
use crate::sidecar::{compute_file_hash, find_sidecar_by_hash, get_sidecar_path, ImageSidecar};

pub struct ScanResult {
	pub images: Vec<ImageEntry>,
	pub skipped: Vec<PathBuf>,
	pub outdated: usize,
	pub filtered: Vec<FilterReason>,
	pub errors: Vec<String>,
}

pub struct ImageEntry {
	pub path: PathBuf,
	pub filename: String,
	pub sidecar_path: PathBuf,
}

#[derive(Debug)]
pub struct FilterReason {
	pub path: PathBuf,
	pub reason: String,
}

impl ScanResult {
	pub fn total(&self) -> usize {
		self.images.len() + self.skipped.len()
	}
}

pub fn scan_directory(
	directory: &Path,
	recursive: bool,
	force: bool,
	filters: &ScanFilters,
) -> Result<ScanResult> {
	let root = directory.canonicalize().unwrap_or_else(|_| directory.to_path_buf());
	log(
		Level::Debug,
		&format!(
			"Scanning directory: {}, Recursive: {}, Force: {}",
			root.display(),
			recursive,
			force
		),
	);

	// Log active filters
	if filters.min_width > 0 || filters.min_height > 0 {
		log(
			Level::Debug,
			&format!(
				"Minimum resolution: {}x{}",
				filters.min_width, filters.min_height
			),
		);
	}
	if filters.min_size_kb > 0 {
		log(
			Level::Debug,
			&format!("Minimum file size: {}KB", filters.min_size_kb),
		);
	}
	if let Some(max) = filters.max_size_mb {
		log(Level::Debug, &format!("Maximum file size: {}MB", max));
	}
	if !filters.exclude_patterns.is_empty() {
		log(
			Level::Debug,
			&format!("Exclude patterns: {}", filters.exclude_patterns.join(", ")),
		);
	}

	let mut result = ScanResult {
		images: Vec::new(),
		skipped: Vec::new(),
		outdated: 0,
		filtered: Vec::new(),
		errors: Vec::new(),
	};
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

		// Apply filters
		if let Some(reason) = should_filter(&canonical, filters) {
			result.filtered.push(FilterReason {
				path: canonical,
				reason,
			});
			continue;
		}

		let hash = match compute_file_hash(&canonical) {
			Ok(h) => h,
			Err(e) => {
				result
					.errors
					.push(format!("{}: {}", canonical.display(), e));
				continue;
			}
		};

		// Get the directory containing the image
		let image_dir = canonical.parent().unwrap_or(&canonical).to_path_buf();
		let filename = canonical
			.file_name()
			.map(|n| n.to_string_lossy().to_string())
			.unwrap_or_default();

		log(
			Level::Debug,
			&format!("Computed hash for {}: {}", filename, &hash[..8]),
		);

		// Check for existing sidecar in the image's directory
		if !force {
			if let Some(sidecar_path) = find_sidecar_by_hash(&hash, &image_dir) {
				// Check if sidecar version matches current program version
				if let Ok(sidecar) = ImageSidecar::load(&sidecar_path) {
					if sidecar.is_current_version() {
						result.skipped.push(canonical);
						continue;
					}
					// Outdated version - needs reprocessing
					result.outdated += 1;
					log(
						Level::Debug,
						&format!("Outdated sidecar for {}: v{}", filename, sidecar.version),
					);
				}
				// If sidecar can't be loaded, reprocess it
			}
		}

		result.images.push(ImageEntry {
			path: canonical,
			filename,
			sidecar_path: get_sidecar_path(&hash, &image_dir),
		});
	}

	Ok(result)
}

/// Checks if an image should be filtered out. Returns Some(reason) if it should be filtered.
fn should_filter(path: &Path, filters: &ScanFilters) -> Option<String> {
	// Check exclude patterns
	if !filters.exclude_patterns.is_empty() {
		let path_str = path.to_string_lossy().to_lowercase();
		for pattern in &filters.exclude_patterns {
			if path_str.contains(&pattern.to_lowercase()) {
				return Some(format!("matches exclude pattern '{}'", pattern));
			}
		}
	}

	// Check file size
	if let Ok(metadata) = std::fs::metadata(path) {
		let size_bytes = metadata.len();
		let size_kb = size_bytes / 1024;
		let size_mb = size_kb / 1024;

		if size_kb < filters.min_size_kb {
			return Some(format!("file too small ({}KB < {}KB)", size_kb, filters.min_size_kb));
		}

		if let Some(max_mb) = filters.max_size_mb {
			if size_mb > max_mb {
				return Some(format!("file too large ({}MB > {}MB)", size_mb, max_mb));
			}
		}
	}

	// Check image dimensions (only if resolution filters are set)
	if filters.min_width > 0 || filters.min_height > 0 {
		match ImageReader::open(path) {
			Ok(reader) => {
				if let Ok(dimensions) = reader.into_dimensions() {
					let (width, height) = dimensions;
					
					if width < filters.min_width || height < filters.min_height {
						return Some(format!(
							"resolution too small ({}x{} < {}x{})",
							width, height, filters.min_width, filters.min_height
						));
					}
				} else {
					// Can't read dimensions, but don't filter - let processor handle it
					log(Level::Debug, &format!("Could not read dimensions for {}", path.display()));
				}
			}
			Err(_) => {
				// Can't open image for dimension check, but don't filter - let processor handle it
				log(Level::Debug, &format!("Could not open for dimension check: {}", path.display()));
			}
		}
	}

	None
}

fn is_image(path: &Path) -> bool {
	path.extension()
		.and_then(|e| e.to_str())
		.is_some_and(|ext| IMAGE_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)))
}

fn is_scout_path(path: &Path) -> bool {
	path.components().any(|c| c.as_os_str() == SIDECAR_DIR)
}