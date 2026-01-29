//! Video frame extraction and processing
//!
//! Extracts evenly-distributed frames from videos for semantic indexing.
//! Only available when compiled with the "video" feature.

#![cfg(feature = "video")]

use anyhow::{Context, Result};
use image::{DynamicImage, RgbImage};
use std::path::Path;

use crate::config::VIDEO_FRAMES_TO_EXTRACT;

/// Extracted frame with timestamp information
pub struct VideoFrame {
	pub image: DynamicImage,
	pub timestamp_secs: f64,
}

/// Extracts evenly-spaced frames from a video file
///
/// # Arguments
/// * `path` - Path to video file
/// * `num_frames` - Number of frames to extract (default: VIDEO_FRAMES_TO_EXTRACT)
///
/// Returns frames with their timestamps in seconds
pub fn extract_frames(path: &Path, num_frames: Option<usize>) -> Result<Vec<VideoFrame>> {
	let num_frames = num_frames.unwrap_or(VIDEO_FRAMES_TO_EXTRACT);
	
	ffmpeg_next::init()
		.context("Failed to initialize FFmpeg")?;

	let mut ictx = ffmpeg_next::format::input(&path)
		.with_context(|| format!("Failed to open video: {}", path.display()))?;

	let video_stream = ictx.streams()
		.best(ffmpeg_next::media::Type::Video)
		.context("No video stream found")?;
	
	let video_stream_index = video_stream.index();
	let time_base = video_stream.time_base();
	let duration = video_stream.duration();
	
	if duration <= 0 {
		anyhow::bail!("Invalid video duration");
	}

	let mut decoder = ffmpeg_next::codec::context::Context::from_parameters(video_stream.parameters())?
		.decoder()
		.video()
		.context("Failed to create video decoder")?;

	let mut scaler = ffmpeg_next::software::scaling::Context::get(
		decoder.format(),
		decoder.width(),
		decoder.height(),
		ffmpeg_next::format::Pixel::RGB24,
		decoder.width(),
		decoder.height(),
		ffmpeg_next::software::scaling::Flags::BILINEAR,
	).context("Failed to create scaler")?;

	let mut frames = Vec::new();
	let frame_interval = duration / num_frames as i64;

	for i in 0..num_frames {
		let target_pts = i as i64 * frame_interval;
		let timestamp = ffmpeg_next::rescale::TIME_BASE.rescale(target_pts, time_base);
		
		ictx.seek(timestamp, ..timestamp)
			.context("Failed to seek in video")?;

		let mut decoded = ffmpeg_next::util::frame::video::Video::empty();
		let mut found = false;

		for (stream, packet) in ictx.packets() {
			if stream.index() == video_stream_index {
				decoder.send_packet(&packet)?;
				
				while decoder.receive_frame(&mut decoded).is_ok() {
					let mut rgb_frame = ffmpeg_next::util::frame::video::Video::empty();
					scaler.run(&decoded, &mut rgb_frame)?;
					
					if let Some(img) = frame_to_image(&rgb_frame) {
						let timestamp_secs = target_pts as f64 * time_base.0 as f64 / time_base.1 as f64;
						frames.push(VideoFrame {
							image: DynamicImage::ImageRgb8(img),
							timestamp_secs,
						});
						found = true;
						break;
					}
				}
				
				if found {
					break;
				}
			}
		}
	}

	if frames.is_empty() {
		anyhow::bail!("No frames could be extracted from video");
	}

	Ok(frames)
}

/// Converts FFmpeg frame to image::RgbImage
fn frame_to_image(frame: &ffmpeg_next::util::frame::video::Video) -> Option<RgbImage> {
	let width = frame.width();
	let height = frame.height();
	let data = frame.data(0);
	let stride = frame.stride(0);

	let mut img = RgbImage::new(width, height);
	
	for y in 0..height {
		let row_start = (y as usize) * stride;
		for x in 0..width {
			let pixel_start = row_start + (x as usize) * 3;
			if pixel_start + 2 < data.len() {
				img.put_pixel(x, y, image::Rgb([
					data[pixel_start],
					data[pixel_start + 1],
					data[pixel_start + 2],
				]));
			}
		}
	}

	Some(img)
}

/// Formats timestamp as MM:SS
pub fn format_timestamp(seconds: f64) -> String {
	let mins = (seconds / 60.0) as u32;
	let secs = (seconds % 60.0) as u32;
	format!("{:02}:{:02}", mins, secs)
}
