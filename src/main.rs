mod cli;
mod commands;
mod config;
mod core;
mod models;
mod processing;
mod runtime;
mod storage;
mod ui;

use clap::Parser;
use colored::*;

fn main() {
	let cli = cli::Cli::parse();

	// Show slogan
	println!("{}", ui::log::random_slogan().bright_white().italic());
	println!();

	ui::Log::set_verbose(cli.verbose);

	// Set custom model directory if provided
	if let Some(dir) = cli.model_dir {
		config::set_model_dir(dir);
	}

	// Set FFmpeg path if provided
	if let Some(path) = cli.ffmpeg_path {
		processing::video::set_ffmpeg_path(path);
	}

	// Set provider
	if let Some(provider) = cli.provider {
		runtime::set_provider(provider);
	}

	let result = match cli.command {
		cli::Command::Scan {
			dir,
			recursive,
			force,
			min_resolution,
			max_size,
			exclude_videos,
		} => commands::scan::run(
			&dir,
			recursive,
			force,
			min_resolution,
			max_size,
			exclude_videos,
		),
		cli::Command::Search {
			query,
			image,
			weight,
			not,
			dir,
			recursive,
			limit,
			score,
			open,
			include_ref,
			exclude_videos,
		} => commands::search::run(
			query.as_deref(),
			image.as_deref(),
			weight,
			not.as_deref(),
			&dir,
			recursive,
			limit,
			score,
			open,
			include_ref,
			exclude_videos,
		),
		cli::Command::Clean { dir, recursive } => commands::clean::run(&dir, recursive),
	};

	if let Err(e) = result {
		ui::error(&format!("{}", e));
		std::process::exit(1);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_embedding_similarity() {
		use crate::core::Embedding;
		let e1 = Embedding::new(vec![1.0, 0.0, 0.0]);
		let e2 = Embedding::new(vec![1.0, 0.0, 0.0]);
		assert!((e1.similarity(&e2) - 1.0).abs() < 0.001);
	}
}
