//! ONNX Runtime configuration

pub mod providers;

pub use providers::{create_session, set_provider};
