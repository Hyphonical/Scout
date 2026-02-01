//! Scan command - index images

use anyhow::Result;
use std::path::Path;
use std::time::Instant;

use crate::models::Models;
use crate::processing;
use crate::storage;
use crate::ui;

pub fn run(
    dir: &Path,
    recursive: bool,
    force: bool,
    min_resolution: Option<u32>,
    max_size: Option<u64>,
) -> Result<()> {
    let start = Instant::now();
    
    ui::info(&format!("Scanning: {}", dir.display()));
    
    let scan_result = processing::scan_directory(dir, recursive, force, min_resolution, max_size);
    
    if scan_result.to_process.is_empty() {
        ui::success(&format!(
            "All {} images already indexed",
            scan_result.already_indexed
        ));
        if scan_result.filtered > 0 {
            ui::info(&format!("{} images filtered out", scan_result.filtered));
        }
        return Ok(());
    }
    
    ui::info(&format!(
        "Processing {} images ({} indexed, {} filtered)",
        scan_result.to_process.len(),
        scan_result.already_indexed,
        scan_result.filtered
    ));
    
    if scan_result.outdated > 0 {
        ui::warn(&format!(
            "{} sidecars need upgrading (outdated version)",
            scan_result.outdated
        ));
    }
    
    let mut models = Models::new()?;
    let mut processed = 0;
    let mut errors = 0;
    
    for file in scan_result.to_process {
        match processing::image::encode(&mut models, &file.path) {
            Ok(embedding) => {
                let media_dir = file.path.parent().unwrap();
                let sidecar = storage::Sidecar::new(
                    file.filename.clone(),
                    file.hash.clone(),
                    embedding,
                );
                
                if let Err(e) = storage::save(&sidecar, media_dir, &file.hash) {
                    ui::error(&format!("Save failed: {}", e));
                    errors += 1;
                } else {
                    processed += 1;
                    ui::debug(&format!("✓ {}", file.filename));
                }
            }
            Err(e) => {
                ui::error(&format!("Encode failed for {}: {}", file.filename, e));
                errors += 1;
            }
        }
    }
    
    let duration = start.elapsed().as_secs_f32();
    
    println!();
    ui::success(&format!("Processed {} images in {:.1}s", processed, duration));
    if errors > 0 {
        ui::warn(&format!("{} errors", errors));
    }
    
    Ok(())
}
