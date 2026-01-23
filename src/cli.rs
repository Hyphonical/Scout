// CLI - Command-line interface with subcommands for scanning and searching

use clap::{builder::Styles, Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

use crate::config::DEFAULT_THRESHOLD;

fn styles() -> Styles {
	Styles::styled()
		.header(anstyle::Style::new().bold().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Cyan))))
		.usage(anstyle::Style::new().bold().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Cyan))))
		.literal(anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))))
		.placeholder(anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))))
		.valid(anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))))
		.invalid(anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))))
}

#[derive(Parser, Debug)]
#[command(
	name = "scout",
	author,
	version,
	about = "AI-powered image tagging and search",
	styles = styles(),
	after_help = format!(
		"{}\n  {} {} ./images/ -r          {}\n  {} {} \"sword -armor\"        {}\n  {} {}                        {}",
		"Examples:".cyan().bold(),
		"scout".green(), "scan".yellow(),   "Tag images recursively".dimmed(),
		"scout".green(), "search".yellow(), "Exclude armor results".dimmed(),
		"scout".green(), "stats".yellow(),  "Show tag statistics".dimmed(),
	),
)]
pub struct Cli {
	#[command(subcommand)]
	pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
	/// Scan and tag images using the ONNX model
	Scan {
		/// Image files, glob patterns, or directories
		#[arg(value_name = "PATH", required = true, num_args = 1..)]
		inputs: Vec<PathBuf>,

		/// Scan directories recursively
		#[arg(short = 'r', long = "recursive")]
		recursive: bool,

		/// Minimum confidence for tags (0.0-1.0)
		#[arg(short = 't', long = "threshold", default_value_t = DEFAULT_THRESHOLD)]
		threshold: f32,

		/// Re-process already tagged images
		#[arg(short = 'f', long = "force")]
		force: bool,

		/// Show all detected tags for each image
		#[arg(short = 'v', long = "verbose")]
		verbose: bool,
	},

	/// Search tagged images by keywords
	#[command(after_help = format!(
		"{}\n  {} : include tag\n  {} : exclude tag\n  {} : fuzzy match (Levenshtein ≤2)\n  {} : wildcard\n  {} : match any\n  {} : semantic similarity",
		"Query Syntax:".cyan().bold(),
		"word".green(),
		"-word".red(),
		"word~".yellow(),
		"wo*rd".yellow(),
		"(a ~ b)".yellow(),
		"--semantic".cyan(),
	))]
	Search {
		/// Search query (space-separated keywords)
		#[arg(value_name = "QUERY", required = true)]
		query: String,

		/// Directory to search (default: current directory)
		#[arg(short = 'd', long = "dir", default_value = ".")]
		directory: PathBuf,

		/// Number of results to show
		#[arg(short = 'n', long = "limit", default_value_t = 10)]
		limit: usize,

		/// Minimum match score (0.0-1.0)
		#[arg(short = 's', long = "score", default_value_t = 0.1)]
		min_score: f32,

		/// Use semantic similarity (requires embeddings)
		#[arg(long = "semantic")]
		semantic: bool,

		/// Open the best match in default viewer
		#[arg(short = 'o', long = "open")]
		open: bool,
	},

	/// Show tag statistics across all indexed images
	Stats {
		/// Directory to analyze (default: current directory)
		#[arg(short = 'd', long = "dir", default_value = ".")]
		directory: PathBuf,

		/// Number of top tags to show
		#[arg(short = 'n', long = "limit", default_value_t = 20)]
		limit: usize,
	},
}
