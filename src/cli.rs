//! Command-line interface

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Provider {
    Auto,
    Cpu,
    Cuda,
    Tensorrt,
    #[value(name = "coreml")]
    CoreML,
    Xnnpack,
}

#[derive(Parser)]
#[command(name = "scout", version, about = "AI-powered semantic image search")]
pub struct Cli {
    #[arg(short, long, global = true)]
    pub verbose: bool,
    
    #[arg(long, global = true, value_enum)]
    pub provider: Option<Provider>,
    
    #[arg(long, global = true, help = "Path to models directory")]
    pub model_dir: Option<PathBuf>,
    
    #[arg(long, global = true, help = "Path to vision model (.onnx)")]
    pub vision_model: Option<PathBuf>,
    
    #[arg(long, global = true, help = "Path to text model (.onnx)")]
    pub text_model: Option<PathBuf>,
    
    #[arg(long, global = true, help = "Path to tokenizer (.json)")]
    pub tokenizer: Option<PathBuf>,
    
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Index images in a directory
    Scan {
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
        
        #[arg(short, long)]
        recursive: bool,
        
        #[arg(short, long)]
        force: bool,
        
        #[arg(long, help = "Minimum resolution (shortest side in pixels)")]
        min_resolution: Option<u32>,
        
        #[arg(long, help = "Maximum file size in MB")]
        max_size: Option<u64>,
    },
    
    /// Search indexed images
    Search {
        /// Search query text (optional if using --image)
        query: Option<String>,
        
        #[arg(short, long, help = "Reference image path")]
        image: Option<PathBuf>,
        
        #[arg(short, long, default_value_t = 0.5, help = "Text weight when combining text+image (0.0-1.0)")]
        weight: f32,
        
        #[arg(long, help = "Negative query to exclude")]
        not: Option<String>,
        
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
        
        #[arg(short, long)]
        recursive: bool,
        
        #[arg(short = 'n', long, default_value_t = crate::config::DEFAULT_LIMIT)]
        limit: usize,
        
        #[arg(short, long, default_value_t = crate::config::DEFAULT_MIN_SCORE)]
        score: f32,
        
        #[arg(short, long)]
        open: bool,
    },
    
    /// Remove orphaned sidecars
    Clean {
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
        
        #[arg(short, long)]
        recursive: bool,
    },
}


