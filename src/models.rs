// Models - Centralized ONNX model management

use anyhow::{Context, Result};
use ndarray::{Array, Array2, IxDyn};
use ort::{session::Session, value::Value};
use std::path::Path;
use tokenizers::Tokenizer;

use crate::config::{get_text_model_path, get_tokenizer_path, get_vision_model_path, EMBEDDING_DIM, INPUT_SIZE};
use crate::logger::{log, Level};
use crate::runtime::create_session;
use crate::types::{Embedding, ImageHash};

/// Vision model for image embeddings
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
		let input = Value::from_array(pixels)?;
		let outputs = self.session.run(ort::inputs!["pixel_values" => input])?;

		let embedding_data = if let Some(pooler) = outputs.get("pooler_output") {
			let (shape, data) = pooler.try_extract_tensor::<f32>()?;
			extract_embedding(data, &shape)
		} else {
			let (_, pooler) = outputs.iter().nth(1).context("No pooler_output in vision model")?;
			let (shape, data) = pooler.try_extract_tensor::<f32>()?;
			extract_embedding(data, &shape)
		};

		Ok(Embedding::new(embedding_data))
	}
}

/// Text model for query embeddings
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
		let input = Array2::from_shape_vec((1, input_ids.len()), input_ids)?;
		let input_val = Value::from_array(input)?;

		let outputs = self.session.run(ort::inputs!["input_ids" => input_val])?;

		let embedding_data = if let Some(pooler) = outputs.get("pooler_output") {
			let (shape, data) = pooler.try_extract_tensor::<f32>()?;
			extract_embedding(data, &shape)
		} else {
			let (_, pooler) = outputs.iter().nth(1).context("No pooler_output in text model")?;
			let (shape, data) = pooler.try_extract_tensor::<f32>()?;
			extract_embedding(data, &shape)
		};

		Ok(Embedding::new(embedding_data))
	}
}

/// Combined manager for both models (lazy loading)
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

	pub fn encode_text(&mut self, text: &str) -> Result<Embedding> {
		if self.text.is_none() {
			log(Level::Debug, "Loading text model");
			self.text = Some(TextModel::load()?);
		}

		self.text.as_mut().unwrap().encode(text)
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
		.with_context(|| format!("Failed to open: {}", path.display()))?
		.with_guessed_format()?
		.decode()
		.with_context(|| format!("Failed to decode: {}", path.display()))?;

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