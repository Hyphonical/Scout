// Scout - AI-powered semantic image search using SigLIP2

mod cli;
mod config;
mod embedder;
mod logger;
mod processor;
mod scanner;
mod search;
mod sidecar;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use colored::Colorize;
use std::path::Path;
use std::time::Instant;

use cli::{Cli, Command};
use config::SIDECAR_DIR;
use logger::{log, summary, Level};
use processor::VisionEncoder;
use scanner::{scan_directory, ImageEntry};
use search::search_images;
use sidecar::ImageSidecar;

fn main() -> Result<()> {
	let cli = Cli::parse();

	logger::set_verbose(cli.verbose);

	match cli.command {
		Command::Scan { directory, recursive, force } => {
			run_scan(&directory, recursive, force)
		}
		Command::Search { query, directory, limit, min_score, open } => {
			run_search(&query, &directory, limit, min_score, open)
		}
		Command::Help { subcommand } => {
			let mut cmd = Cli::command();
			if let Some(sub) = subcommand {
				if let Some(sub_cmd) = cmd.find_subcommand_mut(&sub) {
					sub_cmd.print_help().unwrap();
				} else {
					eprintln!("Unknown subcommand: {}", sub);
					cmd.print_help().unwrap();
				}
			} else {
				cmd.print_help().unwrap();
			}
			Ok(())
		}
	}
}

fn run_scan(directory: &Path, recursive: bool, force: bool) -> Result<()> {
	println!();
	println!("{}", format!("─── Scout v{} ───", env!("CARGO_PKG_VERSION")).bright_blue().bold());

	log(Level::Info, "Scanning for images...");
	log(Level::Debug, &format!("Directory: {}, Recursive: {}, Force: {}", directory.display(), recursive, force));
	let scan = scan_directory(directory, recursive, force)?;

	log(Level::Success, &format!(
		"Found {} images ({} to process, {} already indexed)",
		scan.total(), scan.images.len(), scan.skipped.len()
	));

	for err in &scan.errors {
		log(Level::Warning, err);
	}

	if scan.images.is_empty() {
		log(Level::Info, "No new images to process");
		return Ok(());
	}

	log(Level::Info, "Loading vision model...");
	log(Level::Debug, "Initializing VisionEncoder session");
	let load_start = Instant::now();
	let encoder = VisionEncoder::new().context("Failed to load vision model")?;
	log(Level::Success, &format!("Model ready in {:.2}s", load_start.elapsed().as_secs_f32()));

	println!();
	println!("{}", "─── Processing ───".bright_blue().bold());
	log(Level::Debug, &format!("Processing {} images", scan.images.len()));

	let process_start = Instant::now();
	let (processed, errors) = process_images(&scan.images, &encoder);
	summary(processed, scan.skipped.len(), errors, process_start.elapsed().as_secs_f32());

	if errors > 0 {
		log(Level::Warning, &format!("Completed with {} errors", errors));
	} else {
		log(Level::Success, "All images processed");
	}

	Ok(())
}

fn run_search(query: &str, directory: &Path, limit: usize, min_score: f32, open: bool) -> Result<()> {
	println!();
	println!("{}", format!("─── Scout v{} ───", env!("CARGO_PKG_VERSION")).bright_blue().bold());

	let root = directory.canonicalize().unwrap_or_else(|_| directory.to_path_buf());
	let scout_dir = root.join(SIDECAR_DIR);

	if !scout_dir.exists() {
		log(Level::Error, &format!("No {} directory. Run 'scout scan' first.", SIDECAR_DIR));
		std::process::exit(1);
	}

	log(Level::Info, &format!("Searching: {}", query.bright_blue()));
	log(Level::Debug, &format!("Directory: {}, Limit: {}, Min score: {:.2}, Open: {}", directory.display(), limit, min_score, open));
	let results = search_images(&root, query, min_score);

	if results.is_empty() {
		log(Level::Warning, "No matches found");
		return Ok(());
	}

	log(Level::Success, &format!("Found {} matches", results.len()));
	println!();

	for (i, result) in results.iter().take(limit).enumerate() {
		let name = result.path.file_name()
			.map(|n| n.to_string_lossy().to_string())
			.unwrap_or_else(|| result.path.to_string_lossy().to_string());

		// Color-code results based on score
		let score_display = format!("{:.0}%", result.score * 100.0);
		let score_colored = if result.score >= 0.15 {
			score_display.bright_blue()
		} else if result.score >= 0.08 {
			score_display.yellow()
		} else {
			score_display.dimmed()
		};

		let rank = format!("#{}", i + 1).bright_blue().bold();
		
		println!("  {} {} {}", rank, score_colored, name.white());
		println!("      {}", result.path.to_string_lossy().dimmed());
	}

	if open && !results.is_empty() {
		let best = &results[0].path;
		log(Level::Info, &format!("Opening: {}", best.to_string_lossy()));
		if let Err(e) = open::that(best) {
			log(Level::Warning, &format!("Failed to open: {}", e));
		}
	}

	println!();
	Ok(())
}

fn process_images(images: &[ImageEntry], encoder: &VisionEncoder) -> (usize, usize) {
	let mut processed = 0;
	let mut errors = 0;
	let total = images.len();

	for (i, entry) in images.iter().enumerate() {
		let display = truncate(&entry.path.to_string_lossy(), 50);
		log(Level::Debug, &format!("Processing image {}: {}", i + 1, display));
		let image_start = Instant::now();

		match encoder.process_image(&entry.path) {
			Ok(result) => {
				let sidecar = ImageSidecar::new(
					&entry.path,
					result.image_hash,
					result.embedding,
					result.processing_ms,
				);

				if let Err(e) = sidecar.save(&entry.sidecar_path) {
					log(Level::Error, &format!("{} {}", display, format!("Save: {}", e).red()));
					errors += 1;
					continue;
				}

				let elapsed_ms = image_start.elapsed().as_millis();
				let queue = format!("[{}/{}]", i + 1, total).bright_blue().bold();
				let timing = format!("{}ms", elapsed_ms).dimmed();

				log(Level::Success, &format!("{} {} {}", queue, display, timing));
				processed += 1;
			}
			Err(e) => {
				log(Level::Error, &format!("{} {}", display, format!("{}", e).red()));
				errors += 1;
			}
		}
	}

	(processed, errors)
}

fn truncate(s: &str, max: usize) -> String {
	if s.len() > max {
		format!("...{}", &s[s.len() - max + 3..])
	} else {
		s.to_string()
	}
}