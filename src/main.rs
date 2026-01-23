// Scout - AI-powered image tagging and search
//
// Three main workflows:
// - scan: Analyze images with ONNX model, store tags as JSON sidecars
// - search: Find images by matching keywords against stored tags
// - stats: Show aggregate tag statistics across indexed images

mod cli;
mod config;
mod embedder;
mod logger;
mod processor;
mod scanner;
mod search;
mod sidecar;
mod stats;

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use std::time::Instant;

use cli::{Cli, Command};
use config::{GPU_BATCH_THRESHOLD, SIDECAR_DIR};
use embedder::TextEmbedder;
use logger::{header, log, summary, Level};
use processor::ImageProcessor;
use scanner::{scan_inputs, ImageEntry};
use search::search_images;
use stats::calculate_stats;

fn main() -> Result<()> {
	let cli = Cli::parse();

	match cli.command {
		Command::Scan { inputs, recursive, threshold, force, verbose } => {
			run_scan(&inputs, recursive, threshold, force, verbose)
		}
		Command::Search { query, directory, limit, min_score, semantic, open } => {
			run_search(&query, &directory, limit, min_score, semantic, open)
		}
		Command::Stats { directory, limit } => {
			run_stats(&directory, limit)
		}
	}
}

fn run_scan(
	inputs: &[std::path::PathBuf],
	recursive: bool,
	threshold: f32,
	force: bool,
	verbose: bool,
) -> Result<()> {
	if !(0.0..=1.0).contains(&threshold) {
		log(Level::Error, &format!("Threshold must be between 0.0 and 1.0, got {}", threshold));
		std::process::exit(1);
	}

	header(&format!("Scout v{}", env!("CARGO_PKG_VERSION")));

	if verbose {
		log(Level::Debug, &format!("Threshold: {}, Recursive: {}, Force: {}", threshold, recursive, force));
	}

	log(Level::Info, "Scanning for images...");
	let scan = scan_inputs(inputs, recursive, force)?;

	log(Level::Success, &format!(
		"Found {} images ({} to process, {} already tagged)",
		scan.total_found(), scan.images.len(), scan.skipped.len()
	));

	for err in &scan.errors {
		log(Level::Warning, err);
	}

	if scan.images.is_empty() {
		log(Level::Info, "No new images to process");
		return Ok(());
	}

	let use_gpu = scan.images.len() >= GPU_BATCH_THRESHOLD;
	if verbose {
		log(Level::Debug, &format!(
			"Batch size {} {} GPU threshold of {}",
			scan.images.len(),
			if use_gpu { "≥" } else { "<" },
			GPU_BATCH_THRESHOLD
		));
	}

	log(Level::Info, &format!(
		"Loading model ({})...",
		if use_gpu { "GPU" } else { "CPU" }
	));

	let load_start = Instant::now();
	let processor = ImageProcessor::new(use_gpu).context("Failed to initialize processor")?;

	log(Level::Success, &format!(
		"Model ready ({} tags, {}) in {:.2}s",
		processor.vocabulary_size(),
		processor.execution_provider(),
		load_start.elapsed().as_secs_f32()
	));

	// Try to load embedder for semantic search
	let embedder = if TextEmbedder::is_available() {
		match TextEmbedder::new() {
			Ok(e) => {
				log(Level::Success, "Embedder ready");
				Some(e)
			}
			Err(e) => {
				if verbose {
					log(Level::Warning, &format!("Embedder unavailable: {}", e));
				}
				None
			}
		}
	} else {
		None
	};

	header("Processing");
	let process_start = Instant::now();
	let (processed, errors) = process_images(&scan.images, &processor, embedder.as_ref(), threshold, verbose);

	summary(processed, scan.skipped.len(), errors, process_start.elapsed().as_secs_f32());

	if errors > 0 {
		log(Level::Warning, &format!("Completed with {} errors", errors));
	} else {
		log(Level::Success, "All images processed successfully");
	}

	Ok(())
}

fn run_search(query: &str, directory: &std::path::Path, limit: usize, min_score: f32, semantic: bool, open: bool) -> Result<()> {
	header(&format!("Scout v{}", env!("CARGO_PKG_VERSION")));

	let root = directory.canonicalize().unwrap_or_else(|_| directory.to_path_buf());
	let scout_dir = root.join(SIDECAR_DIR);

	if !scout_dir.exists() {
		log(Level::Error, &format!("No {} directory found. Run 'scout scan' first.", SIDECAR_DIR));
		std::process::exit(1);
	}

	let mode = if semantic { "semantic" } else { "keyword" };
	log(Level::Info, &format!("Searching ({}) for: {}", mode, query.cyan()));

	let results = search_images(&root, query, min_score, semantic);

	if results.is_empty() {
		log(Level::Warning, "No matches found");
		if semantic {
			log(Level::Info, "Tip: Run 'scout scan -f' to generate embeddings for semantic search");
		}
		return Ok(());
	}

	let search_type = if results.first().map(|r| r.semantic).unwrap_or(false) {
		"semantic"
	} else {
		"keyword"
	};
	log(Level::Success, &format!("Found {} {} matches", results.len(), search_type));
	println!();

	for (i, result) in results.iter().take(limit).enumerate() {
		let path_display = result.image_path.file_name()
			.map(|n| n.to_string_lossy().to_string())
			.unwrap_or_else(|| result.image_path.to_string_lossy().to_string());

		let tags: Vec<String> = result.matched_tags.iter()
			.map(|m| format!("{} → {}", m.query_term.yellow(), m.tag_name.green()))
			.collect();

		println!(
			"  {} {} {}",
			format!("#{}", i + 1).cyan().bold(),
			format!("[{:.0}%]", result.score * 100.0).yellow(),
			path_display.white().bold()
		);
		println!("      {}", tags.join(", ").dimmed());
		println!("      {}", result.image_path.to_string_lossy().dimmed());
	}

	if open && !results.is_empty() {
		let best = &results[0].image_path;
		log(Level::Info, &format!("Opening: {}", best.to_string_lossy()));
		if let Err(e) = open::that(best) {
			log(Level::Warning, &format!("Failed to open: {}", e));
		}
	}

	println!();
	Ok(())
}

