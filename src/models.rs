//! ONNX model management and inference
//!
//! Manages SigLIP2 vision and text models with lazy loading.
//! Handles image preprocessing, tokenization, and embedding extraction.

use anyhow::{Context, Result};
use ndarray::{Array, IxDyn};
use ort::{session::Session, session::SessionOutputs, value::Value};
use std::path::Path;
use tokenizers::Tokenizer;

use crate::config::{get_text_model_path, get_tokenizer_path, get_vision_model_path, EMBEDDING_DIM, INPUT_SIZE};
use crate::logger::{log, Level};
use crate::runtime::create_session;
use crate::types::{Embedding, ImageHash};

/// SigLIP2 vision model for image embeddings
pub struct VisionModel {
	session: Session,
}

impl VisionModel {
	pub fn load() -> Result<Self> {
		let path = get_vision_model_path().context("Vision model not found")?;
		let session = create_session(&path)?;
		Ok(Self { session })
	}

	pub fn encode(&mut self, pixels: Array<f32, IxDyn>) -> Result<Embedding> {
		log(Level::Debug, "Running vision inference");
		let shape = pixels.shape().to_vec();
		let data = pixels.into_raw_vec_and_offset().0;
		let input = Value::from_array((shape, data))?;
		let outputs = self.session.run(ort::inputs!["pixel_values" => input])?;
		
		let embedding_data = extract_pooler_output(&outputs, "vision model")?;
		Ok(Embedding::new(embedding_data))
	}
}

/// SigLIP2 text model for query embeddings
pub struct TextModel {
	session: Session,
	tokenizer: Tokenizer,
}

impl TextModel {
	pub fn load() -> Result<Self> {
		let model_path = get_text_model_path().context("Text model not found")?;
		let tokenizer_path = get_tokenizer_path().context("Tokenizer not found")?;

		let session = create_session(&model_path)?;
		let tokenizer = Tokenizer::from_file(&tokenizer_path)
			.map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

		Ok(Self { session, tokenizer })
	}

	pub fn encode(&mut self, text: &str) -> Result<Embedding> {
		log(Level::Debug, &format!("Encoding text: {}", text));

		let encoding = self.tokenizer
			.encode(text, true)
			.map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;

		let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
		let shape = vec![1, input_ids.len()];
		let input_val = Value::from_array((shape, input_ids))?;

		let outputs = self.session.run(ort::inputs!["input_ids" => input_val])?;

		let embedding_data = extract_pooler_output(&outputs, "text model")?;
		Ok(Embedding::new(embedding_data))
	}
}

/// Unified manager for vision and text models with lazy loading
///
/// Models are only loaded when first needed, allowing operations
/// that only require one model type to avoid loading both.
pub struct ModelManager {
	vision: Option<VisionModel>,
	text: Option<TextModel>,
}

impl ModelManager {
	pub fn new() -> Self {
		Self { vision: None, text: None }
	}

	pub fn with_vision() -> Result<Self> {
		Ok(Self {
			vision: Some(VisionModel::load()?),
			text: None,
		})
	}

	pub fn with_text() -> Result<Self> {
		Ok(Self {
			vision: None,
			text: Some(TextModel::load()?),
		})
	}

	pub fn encode_image(&mut self, path: &Path) -> Result<(Embedding, ImageHash)> {
		if self.vision.is_none() {
			log(Level::Debug, "Loading vision model");
			self.vision = Some(VisionModel::load()?);
		}

		let hash = crate::sidecar::compute_file_hash(path)?;
		let pixels = preprocess_image(path)?;
		let embedding = self.vision.as_mut().unwrap().encode(pixels)?;

		Ok((embedding, hash))
	}

