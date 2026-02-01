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
    println!("{}", ui::log::random_slogan().bright_blue().italic());
    println!();
    
    ui::Log::set_verbose(cli.verbose);
    
    // Set custom model paths if provided
    if let Some(dir) = cli.model_dir {
        config::set_model_dir(dir);
    }
    if let Some(path) = cli.vision_model {
        config::set_vision_model(path);
    }
    if let Some(path) = cli.text_model {
        config::set_text_model(path);
    }
    if let Some(path) = cli.tokenizer {
        config::set_tokenizer(path);
    }
    
    // Set provider
    if let Some(provider) = cli.provider {
        runtime::set_provider(provider);
    }
    
    let result = match cli.command {
        cli::Command::Scan { dir, recursive, force, min_resolution, max_size } => {
            commands::scan::run(&dir, recursive, force, min_resolution, max_size)
        }
        cli::Command::Search { query, image, weight, not, dir, recursive, limit, score, open } => {
            commands::search::run(query.as_deref(), image.as_deref(), weight, not.as_deref(), &dir, recursive, limit, score, open)
        }
        cli::Command::Clean { dir, recursive } => {
            commands::clean::run(&dir, recursive)
        }
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
