//! Directory scanning for media files

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

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
        .filter_map(|line| line.ok())
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .collect()
}

fn is_ignored(path: &Path, patterns: &[String]) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();
    patterns.iter().any(|pattern| {
        path_str.contains(&pattern.to_lowercase())
    })
}

#[derive(Clone)]
pub struct MediaFile {
    pub path: PathBuf,
    pub filename: String,
    pub hash: FileHash,
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
    let mut to_process = Vec::new();
    let mut already_indexed = 0;
    let mut outdated = 0;
    let mut filtered = 0;
    let mut seen = HashSet::new();
    
    scan_recursive(root, root, recursive, force, min_resolution, max_size_mb, &mut to_process, &mut already_indexed, &mut outdated, &mut filtered, &mut seen);
    
    ScanResult {
        to_process,
        already_indexed,
        outdated,
        filtered,
    }
}

fn scan_recursive(
    current: &Path,
    root: &Path,
    recursive: bool,
    force: bool,
    min_resolution: Option<u32>,
    max_size_mb: Option<u64>,
    to_process: &mut Vec<MediaFile>,
    already_indexed: &mut usize,
    outdated: &mut usize,
    filtered: &mut usize,
    seen: &mut HashSet<PathBuf>,
) {
    let ignore_patterns = load_scoutignore(current);
    
    let Ok(entries) = fs::read_dir(current) else { return };
    
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        
        // Check ignore patterns
        if !ignore_patterns.is_empty() && is_ignored(&path, &ignore_patterns) {
            ui::debug(&format!("Ignored: {}", path.display()));
            continue;
        }
        
        // Skip .scout directories
        if path.file_name() == Some(std::ffi::OsStr::new(SIDECAR_DIR)) {
            continue;
        }
        
        if path.is_dir() {
            if recursive {
                scan_recursive(&path, root, recursive, force, min_resolution, max_size_mb, to_process, already_indexed, outdated, filtered, seen);
            }
        } else if let Some(media_type) = MediaType::detect(&path) {
            let Ok(canonical) = path.canonicalize() else { continue };
            
            if !seen.insert(canonical.clone()) {
                continue;
            }
            
            // Add size filter
            if let Some(max_mb) = max_size_mb {
                if let Ok(metadata) = fs::metadata(&canonical) {
                    let size_mb = metadata.len() / 1024 / 1024;
                    if size_mb > max_mb {
                        *filtered += 1;
                        ui::debug(&format!("Filtered (too large): {} ({}MB)", canonical.display(), size_mb));
                        continue;
                    }
                }
            }
            
            // Add resolution filter
            if let Some(min_res) = min_resolution {
                if let Ok(img) = image::image_dimensions(&canonical) {
                    let (width, height) = img;
                    let shortest = width.min(height);
                    if shortest < min_res {
                        *filtered += 1;
                        ui::debug(&format!("Filtered (too small): {} ({}x{})", canonical.display(), width, height));
                        continue;
                    }
                }
            }
            
            let Ok(hash) = FileHash::compute(&canonical) else {
                ui::warn(&format!("Failed to hash: {}", canonical.display()));
                continue;
            };
            
            ui::debug(&format!("Hashed {} -> {}", canonical.file_name().unwrap().to_string_lossy(), hash.short()));
            
            if !force {
                let media_dir = canonical.parent().unwrap_or(&canonical);
                if let Some(sidecar_path) = crate::storage::find(media_dir, &hash) {
                    if let Ok(sidecar) = crate::storage::load(&sidecar_path) {
                        if sidecar.is_current_version() {
                            *already_indexed += 1;
                            continue;
                        } else {
                            *outdated += 1;
                        }
                    }
                }
            }
            
            let filename = canonical.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            to_process.push(MediaFile {
                path: canonical,
                filename,
                hash,
                media_type,
            });
        }
    }
}
