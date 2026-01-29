//! Runtime library path configuration
//!
//! Automatically configures FFmpeg library paths relative to the executable.
//! Supports a unified lib/ directory structure across all platforms.

use std::env;
use std::path::PathBuf;

/// Sets up library search paths for FFmpeg libraries in the lib/ directory
/// next to the executable. Call this before any FFmpeg usage.
pub fn setup_library_paths() {
	if let Ok(exe_path) = env::current_exe() {
		if let Some(exe_dir) = exe_path.parent() {
			let lib_dir = exe_dir.join("lib");

			if lib_dir.exists() {
				#[cfg(target_os = "linux")]
				{
					// Add lib/ to LD_LIBRARY_PATH
					let current = env::var("LD_LIBRARY_PATH").unwrap_or_default();
					let lib_path = lib_dir.to_string_lossy();
					let new_path = if current.is_empty() {
						lib_path.to_string()
					} else {
						format!("{}:{}", lib_path, current)
					};
					env::set_var("LD_LIBRARY_PATH", new_path);
				}

				#[cfg(target_os = "macos")]
				{
					// Add lib/ to DYLD_LIBRARY_PATH
					let current = env::var("DYLD_LIBRARY_PATH").unwrap_or_default();
					let lib_path = lib_dir.to_string_lossy();
					let new_path = if current.is_empty() {
						lib_path.to_string()
					} else {
						format!("{}:{}", lib_path, current)
					};
					env::set_var("DYLD_LIBRARY_PATH", new_path);
				}

				#[cfg(target_os = "windows")]
				{
					// Add lib/ to PATH
					let current = env::var("PATH").unwrap_or_default();
					let lib_path = lib_dir.to_string_lossy();
					let new_path = if current.is_empty() {
						lib_path.to_string()
					} else {
						format!("{};{}", lib_path, current)
					};
					env::set_var("PATH", new_path);
				}
			}
		}
	}
}
