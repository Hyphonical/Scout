// Processor - Vision encoder for image embeddings

use anyhow::{Context, Result};
use image::{imageops::FilterType, ImageReader};
use ndarray::{Array, IxDyn};
use ort::value::Value;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use crate::config::{get_vision_model_path, INPUT_SIZE};
use crate::embedding::{extract_vision_embedding, normalize};
use crate::logger::{log, Level};
use crate::runtime::create_session;
use crate::sidecar::compute_file_hash;

pub struct VisionEncoder {
	session: Mutex<ort::session::Session>,
}

pub struct ProcessingResult {
	pub embedding: Vec<f32>,
	pub image_hash: String,
	pub processing_ms: u64,
}

impl VisionEncoder {
	pub fn new() -> Result<Self> {
		let model_path = get_vision_model_path()
			.ok_or_else(|| anyhow::anyhow!("Vision model not found"))?;

		let session = create_session(&model_path)?;
		Ok(Self { session: Mutex::new(session) })
	}

	pub fn process_image(&self, path: &Path) -> Result<ProcessingResult> {
		let start = Instant::now();
		log(Level::Debug, &format!("Processing image: {}", path.display()));
		let image_hash = compute_file_hash(path)?;
		let input = preprocess_image(path)?;
		let embedding = self.encode(input)?;

		Ok(ProcessingResult {
			embedding,
			image_hash,
			processing_ms: start.elapsed().as_millis() as u64,
		})
	}

	fn encode(&self, input: Array<f32, IxDyn>) -> Result<Vec<f32>> {
		log(Level::Debug, "Running vision inference");
		let input_value = Value::from_array(input).context("Failed to create tensor")?;
		let mut session = self.session.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

		let outputs = session
			.run(ort::inputs!["pixel_values" => input_value])
			.context("Vision inference failed")?;

		// Use pooler_output (second output) for aligned embeddings
		let output = outputs
			.iter()
			.nth(1)
			.or_else(|| outputs.iter().next())
			.context("No output tensor")?
			.1;

		let (shape, data) = output.try_extract_tensor::<f32>()?;
		Ok(normalize(&extract_vision_embedding(data, &shape)))
	}
}

fn preprocess_image(path: &Path) -> Result<Array<f32, IxDyn>> {
	let img = ImageReader::open(path)
		.context("Failed to open image")?
		.with_guessed_format()
		.context("Failed to detect format")?
		.decode()
		.context("Failed to decode image")?;

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