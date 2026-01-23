// Processor - ONNX model inference for image tagging
//
// Loads the tagging model once and reuses it for batch processing.
// Automatically selects CUDA or CPU based on availability and batch size.

use anyhow::{Context, Result};
use image::{imageops::FilterType, ImageReader};
use ndarray::{Array, IxDyn};
use ort::{
	execution_providers::{CUDAExecutionProvider, ExecutionProvider},
	session::{builder::GraphOptimizationLevel, Session},
	value::Value,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use crate::config::{self, INPUT_SIZE};
use crate::sidecar::{compute_file_hash, ImageSidecar, TagEntry, TagStats};

pub struct ImageProcessor {
	session: Mutex<Session>,
	tag_mappings: HashMap<String, String>,
	execution_provider: String,
}

pub struct ProcessingResult {
	pub tags: Vec<TagEntry>,
	pub stats: TagStats,
	pub image_hash: String,
}

impl ImageProcessor {
	/// Initializes the ONNX model session. When `use_gpu` is true and CUDA is available,
	/// runs inference on the GPU; otherwise falls back to CPU.
	pub fn new(use_gpu: bool) -> Result<Self> {
		let model_path = config::get_tagger_model_path()
			.ok_or_else(|| anyhow::anyhow!("Tagger model not found. Expected models/tagger/model.onnx"))?;
		let mappings_path = config::get_tagger_mappings_path()
			.ok_or_else(|| anyhow::anyhow!("Tagger mappings not found. Expected models/tagger/mappings.json"))?;

		let mappings_str = std::fs::read_to_string(&mappings_path)
			.with_context(|| format!("Failed to read {:?}", mappings_path))?;
		let tag_mappings: HashMap<String, String> = serde_json::from_str(&mappings_str)
			.context("Invalid mappings JSON")?;

		let mut builder = Session::builder()
			.context("Session builder failed")?
			.with_optimization_level(GraphOptimizationLevel::Level3)
			.context("Optimization config failed")?
			.with_intra_threads(4)
			.context("Thread config failed")?;

		// Only attempt CUDA if requested and available
		let execution_provider = if use_gpu && CUDAExecutionProvider::default().is_available().unwrap_or(false) {
			let cuda = CUDAExecutionProvider::default()
				.with_device_id(0)
				.with_memory_limit(0)
				.build();

			builder = builder.with_execution_providers([cuda])
				.context("CUDA config failed")?;
			"CUDA".to_string()
		} else {
			"CPU".to_string()
		};

		let session = builder.commit_from_file(&model_path)
			.with_context(|| format!("Failed to load model {:?}", model_path))?;

		Ok(Self {
			session: Mutex::new(session),
			tag_mappings,
			execution_provider,
		})
	}

	pub fn execution_provider(&self) -> &str {
		&self.execution_provider
	}

	pub fn vocabulary_size(&self) -> usize {
		self.tag_mappings.len()
	}

	/// Processes an image file: computes hash, runs inference, filters tags above threshold.
	pub fn process_image(&self, path: &Path, threshold: f32) -> Result<ProcessingResult> {
		let start = Instant::now();
		let image_hash = compute_file_hash(path)?;
		let input = preprocess_image(path)?;
		let probs = self.run_inference(input)?;
		let tags = self.filter_tags(&probs, threshold);

		Ok(ProcessingResult {
			stats: TagStats {
				total_tags: tags.len(),
				processing_ms: start.elapsed().as_millis() as u64,
			},
			tags,
			image_hash,
		})
	}

	/// Creates a sidecar structure from the processing result.
	pub fn create_sidecar(&self, path: &Path, result: ProcessingResult, threshold: f32) -> ImageSidecar {
		let mut sidecar = ImageSidecar::new(path, result.image_hash, result.tags, threshold, result.stats.processing_ms);
		sidecar.stats = result.stats;
		sidecar
	}

	fn run_inference(&self, input: Array<f32, IxDyn>) -> Result<Vec<f32>> {
		let input_value = Value::from_array(input).context("Input tensor creation failed")?;
		let mut session = self.session.lock().map_err(|e| anyhow::anyhow!("Session lock: {}", e))?;

		let outputs = session.run(ort::inputs!["input" => input_value]).context("Inference failed")?;

		// Use refined logits (second output) and apply sigmoid
		let logits = outputs[1].try_extract_tensor::<f32>().context("Logit extraction failed")?;
		Ok(logits.1.iter().map(|&x| 1.0 / (1.0 + (-x).exp())).collect())
	}

	fn filter_tags(&self, probs: &[f32], threshold: f32) -> Vec<TagEntry> {
		let mut tags: Vec<TagEntry> = probs.iter()
			.enumerate()
			.filter(|(_, &p)| p >= threshold)
			.map(|(i, &p)| TagEntry {
				id: i,
				name: self.tag_mappings.get(&i.to_string())
					.cloned()
					.unwrap_or_else(|| format!("tag_{}", i)),
				confidence: p,
			})
			.collect();

		tags.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
		tags
	}
}

/// Loads, resizes, and normalizes an image to NCHW tensor format.
/// Uses content-based format detection to handle mislabeled files
fn preprocess_image(path: &Path) -> Result<Array<f32, IxDyn>> {
	let img = ImageReader::open(path)
		.context("Open failed")?
		.with_guessed_format()
		.context("Format detection failed")?
		.decode()
		.context("Decode failed")?;

	let resized = img.resize_exact(INPUT_SIZE, INPUT_SIZE, FilterType::Lanczos3);
	let rgb = resized.to_rgb8();

	let (w, h) = (INPUT_SIZE as usize, INPUT_SIZE as usize);
	let mut arr = Array::zeros(IxDyn(&[1, 3, h, w]));

	for y in 0..h {
		for x in 0..w {
			let px = rgb.get_pixel(x as u32, y as u32);
			arr[[0, 0, y, x]] = px[0] as f32 / 255.0;
			arr[[0, 1, y, x]] = px[1] as f32 / 255.0;
			arr[[0, 2, y, x]] = px[2] as f32 / 255.0;
		}
	}

	Ok(arr)
}