fn run_stats(directory: &std::path::Path, limit: usize) -> Result<()> {
	header(&format!("Scout v{}", env!("CARGO_PKG_VERSION")));

	let root = directory.canonicalize().unwrap_or_else(|_| directory.to_path_buf());
	let scout_dir = root.join(SIDECAR_DIR);

	if !scout_dir.exists() {
		log(Level::Error, &format!("No {} directory found. Run 'scout scan' first.", SIDECAR_DIR));
		std::process::exit(1);
	}

	log(Level::Info, "Calculating tag statistics...");

	let stats = calculate_stats(&root, limit);

	if stats.total_images == 0 {
		log(Level::Warning, "No indexed images found");
		return Ok(());
	}

	println!();
	println!("  {} {}", "Total images:".cyan(), stats.total_images.to_string().white().bold());
	println!("  {} {}", "Total tags:  ".cyan(), stats.total_tags.to_string().white().bold());
	println!("  {} {}", "Unique tags: ".cyan(), stats.unique_tags.to_string().white().bold());
	println!("  {} {:.1}", "Avg per img: ".cyan(), stats.total_tags as f32 / stats.total_images as f32);
	println!();

	header(&format!("Top {} Tags", limit));
	println!();

	for (i, tag) in stats.top_tags.iter().enumerate() {
		let bar_width = (tag.count as f32 / stats.top_tags[0].count as f32 * 20.0) as usize;
		let bar = "━".repeat(bar_width);
		
		println!(
			"  {:>3}. {:<25} {:>5}  {} {:.0}%",
			i + 1,
			tag.name.green(),
			tag.count.to_string().yellow(),
			bar.cyan(),
			tag.avg_confidence * 100.0
		);
	}

	println!();
	Ok(())
}

fn process_images(images: &[ImageEntry], processor: &ImageProcessor, embedder: Option<&TextEmbedder>, threshold: f32, verbose: bool) -> (usize, usize) {
	let mut processed = 0;
	let mut errors = 0;
	let total = images.len();

	for (i, entry) in images.iter().enumerate() {
		let path_str = entry.path.to_string_lossy();
		let display = truncate_path(&path_str, 50);
		let time = chrono::Local::now().format("%H:%M:%S").to_string();

		match processor.process_image(&entry.path, threshold) {
			Ok(result) => {
				let tag_count = result.tags.len();
				let ms = result.stats.processing_ms;

				// In verbose mode, show top tags
				if verbose && !result.tags.is_empty() {
					let top_tags: Vec<String> = result.tags.iter()
						.take(5)
						.map(|t| format!("{} ({:.0}%)", t.name, t.confidence * 100.0))
						.collect();
					log(Level::Debug, &format!("Tags: {}", top_tags.join(", ")));
				}

				let mut sidecar = processor.create_sidecar(&entry.path, result, threshold);

				// Generate embedding if embedder available
				if let Some(emb) = embedder {
					let tag_names: Vec<String> = sidecar.tags.iter().map(|t| t.name.clone()).collect();
					if let Ok(embedding) = emb.embed_tags(&tag_names) {
						sidecar = sidecar.with_embedding(embedding);
					}
				}

				if let Err(e) = sidecar.save(&entry.sidecar_path) {
					println!(
						"[{}] {} {} {}",
						time.dimmed(),
						"✘".red().bold(),
						display,
						format!("Save failed: {}", e).red()
					);
					errors += 1;
					continue;
				}

				let embed_indicator = if sidecar.embedding.is_some() { "+E" } else { "" };
				println!(
					"[{}] {} {} {} {} {} {}",
					time.dimmed(),
					"✔".green().bold(),
					format!("[{}/{}]", i + 1, total).cyan(),
					display,
					format!("({} tags{})", tag_count, embed_indicator).yellow(),
					format!("{}ms", ms).dimmed(),
					if sidecar.embedding.is_some() { "🔍".to_string() } else { String::new() }
				);
				processed += 1;
			}
			Err(e) => {
				println!(
					"[{}] {} {} {}",
					time.dimmed(),
					"✘".red().bold(),
					display,
					format!("{}", e).red()
				);
				errors += 1;
			}
		}
	}

	(processed, errors)
}

fn truncate_path(path: &str, max_len: usize) -> String {
	if path.len() > max_len {
		format!("...{}", &path[path.len() - max_len + 3..])
	} else {
		path.to_string()
	}
}
