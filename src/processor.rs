// Processor - SigLIP2 vision encoder for image embeddings

use anyhow::{Context, Result};
use image::{imageops::FilterType, ImageReader};
use ndarray::{Array, IxDyn};
use ort::value::Value;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use crate::config::{get_vision_model_path, INPUT_SIZE, EMBEDDING_DIM};
use crate::runtime::create_session;
use crate::sidecar::compute_file_hash;
use crate::logger::{log, Level};

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
		let input_value = Value::from_array(input).context("Tensor creation")?;
		let mut session = self.session.lock().map_err(|e| anyhow::anyhow!("Lock: {}", e))?;

		let outputs = session.run(ort::inputs!["pixel_values" => input_value])
			.context("Vision inference")?;

		// Use pooler_output (second output) for aligned embeddings
		let output = outputs.iter().nth(1)
			.or_else(|| outputs.iter().next())
			.context("No output")?
			.1;

		let (shape, data) = output.try_extract_tensor::<f32>()?;
		let embedding = extract_embedding(data, &shape);
		Ok(normalize(&embedding))
	}
}

fn extract_embedding(data: &[f32], shape: &[i64]) -> Vec<f32> {
	let dims: Vec<usize> = shape.iter().map(|&x| x as usize).collect();
	match dims.as_slice() {
		[1, dim] if *dim == EMBEDDING_DIM => data.to_vec(),
		[1, num_patches, dim] if *dim == EMBEDDING_DIM => {
			// Apply mean pooling across patches if needed
			let mut pooled = vec![0.0; *dim];
			for patch_idx in 0..*num_patches {
				let start = patch_idx * dim;
				for (i, val) in pooled.iter_mut().enumerate() {
					*val += data[start + i];
				}
			}
			pooled.iter_mut().for_each(|v| *v /= *num_patches as f32);
			pooled
		},
		_ => data.iter().take(EMBEDDING_DIM).copied().collect(),
	}
}

fn normalize(v: &[f32]) -> Vec<f32> {
	let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
	if norm > 0.0 { v.iter().map(|x| x / norm).collect() } else { v.to_vec() }
}

fn preprocess_image(path: &Path) -> Result<Array<f32, IxDyn>> {
	log(Level::Debug, &format!("Preprocessing image: {}", path.display()));
	let img = ImageReader::open(path)
		.context("Open")?
		.with_guessed_format()
		.context("Format")?
		.decode()
		.context("Decode")?;

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