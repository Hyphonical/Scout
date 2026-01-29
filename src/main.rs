//! Scout - AI-powered semantic image search
//!
//! A command-line tool for semantic image search using SigLIP2 models.
//! Supports text-based, image-based, and hybrid search queries.

mod cli;
mod config;
mod live;
mod logger;
mod models;
mod runtime;
mod scanner;
mod search;
mod sidecar;
mod types;
mod video;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use colored::Colorize;
use image::DynamicImage;
use std::path::Path;
use std::time::Instant;

use cli::{Cli, Command};
use logger::{log, summary, Level};
use models::ModelManager;
use runtime::set_provider;
use scanner::{scan_directory, ScanFilters};
use search::{search, SearchQuery};
use sidecar::ImageSidecar;
use sidecar::VideoSidecar;
use types::{CombineWeight, MediaType};

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
			let filters = ScanFilters::new(min_width, min_height, min_size_kb, max_size_mb, exclude_patterns);
			run_scan(&directory, recursive, force, &filters)
		}
		Command::Search {
			query,
			image,
			weight,
			directory,
			recursive,
			limit,
			min_score,
			open,
			include_ref,
		} => {
			run_search(
				query.as_deref(),
				image.as_deref(),
				weight,
				&directory,
				recursive,
				limit,
				min_score,
				open,
				include_ref,
			)
		}
		Command::Live { directory, recursive } => {
			live::run(&directory, recursive)
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

		Command::Clean { directory, recursive, auto_confirm } => {
			run_clean(&directory, recursive, auto_confirm)
		}
	}
}

