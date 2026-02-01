//! Application configuration and constants

use std::path::PathBuf;
use std::sync::OnceLock;

static CUSTOM_MODEL_DIR: OnceLock<PathBuf> = OnceLock::new();
static CUSTOM_VISION: OnceLock<PathBuf> = OnceLock::new();
static CUSTOM_TEXT: OnceLock<PathBuf> = OnceLock::new();
static CUSTOM_TOKENIZER: OnceLock<PathBuf> = OnceLock::new();

// === Model Files ===
pub const VISION_MODEL: &str = "vision_model_q4f16.onnx";
pub const TEXT_MODEL: &str = "text_model_q4f16.onnx";
pub const TOKENIZER: &str = "tokenizer.json";

// === Model Parameters ===
pub const INPUT_SIZE: u32 = 512;
pub const EMBEDDING_DIM: usize = 1024;

// === Storage ===
pub const SIDECAR_DIR: &str = ".scout";
pub const SIDECAR_EXT: &str = "msgpack";

// === File Extensions ===
pub const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "ico", "avif",
];

// === Search Defaults ===
pub const DEFAULT_LIMIT: usize = 10;
pub const DEFAULT_MIN_SCORE: f32 = 0.0;
pub const NEGATIVE_WEIGHT: f32 = 0.7;

pub fn set_model_dir(path: PathBuf) {
    let _ = CUSTOM_MODEL_DIR.set(path);
}

pub fn set_vision_model(path: PathBuf) {
    let _ = CUSTOM_VISION.set(path);
}

pub fn set_text_model(path: PathBuf) {
    let _ = CUSTOM_TEXT.set(path);
}

pub fn set_tokenizer(path: PathBuf) {
    let _ = CUSTOM_TOKENIZER.set(path);
}

/// Get models directory (same dir as executable, or SCOUT_MODELS_DIR env var)
pub fn models_dir() -> Option<PathBuf> {
    // Check custom model dir first
    if let Some(custom) = CUSTOM_MODEL_DIR.get() {
        crate::ui::debug(&format!("Using custom model dir: {}", custom.display()));
        return Some(custom.clone());
    }
    
    // Check environment variable first
    if let Ok(env_path) = std::env::var("SCOUT_MODELS_DIR") {
        let path = PathBuf::from(&env_path);
        if path.is_dir() {
            crate::ui::debug(&format!("Using SCOUT_MODELS_DIR: {}", env_path));
            return Some(path);
        }
    }
    
    // Check next to executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let models = dir.join("models");
            if models.is_dir() {
                crate::ui::debug(&format!("Found models at: {}", models.display()));
                return Some(models);
            }
        }
    }
    
    None
}

pub fn get_vision_model_path() -> Option<PathBuf> {
    if let Some(custom) = CUSTOM_VISION.get() {
        return Some(custom.clone());
    }
    models_dir().map(|d| d.join(VISION_MODEL))
}

pub fn get_text_model_path() -> Option<PathBuf> {
    if let Some(custom) = CUSTOM_TEXT.get() {
        return Some(custom.clone());
    }
    models_dir().map(|d| d.join(TEXT_MODEL))
}

pub fn get_tokenizer_path() -> Option<PathBuf> {
    if let Some(custom) = CUSTOM_TOKENIZER.get() {
        return Some(custom.clone());
    }
    models_dir().map(|d| d.join(TOKENIZER))
}
