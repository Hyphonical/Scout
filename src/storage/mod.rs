//! Embedding storage system

pub mod index;
pub mod sidecar;

pub use index::{find, scan};
pub use sidecar::{load, save_image, save_video, ImageSidecar, Sidecar, VideoSidecar};
