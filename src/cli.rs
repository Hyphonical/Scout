//! Command-line interface definitions
//!
//! Defines the CLI structure, commands, and argument parsing using clap.

use clap::{builder::Styles, Parser, Subcommand, ValueEnum};
use colored::Colorize;
use std::path::PathBuf;

/// ONNX Runtime execution provider options
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum Provider {
	#[default]
	Auto,
	Cpu,
	Xnnpack,
	Cuda,
	Tensorrt,
	Coreml,
}

/// Validates weight parameter is in range [0.0, 1.0]
fn parse_weight(s: &str) -> Result<f32, String> {
	let val: f32 = s.parse().map_err(|_| format!("'{}' is not a valid number", s))?;
	if (0.0..=1.0).contains(&val) {
		Ok(val)
	} else {
		Err(format!("weight must be [0.0, 1.0], got {}", val))
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
	about = "AI-powered semantic image and video search.",
	styles = styles(),
	disable_help_subcommand = true,
	after_help = format!(
		"{title}
  {scout} {scan}    {scan_args}                {scan_desc}
  {scout} {search}  {search_args}                   {search_desc}
  {scout} {search}  {search_img_args}      {search_img_desc}
  {scout} {help}    {help_args}                           {help_desc}
  {scout} {live}    {live_args}                   {live_desc}
  {scout} {clean}   {clean_args}                {clean_desc}",
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
		clean = "clean".yellow(),
		clean_args = "-d ./images/ -r",
		clean_desc = "Remove unindexed images".dimmed()
	),
)]
pub struct Cli {
	#[arg(short = 'v', long = "verbose", global = true)]
	pub verbose: bool,

	#[arg(short = 'p', long = "provider", global = true, default_value = "auto")]
	pub provider: Provider,

	#[command(subcommand)]
	pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
	Scan {
		#[arg(short = 'd', long = "dir", default_value = ".")]
		directory: PathBuf,

		#[arg(short = 'r', long = "recursive")]
		recursive: bool,

		#[arg(short = 'f', long = "force")]
		force: bool,

		#[arg(long = "min-width", default_value_t = 64)]
		min_width: u32,

		#[arg(long = "min-height", default_value_t = 64)]
		min_height: u32,

		#[arg(long = "min-size", default_value_t = 0)]
		min_size_kb: u64,

		#[arg(long = "max-size")]
		max_size_mb: Option<u64>,

		#[arg(long = "exclude", value_delimiter = ',')]
		exclude_patterns: Vec<String>,
	},

	Search {
		#[arg(value_name = "QUERY")]
		query: Option<String>,

		#[arg(short = 'i', long = "image", value_name = "PATH")]
		image: Option<PathBuf>,

		#[arg(short = 'w', long = "weight", default_value_t = 0.5, value_parser = parse_weight)]
		weight: f32,

		#[arg(short = 'd', long = "dir", default_value = ".")]
		directory: PathBuf,

		#[arg(short = 'r', long = "recursive")]
		recursive: bool,

		#[arg(short = 'n', long = "limit", default_value_t = 10)]
		limit: usize,

		#[arg(short = 's', long = "score", default_value_t = 0.0)]
		min_score: f32,

		#[arg(short = 'o', long = "open")]
		open: bool,

		#[arg(long = "include-ref")]
		include_ref: bool,
	},

	Live {
		#[arg(short = 'd', long = "dir", default_value = ".")]
		directory: PathBuf,

		#[arg(short = 'r', long = "recursive")]
		recursive: bool,
	},

	Help {
		subcommand: Option<String>,
	},

	Clean {
		#[arg(short = 'd', long = "dir", default_value = ".")]
		directory: PathBuf,

		#[arg(short = 'r', long = "recursive")]
		recursive: bool,

		#[arg(short = 'y', long = "yes", help = "Skip confirmation")]
		auto_confirm: bool,
	},
}
