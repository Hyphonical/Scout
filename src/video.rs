//! Video frame extraction using FFmpeg via rsmpeg
//!
//! Extracts evenly-spaced frames from video files for embedding generation.
//! Uses rsmpeg (maintained FFmpeg bindings with Windows static build support).

#![cfg(feature = "video")]

use anyhow::{Context, Result};
use image::RgbImage;
use rsmpeg::{
	avcodec::AVCodecContext,
	avformat::AVFormatContextInput,
	avutil::AVFrame,
	error::RsmpegError,
	ffi,
	swscale::SwsContext,
};
use std::ffi::CString;
use std::path::Path;

/// Extracts N evenly-spaced frames from a video file
///
/// # Arguments
/// * `video_path` - Path to the video file
/// * `count` - Number of frames to extract
///
/// # Returns
/// Vector of tuples: (timestamp_seconds, RgbImage)
pub fn extract_frames(video_path: &Path, count: usize) -> Result<Vec<(f64, RgbImage)>> {
	if count == 0 {
		anyhow::bail!("Frame count must be at least 1");
	}

	// Open video file
	let path_cstr = CString::new(video_path.to_string_lossy().as_ref())
		.context("Failed to convert path to CString")?;
	let mut input_ctx = AVFormatContextInput::open(&path_cstr)
		.context("Failed to open video file")?;

	// Find video stream
	let (video_stream_idx, decoder) = input_ctx
		.find_best_stream(ffi::AVMEDIA_TYPE_VIDEO)
		.context("Failed to find video stream")?
		.context("No video stream found in file")?;

	let video_stream = &input_ctx.streams()[video_stream_idx];
	let time_base = video_stream.time_base;

	// Initialize decoder
	let mut decode_ctx = AVCodecContext::new(&decoder);
	decode_ctx
		.apply_codecpar(&video_stream.codecpar())
		.context("Failed to apply codec parameters")?;
	decode_ctx.open(None).context("Failed to open decoder")?;

	// Calculate frame positions
	let duration = video_stream.duration as f64 * time_base.num as f64 / time_base.den as f64;
	if duration <= 0.0 {
		anyhow::bail!("Invalid video duration: {}", duration);
	}

	let interval = duration / count as f64;
	let target_timestamps: Vec<f64> = (0..count).map(|i| (i as f64 + 0.5) * interval).collect();

	let mut frames = Vec::new();
	let mut current_ts_idx = 0;

	// Read and decode packets
	while let Some(packet) = input_ctx.read_packet()? {
		if packet.stream_index != video_stream_idx as i32 {
			continue;
		}

		// Send packet to decoder
		decode_ctx.send_packet(Some(&packet))?;

		// Retrieve all frames from this packet
		loop {
			let frame = match decode_ctx.receive_frame() {
				Ok(f) => f,
				Err(RsmpegError::DecoderDrainError) | Err(RsmpegError::DecoderFlushedError) => break,
				Err(e) => return Err(e).context("Error decoding frame")?,
			};

			// Calculate timestamp in seconds
			let pts = frame.pts;
			let timestamp = pts as f64 * time_base.num as f64 / time_base.den as f64;

			// Check if this frame matches our target timestamp
			if current_ts_idx < target_timestamps.len()
				&& timestamp >= target_timestamps[current_ts_idx]
			{
				let rgb_image = frame_to_rgb(&frame, &decode_ctx)?;
				frames.push((timestamp, rgb_image));
				current_ts_idx += 1;

				// Exit early if we have all frames
				if current_ts_idx >= count {
					return Ok(frames);
				}
			}
		}
	}

	// Flush decoder
	decode_ctx.send_packet(None)?;
	loop {
		let frame = match decode_ctx.receive_frame() {
			Ok(f) => f,
			Err(RsmpegError::DecoderDrainError) | Err(RsmpegError::DecoderFlushedError) => break,
			Err(e) => return Err(e).context("Error flushing decoder")?,
		};

		let pts = frame.pts;
		let timestamp = pts as f64 * time_base.num as f64 / time_base.den as f64;

		if current_ts_idx < target_timestamps.len()
			&& timestamp >= target_timestamps[current_ts_idx]
		{
			let rgb_image = frame_to_rgb(&frame, &decode_ctx)?;
			frames.push((timestamp, rgb_image));
			current_ts_idx += 1;

			if current_ts_idx >= count {
				break;
			}
		}
	}

	if frames.is_empty() {
		anyhow::bail!("Failed to extract any frames from video");
	}

	Ok(frames)
}

/// Converts an AVFrame to RgbImage using swscale
fn frame_to_rgb(frame: &AVFrame, decode_ctx: &AVCodecContext) -> Result<RgbImage> {
	let width = decode_ctx.width as u32;
	let height = decode_ctx.height as u32;

	// Create output buffer for RGB24
	let dst_linesize = width as i32 * 3;
	let buffer_size = (dst_linesize * height as i32) as usize;
	let mut rgb_data = vec![0u8; buffer_size];

	// Initialize swscale context
	let mut sws_ctx = SwsContext::get_context(
		decode_ctx.width,
		decode_ctx.height,
		decode_ctx.pix_fmt,
		decode_ctx.width,
		decode_ctx.height,
		ffi::AV_PIX_FMT_RGB24,
		ffi::SWS_BILINEAR,
	)
	.context("Failed to initialize swscale context")?;

	// Convert frame to RGB24
	let dst_data = [rgb_data.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut()];
	let dst_linesize = [dst_linesize, 0, 0, 0];

	unsafe {
		sws_ctx.scale_frame(
			frame.data.as_ptr() as *const *const u8,
			frame.linesize.as_ptr(),
			0,
			decode_ctx.height,
			dst_data.as_ptr() as *const *mut u8,
			dst_linesize.as_ptr(),
		)
		.context("Failed to scale frame")?;
	}

	// Create RgbImage from buffer
	RgbImage::from_raw(width, height, rgb_data)
		.context("Failed to create RgbImage from raw data")
}

/// Formats a timestamp in seconds to MM:SS format
pub fn format_timestamp(seconds: f64) -> String {
	let total_seconds = seconds.floor() as u64;
	let minutes = total_seconds / 60;
	let secs = total_seconds % 60;
	format!("{:02}:{:02}", minutes, secs)
}
