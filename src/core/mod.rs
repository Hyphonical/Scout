//! # Core Domain Types
//!
//! Fundamental data structures: embeddings, hashes, clusters, and media types.
//! These types are used throughout the application.

pub mod cluster;
pub mod embedding;
pub mod hash;
pub mod media;

pub use cluster::{compute_content_hash, Cluster, ClusterDatabase, ClusterParams};
pub use embedding::Embedding;
pub use hash::FileHash;
pub use media::MediaType;
