// Embedder - Text embedding for semantic search
//
// Converts tag lists into dense vectors for similarity matching.
// Uses HuggingFace tokenizers + ONNX Runtime for inference.

use anyhow::{Context, Result};
use ndarray::Array2;
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Value;
use std::sync::Mutex;
use tokenizers::Tokenizer;

use crate::config::{find_models_dir, EMBED_DIR, EMBED_MODEL, EMBED_TOKENIZER, EMBEDDING_DIM};

pub struct TextEmbedder {
	session: Mutex<Session>,
	tokenizer: Tokenizer,
}

impl TextEmbedder {
	pub fn new() -> Result<Self> {
		let models_dir = find_models_dir().context("Models directory not found")?;
		let model_path = models_dir.join(EMBED_DIR).join(EMBED_MODEL);
		let tokenizer_path = models_dir.join(EMBED_DIR).join(EMBED_TOKENIZER);

		if !model_path.exists() {
			anyhow::bail!("Embedding model not found: {}", model_path.display());
		}
		if !tokenizer_path.exists() {
			anyhow::bail!("Tokenizer not found: {}", tokenizer_path.display());
		}

		let session = Session::builder()?
			.with_optimization_level(GraphOptimizationLevel::Level3)?
			.commit_from_file(&model_path)
			.context("Load embedding model")?;

		let tokenizer = Tokenizer::from_file(&tokenizer_path)
			.map_err(|e| anyhow::anyhow!("Load tokenizer: {}", e))?;

		Ok(Self { session: Mutex::new(session), tokenizer })
	}

	/// Checks if embedding model files are available.
	pub fn is_available() -> bool {
		if let Some(models_dir) = find_models_dir() {
			let model = models_dir.join(EMBED_DIR).join(EMBED_MODEL);
			let tokenizer = models_dir.join(EMBED_DIR).join(EMBED_TOKENIZER);
			model.exists() && tokenizer.exists()
		} else {
			false
		}
	}

	/// Embeds a list of tags into a single vector (mean of tag embeddings).
	pub fn embed_tags(&self, tags: &[String]) -> Result<Vec<f32>> {
		if tags.is_empty() {
			return Ok(vec![0.0; EMBEDDING_DIM]);
		}

		// Join tags with spaces for a single embedding
		let text = tags.join(" ");
		self.embed_text(&text)
	}

	/// Embeds a single text string.
	pub fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
		let encoding = self.tokenizer.encode(text, true)
			.map_err(|e| anyhow::anyhow!("Tokenize: {}", e))?;

		let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
		let attention_mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&x| x as i64).collect();
		let token_type_ids: Vec<i64> = vec![0i64; input_ids.len()];

		let seq_len = input_ids.len();

		let input_ids_arr = Array2::from_shape_vec((1, seq_len), input_ids)?;
		let attention_mask_arr = Array2::from_shape_vec((1, seq_len), attention_mask.clone())?;
		let token_type_ids_arr = Array2::from_shape_vec((1, seq_len), token_type_ids)?;

		// Create ort Values from arrays
		let input_ids_val = Value::from_array(input_ids_arr)?;
		let attention_mask_val = Value::from_array(attention_mask_arr)?;
		let token_type_ids_val = Value::from_array(token_type_ids_arr)?;

		let mut session = self.session.lock().map_err(|e| anyhow::anyhow!("Session lock: {}", e))?;

		let outputs = session.run(ort::inputs![
			"input_ids" => input_ids_val,
			"attention_mask" => attention_mask_val,
			"token_type_ids" => token_type_ids_val,
		])?;

		// Get last_hidden_state and mean pool
		let output = outputs.get("last_hidden_state")
			.or_else(|| outputs.get("sentence_embedding"))
			.context("Model output not found")?;

		let (shape, data) = output.try_extract_tensor::<f32>()?;
		let dims: Vec<usize> = shape.iter().map(|&x| x as usize).collect();

		let embedding = if dims.len() == 3 {
			// Shape: [1, seq_len, hidden_size] - need mean pooling
			let hidden_size = dims[2];
			let seq_len = dims[1];
			mean_pool_flat(data, seq_len, hidden_size, &attention_mask)
		} else if dims.len() == 2 {
			// Shape: [1, hidden_size] - already pooled
			data.to_vec()
		} else {
			anyhow::bail!("Unexpected output shape: {:?}", dims);
		};

		// L2 normalize
		let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
		let normalized: Vec<f32> = if norm > 0.0 {
			embedding.iter().map(|x| x / norm).collect()
		} else {
			embedding
		};

		Ok(normalized)
	}

	#[allow(dead_code)]
	pub fn embedding_dim(&self) -> usize {
		EMBEDDING_DIM
	}
}

/// Mean pooling with attention mask over flat data
fn mean_pool_flat(data: &[f32], seq_len: usize, hidden_size: usize, attention_mask: &[i64]) -> Vec<f32> {
	let mut sum = vec![0.0f32; hidden_size];
	let mut count = 0.0f32;

	for i in 0..seq_len {
		if attention_mask.get(i).copied().unwrap_or(0) == 1 {
			let offset = i * hidden_size;
			for j in 0..hidden_size {
				sum[j] += data[offset + j];
			}
			count += 1.0;
		}
	}

	if count > 0.0 {
		sum.iter_mut().for_each(|x| *x /= count);
	}

	sum
}

/// Cosine similarity between two normalized vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
	a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}
