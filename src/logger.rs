// Logger - Colored console output

use chrono::Local;
use colored::*;
use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn set_verbose(v: bool) {
	VERBOSE.store(v, Ordering::Relaxed);
}

pub fn is_verbose() -> bool {
	VERBOSE.load(Ordering::Relaxed)
}

#[derive(Clone, Copy, PartialEq)]
pub enum Level {
	Info,
	Success,
	Warning,
	Error,
	Debug,
}

pub fn log(level: Level, message: &str) {
	if level == Level::Debug && !is_verbose() {
		return;
	}

	let time = Local::now().format("%H:%M:%S").to_string().dimmed();
	let icon = match level {
		Level::Info => "ℹ".blue().bold(),
		Level::Success => "✔".bright_green().bold(),
		Level::Warning => "⚠".yellow().bold(),
		Level::Error => "✘".red().bold(),
		Level::Debug => "⚙".bright_blue().bold(),
	};
	println!("[{}] {} {}", time, icon, message);
}

pub fn header(title: &str) {
	println!();
	println!("{}", format!("─── {} ───", title).bright_blue().bold());
}

pub fn summary(processed: usize, skipped: usize, errors: usize, duration_secs: f32) {
	println!();
	header("Summary");

	if processed > 0 {
		println!("  {} {}", "Processed:".green(), processed);
	}
	if skipped > 0 {
		println!("  {} {}", "Skipped:".yellow(), skipped);
	}
	if errors > 0 {
		println!("  {} {}", "Errors:".red(), errors);
	}

	println!("  {} {:.2}s", "Duration:".bright_blue(), duration_secs);
	if processed > 0 {
		let avg_ms = (duration_secs * 1000.0) / processed as f32;
		println!("  {} {:.0}ms/image", "Average:".bright_blue(), avg_ms);
	}
	println!();
}