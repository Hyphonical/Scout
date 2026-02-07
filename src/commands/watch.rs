//! # Watch Command
//!
//! Monitor directory for file changes and auto-index new media.
//! Uses debounced filesystem events for efficient processing.

use anyhow::{Context, Result};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::core::{FileHash, MediaType};
use crate::models::Models;
use crate::processing;
use crate::storage;
use crate::ui;

/// Task to be processed by the worker thread
struct WatchTask {
	path: PathBuf,
	media_type: MediaType,
	max_frames: usize,
	scene_threshold: f32,
}

pub fn run(
	dir: &Path,
	recursive: bool,
	min_resolution: Option<u32>,
	max_size: Option<u64>,
	exclude_videos: bool,
	max_frames: Option<usize>,
	scene_threshold: Option<f32>,
) -> Result<()> {
	ui::info(&format!("Watching: {}", dir.display()));

	let max_frames = max_frames.unwrap_or(crate::config::MAX_VIDEO_FRAMES);
	let scene_threshold = scene_threshold.unwrap_or(crate::config::SCENE_THRESHOLD);

	// 1. Check FFmpeg availability
	let video_supported = if exclude_videos {
		false
	} else {
		processing::video::is_available()
	};

	if exclude_videos {
		ui::debug("Videos excluded by --exclude-videos flag");
	} else if !video_supported {
		ui::warn("FFmpeg not found - videos will be skipped");
	}

	// 2. Load models safely (Shared ownership)
	// We wrap Models in a Mutex so the worker thread can lock it briefly when needed
	let models = Arc::new(Mutex::new(Models::new()?));

	// 3. Setup the Worker Thread (The Queue)
	// We use a channel to decouple "detection" from "processing"
	let (task_tx, task_rx) = channel::<WatchTask>();
	let worker_models = Arc::clone(&models);

	// Spawn the background worker
	thread::spawn(move || {
		// This loop runs forever (or until the main program closes the channel)
		while let Ok(task) = task_rx.recv() {
			// Process files one by one to avoid CPU spikes
			if let Err(e) = process_task(&worker_models, &task) {
				// Log errors but don't crash the worker
				ui::error(&format!("Error processing {}: {}", task.path.display(), e));
			}
		}
	});

	ui::success("Ready - watching for file changes (Ctrl+C to stop)");
	println!();

	// 4. Helper closure to filter and queue files
	// This removes duplicate logic for handling direct files vs folder contents
	let tx = task_tx.clone();
	let queue_file = move |path: PathBuf| {
		// Check filtering options
		if let Some(media_type) = MediaType::detect(&path) {
			// Skip video if not supported
			if matches!(media_type, MediaType::Video) && !video_supported {
				return;
			}

			// Check Max Size
			if let Some(max_mb) = max_size {
				if let Ok(meta) = std::fs::metadata(&path) {
					if meta.len() > max_mb * 1024 * 1024 {
						ui::debug(&format!("Skipped (too large): {}", path.display()));
						return;
					}
				}
			}

			// Check Resolution (Images only)
			if let Some(min_res) = min_resolution {
				if matches!(media_type, MediaType::Image) {
					if let Ok((w, h)) = image::image_dimensions(&path) {
						if w.min(h) < min_res {
							ui::debug(&format!("Skipped (too small): {}", path.display()));
							return;
						}
					}
				}
			}

			// Send to worker
			let _ = tx.send(WatchTask {
				path,
				media_type,
				max_frames,
				scene_threshold,
			});
		}
	};

	// 5. Setup the Debouncer
	// 1-second timeout allows OS file copies to "settle" before we trigger events
	let mut debouncer = new_debouncer(
		Duration::from_secs(1),
		move |result: DebounceEventResult| {
			match result {
				Ok(events) => {
					for event in events {
						let path = event.path;

						if path.is_dir() {
							// HANDLE DIRECTORIES
							// If user drops a folder, we only scan it if recursive is ON
							if recursive {
								if let Ok(entries) = std::fs::read_dir(&path) {
									for entry in entries.flatten() {
										let sub_path = entry.path();
										if sub_path.is_file() {
											queue_file(sub_path);
										}
									}
								}
							}
						} else {
							// HANDLE FILES
							queue_file(path);
						}
					}
				}
				Err(e) => ui::error(&format!("Watch error: {:?}", e)),
			}
		},
	)
	.context("Failed to create file watcher")?;

	// 6. Start Watching
	let watch_mode = if recursive {
		notify_debouncer_mini::notify::RecursiveMode::Recursive
	} else {
		notify_debouncer_mini::notify::RecursiveMode::NonRecursive
	};

	debouncer
		.watcher()
		.watch(dir, watch_mode)
		.context("Failed to watch directory")?;

	// Keep the main thread alive indefinitely
	loop {
		thread::sleep(Duration::from_secs(3600));
	}
}

