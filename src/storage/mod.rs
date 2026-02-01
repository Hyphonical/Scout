//! Embedding storage system

pub mod sidecar;
pub mod index;

pub use sidecar::{Sidecar, save, load};
pub use index::{find, scan};