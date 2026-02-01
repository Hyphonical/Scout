//! Unified logging system

use colored::*;
use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE: AtomicBool = AtomicBool::new(false);

const SLOGANS: &[&str] = &[
    "Finding pixels in a digital haystack",
    "Teaching computers to see your vibe",
    "Semantic search, but make it fast",
    "Your images, now searchable",
    "AI-powered. Human-approved.",
    "Making CTRL+F jealous since 2024",
    "Where embeddings meet aesthetics",
];

pub fn random_slogan() -> &'static str {
    let idx = rand::rng().random_range(0..SLOGANS.len());
    SLOGANS[idx]
}

pub struct Log;

impl Log {
    pub fn set_verbose(enabled: bool) {
        VERBOSE.store(enabled, Ordering::Relaxed);
    }
    
    pub fn is_verbose() -> bool {
        VERBOSE.load(Ordering::Relaxed)
    }
}

pub fn info(msg: &str) {
    println!("{} {}", "ℹ".bright_blue().bold(), msg.white());
}

pub fn success(msg: &str) {
    println!("{} {}", "✓".bright_blue().bold(), msg.white());
}

pub fn warn(msg: &str) {
    println!("{} {}", "⚠".bright_blue().bold(), msg.white());
}

pub fn error(msg: &str) {
    println!("{} {}", "✗".red().bold(), msg.white());
}

pub fn debug(msg: &str) {
    if Log::is_verbose() {
        println!("{} {}", "⚙".bright_black().bold(), msg.dimmed());
    }
}

pub fn header(text: &str) {
    println!("\n{}", text.bright_blue().bold());
}

/// Clickable file path (OSC 8 terminal hyperlink)
pub fn path_link(path: &std::path::Path) -> String {
    let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let uri = format!("file://{}", absolute.display());
    format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", uri, path.display())
}