/// The main logic run by the background worker
fn process_task(models: &Arc<Mutex<Models>>, task: &WatchTask) -> Result<()> {
	let file_start = Instant::now();

	// 1. Wait for file to be safe (unlocked and fully written)
	let canonical = wait_for_file_stable(&task.path)?;
	let media_dir = canonical.parent().context("No parent directory")?;

	// 2. Compute Hash
	let hash = FileHash::compute(&canonical)?;

	// 3. Check DB (Avoid locking models if we don't need to)
	if let Some(sidecar_path) = storage::find(media_dir, &hash) {
		if let Ok(sidecar) = storage::load(&sidecar_path) {
			if sidecar.is_current_version() {
				ui::debug(&format!("Already indexed: {}", task.path.display()));
				return Ok(());
			}
		}
	}

	let filename = canonical
		.file_name()
		.and_then(|n| n.to_str())
		.unwrap_or("unknown")
		.to_string();

	let file = processing::scan::MediaFile {
		path: canonical.clone(),
		filename,
		hash,
		media_type: task.media_type,
	};

	// 4. Run AI Scan (Thread-safe lock)
	// We block here until it's our turn to use the models
	{
		let mut models_guard = models.lock().unwrap(); // Wait for lock
		match task.media_type {
			MediaType::Image => {
				crate::commands::scan::process_image(&mut models_guard, &file, media_dir)?
			}
			MediaType::Video => crate::commands::scan::process_video(
				&mut models_guard,
				&file,
				media_dir,
				task.max_frames,
				task.scene_threshold,
			)?,
		}
	} // Lock is automatically released here

	let duration_ms = file_start.elapsed().as_millis();
	ui::log::file_processed(&canonical, duration_ms);

	Ok(())
}

/// Smart wait that handles both "File Busy" (Windows) and "Slow Copy" (Linux/Network)
fn wait_for_file_stable(path: &Path) -> Result<PathBuf> {
	let mut last_size = u64::MAX;
	let mut stable_counts = 0;
	let max_attempts = 20; // Try for 10 seconds total

	for _ in 0..max_attempts {
		// Check 1: Does file exist and can we read metadata?
		if let Ok(meta) = std::fs::metadata(path) {
			let current_size = meta.len();

			// Check 2: Is size stable?
			if current_size > 0 && current_size == last_size {
				stable_counts += 1;
			} else {
				// Size changed or is 0, reset counter
				stable_counts = 0;
			}
			last_size = current_size;

			// Check 3: If size has been stable for 1s (2 checks), try to open
			if stable_counts >= 2 {
				// Try to open read-only. This is the final check for Windows locks.
				if std::fs::File::open(path).is_ok() {
					return Ok(path.canonicalize()?);
				}
			}
		} else {
			// File might have been deleted or permission denied
			stable_counts = 0;
		}

		// Wait 500ms before next check
		thread::sleep(Duration::from_millis(500));
	}

	anyhow::bail!("File busy or locked: {}", path.display());
}
