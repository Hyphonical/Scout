// Runtime - Execution provider selection and session building

use anyhow::{Context, Result};
use ort::session::builder::{GraphOptimizationLevel, SessionBuilder};
use ort::session::Session;
use std::path::Path;

use crate::logger::{log, Level};

/// Execution provider preference for ONNX Runtime sessions.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum ExecutionProviderPreference {
	/// Automatically select the best available provider (CUDA → CoreML → CPU)
	#[default]
	Auto,
	/// Force CUDA (NVIDIA GPU)
	Cuda,
	/// Force CoreML (Apple Silicon/Neural Engine)
	CoreML,
	/// Force CPU execution
	Cpu,
}

impl ExecutionProviderPreference {
	pub fn from_flags(cpu: bool, cuda: bool, coreml: bool) -> Self {
		if cpu {
			Self::Cpu
		} else if cuda {
			Self::Cuda
		} else if coreml {
			Self::CoreML
		} else {
			Self::Auto
		}
	}
}

/// Thread-safe storage for the global EP preference
static EP_PREFERENCE: std::sync::OnceLock<ExecutionProviderPreference> = std::sync::OnceLock::new();

/// Sets the global execution provider preference. Call once at startup.
pub fn set_ep_preference(pref: ExecutionProviderPreference) {
	let _ = EP_PREFERENCE.set(pref);
}

/// Gets the current execution provider preference.
pub fn get_ep_preference() -> ExecutionProviderPreference {
	EP_PREFERENCE.get().copied().unwrap_or_default()
}

/// Registers execution providers based on the current preference.
/// Falls back gracefully: CUDA → CoreML → CPU (for Auto mode)
pub fn register_execution_providers(builder: &mut SessionBuilder) {
	let pref = get_ep_preference();

	match pref {
		ExecutionProviderPreference::Cpu => {
			log(Level::Info, "Using CPU execution provider (forced)");
		}
		ExecutionProviderPreference::Cuda => {
			if !try_register_cuda(builder) {
				log(Level::Error, "CUDA was requested but failed to register");
				log(Level::Info, "Falling back to CPU execution provider");
			}
		}
		ExecutionProviderPreference::CoreML => {
			if !try_register_coreml(builder) {
				log(Level::Error, "CoreML was requested but failed to register");
				log(Level::Info, "Falling back to CPU execution provider");
			}
		}
		ExecutionProviderPreference::Auto => {
			// Try CUDA first (NVIDIA GPUs)
			if try_register_cuda(builder) {
				return;
			}

			// Try CoreML on macOS (Apple Silicon)
			if try_register_coreml(builder) {
				return;
			}

			// Fall back to CPU
			log(Level::Info, "Using CPU execution provider");
		}
	}
}

/// Attempts to register CUDA. Returns true on success.
fn try_register_cuda(builder: &mut SessionBuilder) -> bool {
	use ort::ep::{ExecutionProvider, CUDA};
	let cuda = CUDA::default();
	if cuda.is_available().unwrap_or(false) {
		match cuda.register(builder) {
			Ok(_) => {
				log(Level::Success, "Using CUDA execution provider (GPU)");
				return true;
			}
			Err(e) => {
				log(Level::Warning, &format!("Failed to register CUDA: {}", e));
			}
		}
	} else {
		log(Level::Debug, "CUDA not available on this system");
	}
	false
}

/// Attempts to register CoreML. Returns true on success.
fn try_register_coreml(builder: &mut SessionBuilder) -> bool {
	#[cfg(target_os = "macos")]
	{
		use ort::ep::{ExecutionProvider, CoreML};
		let coreml = CoreML::default();
		if coreml.is_available().unwrap_or(false) {
			match coreml.register(builder) {
				Ok(_) => {
					log(Level::Success, "Using CoreML execution provider (Apple Silicon)");
					return true;
				}
				Err(e) => {
					log(Level::Warning, &format!("Failed to register CoreML: {}", e));
				}
			}
		} else {
			log(Level::Debug, "CoreML not available on this system");
		}
	}
	#[cfg(not(target_os = "macos"))]
	{
		let _ = builder;
		log(Level::Debug, "CoreML only available on macOS");
	}
	false
}

/// Creates an ONNX Runtime session with the configured execution providers.
pub fn create_session(model_path: &Path) -> Result<Session> {
	let mut builder = Session::builder().context("Session builder")?;

	register_execution_providers(&mut builder);

	builder
		.with_optimization_level(GraphOptimizationLevel::Level3)
		.context("Optimization")?
		.with_intra_threads(4)
		.context("Threads")?
		.commit_from_file(model_path)
		.context("Load model")
}
