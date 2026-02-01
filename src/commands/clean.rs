//! Clean command - remove orphaned sidecars

use std::fs;
use std::path::Path;

use crate::storage;
use crate::ui;

pub fn run(dir: &Path, recursive: bool) -> anyhow::Result<()> {
    ui::info(&format!("Scanning: {}", dir.display()));
    
    let sidecars = storage::scan(dir, recursive);
    let mut orphaned = Vec::new();
    
    for (sidecar_path, media_dir) in sidecars {
        let Ok(sidecar) = storage::load(&sidecar_path) else { continue };
        let image_path = media_dir.join(sidecar.filename());
        
        if !image_path.exists() {
            orphaned.push(sidecar_path);
        }
    }
    
    if orphaned.is_empty() {
        ui::success("No orphaned sidecars found");
        return Ok(());
    }
    
    ui::warn(&format!("Found {} orphaned sidecars", orphaned.len()));
    
    for path in &orphaned {
        fs::remove_file(path)?;
        ui::debug(&format!("Deleted: {}", path.display()));
    }
    
    ui::success(&format!("Cleaned {} sidecars", orphaned.len()));
    
    Ok(())
}
