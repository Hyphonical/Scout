//! # Media Processing
//!
//! Image/video processing, directory scanning, clustering, and UMAP.

pub mod cluster;
pub mod image;
pub mod scan;
pub mod umap;
pub mod video;

pub use scan::scan_directory;
