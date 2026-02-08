//! # Scout Library
//!
//! AI-powered semantic media search using CLIP/SigLIP2 embeddings.
//! Provides image and video indexing, semantic search, and clustering.
//!
//! # Library Usage
//!
//! Scout can be used as a library in your own Rust programs to encode images
//! and text into semantic embeddings, and compare them.
//!
//! ```no_run
//! use scout::{Scout, Embedding};
//!
//! let mut scout = Scout::builder()
//!     .model_dir("path/to/models")
//!     .build()
//!     .expect("Failed to initialize Scout");
//!
//! // Encode an image from bytes (e.g. downloaded from a URL)
//! let image_embedding = scout.encode_image_bytes(&image_bytes)
//!     .expect("Failed to encode image");
//!
//! // Encode a text query
//! let text_embedding = scout.encode_text("a cat sitting on a chair")
//!     .expect("Failed to encode text");
//!
//! // Compare them (cosine similarity, 0.0 to 1.0)
//! let score = text_embedding.similarity(&image_embedding);
//! println!("Similarity: {:.2}%", score * 100.0);
//! ```
//!
//! # Storing Embeddings
//!
//! [`Embedding`] implements `Serialize` and `Deserialize`, so you can store
//! them however you like (JSON, MessagePack, a database, etc.):
//!
//! ```no_run
//! let json = serde_json::to_string(&embedding).unwrap();
//! let restored: scout::Embedding = serde_json::from_str(&json).unwrap();
//! ```

pub mod cli;
pub mod commands;
pub mod config;
pub mod core;
pub mod models;
pub mod processing;
pub mod runtime;
pub mod storage;
pub mod ui;

// === Public Library API ===

pub use crate::core::Embedding;
pub use crate::cli::Provider;

/// Re-export the `image` crate so library consumers can use `scout::image::DynamicImage`
/// without adding `image` as a separate dependency.
pub use image;

use anyhow::{Context, Result};
use std::path::PathBuf;

/// The main entry point for using Scout as a library.
///
/// Wraps the ONNX vision and text models with a clean, ergonomic API.
/// Construct one via [`Scout::builder()`].
pub struct Scout {
	models: models::Models,
}

/// Builder for configuring and constructing a [`Scout`] instance.
///
/// # Example
/// ```no_run
/// let scout = scout::Scout::builder()
///     .model_dir("./models")
///     .provider(scout::Provider::Cuda)
///     .verbose(true)
///     .build()?;
/// ```
pub struct ScoutBuilder {
	model_dir: Option<PathBuf>,
	vision_path: Option<PathBuf>,
	text_path: Option<PathBuf>,
	tokenizer_path: Option<PathBuf>,
	provider: Option<Provider>,
	verbose: bool,
}

impl Scout {
	/// Create a new builder to configure Scout.
	pub fn builder() -> ScoutBuilder {
		ScoutBuilder {
			model_dir: None,
			vision_path: None,
			text_path: None,
			tokenizer_path: None,
			provider: None,
			verbose: false,
		}
	}

	/// Encode an [`image::DynamicImage`] into an [`Embedding`].
	pub fn encode_image(&mut self, image: &image::DynamicImage) -> Result<Embedding> {
		self.models.encode_image(image)
	}

	/// Load an image from raw bytes (JPEG, PNG, WebP, etc.) and encode it.
	///
	/// This is typically what you want when downloading images from URLs.
	pub fn encode_image_bytes(&mut self, bytes: &[u8]) -> Result<Embedding> {
		let image = image::load_from_memory(bytes)
			.context("Failed to decode image from bytes")?;
		self.models.encode_image(&image)
	}

	/// Encode a text query into an [`Embedding`].
	///
	/// The text is tokenized and run through the SigLIP2 text encoder.
	/// For best results, use descriptive phrases like "a red car on a highway".
	pub fn encode_text(&mut self, text: &str) -> Result<Embedding> {
		self.models.encode_text(text)
	}

