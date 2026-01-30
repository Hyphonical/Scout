//! Application configuration constants
//!
//! Centralized configuration for model files, paths, and runtime parameters.

use std::path::PathBuf;

pub const VISION_MODEL: &str = "vision_model_q4f16.onnx";
pub const TEXT_MODEL: &str = "text_model_q4f16.onnx";
pub const TOKENIZER: &str = "tokenizer.json";

pub const INPUT_SIZE: u32 = 512;
pub const EMBEDDING_DIM: usize = 1024;

pub const SIDECAR_DIR: &str = ".scout";
pub const SIDECAR_EXT: &str = "msgpack";

pub const DEBOUNCE_MS: u64 = 400;
pub const CURSOR_BLINK_MS: u64 = 530;

pub const LIVE_RESULTS_LIMIT: usize = 50;
pub const LIVE_INDEX_PROGRESS: usize = 100;

pub const SCORE_HIGH: f32 = 0.15;
pub const SCORE_MED: f32 = 0.08;

pub const IMAGE_EXTENSIONS: &[&str] = &[
	"jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "ico", "avif",
];

pub const VIDEO_EXTENSIONS: &[&str] = &[
	"mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg",
];

#[cfg(feature = "video")]
pub const VIDEO_FRAMES_TO_EXTRACT: usize = 10;

/// Locates the models directory by searching up to 5 levels from executable,
/// then falling back to current working directory
pub fn find_models_dir() -> Option<PathBuf> {
	if let Ok(exe) = std::env::current_exe() {
		let mut dir = exe.parent();
		for _ in 0..5 {
			if let Some(d) = dir {
				let models = d.join("models");
				if models.is_dir() {
					return Some(models);
				}
				dir = d.parent();
			} else {
				break;
			}
		}
	}

	let cwd = std::env::current_dir().ok()?.join("models");
	cwd.is_dir().then_some(cwd)
}

/// Returns the absolute path to the vision model file if it exists
pub fn get_vision_model_path() -> Option<PathBuf> {
	let models = find_models_dir()?;
	let path = models.join(VISION_MODEL);
	path.exists().then_some(path)
}

/// Returns the absolute path to the text model file if it exists
pub fn get_text_model_path() -> Option<PathBuf> {
	let models = find_models_dir()?;
	let path = models.join(TEXT_MODEL);
	path.exists().then_some(path)
}

/// Returns the absolute path to the tokenizer file if it exists
pub fn get_tokenizer_path() -> Option<PathBuf> {
	let models = find_models_dir()?;
	let path = models.join(TOKENIZER);
	path.exists().then_some(path)
}
