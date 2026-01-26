// Embedding utilities - shared functions for vector operations

use crate::config::EMBEDDING_DIM;

/// Normalizes a vector to unit length.
pub fn normalize(v: &[f32]) -> Vec<f32> {
	let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
	if norm > 0.0 {
		v.iter().map(|x| x / norm).collect()
	} else {
		v.to_vec()
	}
}

/// Computes cosine similarity between two normalized vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
	a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Extracts embedding from vision model output.
/// Handles both [1, dim] and [1, patches, dim] shapes.
pub fn extract_vision_embedding(data: &[f32], shape: &[i64]) -> Vec<f32> {
	let dims: Vec<usize> = shape.iter().map(|&x| x as usize).collect();
	match dims.as_slice() {
		[1, dim] if *dim == EMBEDDING_DIM => data.to_vec(),
		[1, num_patches, dim] if *dim == EMBEDDING_DIM => {
			// Mean pooling across patches
			let mut pooled = vec![0.0; *dim];
			for patch in 0..*num_patches {
				let start = patch * dim;
				for (i, val) in pooled.iter_mut().enumerate() {
					*val += data[start + i];
				}
			}
			pooled.iter_mut().for_each(|v| *v /= *num_patches as f32);
			pooled
		}
		_ => data.iter().take(EMBEDDING_DIM).copied().collect(),
	}
}

/// Extracts embedding from text model output.
/// Handles both [1, dim] and [1, seq_len, dim] shapes.
pub fn extract_text_embedding(data: &[f32], shape: &[i64]) -> Vec<f32> {
	let dims: Vec<usize> = shape.iter().map(|&x| x as usize).collect();
	match dims.as_slice() {
		[1, dim] if *dim == EMBEDDING_DIM => data.to_vec(),
		[1, seq_len, dim] if *dim == EMBEDDING_DIM => {
			// Last token embedding for text
			let start = (seq_len - 1) * dim;
			data[start..start + EMBEDDING_DIM].to_vec()
		}
		_ => data.iter().take(EMBEDDING_DIM).copied().collect(),
	}
}

/// Combines two embeddings with weights and normalizes.
pub fn combine_embeddings(
	text_emb: Option<&[f32]>,
	image_emb: Option<&[f32]>,
	text_weight: f32,
) -> Option<Vec<f32>> {
	match (text_emb, image_emb) {
		(Some(text), Some(image)) => {
			let image_weight = 1.0 - text_weight;
			let combined: Vec<f32> = text
				.iter()
				.zip(image.iter())
				.map(|(t, i)| t * text_weight + i * image_weight)
				.collect();
			Some(normalize(&combined))
		}
		(Some(text), None) => Some(text.to_vec()),
		(None, Some(image)) => Some(image.to_vec()),
		(None, None) => None,
	}
}
