// Scout - AI-powered semantic image search

mod cli;
mod config;
mod embedder;
mod embedding;
mod live;
mod logger;
mod processor;
mod runtime;
mod scanner;
mod search;
mod sidecar;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::time::Instant;

use cli::{Cli, Command, ScanFilters};
use logger::{log, summary, Level};
use processor::VisionEncoder;
use runtime::set_provider;
use scanner::{scan_directory, ImageEntry};
use search::{search_with_query, SearchQuery};
use sidecar::ImageSidecar;
use live::run_live_search;

fn main() -> Result<()> {
	let cli = Cli::parse();

	logger::set_verbose(cli.verbose);
	set_provider(cli.provider);

	match cli.command {
		Command::Scan {
			directory,
			recursive,
			force,
			min_width,
			min_height,
			min_size_kb,
			max_size_mb,
			exclude_patterns,
		} => {
			let filters = ScanFilters::from_scan_command(
				min_width,
				min_height,
				min_size_kb,
				max_size_mb,
				exclude_patterns,
			);
			run_scan(&directory, recursive, force, &filters)
		}
		Command::Search { query, image, weight, directory, recursive, limit, min_score, open, include_ref } => {
			run_search(query.as_deref(), image.as_ref(), weight, &directory, recursive, limit, min_score, open, include_ref)
		}
		Command::Live { directory, recursive } => {
			run_live_search(&directory, recursive)
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

fn run_scan(
	directory: &Path,
	recursive: bool,
	force: bool,
	filters: &ScanFilters,
) -> Result<()> {
	println!();
	println!(
		"{}",
		format!("─── Scout v{} ───", env!("CARGO_PKG_VERSION"))
			.bright_blue()
			.bold()
	);

	log(Level::Info, "Scanning for images...");
	let scan = scan_directory(directory, recursive, force, filters)?;

	if !scan.filtered.is_empty() {
		log(
			Level::Info,
			&format!("Filtered {} images (--verbose for details)", scan.filtered.len()),
		);
		for filtered in &scan.filtered {
			log(
				Level::Debug,
				&format!("Filtered: {} - {}", filtered.path.display(), filtered.reason),
			);
		}
	}

	log(
		Level::Success,
		&format!(
			"Found {} images ({} to process, {} indexed, {} filtered)",
			scan.total(),
			scan.images.len(),
			scan.skipped.len(),
			scan.filtered.len()
		),
	);

	if scan.outdated > 0 {
		log(
			Level::Info,
			&format!("Upgrading {} outdated sidecars to v{}", scan.outdated, env!("CARGO_PKG_VERSION")),
		);
	}

	for err in &scan.errors {
		log(Level::Warning, err);
	}

	if scan.images.is_empty() {
		log(Level::Info, "No new images to process");
		return Ok(());
	}

	log(Level::Info, "Loading vision model...");
	let load_start = Instant::now();
	let encoder = VisionEncoder::new().context("Failed to load vision model")?;
	log(
		Level::Success,
		&format!("Model ready in {:.2}s", load_start.elapsed().as_secs_f32()),
	);

	let process_start = Instant::now();
	let (processed, errors) = process_images(&scan.images, &encoder);
	summary(
		processed,
		scan.skipped.len(),
		errors,
		process_start.elapsed().as_secs_f32(),
	);

	if errors > 0 {
		log(Level::Warning, &format!("Completed with {} errors", errors));
	} else {
		log(Level::Success, "All images processed");
	}

	Ok(())
}

fn run_search(
	query: Option<&str>,
	image: Option<&PathBuf>,
	weight: f32,
	directory: &Path,
	recursive: bool,
	limit: usize,
	min_score: f32,
	open: bool,
	include_ref: bool,
) -> Result<()> {
	if query.is_none() && image.is_none() {
		log(Level::Error, "Must provide a text query or --image (or both)");
		std::process::exit(1);
	}

	println!();
	println!(
		"{}",
		format!("─── Scout v{} ───", env!("CARGO_PKG_VERSION"))
			.bright_blue()
			.bold()
	);

	let root = directory.canonicalize().unwrap_or_else(|_| directory.to_path_buf());

	// Build search description
	let search_desc = match (&query, &image) {
		(Some(q), Some(img)) => {
			let name = img.file_name().map(|n| n.to_string_lossy()).unwrap_or_else(|| img.to_string_lossy());
			format!("\"{}\" + {} ({:.0}% text)", q.bright_blue(), name.yellow(), weight * 100.0)
		}
		(Some(q), None) => format!("{}", q.bright_blue()),
		(None, Some(img)) => {
			let name = img.file_name().map(|n| n.to_string_lossy()).unwrap_or_else(|| img.to_string_lossy());
			format!("similar to {}", name.yellow())
		}
		(None, None) => unreachable!(),
	};
	log(Level::Info, &format!("Searching: {}", search_desc));

	let search_query = match (&query, &image) {
		(Some(q), Some(img)) => SearchQuery::combined(q, img, weight),
		(Some(q), None) => SearchQuery::text_only(q),
		(None, Some(img)) => SearchQuery::image_only(img),
		(None, None) => unreachable!(),
	};

	let exclude_path = if include_ref { None } else { image.map(|p| p.as_path()) };
	let results = search_with_query(&root, search_query, min_score, exclude_path, recursive);

	if results.is_empty() {
		log(Level::Warning, "No matches found");
		return Ok(());
	}

	log(Level::Success, &format!("Found {} matches", results.len()));
	println!();

	for (i, result) in results.iter().take(limit).enumerate() {
		let name = result
			.path
			.file_name()
			.map(|n| n.to_string_lossy().to_string())
			.unwrap_or_else(|| result.path.to_string_lossy().to_string());

		let score_pct = format!("{:.0}%", result.score * 100.0);
		let score_colored = if result.score >= 0.15 {
			score_pct.bright_blue()
		} else if result.score >= 0.08 {
			score_pct.yellow()
		} else {
			score_pct.dimmed()
		};

		println!(
			"  {} {} {}",
			format!("#{}", i + 1).bright_blue().bold(),
			score_colored,
			name.white()
		);
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
	let total = images.len();
	let mut processed = 0;
	let mut errors = 0;

	println!();
	println!("{}", "─── Processing ───".bright_blue().bold());

	for (index, entry) in images.iter().enumerate() {
		let name = &entry.filename;
		let display = truncate(&entry.path.to_string_lossy(), 50);
		let queue = format!("[{}/{}]", index + 1, total).bright_blue().bold();

		match encoder.process_image(&entry.path) {
			Ok(proc_result) => {
				let sidecar = ImageSidecar::new(
					&entry.filename,
					proc_result.image_hash,
					proc_result.embedding,
					proc_result.processing_ms,
				);

				if let Err(e) = sidecar.save(&entry.sidecar_path) {
					log(Level::Error, &format!("{} {}: {}", queue, name, e));
					errors += 1;
					continue;
				}

				let timing = format!("{}ms", proc_result.processing_ms).dimmed();
				log(Level::Success, &format!("{} {} {}", queue, display, timing));
				processed += 1;
			}
			Err(e) => {
				log(Level::Error, &format!("{} {}: {}", queue, name, e));
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