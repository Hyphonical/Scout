//! Search command - find similar images

use anyhow::{anyhow, Result};
use colored::*;
use std::path::Path;

use crate::config::NEGATIVE_WEIGHT;
use crate::core::Embedding;
use crate::models::Models;
use crate::storage;
use crate::ui;

pub struct Match {
    pub path: String,
    pub score: f32,
}

pub fn run(
    query_text: Option<&str>,
    query_image: Option<&Path>,
    weight: f32,
    negative: Option<&str>,
    dir: &Path,
    recursive: bool,
    limit: usize,
    min_score: f32,
    open_first: bool,
) -> Result<()> {
    // Build query embedding
    let mut models = Models::new()?;
    
    let query_emb = match (query_text, query_image) {
        (Some(text), None) => {
            ui::info(&format!("Searching for: \"{}\"", text));
            models.encode_text(text)?
        }
        (None, Some(img_path)) => {
            ui::info(&format!("Searching by image: {}", img_path.display()));
            let img = image::open(img_path)?;
            models.encode_image(&img)?
        }
        (Some(text), Some(img_path)) => {
            ui::info(&format!(
                "Combined search: \"{}\" + {} (weight: {:.2})",
                text,
                img_path.file_name().unwrap().to_string_lossy(),
                weight
            ));
            let text_emb = models.encode_text(text)?;
            let img = image::open(img_path)?;
            let img_emb = models.encode_image(&img)?;
            Embedding::blend(&text_emb, &img_emb, weight)
        }
        (None, None) => {
            return Err(anyhow!("Must provide either query text or --image"));
        }
    };
    
    // Build negative embedding if provided
    let negative_emb = if let Some(neg) = negative {
        ui::debug(&format!("Negative prompt: \"{}\"", neg));
        Some(models.encode_text(neg)?)
    } else {
        None
    };
    
    let sidecars = storage::scan(dir, recursive);
    
    if sidecars.is_empty() {
        ui::warn("No indexed images found. Run 'scout scan' first.");
        return Ok(());
    }
    
    let mut matches = Vec::new();
    let mut outdated = 0;
    
    for (sidecar_path, media_dir) in sidecars {
        let Ok(sidecar) = storage::load(&sidecar_path) else { continue };
        
        if !sidecar.is_current_version() {
            outdated += 1;
        }
        
        let mut score = query_emb.similarity(&sidecar.embedding());
        
        // Apply negative penalty
        if let Some(ref neg_emb) = negative_emb {
            let neg_score = neg_emb.similarity(&sidecar.embedding());
            score = score - (neg_score * NEGATIVE_WEIGHT);
        }
        
        if score >= min_score {
            let image_path = media_dir.join(sidecar.filename());
            matches.push(Match {
                path: image_path.to_string_lossy().to_string(),
                score,
            });
        }
    }
    
    if outdated > 0 {
        ui::warn(&format!(
            "{} sidecars are outdated. Run 'scout scan --force' to upgrade.",
            outdated
        ));
    }
    
    matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    matches.truncate(limit);
    
    if matches.is_empty() {
        ui::warn("No matches found");
        return Ok(());
    }
    
    ui::header("Results");
    
    for (i, m) in matches.iter().enumerate() {
        let path = Path::new(&m.path);
        
        let link = ui::log::path_link(path);
        let percentage = (m.score * 100.0).round() as u32;
        
        println!(
            "{}. {} {} {}",
            format!("{:2}", i + 1).bright_blue().bold(),
            link.bright_blue(),
            format!("{}%", percentage).dimmed(),
            if m.score > 0.8 { "🔥" } else { "" }
        );
    }
    
    println!();
    ui::success(&format!("Found {} matches", matches.len()));
    
    if open_first && !matches.is_empty() {
        if let Err(e) = open::that(&matches[0].path) {
            ui::warn(&format!("Failed to open: {}", e));
        }
    }
    
    Ok(())
}
