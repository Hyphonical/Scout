//! Execution provider selection

use anyhow::{Context, Result};
use ort::session::{builder::GraphOptimizationLevel, Session};
use std::path::Path;
use std::sync::Mutex;

use crate::ui;

pub use crate::cli::Provider;

static mut SELECTED_PROVIDER: Provider = Provider::Auto;
static PROVIDER_LOGGED: Mutex<bool> = Mutex::new(false);

pub fn set_provider(p: Provider) {
	unsafe {
		SELECTED_PROVIDER = p;
	}
}

fn get_provider() -> Provider {
	unsafe { SELECTED_PROVIDER }
}

pub fn create_session(model_path: &Path) -> Result<Session> {
	let mut builder = Session::builder().context("Failed to create session builder")?;

	match get_provider() {
		Provider::Auto => register_best(&mut builder),
		Provider::Cpu => {
			let mut logged = PROVIDER_LOGGED.lock().unwrap();
			if !*logged {
				ui::info("Using CPU execution provider (forced)");
				*logged = true;
			}
		}
		Provider::Cuda => {
			if !try_cuda(&mut builder) {
				ui::error("CUDA requested but unavailable, falling back to CPU");
			}
		}
		Provider::Tensorrt => {
			if !try_tensorrt(&mut builder) {
				ui::error("TensorRT requested but unavailable, falling back to CPU");
			}
		}
		Provider::CoreML => {
			#[cfg(target_os = "macos")]
			if !try_coreml(&mut builder) {
				ui::error("CoreML requested but unavailable, falling back to CPU");
			}
			#[cfg(not(target_os = "macos"))]
			ui::error("CoreML only available on macOS, falling back to CPU");
		}
		Provider::Xnnpack => {
			if !try_xnnpack(&mut builder) {
				ui::error("XNNPACK requested but unavailable, falling back to CPU");
			}
		}
	}

	builder
		.with_optimization_level(GraphOptimizationLevel::Level3)?
		.with_intra_threads(4)?
		.commit_from_file(model_path)
		.context("Failed to load model")
}

fn register_best(builder: &mut ort::session::builder::SessionBuilder) {
	if try_tensorrt(builder) {
		return;
	}
	if try_cuda(builder) {
		return;
	}

	#[cfg(target_os = "macos")]
	if try_coreml(builder) {
		return;
	}

	if try_xnnpack(builder) {
		return;
	}

	let mut logged = PROVIDER_LOGGED.lock().unwrap();
	if !*logged {
		ui::info("Using CPU execution provider");
		*logged = true;
	}
}

macro_rules! try_provider {
	($builder:expr, $provider_type:ty, $name:expr) => {{
		use ort::ep::ExecutionProvider;

		crate::ui::debug(&format!("Trying provider: {}", $name));

		let provider = <$provider_type>::default();
		if !provider.is_available().unwrap_or(false) {
			crate::ui::debug(&format!("{} not available", $name));
			return false;
		}

		match provider.register($builder) {
			Ok(_) => {
				let mut logged = PROVIDER_LOGGED.lock().unwrap();
				if !*logged {
					crate::ui::success(&format!("Using {} execution provider", $name));
					*logged = true;
				}
				true
			}
			Err(e) => {
				crate::ui::debug(&format!("{} registration failed: {}", $name, e));
				false
			}
		}
	}};
}

fn try_cuda(builder: &mut ort::session::builder::SessionBuilder) -> bool {
	use ort::ep::CUDA;
	try_provider!(builder, CUDA, "CUDA")
}

#[cfg(target_os = "macos")]
fn try_coreml(builder: &mut ort::session::builder::SessionBuilder) -> bool {
	use ort::ep::CoreML;
	try_provider!(builder, CoreML, "CoreML")
}

fn try_tensorrt(builder: &mut ort::session::builder::SessionBuilder) -> bool {
	use ort::ep::TensorRT;
	try_provider!(builder, TensorRT, "TensorRT")
}

fn try_xnnpack(builder: &mut ort::session::builder::SessionBuilder) -> bool {
	use ort::ep::XNNPACK;
	try_provider!(builder, XNNPACK, "XNNPACK")
}
