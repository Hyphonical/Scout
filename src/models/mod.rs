//! # ONNX Model Management
//!
//! Lazy-loading model coordinator for vision and text encoders.

pub mod manager;
pub mod text;
pub mod vision;

pub use manager::Models;
