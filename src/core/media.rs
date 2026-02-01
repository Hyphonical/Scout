//! Media type detection

use std::path::Path;
use crate::config::{IMAGE_EXTENSIONS, VIDEO_EXTENSIONS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
	Image,
	Video,
}

impl MediaType {
	/// Detect media type from file extension
	pub fn detect(path: &Path) -> Option<Self> {
		let ext = path.extension()?.to_str()?;
		
		if IMAGE_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
			Some(MediaType::Image)
		} else if VIDEO_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
			Some(MediaType::Video)
		} else {
			None
		}
	}
}