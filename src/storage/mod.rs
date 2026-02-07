//! # Embedding Storage
//!
//! Sidecar file I/O for persisting embeddings alongside media.
//! Uses MessagePack for compact binary storage.

pub mod index;
pub mod sidecar;

pub use index::{find, find_file_by_hash, load_all_sidecars, scan};
pub use sidecar::{load, save_image, save_video, ImageSidecar, Sidecar, VideoSidecar};
