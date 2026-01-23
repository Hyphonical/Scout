// Config - Application constants and path utilities

use std::path::PathBuf;

// Tagger model paths (relative to models/)
pub const TAGGER_DIR: &str = "tagger";
pub const TAGGER_MODEL: &str = "camie-tagger-v2.onnx";
pub const TAGGER_MAPPINGS: &str = "camie-tagger-v2-mappings.json";

// Embedder model paths (relative to models/)
pub const EMBED_DIR: &str = "clip";
pub const EMBED_MODEL: &str = "MiniLM-L6-v2.onnx";
pub const EMBED_TOKENIZER: &str = "tokenizer.json";
pub const EMBEDDING_DIM: usize = 384;

// Processing
pub const INPUT_SIZE: u32 = 512;
pub const MAX_TAGS: usize = 100;
pub const DEFAULT_THRESHOLD: f32 = 0.70;
pub const GPU_BATCH_THRESHOLD: usize = 3;

// Sidecar storage - hash-based layout: .scout/ab/abcdef123.json
pub const SIDECAR_DIR: &str = ".scout";
pub const HASH_BUFFER_SIZE: usize = 65536;

// Supported image formats
pub const IMAGE_EXTENSIONS: &[&str] = &[
	"jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "ico", "avif"
];

/// Finds the models directory by walking up from executable, then checking cwd.
pub fn find_models_dir() -> Option<PathBuf> {
	let exe_path = std::env::current_exe().ok()?;
	let mut current = exe_path.parent()?;

	for _ in 0..5 {
		let models = current.join("models");
		if models.is_dir() {
			return Some(models);
		}
		current = current.parent()?;
	}

	let cwd_models = std::env::current_dir().ok()?.join("models");
	cwd_models.is_dir().then_some(cwd_models)
}

pub fn get_tagger_model_path() -> Option<PathBuf> {
	let models = find_models_dir()?;
	let path = models.join(TAGGER_DIR).join(TAGGER_MODEL);
	
	// Fallback to old flat structure for backwards compatibility
	if path.exists() {
		Some(path)
	} else {
		let legacy = models.join("camie-tagger-v2.onnx");
		legacy.exists().then_some(legacy)
	}
}

pub fn get_tagger_mappings_path() -> Option<PathBuf> {
	let models = find_models_dir()?;
	let path = models.join(TAGGER_DIR).join(TAGGER_MAPPINGS);
	
	if path.exists() {
		Some(path)
	} else {
		let legacy = models.join("camie-tagger-v2-mappings.json");
		legacy.exists().then_some(legacy)
	}
}
