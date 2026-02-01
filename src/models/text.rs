//! Text model (SigLIP2) for query embeddings

use anyhow::{Context, Result};
use ort::session::Session;
use std::path::Path;
use tokenizers::Tokenizer;

use crate::config::EMBEDDING_DIM;
use crate::core::Embedding;

pub struct TextModel {
    session: Session,
    tokenizer: Tokenizer,
}

impl TextModel {
    pub fn load(model_path: &Path, tokenizer_path: &Path) -> Result<Self> {
        let session = crate::runtime::create_session(model_path)
            .context("Failed to load text model")?;
        
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;
        
        Ok(Self { session, tokenizer })
    }
    
    pub fn encode(&mut self, text: &str) -> Result<Embedding> {
        let encoding = self.tokenizer.encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;
        
        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
        let shape = vec![1, input_ids.len()];
        let input = ort::value::Value::from_array((shape, input_ids))?;
        
        let outputs = self.session.run(ort::inputs!["input_ids" => input])?;
        let embedding = extract_embedding(&outputs)?;
        
        Ok(Embedding::new(embedding))
    }
}

fn extract_embedding(outputs: &ort::session::SessionOutputs) -> Result<Vec<f32>> {
    let pooler = outputs.get("pooler_output")
        .context("No pooler output found")?;
    
    let (shape, data) = pooler.try_extract_tensor::<f32>()?;
    let dims: Vec<usize> = shape.iter().map(|&x| x as usize).collect();
    
    match dims.as_slice() {
        [1, dim] if *dim == EMBEDDING_DIM => Ok(data.to_vec()),
        _ => Ok(data.iter().take(EMBEDDING_DIM).copied().collect()),
    }
}
