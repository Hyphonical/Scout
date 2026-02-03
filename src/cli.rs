//! Command-line interface

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Provider {
	Auto,
	Cpu,
	Cuda,
	Tensorrt,
	#[value(name = "coreml")]
	CoreML,
	Xnnpack,
}

#[derive(Parser)]
#[command(name = "scout", version, about = "AI-powered semantic image search")]
pub struct Cli {
	#[arg(short, long, global = true, help = "Enable verbose logging")]
	pub verbose: bool,

	#[arg(short, long, global = true, help = "Recursively process directories")]
	pub recursive: bool,

	#[arg(long, global = true, value_enum, help = "Compute provider to use")]
	pub provider: Option<Provider>,

	#[arg(long, global = true, help = "Path to models directory")]
	pub model_dir: Option<PathBuf>,

	#[arg(long, global = true, help = "Path to FFmpeg binary")]
	pub ffmpeg_path: Option<PathBuf>,

	#[command(subcommand)]
	pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
	/// Index images in a directory
	Scan {
		#[arg(short, long, default_value = ".")]
		dir: PathBuf,

		#[arg(short, long)]
		force: bool,

		#[arg(long, help = "Minimum resolution (shortest side in pixels)")]
		min_resolution: Option<u32>,

		#[arg(long, help = "Maximum file size in MB")]
		max_size: Option<u64>,

		#[arg(long, help = "Skip video files")]
		exclude_videos: bool,
	},

	/// Search indexed images
	Search {
		/// Search query text (optional if using --image)
		query: Option<String>,

		#[arg(short, long, help = "Reference image path")]
		image: Option<PathBuf>,

		#[arg(
			short,
			long,
			default_value_t = 0.5,
			help = "Text weight when combining text+image (0.0-1.0)"
		)]
		weight: f32,

		#[arg(long, help = "Negative query to exclude")]
		not: Option<String>,

		#[arg(short, long, default_value = ".")]
		dir: PathBuf,

		#[arg(short = 'n', long, default_value_t = crate::config::DEFAULT_LIMIT)]
		limit: usize,

		#[arg(short, long, default_value_t = crate::config::DEFAULT_MIN_SCORE)]
		score: f32,

		#[arg(short, long)]
		open: bool,

		#[arg(long, help = "Include reference image in results")]
		include_ref: bool,

		#[arg(long, help = "Exclude videos from results")]
		exclude_videos: bool,
	},

	/// Cluster images by visual similarity
	Cluster {
		#[arg(short, long, default_value = ".")]
		dir: PathBuf,

		#[arg(short, long, help = "Force reclustering even if cached")]
		force: bool,

		#[arg(long, default_value_t = 5, help = "Minimum images per cluster")]
		min_cluster_size: usize,

		#[arg(long, help = "Minimum samples for core points")]
		min_samples: Option<usize>,

		#[arg(
			long,
			help = "Use UMAP dimensionality reduction (faster for large datasets)"
		)]
		use_umap: bool,
	},

	/// Remove orphaned sidecars
	Clean {
		#[arg(short, long, default_value = ".")]
		dir: PathBuf,
	},

	/// Watch directory for changes and auto-index
	Watch {
		#[arg(short, long, default_value = ".")]
		dir: PathBuf,

		#[arg(long, help = "Minimum resolution (shortest side in pixels)")]
		min_resolution: Option<u32>,

		#[arg(long, help = "Maximum file size in MB")]
		max_size: Option<u64>,

		#[arg(long, help = "Skip video files")]
		exclude_videos: bool,
	},
}