	/// Find the best matches from a list of candidate embeddings.
	///
	/// Returns indices and scores sorted by descending similarity,
	/// filtered to those above `min_score`.
	///
	/// # Example
	/// ```no_run
	/// let query = scout.encode_text("sunset over water")?;
	/// let matches = scout.search(&query, &stored_embeddings, 10, 0.05);
	/// for (index, score) in matches {
	///     println!("#{}: {:.1}%", index, score * 100.0);
	/// }
	/// ```
	pub fn search(
		&self,
		query: &Embedding,
		candidates: &[Embedding],
		limit: usize,
		min_score: f32,
	) -> Vec<(usize, f32)> {
		let mut results: Vec<(usize, f32)> = candidates
			.iter()
			.enumerate()
			.map(|(i, emb)| (i, query.similarity(emb)))
			.filter(|(_, score)| *score >= min_score)
			.collect();

		results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
		results.truncate(limit);
		results
	}
}

impl ScoutBuilder {
	/// Set the directory containing model files.
	///
	/// The directory should contain:
	/// - `vision_model_q4f16.onnx`
	/// - `text_model_q4f16.onnx`
	/// - `tokenizer.json`
	pub fn model_dir(mut self, path: impl Into<PathBuf>) -> Self {
		self.model_dir = Some(path.into());
		self
	}

	/// Set individual model file paths (overrides `model_dir`).
	pub fn vision_model(mut self, path: impl Into<PathBuf>) -> Self {
		self.vision_path = Some(path.into());
		self
	}

	/// Set individual model file paths (overrides `model_dir`).
	pub fn text_model(mut self, path: impl Into<PathBuf>) -> Self {
		self.text_path = Some(path.into());
		self
	}

	/// Set individual model file paths (overrides `model_dir`).
	pub fn tokenizer(mut self, path: impl Into<PathBuf>) -> Self {
		self.tokenizer_path = Some(path.into());
		self
	}

	/// Set the hardware execution provider (CUDA, TensorRT, CoreML, etc.).
	///
	/// Defaults to `Provider::Auto` which picks the best available.
	pub fn provider(mut self, provider: Provider) -> Self {
		self.provider = Some(provider);
		self
	}

	/// Enable or disable verbose logging to stderr.
	///
	/// Defaults to `false` (quiet) for library use.
	pub fn verbose(mut self, enabled: bool) -> Self {
		self.verbose = enabled;
		self
	}

	/// Build the [`Scout`] instance, loading model metadata and validating paths.
	///
	/// The actual ONNX models are lazy-loaded on first use (first `encode_*` call).
	pub fn build(self) -> Result<Scout> {
		// Configure verbose logging
		ui::Log::set_verbose(self.verbose);

		// Configure provider if set
		if let Some(provider) = self.provider {
			runtime::set_provider(provider);
		}

		// Build models from explicit paths or model_dir
		let models = if self.vision_path.is_some() || self.text_path.is_some() || self.tokenizer_path.is_some() {
			// Use individual paths (all three must be set)
			let vision = self.vision_path.context(
				"vision_model path required when using individual model paths",
			)?;
			let text = self.text_path.context(
				"text_model path required when using individual model paths",
			)?;
			let tokenizer = self.tokenizer_path.context(
				"tokenizer path required when using individual model paths",
			)?;
			let mut m = models::Models::with_paths(vision, text, tokenizer)?;
			m.quiet = !self.verbose;
			m
		} else if let Some(dir) = self.model_dir {
			let mut m = models::Models::from_dir(dir)?;
			m.quiet = !self.verbose;
			m
		} else {
			// Fall back to auto-discovery (env var, exe dir)
			let mut m = models::Models::new()
				.context("Could not find models. Set model_dir() or the SCOUT_MODELS_DIR env var")?;
			m.quiet = !self.verbose;
			m
		};

		Ok(Scout { models })
	}
}
