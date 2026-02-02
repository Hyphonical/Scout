//! Scan command - index images and videos

use anyhow::Result;
use std::path::Path;
use std::time::Instant;

use crate::config::VIDEO_FRAME_COUNT;
use crate::core::MediaType;
use crate::models::Models;
use crate::processing;
use crate::storage;
use crate::ui;

pub fn run(
	dir: &Path,
	recursive: bool,
	force: bool,
	min_resolution: Option<u32>,
	max_size: Option<u64>,
	exclude_videos: bool,
) -> Result<()> {
	let start = Instant::now();

	ui::info(&format!("Scanning: {}", dir.display()));

	// Check FFmpeg availability for videos
	let video_supported = if exclude_videos {
		false
	} else {
		processing::video::is_available()
	};

	if exclude_videos {
		ui::debug("Videos excluded by --exclude-videos flag");
	} else if !video_supported {
		ui::warn("FFmpeg not found - videos will be skipped");
		ui::debug("Install FFmpeg to enable video support");
	}

	let scan_result = processing::scan_directory(dir, recursive, force, min_resolution, max_size);

	if scan_result.to_process.is_empty() {
		ui::success(&format!(
			"All {} files already indexed",
			scan_result.already_indexed
		));
		if scan_result.filtered > 0 {
			ui::info(&format!("{} files filtered out", scan_result.filtered));
		}
		return Ok(());
	}

	ui::info(&format!(
		"Processing {} files ({} indexed, {} filtered)",
		scan_result.to_process.len(),
		scan_result.already_indexed,
		scan_result.filtered
	));

	if scan_result.outdated > 0 {
		ui::warn(&format!(
			"{} sidecars outdated - will be upgraded",
			scan_result.outdated
		));
	}

	let mut models = Models::new()?;
	let mut processed = 0;
	let mut errors = 0;
	let mut skipped_videos = 0;

	for file in scan_result.to_process {
		let media_dir = file.path.parent().unwrap();
		let file_start = Instant::now();

		let result = match file.media_type {
			MediaType::Image => process_image(&mut models, &file, media_dir),
			MediaType::Video => {
				if !video_supported {
					skipped_videos += 1;
					continue;
				}
				process_video(&mut models, &file, media_dir)
			}
		};

		match result {
			Ok(_) => {
				let duration_ms = file_start.elapsed().as_millis();
				ui::log::file_processed(&file.path, duration_ms);
				processed += 1;
			}
			Err(e) => {
				ui::error(&format!("{}: {}", file.filename, e));
				errors += 1;
			}
		}
	}

	let duration = start.elapsed().as_secs_f32();

	println!();
	ui::success(&format!(
		"Processed {} files in {:.1}s",
		processed, duration
	));

	if errors > 0 {
		ui::warn(&format!("{} errors", errors));
	}

	if skipped_videos > 0 {
		ui::info(&format!(
			"{} videos skipped (FFmpeg not available)",
			skipped_videos
		));
	}

	Ok(())
}

pub fn process_image(
	models: &mut Models,
	file: &processing::scan::MediaFile,
	media_dir: &Path,
) -> Result<()> {
	let embedding = processing::image::encode(models, &file.path)?;
	let sidecar = storage::ImageSidecar::new(file.filename.clone(), file.hash.clone(), embedding);
	storage::save_image(&sidecar, media_dir, &file.hash)?;
	Ok(())
}

pub fn process_video(
	models: &mut Models,
	file: &processing::scan::MediaFile,
	media_dir: &Path,
) -> Result<()> {
	let frames = processing::video::extract_frames(&file.path, VIDEO_FRAME_COUNT)?;

	let mut encoded_frames = Vec::new();
	for (timestamp, frame_img) in frames {
		let dynamic_img = image::DynamicImage::ImageRgb8(frame_img);
		let embedding = processing::image::encode_image(models, &dynamic_img)?;
		encoded_frames.push((timestamp, embedding));
	}

	let sidecar =
		storage::VideoSidecar::new(file.filename.clone(), file.hash.clone(), encoded_frames);

	storage::save_video(&sidecar, media_dir, &file.hash)?;
	Ok(())
}
