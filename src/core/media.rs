//! Media type detection

use crate::config::{IMAGE_EXTENSIONS, VIDEO_EXTENSIONS};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
	Image,
	Video,
}

impl MediaType {
	/// Detect media type from file
	pub fn detect(path: &Path) -> Option<Self> {
		// Quick extension check first
		if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
			if IMAGE_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
				return Some(MediaType::Image);
			}
			if VIDEO_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
				return Some(MediaType::Video);
			}
		}

		None
	}
}
