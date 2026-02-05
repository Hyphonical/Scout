//! Application configuration and constants

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static CUSTOM_MODEL_DIR: OnceLock<PathBuf> = OnceLock::new();
static MODEL_DIR_LOGGED: AtomicBool = AtomicBool::new(false);

// === Model Files ===
pub const VISION_MODEL: &str = "vision_model_q4f16.onnx";
pub const TEXT_MODEL: &str = "text_model_q4f16.onnx";
pub const TOKENIZER: &str = "tokenizer.json";

// === Model Parameters ===
pub const INPUT_SIZE: u32 = 512;
pub const EMBEDDING_DIM: usize = 1024; // SigLIP2
pub const MAX_QUERY_TOKENS: usize = 64; // SigLIP2 text encoder max sequence length

// === Storage ===
pub const SIDECAR_DIR: &str = ".scout";
pub const SIDECAR_EXT: &str = "msgpack";
pub const CLUSTERS_FILE: &str = "clusters.msgpack";

// === File Extensions ===
pub const IMAGE_EXTENSIONS: &[&str] = &[
	"jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "ico", "avif",
];

pub const VIDEO_EXTENSIONS: &[&str] = &[
	"mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg",
];

/// Maximum number of frames to extract from videos (used with scene detection)
pub const MAX_VIDEO_FRAMES: usize = 15;

/// Scene detection threshold (0.0-1.0). Lower = more sensitive
pub const SCENE_THRESHOLD: f32 = 0.3;

// === Search Defaults ===
pub const DEFAULT_LIMIT: usize = 10;
pub const DEFAULT_MIN_SCORE: f32 = 0.05;
pub const NEGATIVE_WEIGHT: f32 = 0.7;

// === Cluster Defaults ===
pub const DEFAULT_MIN_CLUSTER_SIZE: usize = 5;
pub const DEFAULT_COHESION_THRESHOLD: f32 = 0.70;
pub const DEFAULT_UMAP_NEIGHBORS: usize = 50;
pub const DEFAULT_UMAP_COMPONENTS: usize = 64;
pub const DEFAULT_CLUSTER_PREVIEW: i32 = 5;

pub fn set_model_dir(path: PathBuf) {
	let _ = CUSTOM_MODEL_DIR.set(path);
}

/// Get models directory (same dir as executable, or SCOUT_MODELS_DIR env var)
pub fn models_dir() -> Option<PathBuf> {
	// Check custom model dir
	if let Some(custom) = CUSTOM_MODEL_DIR.get() {
		if !MODEL_DIR_LOGGED.swap(true, Ordering::Relaxed) {
			crate::ui::debug(&format!("Using custom model dir: {}", custom.display()));
		}
		return Some(custom.clone());
	}

	// Check environment variable
	if let Ok(env_path) = std::env::var("SCOUT_MODELS_DIR") {
		let path = PathBuf::from(&env_path);
		if path.is_dir() {
			if !MODEL_DIR_LOGGED.swap(true, Ordering::Relaxed) {
				crate::ui::debug(&format!("Using SCOUT_MODELS_DIR: {}", env_path));
			}
			return Some(path);
		}
	}

	// Check next to executable
	if let Ok(exe) = std::env::current_exe() {
		if let Some(dir) = exe.parent() {
			let models = dir.join("models");
			if models.is_dir() {
				if !MODEL_DIR_LOGGED.swap(true, Ordering::Relaxed) {
					crate::ui::debug(&format!("Found models at: {}", models.display()));
				}
				return Some(models);
			}
		}
	}

	None
}

pub fn get_vision_model_path() -> Option<PathBuf> {
	models_dir().map(|d| d.join(VISION_MODEL))
}

pub fn get_text_model_path() -> Option<PathBuf> {
	models_dir().map(|d| d.join(TEXT_MODEL))
}

pub fn get_tokenizer_path() -> Option<PathBuf> {
	models_dir().map(|d| d.join(TOKENIZER))
}