fn run_scan(directory: &Path, recursive: bool, force: bool, filters: &ScanFilters) -> Result<()> {
	print_header();

	log(Level::Info, "Scanning for images...");
	let scan = scan_directory(directory, recursive, force, filters)?;

	if scan.filtered_count > 0 {
		log(
			Level::Info,
			&format!("Filtered {} images (--verbose for details)", scan.filtered_count),
		);
	}

	log(
		Level::Success,
		&format!(
			"Found {} images ({} to process, {} indexed, {} filtered)",
			scan.total(),
			scan.to_process.len(),
			scan.indexed_count,
			scan.filtered_count
		),
	);

	if scan.outdated_count > 0 {
		log(
			Level::Info,
			&format!("Upgrading {} outdated sidecars to v{}", scan.outdated_count, env!("CARGO_PKG_VERSION")),
		);
	}

	if scan.error_count > 0 {
		log(Level::Warning, &format!("{} errors during scan", scan.error_count));
	}

	if scan.to_process.is_empty() {
		log(Level::Info, "No new images to process");
		return Ok(());
	}

	log(Level::Info, "Loading vision model...");
	let load_start = Instant::now();
	let mut models = ModelManager::with_vision()?;
	log(Level::Success, &format!("Model ready in {:.2}s", load_start.elapsed().as_secs_f32()));

	let process_start = Instant::now();
	let (processed, errors) = process_images(&scan.to_process, &mut models)?;

	summary(
		processed,
		scan.indexed_count,
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

#[allow(clippy::too_many_arguments)]
fn run_search(
	query: Option<&str>,
	image: Option<&Path>,
	weight: f32,
	directory: &Path,
	recursive: bool,
	limit: usize,
	min_score: f32,
	open_result: bool,
	include_ref: bool,
) -> Result<()> {
	if query.is_none() && image.is_none() {
		log(Level::Error, "Must provide text query or --image (or both)");
		std::process::exit(1);
	}

	print_header();

	let root = directory.canonicalize().unwrap_or_else(|_| directory.to_path_buf());
	let weight = CombineWeight::new(weight).unwrap();

	let search_desc = match (&query, &image) {
		(Some(q), Some(img)) => {
			let name = img.file_name().map(|n| n.to_string_lossy()).unwrap_or_else(|| img.to_string_lossy());
			format!("\"{}\" + {} ({:.0}% text)", q.bright_blue(), name.yellow(), weight.value() * 100.0)
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
		(Some(q), Some(img)) => SearchQuery::Combined { text: q, image: img, weight },
		(Some(q), None) => SearchQuery::Text(q),
		(None, Some(img)) => SearchQuery::Image(img),
		(None, None) => unreachable!(),
	};

	let exclude = if include_ref { None } else { image };
	let results = search(&root, search_query, min_score, exclude, recursive);

	if results.is_empty() {
		log(Level::Warning, "No matches found");
		return Ok(());
	}

	log(Level::Success, &format!("Found {} matches", results.len()));
	println!();

	for (i, result) in results.iter().take(limit).enumerate() {
		let name = result.path
			.file_name()
			.and_then(|n| n.to_str())
			.unwrap_or("unknown");

		let score_pct = format!("{:.0}%", result.score * 100.0).dimmed();
		let rank = format!("#{}", i + 1).bright_blue().bold();
		let link = logger::hyperlink(name, &result.path);

		if result.media_type == types::MediaType::Video {
			if let Some(ts) = result.timestamp {
				let timestamp = video::format_timestamp(ts);
				let ts_display = format!("@{}", timestamp).yellow();
				println!("  {} {} {} {}", rank, link, ts_display, score_pct);
			} else {
				println!("  {} {} {}", rank, link, score_pct);
			}
		} else {
			println!("  {} {} {}", rank, link, score_pct);
		}
	}

	if open_result && !results.is_empty() {
		let best = &results[0].path;
		log(Level::Info, &format!("Opening: {}", best.to_string_lossy()));
		if let Err(e) = open::that(best) {
			log(Level::Warning, &format!("Failed to open: {}", e));
		}
	}

	println!();
	Ok(())
}

fn process_images(images: &[scanner::ImageEntry], models: &mut ModelManager) -> Result<(usize, usize)> {
	let total = images.len();
	let mut processed = 0;
	let mut errors = 0;

	println!();
	println!("{}", "─── Processing ───".bright_blue().bold());

	for (index, entry) in images.iter().enumerate() {
		let queue = format!("[{}/{}]", index + 1, total).bright_blue().bold();

		let start = Instant::now();
		
		match entry.media_type {
			MediaType::Image => {
				match models.encode_image(&entry.path) {
					Ok((embedding, hash)) => {
						let processing_ms = start.elapsed().as_millis() as u64;
						let sidecar = ImageSidecar::new(&entry.filename, hash, embedding, processing_ms);

						if let Err(e) = sidecar.save(&entry.sidecar_path) {
							log(Level::Error, &format!("{} {}: {}", queue, entry.filename, e));
							errors += 1;
							continue;
						}

						let timing = format!("{}ms", processing_ms).dimmed();
						let link = logger::hyperlink(&entry.filename, &entry.path);
						log(Level::Success, &format!("{} {} {}", queue, link, timing));
						processed += 1;
					}
					Err(e) => {
						let link = logger::hyperlink(&entry.filename, &entry.path);
						log(Level::Error, &format!("{} {}: {}", queue, link, e));
						errors += 1;
					}
				}
			}
			MediaType::Video => {
				if !video::is_ffmpeg_available() {
					video::show_ffmpeg_warning_once();
					errors += 1;
					continue;
				}
				
				match video::extract_frames(&entry.path, 10) {
					Ok(frames) => {
						let mut frame_embeddings = Vec::new();
						
						for (timestamp_secs, image) in frames {
							match models.encode_image_from_dynamic(&DynamicImage::ImageRgb8(image)) {
								Ok((emb, _)) => {
									frame_embeddings.push((timestamp_secs, emb));
								}
								Err(e) => {
									log(Level::Warning, &format!("{} Frame extraction error: {}", queue, e));
								}
							}
						}

						if frame_embeddings.is_empty() {
							log(Level::Error, &format!("{} {}: No frames extracted", queue, entry.filename));
							errors += 1;
							continue;
						}

						let processing_ms = start.elapsed().as_millis() as u64;
						let hash = sidecar::compute_file_hash(&entry.path)?;
						let sidecar = VideoSidecar::new(&entry.filename, hash, frame_embeddings, processing_ms);

						if let Err(e) = sidecar.save(&entry.sidecar_path) {
							log(Level::Error, &format!("{} {}: {}", queue, entry.filename, e));
							errors += 1;
							continue;
						}

						let timing = format!("{}ms", processing_ms).dimmed();
						let link = logger::hyperlink(&entry.filename, &entry.path);
						log(Level::Success, &format!("{} {} {} 🎥", queue, link, timing));
						processed += 1;
					}
					Err(e) => {
						let link = logger::hyperlink(&entry.filename, &entry.path);
						log(Level::Error, &format!("{} {}: {}", queue, link, e));
						errors += 1;
					}
				}
			}
		}
	}

	Ok((processed, errors))
}

fn run_clean(directory: &Path, recursive: bool, auto_confirm: bool) -> Result<()> {
	use std::io::{self, Write};

	print_header();
	log(Level::Info, "Scanning for orphaned sidecars...");

	let root = directory.canonicalize().unwrap_or_else(|_| directory.to_path_buf());
	let mut orphaned = Vec::new();

	for (sidecar_path, base_dir) in sidecar::iter_sidecars(&root, recursive) {
		if let Ok(sidecar) = sidecar::ImageSidecar::load(&sidecar_path) {
			let image_path = base_dir.join(&sidecar.filename);
			if !image_path.exists() {
				orphaned.push((sidecar_path, image_path));
			}
		}
	}

	if orphaned.is_empty() {
		log(Level::Success, "No orphaned sidecars found");
		return Ok(());
	}

	log(Level::Warning, &format!("Found {} orphaned sidecars", orphaned.len()));

	for (_, missing) in &orphaned {
		log(Level::Error, &missing.display().to_string().dimmed().to_string());
	}

	if !auto_confirm {
		print!("\nDelete these sidecars? [y/N]: ");
		io::stdout().flush()?;

		let mut input = String::new();
		io::stdin().read_line(&mut input)?;

		if !input.trim().eq_ignore_ascii_case("y") {
			log(Level::Info, "Cancelled");
			return Ok(());
		}
	}

	let mut deleted = 0;
	let mut errors = 0;

	for (sidecar_path, _) in orphaned {
		match std::fs::remove_file(&sidecar_path) {
			Ok(_) => deleted += 1,
			Err(e) => {
				log(Level::Error, &format!("Failed to delete {}: {}", sidecar_path.display(), e));
				errors += 1;
			}
		}
	}

	log(Level::Success, &format!("Deleted {} orphaned sidecars", deleted));
	if errors > 0 {
		log(Level::Warning, &format!("{} errors", errors));
	}

	Ok(())
}

fn print_header() {
	println!();
	println!(
		"{}",
		format!("─── Scout v{} ───", env!("CARGO_PKG_VERSION"))
			.bright_blue()
			.bold()
	);
}
