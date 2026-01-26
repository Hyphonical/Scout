use clap::{builder::Styles, Parser, Subcommand, ValueEnum};
use colored::Colorize;
use std::path::PathBuf;

/// Execution provider for ONNX Runtime
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum Provider {
	/// Auto-detect best available (TensorRT → CUDA → CoreML → CPU)
	#[default]
	Auto,
	/// CPU only
	Cpu,
	/// NVIDIA CUDA GPU
	Cuda,
	/// NVIDIA TensorRT (optimized inference)
	Tensorrt,
	/// Apple CoreML (macOS only)
	Coreml,
}

fn parse_weight(s: &str) -> Result<f32, String> {
	let val: f32 = s.parse().map_err(|_| format!("'{}' is not a valid number", s))?;
	if val < 0.0 || val > 1.0 {
		Err(format!("weight must be between 0.0 and 1.0, got {}", val))
	} else {
		Ok(val)
	}
}

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
  {scout} {search}  {search_img_args}      {search_img_desc}
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
		search_img_args = "\"green\" -i car.png -w 0.3",
		search_img_desc = "Combined text + image".dimmed(),
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

	/// Execution provider: auto, cpu, cuda, coreml
	#[arg(short = 'p', long = "provider", global = true, default_value = "auto")]
	pub provider: Provider,

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

	/// Search images by text description and/or reference image
	Search {
		/// Search query (text description)
		#[arg(value_name = "QUERY")]
		query: Option<String>,

		/// Reference image to find similar images
		#[arg(short = 'i', long = "image", value_name = "PATH")]
		image: Option<PathBuf>,

		/// Weight for text vs image (0.0 = image only, 1.0 = text only, 0.5 = balanced)
		#[arg(short = 'w', long = "weight", default_value_t = 0.5, value_parser = parse_weight)]
		weight: f32,

		/// Directory to search
		#[arg(short = 'd', long = "dir", default_value = ".")]
		directory: PathBuf,

		/// Search directories recursively
		#[arg(short = 'r', long = "recursive")]
		recursive: bool,

		/// Number of results
		#[arg(short = 'n', long = "limit", default_value_t = 10)]
		limit: usize,

		/// Minimum similarity score (0.0-1.0)
		#[arg(short = 's', long = "score", default_value_t = 0.0)]
		min_score: f32,

		/// Open best match in default viewer
		#[arg(short = 'o', long = "open")]
		open: bool,

		/// Include the reference image in results (useful for duplicate detection)
		#[arg(long = "include-ref")]
		include_ref: bool,
	},

	/// Live interactive search in terminal
	Live {
		/// Directory to search
		#[arg(short = 'd', long = "dir", default_value = ".")]
		directory: PathBuf,

		/// Search directories recursively
		#[arg(short = 'r', long = "recursive")]
		recursive: bool,
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