// Runtime - ONNX execution provider selection and session management

use anyhow::{Context, Result};
use ort::session::builder::{GraphOptimizationLevel, SessionBuilder};
use ort::session::Session;
use std::path::Path;
use std::sync::OnceLock;

use crate::cli::Provider;
use crate::logger::{log, Level};

static EP_PREFERENCE: OnceLock<Provider> = OnceLock::new();

pub fn set_provider(provider: Provider) {
	let _ = EP_PREFERENCE.set(provider);
}

fn get_provider() -> Provider {
	EP_PREFERENCE.get().copied().unwrap_or_default()
}

fn register_execution_providers(builder: &mut SessionBuilder) {
	match get_provider() {
		Provider::Cpu => {
			log(Level::Info, "Using CPU execution provider");
		}
		Provider::Cuda => {
			if !try_register_cuda(builder) {
				log(Level::Error, "CUDA requested but unavailable, falling back to CPU");
			}
		}
		Provider::Tensorrt => {
			if !try_register_tensorrt(builder) {
				log(Level::Error, "TensorRT requested but unavailable, falling back to CPU");
			}
		}
		Provider::Coreml => {
			if !try_register_coreml(builder) {
				log(Level::Error, "CoreML requested but unavailable, falling back to CPU");
			}
		}
		Provider::Auto => {
			if try_register_tensorrt(builder) {
				return;
			}
			if try_register_cuda(builder) {
				return;
			}
			if try_register_coreml(builder) {
				return;
			}
			log(Level::Info, "Using CPU execution provider");
		}
	}
}

fn try_register_cuda(builder: &mut SessionBuilder) -> bool {
	use ort::ep::{ExecutionProvider, CUDA};
	let cuda = CUDA::default();
	if !cuda.is_available().unwrap_or(false) {
		log(Level::Debug, "CUDA not available");
		return false;
	}
	match cuda.register(builder) {
		Ok(_) => {
			log(Level::Success, "Using CUDA execution provider");
			true
		}
		Err(e) => {
			log(Level::Warning, &format!("CUDA registration failed: {}", e));
			false
		}
	}
}

fn try_register_tensorrt(builder: &mut SessionBuilder) -> bool {
	use ort::ep::{ExecutionProvider, TensorRT};
	let trt = TensorRT::default();
	if !trt.is_available().unwrap_or(false) {
		log(Level::Debug, "TensorRT not available");
		return false;
	}
	match trt.register(builder) {
		Ok(_) => {
			log(Level::Success, "Using TensorRT execution provider");
			true
		}
		Err(e) => {
			log(Level::Warning, &format!("TensorRT registration failed: {}", e));
			false
		}
	}
}

fn try_register_coreml(builder: &mut SessionBuilder) -> bool {
	#[cfg(target_os = "macos")]
	{
		use ort::ep::{CoreML, ExecutionProvider};
		let coreml = CoreML::default();
		if !coreml.is_available().unwrap_or(false) {
			log(Level::Debug, "CoreML not available");
			return false;
		}
		match coreml.register(builder) {
			Ok(_) => {
				log(Level::Success, "Using CoreML execution provider");
				return true;
			}
			Err(e) => {
				log(Level::Warning, &format!("CoreML registration failed: {}", e));
			}
		}
	}
	#[cfg(not(target_os = "macos"))]
	{
		let _ = builder;
	}
	false
}

pub fn create_session(model_path: &Path) -> Result<Session> {
	let mut builder = Session::builder().context("Failed to create session builder")?;
	register_execution_providers(&mut builder);
	builder
		.with_optimization_level(GraphOptimizationLevel::Level3)
		.context("Failed to set optimization level")?
		.with_intra_threads(4)
		.context("Failed to set thread count")?
		.commit_from_file(model_path)
		.context("Failed to load model")
}
