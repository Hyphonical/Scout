//! # Model Manager
//!
//! Lazy-loads vision and text models on first use.
//! Validates model paths and provides unified encoding interface.

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::config;
use crate::core::Embedding;

pub struct Models {
	vision: Option<super::vision::VisionModel>,
	text: Option<super::text::TextModel>,
	vision_path: PathBuf,
	text_path: PathBuf,
	tokenizer_path: PathBuf,
	/// If true, suppress UI output (for library use)
	pub(crate) quiet: bool,
}

impl Models {
	pub fn new() -> Result<Self> {
		let vision_path = config::get_vision_model_path().context(format!(
			"Vision model not found. Ensure {} exists",
			config::VISION_MODEL
		))?;
		let text_path = config::get_text_model_path().context(format!(
			"Text model not found. Ensure {} exists",
			config::TEXT_MODEL
		))?;
		let tokenizer_path = config::get_tokenizer_path().context(format!(
			"Tokenizer not found. Ensure {} exists",
			config::TOKENIZER
		))?;

		Self::validate_and_build(vision_path, text_path, tokenizer_path, false)
	}

	/// Create Models with explicit file paths (for library use).
	///
	/// - `vision_path`: path to the vision ONNX model
	/// - `text_path`: path to the text ONNX model
	/// - `tokenizer_path`: path to the tokenizer JSON
	pub fn with_paths(
		vision_path: PathBuf,
		text_path: PathBuf,
		tokenizer_path: PathBuf,
	) -> Result<Self> {
		Self::validate_and_build(vision_path, text_path, tokenizer_path, false)
	}

	/// Create Models from a directory containing all three model files.
	///
	/// Expects the directory to contain:
	/// - `vision_model_q4f16.onnx`
	/// - `text_model_q4f16.onnx`
	/// - `tokenizer.json`
	pub fn from_dir(model_dir: PathBuf) -> Result<Self> {
		let vision_path = model_dir.join(config::VISION_MODEL);
		let text_path = model_dir.join(config::TEXT_MODEL);
		let tokenizer_path = model_dir.join(config::TOKENIZER);
		Self::validate_and_build(vision_path, text_path, tokenizer_path, false)
	}

	fn validate_and_build(
		vision_path: PathBuf,
		text_path: PathBuf,
		tokenizer_path: PathBuf,
		quiet: bool,
	) -> Result<Self> {
		if !vision_path.exists() {
			anyhow::bail!(
				"Vision model file does not exist: {}",
				vision_path.display()
			);
		}
		if !text_path.exists() {
			anyhow::bail!("Text model file does not exist: {}", text_path.display());
		}
		if !tokenizer_path.exists() {
			anyhow::bail!(
				"Tokenizer file does not exist: {}",
				tokenizer_path.display()
			);
		}

		Ok(Self {
			vision: None,
			text: None,
			vision_path,
			text_path,
			tokenizer_path,
			quiet,
		})
	}

	pub fn encode_image(&mut self, image: &image::DynamicImage) -> Result<Embedding> {
		if self.vision.is_none() {
			if !self.quiet {
				crate::ui::debug(&format!(
					"Loading vision model: {}",
					self.vision_path.display()
				));
			}
			self.vision = Some(super::vision::VisionModel::load(&self.vision_path)?);
			if !self.quiet {
				crate::ui::success("Vision model loaded");
			}
		}

		self.vision.as_mut().unwrap().encode(image)
	}

	pub fn encode_text(&mut self, text: &str) -> Result<Embedding> {
		if self.text.is_none() {
			if !self.quiet {
				crate::ui::debug(&format!("Loading text model: {}", self.text_path.display()));
			}
			self.text = Some(super::text::TextModel::load(
				&self.text_path,
				&self.tokenizer_path,
			)?);
			if !self.quiet {
				crate::ui::success("Text model loaded");
			}
		}

		self.text.as_mut().unwrap().encode(text)
	}
}
