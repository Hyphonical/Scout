//! # Command-Line Interface
//!
//! Defines CLI structure using clap derive macros.
//! All commands and global flags are declared here.

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
	/// Index media files in a directory
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

		#[arg(long, help = "Maximum frames to extract per video")]
		max_frames: Option<usize>,

		#[arg(long, help = "Scene detection threshold (0.0-1.0)")]
		scene_threshold: Option<f32>,
	},

	/// Search indexed media
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

		#[arg(long, help = "Output only paths to stdout")]
		paths: bool,

		#[arg(long, help = "Export results as JSON to file")]
		export: Option<PathBuf>,
	},

	/// Cluster media by visual similarity
	Cluster {
		#[arg(short, long, default_value = ".")]
		dir: PathBuf,

		#[arg(short, long, help = "Force reclustering even if cached")]
		force: bool,

		#[arg(long, default_value_t = crate::config::DEFAULT_MIN_CLUSTER_SIZE, help = "Minimum images per cluster")]
		min_cluster_size: usize,

		#[arg(long, help = "Minimum samples for core points")]
		min_samples: Option<usize>,

		#[arg(long, default_value_t = crate::config::DEFAULT_COHESION_THRESHOLD, help = "Minimum cohesion threshold (0.0-1.0)")]
		threshold: f32,

		#[arg(
			long,
			help = "Use UMAP dimensionality reduction (faster for large datasets)"
		)]
		use_umap: bool,

		#[arg(long, default_value_t = crate::config::DEFAULT_UMAP_NEIGHBORS, help = "UMAP n_neighbors parameter")]
		umap_neighbors: usize,

		#[arg(long, default_value_t = crate::config::DEFAULT_UMAP_COMPONENTS, help = "UMAP n_components (target dimensions)")]
		umap_components: usize,

		#[arg(
			short = 'p',
			long,
			default_value_t = crate::config::DEFAULT_CLUSTER_PREVIEW,
			help = "Number of images to show per cluster"
		)]
		preview_count: i32,

		#[arg(long, help = "Export results as JSON to file")]
		export: Option<PathBuf>,
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

		#[arg(long, help = "Maximum frames to extract per video")]
		max_frames: Option<usize>,

		#[arg(long, help = "Scene detection threshold (0.0-1.0)")]
		scene_threshold: Option<f32>,
	},

	/// Find statistically unusual media (outliers)
	Outliers {
		#[arg(short, long, default_value = ".")]
		dir: PathBuf,

		#[arg(short = 'n', long, default_value_t = crate::config::DEFAULT_OUTLIER_PREVIEW, help = "Number of outliers to show")]
		limit: usize,

		#[arg(short = 'k', long, default_value_t = crate::config::DEFAULT_OUTLIER_NEIGHBORS, help = "Number of neighbors for LOF")]
		neighbors: usize,

		#[arg(long, help = "Export results as JSON to file")]
		export: Option<PathBuf>,
	},
}
