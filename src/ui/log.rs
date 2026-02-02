//! Unified logging system

use colored::*;
use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE: AtomicBool = AtomicBool::new(false);

const LOGO: &str = r#"
   _____                  __ 
  / ___/_________  __  __/ /_
  \__ \/ ___/ __ \/ / / / __/
 ___/ / /__/ /_/ / /_/ / /_  
/____/\___/\____/\____/\__/  "#;

const SLOGANS: &[&str] = &[
	"Semantic search but cooler",
	"Where embeddings meet aesthetics",
	"BEEP. BOOP. Done!",
	"Powered by some oxidizing C clone",
	"We make IMG_404 found",
	"@Grok, where is this image?",
	"Because CTRL+F is soo 2010s",
	"Enhance! Enhance!",
	"I know what you did last screenshot °_°",
	"This folder better contain cats! =^..^=",
	"That's not SFW...",
	"\"Trust me bro, it's in here\"",
	"Ahw a Chihuahua!... Oh no wait, it's a muffin",
];

pub fn random_slogan() -> &'static str {
	let idx = rand::rng().random_range(0..SLOGANS.len());
	SLOGANS[idx]
}

pub fn print_logo() {
	println!("{}", LOGO.bright_blue().bold());
	println!("{}", random_slogan().dimmed().italic());
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
	println!("{} {}", "ℹ".bright_blue().bold(), msg.bright_white());
}

pub fn success(msg: &str) {
	println!("{} {}", "✓".bright_green().bold(), msg.bright_white());
}

pub fn warn(msg: &str) {
	println!("{} {}", "⚠".bright_yellow().bold(), msg.bright_white());
}

pub fn error(msg: &str) {
	println!("{} {}", "✗".bright_red().bold(), msg.bright_white());
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
pub fn path_link(path: &std::path::Path, max_len: usize) -> String {
	let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

	let uri = if cfg!(windows) {
		let path_str = absolute.to_string_lossy();
		let cleaned = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);
		format!("file:///{}", cleaned.replace('\\', "/"))
	} else {
		format!("file://{}", absolute.display())
	};

	let filename = path
		.file_name()
		.and_then(|n| n.to_str())
		.unwrap_or("unknown");

	let display_name = if filename.len() > max_len {
		format!(
			"{}...{}",
			&filename[..max_len / 2],
			&filename[filename.len() - (max_len / 2 - 3)..]
		)
	} else {
		filename.to_string()
	};

	format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", uri, display_name)
}

/// Log a processed file with bright white filename and dimmed time
pub fn file_processed(path: &std::path::Path, duration_ms: u128) {
	let link = path_link(path, 60);
	info(&format!(
		"{} {}",
		link.bright_white(),
		format!("{}ms", duration_ms).dimmed()
	));
}
