// Embedder - Text encoder for query embeddings

use anyhow::{Context, Result};
use ndarray::Array2;
use ort::value::Value;
use std::sync::Mutex;
use tokenizers::Tokenizer;

use crate::config::{get_text_model_path, get_tokenizer_path};
use crate::embedding::{extract_text_embedding, normalize};
use crate::logger::{log, Level};
use crate::runtime::create_session;

pub struct TextEncoder {
	session: Mutex<ort::session::Session>,
	tokenizer: Tokenizer,
}

impl TextEncoder {
	pub fn new() -> Result<Self> {
		let model_path = get_text_model_path().context("Text model not found")?;
		let tokenizer_path = get_tokenizer_path().context("Tokenizer not found")?;

		let session = create_session(&model_path)?;

		let tokenizer = Tokenizer::from_file(&tokenizer_path)
			.map_err(|e| anyhow::anyhow!("Load tokenizer: {}", e))?;

		Ok(Self { session: Mutex::new(session), tokenizer })
	}

	pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
		log(Level::Debug, &format!("Embedding text: {}", text));
		let encoding = self
			.tokenizer
			.encode(text, true)
			.map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;

		let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
		let seq_len = input_ids.len();

		let input_ids_arr = Array2::from_shape_vec((1, seq_len), input_ids)?;
		let input_ids_val = Value::from_array(input_ids_arr)?;

		let mut session = self.session.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

		let outputs = session
			.run(ort::inputs!["input_ids" => input_ids_val])
			.context("Text inference failed")?;

		// Use pooler_output (second output) for aligned embeddings
		let output = outputs
			.iter()
			.nth(1)
			.or_else(|| outputs.iter().next())
			.context("No output tensor")?
			.1;

		let (shape, data) = output.try_extract_tensor::<f32>()?;
		Ok(normalize(&extract_text_embedding(data, &shape)))
	}
}