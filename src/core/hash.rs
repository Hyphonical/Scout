//! Content-based file hashing

use std::fs::File;
use std::io::Read;
use std::path::Path;
use xxhash_rust::xxh3::xxh3_64;

const HASH_BUFFER_SIZE: usize = 65536; // 64KB

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileHash(String);

impl FileHash {
	/// Compute hash from file's first 64KB
	pub fn compute(path: &Path) -> std::io::Result<Self> {
		let mut file = File::open(path)?;
		let mut buffer = vec![0u8; HASH_BUFFER_SIZE];
		let n = file.read(&mut buffer)?;
		buffer.truncate(n);

		let hash = xxh3_64(&buffer);
		Ok(Self(format!("{:016x}", hash)))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}

	pub fn short(&self) -> &str {
		&self.0[..8]
	}
}

impl std::fmt::Display for FileHash {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}
