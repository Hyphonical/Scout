//! # ONNX Runtime
//!
//! Session creation and execution provider selection.

pub mod providers;

pub use providers::{create_session, set_provider};
