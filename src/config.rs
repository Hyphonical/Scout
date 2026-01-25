// Config - Application constants and path utilities

use std::path::PathBuf;

// SigLIP2 model paths (all in models/)
pub const VISION_MODEL: &str = "vision_model_q4f16.onnx";
pub const TEXT_MODEL: &str = "text_model_q4f16.onnx";
pub const TOKENIZER: &str = "tokenizer.json";

// SigLIP2 parameters
pub const INPUT_SIZE: u32 = 512;
pub const EMBEDDING_DIM: usize = 1024;

// Interactive debounce time (ms)
pub const DEBOUNCE_TIME_MS: u64 = 400;

// Sidecar storage: .scout/ab/abcdef123.json
pub const SIDECAR_DIR: &str = ".scout";
pub const HASH_BUFFER_SIZE: usize = 65536;

// Supported image formats
pub const IMAGE_EXTENSIONS: &[&str] = &[
	"jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "ico", "avif",
];

/// Finds the models directory by walking up from executable, then checking cwd.
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

pub fn get_vision_model_path() -> Option<PathBuf> {
	let models = find_models_dir()?;
	let path = models.join(VISION_MODEL);
	path.exists().then_some(path)
}

pub fn get_text_model_path() -> Option<PathBuf> {
	let models = find_models_dir()?;
	let path = models.join(TEXT_MODEL);
	path.exists().then_some(path)
}

pub fn get_tokenizer_path() -> Option<PathBuf> {
	let models = find_models_dir()?;
	let path = models.join(TOKENIZER);
	path.exists().then_some(path)
}
