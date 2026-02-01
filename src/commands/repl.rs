//! REPL mode - interactive search session

use anyhow::Result;
use colored::Colorize;
use std::io::{self, Write};
use std::path::Path;

use crate::models::Models;
use crate::storage;
use crate::ui;

pub fn run(
	dir: &Path,
	recursive: bool,
	limit: usize,
	min_score: f32,
	exclude_videos: bool,
) -> Result<()> {
	ui::info("Starting interactive search mode");
	ui::info("Type your queries, or 'exit' to quit");
	println!();

	// Load models once
	let mut models = Models::new()?;

	// Pre-scan sidecars
	ui::debug("Loading index...");
	let sidecars = storage::scan(dir, recursive);

	if sidecars.is_empty() {
		ui::warn("No indexed files found. Run 'scout scan' first.");
		return Ok(());
	}

	ui::success(&format!("Loaded {} indexed files", sidecars.len()));
	println!();

	loop {
		print!("{} ", "scout>".bright_blue().bold());
		io::stdout().flush()?;

		let mut input = String::new();
		io::stdin().read_line(&mut input)?;

		let query = input.trim();

		if query.is_empty() {
			continue;
		}

		if query == "exit" || query == "quit" || query == "q" {
			ui::info("Goodbye!");
			break;
		}

		if query == "help" {
			show_help();
			continue;
		}

		// Perform search
		match search_once(
			&mut models,
			query,
			&sidecars,
			limit,
			min_score,
			exclude_videos,
		) {
			Ok(count) => {
				if count == 0 {
					ui::warn("No matches found");
				}
			}
			Err(e) => {
				ui::error(&format!("Search failed: {}", e));
			}
		}

		println!();
	}

	Ok(())
}

fn search_once(
	models: &mut Models,
	query: &str,
	sidecars: &[(std::path::PathBuf, std::path::PathBuf)],
	limit: usize,
	min_score: f32,
	exclude_videos: bool,
) -> Result<usize> {
	let start = std::time::Instant::now();

	// Encode query
	let query_emb = models.encode_text(query)?;

	// Search
	let mut matches = Vec::new();

	for (sidecar_path, media_dir) in sidecars {
		let Ok(sidecar) = storage::load(sidecar_path) else {
			continue;
		};

		match sidecar {
			storage::Sidecar::Image(img) => {
				let score = query_emb.similarity(&img.embedding());

				if score >= min_score {
					let path = media_dir.join(img.filename());
					matches.push((path, score, None));
				}
			}
			storage::Sidecar::Video(vid) => {
				if exclude_videos {
					continue;
				}

				let mut best_score = 0.0;
				let mut best_ts = 0.0;

				for (ts, emb) in vid.frames() {
					let score = query_emb.similarity(&emb);
					if score > best_score {
						best_score = score;
						best_ts = ts;
					}
				}

				if best_score >= min_score {
					let path = media_dir.join(vid.filename());
					matches.push((path, best_score, Some(best_ts)));
				}
			}
		}
	}

	matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
	matches.truncate(limit);

	let duration = start.elapsed().as_millis();

	// Display results
	if !matches.is_empty() {
		for (i, (path, score, timestamp)) in matches.iter().enumerate() {
			let link = ui::log::path_link(path, 100);
			let percentage = (score * 100.0).round() as u32;

			let timestamp_str = if let Some(ts) = timestamp {
				format!(" @ {}", crate::processing::video::format_timestamp(*ts))
			} else {
				String::new()
			};

			println!(
				"{}. {}{} {}",
				format!("{:2}", i + 1).bright_blue().bold(),
				link.bright_blue(),
				timestamp_str.dimmed(),
				format!("{}%", percentage).dimmed(),
			);
		}

		println!(
			"\n{} {} in {}ms",
			"✓".bright_blue().bold(),
			format!("Found {} matches", matches.len()).bright_white(),
			duration
		);
	}

	Ok(matches.len())
}

fn show_help() {
	println!("{}", "REPL Commands:".bright_blue().bold());
	println!("  {}  Enter a search query", "<text>".dimmed());
	println!("  {}    Show this help message", "help".dimmed());
	println!("  {}    Exit REPL mode", "exit".dimmed());
}
