//! Video frame extraction using FFmpeg via rsmpeg
//!
//! Extracts evenly-spaced frames from video files for embedding generation.
//! Uses system-installed FFmpeg via rsmpeg bindings.
//! Video support is enabled at runtime only if FFmpeg is found on the system.

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
use std::sync::OnceLock;

static FFMPEG_AVAILABLE: OnceLock<bool> = OnceLock::new();
static FFMPEG_WARNING_SHOWN: OnceLock<bool> = OnceLock::new();

/// Checks if FFmpeg is available on the system at runtime
pub fn is_ffmpeg_available() -> bool {
	*FFMPEG_AVAILABLE.get_or_init(|| {
		// Try to initialize FFmpeg by checking if we can access the version
		// This will fail if FFmpeg libraries are not installed
		std::panic::catch_unwind(|| {
			unsafe { ffi::av_version_info() };
			true
		}).unwrap_or(false)
	})
}

/// Shows a one-time warning that FFmpeg is not installed
pub fn show_ffmpeg_warning_once() {
	FFMPEG_WARNING_SHOWN.get_or_init(|| {
		crate::logger::log(
			crate::logger::Level::Warning,
			"Video files found but FFmpeg not installed. Skipping videos. Install FFmpeg for video support.",
		);
		true
	});
}

/// Extracts N evenly-spaced frames from a video file
///
/// # Arguments
/// * `video_path` - Path to the video file
/// * `count` - Number of frames to extract
///
/// # Returns
/// Vector of tuples: (timestamp_seconds, RgbImage)
pub fn extract_frames(video_path: &Path, count: usize) -> Result<Vec<(f64, RgbImage)>> {
	if !is_ffmpeg_available() {
		anyhow::bail!("FFmpeg not found. Install FFmpeg to enable video support.");
	}

	if count == 0 {
		anyhow::bail!("frame count must be at least 1");
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

	// Initialize swscale context with all 10 parameters
	let mut sws_ctx = SwsContext::get_context(
		decode_ctx.width,
		decode_ctx.height,
		decode_ctx.pix_fmt,
		decode_ctx.width,
		decode_ctx.height,
		ffi::AV_PIX_FMT_RGB24,
		ffi::SWS_BILINEAR,
		None, // src_filter
		None, // dst_filter
		None, // param
	)
	.context("Failed to initialize swscale context")?;

	// Create destination frame for RGB24
	let mut dst_frame = AVFrame::new();
	dst_frame.set_format(ffi::AV_PIX_FMT_RGB24);
	dst_frame.set_width(decode_ctx.width);
	dst_frame.set_height(decode_ctx.height);
	dst_frame.alloc_buffer().context("Failed to allocate destination frame buffer")?;

	// Convert frame to RGB24
	sws_ctx.scale_frame(&frame, 0, decode_ctx.height, &mut dst_frame)
		.context("Failed to scale frame")?;

	// Copy RGB data from dst_frame
	let line_size = dst_frame.linesize[0] as usize;
	let expected_line_size = (width * 3) as usize;
	unsafe {
		let src_ptr = dst_frame.data[0];
		for y in 0..height as usize {
			let src_row = std::slice::from_raw_parts(src_ptr.add(y * line_size), expected_line_size);
			let dst_row = &mut rgb_data[y * expected_line_size..(y + 1) * expected_line_size];
			dst_row.copy_from_slice(src_row);
		}
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
