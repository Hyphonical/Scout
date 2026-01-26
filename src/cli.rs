use clap::{builder::Styles, Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

fn styles() -> Styles {
	Styles::styled()
		.header(anstyle::Style::new().bold().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Blue))))
		.usage(anstyle::Style::new().bold().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Blue))))
		.literal(anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Blue))))
		.placeholder(anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))))
		.valid(anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Blue))))
		.invalid(anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))))
}

#[derive(Parser, Debug)]
#[command(
	name = "scout",
	author,
	version,
	about = "AI-powered semantic image search",
	styles = styles(),
	disable_help_subcommand = true,
	after_help = format!(
		"{title}
  {scout} {scan}    {scan_args}   {scan_desc}
  {scout} {search}  {search_args}      {search_desc}
  {scout} {help}    {help_args}              {help_desc},
  {scout} {live}    {live_args}      {live_desc}",
		title = "Examples:".bright_blue().bold(),
		scout = "scout".bright_blue(),
		scan = "scan".yellow(),
		scan_args = "-d ./images/ -r",
		scan_desc = "Index images recursively".dimmed(),
		search = "search".yellow(),
		search_args = "-d ./images/",
		search_desc = "Search by description".dimmed(),
		help = "help".yellow(),
		help_args = "scan",
		help_desc = "Show help for scan".dimmed(),
		live = "live".yellow(),
		live_args = "-d ./images/",
		live_desc = "Live search in terminal".dimmed(),
	),
)]
pub struct Cli {
	/// Enable verbose debug output
	#[arg(short = 'v', long = "verbose", global = true)]
	pub verbose: bool,

	/// Force CPU execution (no GPU acceleration)
	#[arg(long = "cpu", global = true, conflicts_with_all = ["cuda", "coreml"])]
	pub cpu: bool,

	/// Force CUDA execution (NVIDIA GPU)
	#[arg(long = "cuda", global = true, conflicts_with_all = ["cpu", "coreml"])]
	pub cuda: bool,

	/// Force CoreML execution (Apple Silicon)
	#[arg(long = "coreml", global = true, conflicts_with_all = ["cpu", "cuda"])]
	pub coreml: bool,

	#[command(subcommand)]
	pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
	/// Index images by generating embeddings
	Scan {
		/// Directory to scan
		#[arg(short = 'd', long = "dir", default_value = ".")]
		directory: PathBuf,

		/// Scan directories recursively
		#[arg(short = 'r', long = "recursive")]
		recursive: bool,

		/// Re-process already indexed images
		#[arg(short = 'f', long = "force")]
		force: bool,

		/// Minimum image width in pixels (default: 64)
		#[arg(long = "min-width", default_value_t = 64)]
		min_width: u32,

		/// Minimum image height in pixels (default: 64)
		#[arg(long = "min-height", default_value_t = 64)]
		min_height: u32,

		/// Minimum file size in KB (default: 0)
		#[arg(long = "min-size", default_value_t = 0)]
		min_size_kb: u64,

		/// Maximum file size in MB (default: unlimited)
		#[arg(long = "max-size")]
		max_size_mb: Option<u64>,

		/// Skip images matching these patterns (comma-separated, e.g., "thumb,icon,avatar")
		#[arg(long = "exclude", value_delimiter = ',')]
		exclude_patterns: Vec<String>,
	},

	/// Search images by text description
	Search {
		/// Search query
		#[arg(value_name = "QUERY", required = true)]
		query: String,

		/// Directory to search
		#[arg(short = 'd', long = "dir", default_value = ".")]
		directory: PathBuf,

		/// Number of results
		#[arg(short = 'n', long = "limit", default_value_t = 10)]
		limit: usize,

		/// Minimum similarity score (0.0-1.0)
		#[arg(short = 's', long = "score", default_value_t = 0.0)]
		min_score: f32,

		/// Open best match in default viewer
		#[arg(short = 'o', long = "open")]
		open: bool,
	},

	/// Live interactive search in terminal
	Live {
		/// Directory to search
		#[arg(short = 'd', long = "dir", default_value = ".")]
		directory: PathBuf,
	},

	/// Show help for a subcommand
	Help {
		/// Subcommand name
		subcommand: Option<String>,
	},
}

/// Filtering criteria for image scanning
#[derive(Debug, Clone)]
pub struct ScanFilters {
	pub min_width: u32,
	pub min_height: u32,
	pub min_size_kb: u64,
	pub max_size_mb: Option<u64>,
	pub exclude_patterns: Vec<String>,
}

impl ScanFilters {
	pub fn from_scan_command(
		min_width: u32,
		min_height: u32,
		min_size_kb: u64,
		max_size_mb: Option<u64>,
		exclude_patterns: Vec<String>,
	) -> Self {
		Self {
			min_width,
			min_height,
			min_size_kb,
			max_size_mb,
			exclude_patterns,
		}
	}
}