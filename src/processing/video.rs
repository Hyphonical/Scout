//! Video frame extraction using FFmpeg

use anyhow::{Context, Result};
use image::RgbImage;
use serde::Deserialize;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use crate::ui;

static FFMPEG_AVAILABLE: OnceLock<bool> = OnceLock::new();
static FFPROBE_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if FFmpeg is available in PATH
pub fn is_available() -> bool {
	*FFMPEG_AVAILABLE.get_or_init(|| {
		Command::new("ffmpeg")
			.arg("-version")
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.map(|s| s.success())
			.unwrap_or(false)
	})
}

/// Check if ffprobe is available in PATH
fn is_ffprobe_available() -> bool {
	*FFPROBE_AVAILABLE.get_or_init(|| {
		Command::new("ffprobe")
			.arg("-version")
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.map(|s| s.success())
			.unwrap_or(false)
	})
}

#[derive(Deserialize)]
struct ProbeFormat {
	duration: Option<String>,
}

#[derive(Deserialize)]
struct ProbeStream {
	codec_type: String,
	width: Option<u32>,
	height: Option<u32>,
	r_frame_rate: Option<String>,
}

#[derive(Deserialize)]
struct ProbeOutput {
	streams: Vec<ProbeStream>,
	format: ProbeFormat,
}

/// Get video metadata (duration, dimensions, fps)
fn probe_video(path: &Path) -> Result<(f64, u32, u32, f64)> {
	if !is_ffprobe_available() {
		anyhow::bail!("ffprobe not found in PATH");
	}
	
	let output = Command::new("ffprobe")
		.arg("-v").arg("error")
		.arg("-print_format").arg("json")
		.arg("-show_format")
		.arg("-show_streams")
		.arg(path)
		.output()
		.context("Failed to run ffprobe")?;
	
	if !output.status.success() {
		anyhow::bail!("ffprobe failed");
	}
	
	let probe: ProbeOutput = serde_json::from_slice(&output.stdout)
		.context("Failed to parse ffprobe output")?;
	
	let video_stream = probe.streams.iter()
		.find(|s| s.codec_type == "video")
		.context("No video stream found")?;
	
	let width = video_stream.width.context("Missing width")?;
	let height = video_stream.height.context("Missing height")?;
	
	let duration: f64 = probe.format.duration
		.context("Missing duration")?
		.parse()
		.context("Invalid duration")?;
	
	// Parse frame rate (format: "30/1" or "24000/1001")
	let fps = if let Some(fps_str) = &video_stream.r_frame_rate {
		parse_fraction(fps_str).unwrap_or(30.0)
	} else {
		30.0
	};
	
	Ok((duration, width, height, fps))
}

fn parse_fraction(s: &str) -> Option<f64> {
	let parts: Vec<&str> = s.split('/').collect();
	if parts.len() == 2 {
		let num: f64 = parts[0].parse().ok()?;
		let den: f64 = parts[1].parse().ok()?;
		Some(num / den)
	} else {
		s.parse().ok()
	}
}

/// Extract evenly-spaced frames from video
pub fn extract_frames(path: &Path, count: usize) -> Result<Vec<(f64, RgbImage)>> {
	if !is_available() {
		anyhow::bail!("FFmpeg not found in PATH");
	}
	
	if count == 0 {
		anyhow::bail!("Frame count must be at least 1");
	}
	
	ui::debug(&format!("Extracting {} frames from: {}", count, path.display()));
	
	let (duration, width, height, fps) = probe_video(path)?;
	
	if duration <= 0.0 {
		anyhow::bail!("Invalid video duration: {:.2}s", duration);
	}
	
	// Calculate timestamps for evenly-spaced frames
	let interval = duration / count as f64;
	let timestamps: Vec<f64> = (0..count)
		.map(|i| (i as f64 + 0.5) * interval)
		.collect();
	
	// Calculate frame numbers
	let frame_numbers: Vec<usize> = timestamps.iter()
		.map(|&ts| (ts * fps).round() as usize)
		.collect();
	
	// Build select filter
	let select_expr = frame_numbers.iter()
		.map(|n| format!("eq(n,{})", n))
		.collect::<Vec<_>>()
		.join("+");
	
	ui::debug(&format!("Video: {:.1}s, {}x{}, {:.1}fps", duration, width, height, fps));
	
	// Extract frames in one FFmpeg call
	let mut child = Command::new("ffmpeg")
		.arg("-i").arg(path)
		.arg("-vf").arg(format!("select='{}'", select_expr))
		.arg("-vsync").arg("0")
		.arg("-f").arg("rawvideo")
		.arg("-pix_fmt").arg("rgb24")
		.arg("-hide_banner")
		.arg("-loglevel").arg("error")
		.arg("pipe:1")
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.context("Failed to spawn FFmpeg")?;
	
	// Read all frame data
	let mut frame_data = Vec::new();
	if let Some(mut stdout) = child.stdout.take() {
		stdout.read_to_end(&mut frame_data)
			.context("Failed to read frames from FFmpeg")?;
	}
	
	let status = child.wait().context("FFmpeg process failed")?;
	
	if !status.success() {
		let mut stderr = String::new();
		if let Some(mut err) = child.stderr {
			err.read_to_string(&mut stderr).ok();
		}
		anyhow::bail!("FFmpeg failed: {}", stderr.trim());
	}
	
	// Parse frames
	let frame_size = (width * height * 3) as usize;
	let actual_count = frame_data.len() / frame_size;
	
	if actual_count == 0 {
		anyhow::bail!("No frames extracted");
	}
	
	let mut frames = Vec::new();
	for (i, chunk) in frame_data.chunks_exact(frame_size).enumerate() {
		if i >= timestamps.len() {
			break;
		}
		
		let image = RgbImage::from_raw(width, height, chunk.to_vec())
			.context("Failed to create image from frame data")?;
		
		frames.push((timestamps[i], image));
	}
	
	ui::debug(&format!("Extracted {} frames", frames.len()));
	
	Ok(frames)
}

/// Format timestamp as MM:SS
pub fn format_timestamp(seconds: f64) -> String {
	let total = seconds.floor() as u64;
	let minutes = total / 60;
	let secs = total % 60;
	format!("{:02}:{:02}", minutes, secs)
}
