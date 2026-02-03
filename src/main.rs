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

fn main() {
	let cli = cli::Cli::parse();

	ui::log::print_logo();
	eprintln!();

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
			force,
			min_resolution,
			max_size,
			exclude_videos,
			max_frames,
			scene_threshold,
		} => commands::scan::run(
			&dir,
			cli.recursive,
			force,
			min_resolution,
			max_size,
			exclude_videos,
			max_frames,
			scene_threshold,
		),
		cli::Command::Search {
			query,
			image,
			weight,
			not,
			dir,
			limit,
			score,
			open,
			include_ref,
			exclude_videos,
			paths,
			export,
		} => commands::search::run(
			query.as_deref(),
			image.as_deref(),
			weight,
			not.as_deref(),
			&dir,
			cli.recursive,
			limit,
			score,
			open,
			include_ref,
			exclude_videos,
			paths,
			export.as_deref(),
		),
		cli::Command::Cluster {
			dir,
			force,
			min_cluster_size,
			min_samples,
			use_umap,
			preview_count,
			export,
		} => commands::cluster::run(
			&dir,
			cli.recursive,
			force,
			min_cluster_size,
			min_samples,
			use_umap,
			preview_count,
			export.as_deref(),
		),
		cli::Command::Clean { dir } => commands::clean::run(&dir, cli.recursive),
		cli::Command::Watch {
			dir,
			min_resolution,
			max_size,
			exclude_videos,
			max_frames,
			scene_threshold,
		} => commands::watch::run(
			&dir,
			cli.recursive,
			min_resolution,
			max_size,
			exclude_videos,
			max_frames,
			scene_threshold,
		),
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
