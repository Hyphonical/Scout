//! # Video Processing
//!
//! Extract representative frames using FFmpeg scene detection.
//! Encodes each frame for temporal video search.

use anyhow::{Context, Result};
use image::RgbImage;
use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use crate::ui;

static FFMPEG_AVAILABLE: OnceLock<bool> = OnceLock::new();
static FFPROBE_AVAILABLE: OnceLock<bool> = OnceLock::new();
static CUSTOM_FFMPEG: OnceLock<PathBuf> = OnceLock::new();

pub fn set_ffmpeg_path(path: PathBuf) {
	let _ = CUSTOM_FFMPEG.set(path);
}

fn get_ffmpeg_tool_binary(tool_name: &str) -> String {
	if let Some(custom) = CUSTOM_FFMPEG.get() {
		if tool_name == "ffmpeg" {
			return custom.to_string_lossy().to_string();
		}
		// For other tools (ffprobe), look in same directory as custom ffmpeg
		if let Some(parent) = custom.parent() {
			let tool_path = parent.join(tool_name);
			if tool_path.exists() {
				return tool_path.to_string_lossy().to_string();
			}
			// Try with .exe extension on Windows
			let tool_exe = parent.join(format!("{}.exe", tool_name));
			if tool_exe.exists() {
				return tool_exe.to_string_lossy().to_string();
			}
		}
	}
	tool_name.to_string()
}

fn get_ffmpeg_binary() -> String {
	get_ffmpeg_tool_binary("ffmpeg")
}

fn get_ffprobe_binary() -> String {
	get_ffmpeg_tool_binary("ffprobe")
}

/// Check if FFmpeg is available in PATH
pub fn is_available() -> bool {
	*FFMPEG_AVAILABLE.get_or_init(|| {
		Command::new(get_ffmpeg_binary())
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
		Command::new(get_ffprobe_binary())
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

	let output = Command::new(get_ffprobe_binary())
		.arg("-v")
		.arg("error")
		.arg("-print_format")
		.arg("json")
		.arg("-show_format")
		.arg("-show_streams")
		.arg(path)
		.output()
		.context("Failed to run ffprobe")?;

	if !output.status.success() {
		anyhow::bail!("ffprobe failed");
	}

	let probe: ProbeOutput =
		serde_json::from_slice(&output.stdout).context("Failed to parse ffprobe output")?;

	let video_stream = probe
		.streams
		.iter()
		.find(|s| s.codec_type == "video")
		.context("No video stream found")?;

	let width = video_stream.width.context("Missing width")?;
	let height = video_stream.height.context("Missing height")?;

	let duration: f64 = probe
		.format
		.duration
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

/// Extract frames using scene detection
pub fn extract_frames_scene(
	path: &Path,
	max_frames: usize,
	threshold: f32,
) -> Result<Vec<(f64, RgbImage)>> {
	if !is_available() {
		anyhow::bail!("FFmpeg not found in PATH");
	}

	if max_frames == 0 {
		anyhow::bail!("Max frames must be at least 1");
	}

	let (duration, width, height, fps) = probe_video(path)?;

	if duration <= 0.0 {
		anyhow::bail!("Invalid video duration: {:.2}s", duration);
	}

	// First pass: detect scene changes
	let scene_times = detect_scenes(path, threshold)?;

	let frame_count = scene_times.len();
	let timestamps = if frame_count <= max_frames {
		// Use all detected scenes
		scene_times
	} else {
		// Too many scenes - sample evenly from detected scenes
		sample_timestamps(&scene_times, max_frames)
	};

	let actual_count = timestamps.len();

	ui::debug(&format!(
		"Video: {:.1}s, {}x{} @ {:.1}fps | Scenes: {} → Frames: {}",
		duration, width, height, fps, frame_count, actual_count
	));

	if timestamps.is_empty() {
		anyhow::bail!("No scene changes detected");
	}

	// Extract frames at detected timestamps
	extract_frames_at_timestamps(path, &timestamps, width, height)
}

/// Detect scene changes in video and return timestamps
fn detect_scenes(path: &Path, threshold: f32) -> Result<Vec<f64>> {
	// Use FFmpeg's scene detection filter
	let output = Command::new(get_ffmpeg_binary())
		.arg("-i")
		.arg(path)
		.arg("-vf")
		.arg(format!("select='gt(scene,{})',showinfo", threshold))
		.arg("-f")
		.arg("null")
		.arg("-")
		.stderr(Stdio::piped())
		.output()
		.context("Failed to run FFmpeg scene detection")?;

	if !output.status.success() {
		anyhow::bail!("FFmpeg scene detection failed");
	}

	// Parse scene timestamps from stderr
	let stderr = String::from_utf8_lossy(&output.stderr);
	let mut timestamps = Vec::new();

	for line in stderr.lines() {
		if line.contains("pts_time:") {
			if let Some(pts_start) = line.find("pts_time:") {
				let pts_str = &line[pts_start + 9..];
				if let Some(end) = pts_str.find(char::is_whitespace) {
					if let Ok(time) = pts_str[..end].parse::<f64>() {
						timestamps.push(time);
					}
				}
			}
		}
	}

	// Always include first frame if no scenes detected
	if timestamps.is_empty() {
		timestamps.push(0.5);
	}

	Ok(timestamps)
}

/// Sample timestamps evenly from a larger set
fn sample_timestamps(timestamps: &[f64], count: usize) -> Vec<f64> {
	if timestamps.len() <= count {
		return timestamps.to_vec();
	}

	let step = timestamps.len() as f64 / count as f64;
	(0..count)
		.map(|i| {
			let idx = (i as f64 * step).floor() as usize;
			timestamps[idx.min(timestamps.len() - 1)]
		})
		.collect()
}

/// Extract frames at specific timestamps
fn extract_frames_at_timestamps(
	path: &Path,
	timestamps: &[f64],
	width: u32,
	height: u32,
) -> Result<Vec<(f64, RgbImage)>> {
	let mut frames = Vec::new();

	for &timestamp in timestamps {
		// Extract single frame at timestamp
		let mut child = Command::new(get_ffmpeg_binary())
			.arg("-ss")
			.arg(format!("{:.3}", timestamp))
			.arg("-i")
			.arg(path)
			.arg("-frames:v")
			.arg("1")
			.arg("-f")
			.arg("rawvideo")
			.arg("-pix_fmt")
			.arg("rgb24")
			.arg("-hide_banner")
			.arg("-loglevel")
			.arg("error")
			.arg("pipe:1")
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()
			.context("Failed to spawn FFmpeg")?;

		let mut frame_data = Vec::new();
		if let Some(mut stdout) = child.stdout.take() {
			stdout
				.read_to_end(&mut frame_data)
				.context("Failed to read frame from FFmpeg")?;
		}

		let status = child.wait().context("FFmpeg process failed")?;

		if !status.success() {
			continue; // Skip failed frames
		}

		let frame_size = (width * height * 3) as usize;
		if frame_data.len() >= frame_size {
			if let Some(image) =
				RgbImage::from_raw(width, height, frame_data[..frame_size].to_vec())
			{
				frames.push((timestamp, image));
			}
		}
	}

	if frames.is_empty() {
		anyhow::bail!("Failed to extract any frames");
	}

	Ok(frames)
}

/// Format timestamp as MM:SS
pub fn format_timestamp(seconds: f64) -> String {
	let total = seconds.floor() as u64;
	let minutes = total / 60;
	let secs = total % 60;
	format!("{:02}:{:02}", minutes, secs)
}
