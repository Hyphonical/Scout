// Embedder - SigLIP2 text encoder for query embeddings

use anyhow::{Context, Result};
use ndarray::Array2;
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::{ep};
use ort::value::Value;
use std::sync::Mutex;
use tokenizers::Tokenizer;

use crate::config::{get_text_model_path, get_tokenizer_path, EMBEDDING_DIM};
use crate::logger::{log, Level};

pub struct TextEncoder {
	session: Mutex<Session>,
	tokenizer: Tokenizer,
}

impl TextEncoder {
	pub fn new() -> Result<Self> {
		let model_path = get_text_model_path().context("Text model not found")?;
		let tokenizer_path = get_tokenizer_path().context("Tokenizer not found")?;

		let session = Session::builder()?
			.with_execution_providers([
				ep::CUDA::default().build().error_on_failure()
			])?
			.with_optimization_level(GraphOptimizationLevel::Level3)?
			.with_intra_threads(4)?
			.commit_from_file(&model_path)
			.context("Load text model")?;

		let tokenizer = Tokenizer::from_file(&tokenizer_path)
			.map_err(|e| anyhow::anyhow!("Load tokenizer: {}", e))?;

		Ok(Self { session: Mutex::new(session), tokenizer })
	}

	pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
		log(Level::Debug, &format!("Embedding text: {}", text));
		let encoding = self.tokenizer.encode(text, true)
			.map_err(|e| anyhow::anyhow!("Tokenize: {}", e))?;

		let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
		let seq_len = input_ids.len();

		let input_ids_arr = Array2::from_shape_vec((1, seq_len), input_ids)?;
		let input_ids_val = Value::from_array(input_ids_arr)?;

		let mut session = self.session.lock().map_err(|e| anyhow::anyhow!("Lock: {}", e))?;

		let outputs = session.run(ort::inputs![
			"input_ids" => input_ids_val,
		]).context("Text inference")?;
		log(Level::Debug, "Text inference completed");

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
		[1, seq_len, dim] if *dim == EMBEDDING_DIM => {
			// Extract last token embedding for text
			let start = (seq_len - 1) * dim;
			let end = start + EMBEDDING_DIM;
			data[start..end].to_vec()
		},
		_ => data.iter().take(EMBEDDING_DIM).copied().collect(),
	}
}

fn normalize(v: &[f32]) -> Vec<f32> {
	let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
	if norm > 0.0 { v.iter().map(|x| x / norm).collect() } else { v.to_vec() }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
	a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}