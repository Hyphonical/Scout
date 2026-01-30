//! Image discovery and filtering
//!
//! Scans directories for images, applies filters, and tracks indexing status.
//! Handles deduplication, version detection, and scan result reporting.

use anyhow::Result;
use image::ImageReader;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::sidecar::{compute_file_hash, find_sidecar, sidecar_path, Sidecar};
use crate::config::{IMAGE_EXTENSIONS, SIDECAR_DIR, VIDEO_EXTENSIONS};
use crate::logger::{log, Level};
use crate::types::MediaType;
use crate::video;

/// Configurable filters for image scanning
#[derive(Debug, Clone)]
pub struct ScanFilters {
	pub min_width: u32,
	pub min_height: u32,
	pub min_size_kb: u64,
	pub max_size_mb: Option<u64>,
	pub exclude_patterns: Vec<String>,
}

impl ScanFilters {
	pub fn new(
		min_width: u32,
		min_height: u32,
		min_size_kb: u64,
		max_size_mb: Option<u64>,
		exclude_patterns: Vec<String>,
	) -> Self {
		Self { min_width, min_height, min_size_kb, max_size_mb, exclude_patterns }
	}

	fn should_filter(&self, path: &Path) -> Option<String> {
		let path_str = path.to_string_lossy().to_lowercase();
		for pattern in &self.exclude_patterns {
			if path_str.contains(&pattern.to_lowercase()) {
				return Some(format!("matches exclude pattern '{}'", pattern));
			}
		}

		if let Ok(metadata) = std::fs::metadata(path) {
			let size_kb = metadata.len() / 1024;
			let size_mb = size_kb / 1024;

			if size_kb < self.min_size_kb {
				return Some(format!("file too small ({}KB < {}KB)", size_kb, self.min_size_kb));
			}

			if let Some(max_mb) = self.max_size_mb {
				if size_mb > max_mb {
					return Some(format!("file too large ({}MB > {}MB)", size_mb, max_mb));
				}
			}
		}

		if self.min_width > 0 || self.min_height > 0 {
			if let Ok(reader) = ImageReader::open(path) {
				if let Ok((width, height)) = reader.into_dimensions() {
					if width < self.min_width || height < self.min_height {
						return Some(format!(
							"resolution too small ({}x{} < {}x{})",
							width, height, self.min_width, self.min_height
						));
					}
				}
			}
		}

		None
	}
}

/// Represents an image that needs processing
pub struct ImageEntry {
	pub path: PathBuf,
	pub filename: String,
	pub sidecar_path: PathBuf,
	pub media_type: MediaType,
}

/// Results from a directory scan operation
pub struct ScanResult {
	pub to_process: Vec<ImageEntry>,
	pub indexed_count: usize,
	pub filtered_count: usize,
	pub outdated_count: usize,
	pub error_count: usize,
	pub skipped_videos: usize,
}

impl ScanResult {
	pub fn total(&self) -> usize {
		self.to_process.len() + self.indexed_count
	}
}

/// Scans a directory for images, checking against existing sidecars
///
/// # Arguments
/// * `directory` - Root directory to scan
/// * `recursive` - Whether to scan subdirectories
/// * `force` - Whether to reprocess already-indexed images
/// * `filters` - Filtering criteria to apply
pub fn scan_directory(
	directory: &Path,
	recursive: bool,
	force: bool,
	filters: &ScanFilters,
) -> Result<ScanResult> {
	let root = directory.canonicalize().unwrap_or_else(|_| directory.to_path_buf());

	log(Level::Debug, &format!("Scanning: {}", root.display()));
	if filters.min_width > 0 || filters.min_height > 0 {
		log(Level::Debug, &format!("Min resolution: {}x{}", filters.min_width, filters.min_height));
	}
	if filters.min_size_kb > 0 {
		log(Level::Debug, &format!("Min size: {}KB", filters.min_size_kb));
	}
	if let Some(max) = filters.max_size_mb {
		log(Level::Debug, &format!("Max size: {}MB", max));
	}
	if !filters.exclude_patterns.is_empty() {
		log(Level::Debug, &format!("Exclude: {}", filters.exclude_patterns.join(", ")));
	}

	let mut to_process = Vec::new();
	let mut indexed = 0;
	let mut filtered = 0;
	let mut outdated = 0;
	let mut errors = 0;
	let mut skipped_videos = 0;
	let mut seen = HashSet::new();

	let walker = if recursive {
		WalkDir::new(&root)
	} else {
		WalkDir::new(&root).max_depth(1)
	};

	for entry in walker.into_iter().filter_map(|e| e.ok()) {
		let path = entry.path();

		if is_scout_path(path) || !path.is_file() {
			continue;
		}

		let media_type = if is_image(path) {
			MediaType::Image
		} else if is_video(path) {
			// Check if video is disabled by CLI flag
			if video::is_video_disabled() {
				skipped_videos += 1;
				continue;
			}
			// Check if video feature is compiled in
			if !video::is_video_feature_enabled() {
				video::show_video_feature_warning_once();
				skipped_videos += 1;
				continue;
			}
			// Video processing will fail gracefully if FFmpeg is not installed
			MediaType::Video
		} else {
			continue
		};

		let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
		if !seen.insert(canonical.clone()) {
			continue;
		}

		if let Some(reason) = filters.should_filter(&canonical) {
			log(Level::Debug, &format!("Filtered {}: {}", canonical.display(), reason));
			filtered += 1;
			continue;
		}

		let hash = match compute_file_hash(&canonical) {
			Ok(h) => h,
			Err(e) => {
				log(Level::Warning, &format!("Hash failed for {}: {}", canonical.display(), e));
				errors += 1;
				continue;
			}
		};

		let image_dir = canonical.parent().unwrap_or(&canonical).to_path_buf();
		let filename = canonical
			.file_name()
			.and_then(|n| n.to_str())
			.unwrap_or("unknown")
			.to_string();

		log(Level::Debug, &format!("Hash for {}: {}", filename, hash.short()));

		if !force {
			if let Some(sidecar_path) = find_sidecar(&hash, &image_dir) {
				if let Ok(sidecar) = Sidecar::load_auto(&sidecar_path) {
					if sidecar.is_current_version() {
						indexed += 1;
						continue;
					}
					outdated += 1;
					log(Level::Debug, &format!("Outdated: {} (v{})", filename, match &sidecar {
						Sidecar::Image(img) => &img.version,
						Sidecar::Video(vid) => &vid.version,
					}));
				}
			}
		}

		to_process.push(ImageEntry {
			path: canonical,
			filename,
			sidecar_path: sidecar_path(&hash, &image_dir),
			media_type,
		});
	}

	Ok(ScanResult {
		to_process,
		indexed_count: indexed,
		filtered_count: filtered,
		outdated_count: outdated,
		error_count: errors,
		skipped_videos,
	})
}

fn is_image(path: &Path) -> bool {
	path.extension()
		.and_then(|e| e.to_str())
		.is_some_and(|ext| IMAGE_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)))
}

fn is_video(path: &Path) -> bool {
	path.extension()
		.and_then(|e| e.to_str())
		.is_some_and(|ext| VIDEO_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)))
}

fn is_scout_path(path: &Path) -> bool {
	path.components().any(|c| c.as_os_str() == SIDECAR_DIR)
}
