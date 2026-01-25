// Logger - Colored console output with timestamps

use chrono::Local;
use colored::*;

#[derive(Clone, Copy)]
pub enum Level {
	Info,
	Success,
	Warning,
	Error,
	Debug,
}

/// Prints a timestamped, colored log message to stdout.
pub fn log(level: Level, message: &str) {
	let time = Local::now().format("%H:%M:%S").to_string().dimmed();
	let icon = match level {
		Level::Info =>    "ℹ".blue().bold(),
		Level::Success => "✔".bright_green().bold(),
		Level::Warning => "⚠".yellow().bold(),
		Level::Error =>   "✘".red().bold(),
		Level::Debug =>   "⚙".bright_blue().bold(),
	};
	println!("[{}] {} {}", time, icon, message);
}

/// Prints a section header with visual separation.
pub fn header(title: &str) {
	println!();
	println!("{}", format!("─── {} ───", title).bright_blue().bold());
}

/// Prints a processing summary with statistics.
pub fn summary(processed: usize, skipped: usize, errors: usize, duration_secs: f32) {
	println!();
	header("Summary");

	if processed > 0 {
		println!("  {} {}", "Processed:".bright_blue(), processed);
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