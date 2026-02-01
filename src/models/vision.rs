//! Vision model (SigLIP2) for image embeddings

use anyhow::{Context, Result};
use ort::session::Session;
use std::path::Path;

use crate::config::{EMBEDDING_DIM, INPUT_SIZE};
use crate::core::Embedding;

pub struct VisionModel {
    session: Session,
}

impl VisionModel {
    pub fn load(model_path: &Path) -> Result<Self> {
        let session = crate::runtime::create_session(model_path)
            .context("Failed to load vision model")?;
        Ok(Self { session })
    }
    
    pub fn encode(&mut self, image: &image::DynamicImage) -> Result<Embedding> {
        let pixels = preprocess(image)?;
        let input = ort::value::Value::from_array(pixels)?;
        
        let outputs = self.session.run(ort::inputs!["pixel_values" => input])?;
        let embedding = extract_embedding(&outputs)?;
        
        Ok(Embedding::new(embedding))
    }
}

fn preprocess(img: &image::DynamicImage) -> Result<(Vec<usize>, Vec<f32>)> {
    use image::imageops::FilterType;
    
    let resized = img.resize_exact(INPUT_SIZE, INPUT_SIZE, FilterType::CatmullRom);
    let rgb = resized.to_rgb8();
    let size = INPUT_SIZE as usize;
    
    let shape = vec![1, 3, size, size];
    let mut data = vec![0.0f32; 1 * 3 * size * size];
    
    for y in 0..size {
        for x in 0..size {
            let px = rgb.get_pixel(x as u32, y as u32);
            let idx = y * size + x;
            data[idx] = px[0] as f32 / 255.0;                    // R
            data[size * size + idx] = px[1] as f32 / 255.0;      // G
            data[2 * size * size + idx] = px[2] as f32 / 255.0;  // B
        }
    }
    
    Ok((shape, data))
}

fn extract_embedding(outputs: &ort::session::SessionOutputs) -> Result<Vec<f32>> {
    let pooler = outputs.get("pooler_output")
        .context("No pooler output found")?;
    
    let (shape, data) = pooler.try_extract_tensor::<f32>()?;
    let dims: Vec<usize> = shape.iter().map(|&x| x as usize).collect();
    
    // Handle different output shapes
    match dims.as_slice() {
        [1, dim] if *dim == EMBEDDING_DIM => Ok(data.to_vec()),
        [1, n, dim] if *dim == EMBEDDING_DIM => {
            // Mean pooling
            let mut pooled = vec![0.0; *dim];
            for i in 0..*n {
                for j in 0..*dim {
                    pooled[j] += data[i * dim + j];
                }
            }
            pooled.iter_mut().for_each(|v| *v /= *n as f32);
            Ok(pooled)
        }
        _ => Ok(data.iter().take(EMBEDDING_DIM).copied().collect()),
    }
}