	#[cfg(feature = "video")]
	pub fn encode_image_from_dynamic(&mut self, img: &image::DynamicImage) -> Result<(Embedding, ImageHash)> {
		if self.vision.is_none() {
			log(Level::Debug, "Loading vision model");
			self.vision = Some(VisionModel::load()?);
		}

		let pixels = preprocess_dynamic_image(img)?;
		let embedding = self.vision.as_mut().unwrap().encode(pixels)?;
		
		// Generate a dummy hash for in-memory images
		let hash = ImageHash(format!("{:016x}", 0));
		
		Ok((embedding, hash))
	}

	pub fn encode_text(&mut self, text: &str) -> Result<Embedding> {
		if self.text.is_none() {
			log(Level::Debug, "Loading text model");
			self.text = Some(TextModel::load()?);
		}

		self.text.as_mut().unwrap().encode(text)
	}
}

/// Extracts and processes pooler output from ONNX model outputs
///
/// Handles both named "pooler_output" and fallback to second output
fn extract_pooler_output(outputs: &SessionOutputs, model_name: &str) -> Result<Vec<f32>> {
	if let Some(pooler) = outputs.get("pooler_output") {
		let (shape, data) = pooler.try_extract_tensor::<f32>()?;
		Ok(extract_embedding(data, shape))
	} else {
		let (_, pooler) = outputs.iter().nth(1)
			.with_context(|| format!("No pooler_output in {}", model_name))?;
		let (shape, data) = pooler.try_extract_tensor::<f32>()?;
		Ok(extract_embedding(data, shape))
	}
}

fn extract_embedding(data: &[f32], shape: &[i64]) -> Vec<f32> {
	let dims: Vec<usize> = shape.iter().map(|&x| x as usize).collect();

	match dims.as_slice() {
		[1, dim] if *dim == EMBEDDING_DIM => data.to_vec(),
		[1, n, dim] if *dim == EMBEDDING_DIM => {
			// Mean pool across sequence/patches
			let mut pooled = vec![0.0; *dim];
			for i in 0..*n {
				let start = i * dim;
				for (j, val) in pooled.iter_mut().enumerate() {
					*val += data[start + j];
				}
			}
			pooled.iter_mut().for_each(|v| *v /= *n as f32);
			pooled
		}
		_ => data.iter().take(EMBEDDING_DIM).copied().collect(),
	}
}

fn preprocess_image(path: &Path) -> Result<Array<f32, IxDyn>> {
	use image::{imageops::FilterType, ImageReader};

	let img = ImageReader::open(path)
		.with_context(|| "Failed to open")?
		.with_guessed_format()?
		.decode()
		.with_context(|| "Failed to decode")?;

	let resized = img.resize_exact(INPUT_SIZE, INPUT_SIZE, FilterType::CatmullRom);
	let rgb = resized.to_rgb8();
	let size = INPUT_SIZE as usize;

	let mut arr = Array::zeros(IxDyn(&[1, 3, size, size]));
	for y in 0..size {
		for x in 0..size {
			let px = rgb.get_pixel(x as u32, y as u32);
			arr[[0, 0, y, x]] = px[0] as f32 / 255.0;
			arr[[0, 1, y, x]] = px[1] as f32 / 255.0;
			arr[[0, 2, y, x]] = px[2] as f32 / 255.0;
		}
	}

	Ok(arr)
}

#[cfg(feature = "video")]
fn preprocess_dynamic_image(img: &image::DynamicImage) -> Result<Array<f32, IxDyn>> {
	use image::imageops::FilterType;

	let resized = img.resize_exact(INPUT_SIZE, INPUT_SIZE, FilterType::CatmullRom);
	let rgb = resized.to_rgb8();
	let size = INPUT_SIZE as usize;

	let mut arr = Array::zeros(IxDyn(&[1, 3, size, size]));
	for y in 0..size {
		for x in 0..size {
			let px = rgb.get_pixel(x as u32, y as u32);
			arr[[0, 0, y, x]] = px[0] as f32 / 255.0;
			arr[[0, 1, y, x]] = px[1] as f32 / 255.0;
			arr[[0, 2, y, x]] = px[2] as f32 / 255.0;
		}
	}

	Ok(arr)
}