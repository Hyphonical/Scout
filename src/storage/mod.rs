//! Embedding storage system

pub mod sidecar;
pub mod index;

pub use sidecar::{Sidecar, ImageSidecar, VideoSidecar, save_image, save_video, load};
pub use index::{find, scan};