//! Lazy model loading coordinator

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

		// Verify files actually exist
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
		})
	}

	pub fn encode_image(&mut self, image: &image::DynamicImage) -> Result<Embedding> {
		if self.vision.is_none() {
			crate::ui::debug(&format!(
				"Loading vision model: {}",
				self.vision_path.display()
			));
			self.vision = Some(super::vision::VisionModel::load(&self.vision_path)?);
			crate::ui::success("Vision model loaded");
		}

		self.vision.as_mut().unwrap().encode(image)
	}

	pub fn encode_text(&mut self, text: &str) -> Result<Embedding> {
		if self.text.is_none() {
			crate::ui::debug(&format!("Loading text model: {}", self.text_path.display()));
			self.text = Some(super::text::TextModel::load(
				&self.text_path,
				&self.tokenizer_path,
			)?);
			crate::ui::success("Text model loaded");
		}

		self.text.as_mut().unwrap().encode(text)
	}
}
