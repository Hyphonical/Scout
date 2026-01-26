// Config - Application constants

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
