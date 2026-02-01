# Sidecar Metadata Format

Scout stores embedding data and metadata in **sidecar files** alongside your media files. This document explains the sidecar format, versioning, and structure.

## Overview

Instead of using a centralized database, Scout stores embeddings in `.scout/` directories next to your images and videos. Each media file gets its own sidecar file named after its content hash.

**Benefits of Sidecar Storage:**
- ✅ Embeddings travel with your files when you move/copy folders
- ✅ No centralized database that can get corrupted
- ✅ Easy to delete (just remove `.scout/` folders)
- ✅ Portable across different machines and storage devices
- ✅ Each file's metadata is independent

## Directory Structure

```
Photos/
├── .scout/
│   ├── a1b2c3d4e5f6g7h8.msgpack  # Sidecar for sunset.jpg
│   ├── 1234567890abcdef.msgpack  # Sidecar for beach.jpg
│   └── f9e8d7c6b5a43210.msgpack  # Sidecar for video.mp4
├── sunset.jpg
├── beach.jpg
└── video.mp4
```

## File Naming

Sidecar files are named using a **content-based hash** of the first 64KB of the media file:

- **Algorithm**: XXH3 (xxHash3 - extremely fast non-cryptographic hash)
- **Format**: 16-character hexadecimal string
- **Extension**: `.msgpack` (MessagePack binary format)

**Example**: `a1b2c3d4e5f6g7h8.msgpack`

The hash ensures:
- **Deduplication**: Identical files get the same hash
- **Change detection**: Modified files get a new hash
- **Fast computation**: Only reads first 64KB for speed

## Image Sidecar Format

For regular images, the sidecar contains:

```rust
struct ImageSidecar {
    version: String,           // Scout version that created this (e.g., "1.3.0")
    filename: String,          // Original filename (e.g., "sunset.jpg")
    hash: String,              // XXH3 hash of the file
    processed: DateTime<Utc>,  // When the embedding was generated
    embedding: Vec<f32>,       // 1024-dimensional normalized embedding vector
    processing_ms: u64,        // Time taken to process (milliseconds)
}
```

**Example values**:
- `version`: `"1.3.0"`
- `filename`: `"sunset.jpg"`
- `hash`: `"a1b2c3d4e5f6g7h8"`
- `processed`: `2026-01-31T14:23:45.123Z`
- `embedding`: `[0.042, -0.18, ..., 0.091]` (1024 floats)
- `processing_ms`: `250`

## Video Sidecar Format

For videos, Scout extracts multiple frames and stores embeddings for each:

```rust
struct VideoSidecar {
    version: String,
    filename: String,
    hash: String,
    processed: DateTime<Utc>,
    frames: Vec<VideoFrameData>,
    processing_ms: u64,
}

struct VideoFrameData {
    timestamp_secs: f64,      // Frame timestamp in seconds
    embedding: Vec<f32>,      // 1024-dimensional embedding for this frame
}
```

By default, Scout extracts **10 evenly-spaced frames** from each video. This allows:
- Searching for scenes within videos
- Finding the exact timestamp of relevant content
- Better coverage of video content than a single thumbnail

**Example**:
A 2-minute video would have frames extracted at: 0:06, 0:18, 0:30, 0:42, 0:54, 1:06, 1:18, 1:30, 1:42, 1:54

## Embedding Vector Details

Scout uses **SigLIP2** vision-language models to generate embeddings:

- **Dimensions**: 1024 floating-point values
- **Normalization**: All vectors are normalized to unit length (L2 norm = 1.0)
- **Similarity**: Computed using dot product (equivalent to cosine similarity for normalized vectors)
- **Range**: Individual values typically range from -1.0 to +1.0
- **Quantization**: Models use Q4F16 (4-bit weights, FP16 activations) for efficiency

### Why Normalized Embeddings?

Normalizing embeddings to unit length allows using the **dot product** for similarity:

```rust
similarity = embedding_a · embedding_b
```

This is computationally faster than calculating cosine similarity directly, while giving identical results.

## Version Tracking

Each sidecar includes a `version` field matching Scout's version number. This enables:

1. **Automatic upgrades**: Scout detects outdated sidecars during scans
2. **Model changes**: If the embedding model is upgraded, old embeddings are re-generated
3. **Backward compatibility**: Older sidecars can still be read

When running `scout scan`, you'll see:
```
⚠ 15 outdated sidecars found. Run 'scout scan -f' to upgrade to v1.3.0
```

Run with `--force` to regenerate embeddings:
```bash
scout scan -f -d ~/Photos
```

## Storage Format: MessagePack

Sidecars use **MessagePack** (.msgpack) for efficient binary serialization:

- **Compact**: ~4KB per image sidecar, ~40KB for 10-frame video
- **Fast**: Faster to read/write than JSON
- **Schemaless**: Easy to evolve the format
- **Cross-platform**: Works identically on all operating systems

### Size Breakdown

For a typical image sidecar (~4KB):
- Embedding data: ~4000 bytes (1024 × 4 bytes per float)
- Metadata: ~100 bytes (strings, timestamps, etc.)

For a typical video sidecar with 10 frames (~40KB):
- Frame embeddings: ~40,000 bytes (10 × 1024 × 4 bytes)
- Metadata: ~200 bytes

## Maintenance Commands

### View Sidecar Status

```bash
# Scan shows outdated sidecar counts
scout scan -d ~/Photos -r
```

### Regenerate Sidecars

```bash
# Force re-processing of all images
scout scan -f -d ~/Photos -r
```

### Clean Orphaned Sidecars

```bash
# Remove sidecars for deleted images
scout clean -d ~/Photos -r
```

### Manual Inspection

Sidecars are binary MessagePack files, but you can inspect them using tools:

```bash
# Install msgpack-tools
pip install msgpack-tools

# View sidecar contents
msgpack2json < .scout/a1b2c3d4e5f6g7h8.msgpack | jq
```

## Best Practices

### Backup Considerations

- ✅ **Include .scout/ folders in backups** - they contain all indexing data
- ✅ Sidecars are regenerable but take time to reprocess

### Moving/Copying Files

**When copying individual files**: Copy the entire folder to keep the `.scout/` directory
```bash
# Good: Copies everything including .scout/
cp -r Photos/ Backup/Photos/

# Bad: Loses sidecars
cp Photos/*.jpg Backup/
```

**When moving files**: The sidecar hash is content-based, not filename-based, so renaming doesn't break anything!

### Storage Cleanup

Delete `.scout/` directories to completely remove all indexing data:
```bash
# Remove all Scout data from a directory tree
find ~/Photos -type d -name ".scout" -exec rm -rf {} +
```

## Future Compatibility

The sidecar format is designed to be forward-compatible:

- New fields can be added without breaking old versions
- MessagePack supports schema evolution
- Version field enables migration strategies

If Scout's model or format changes significantly, the tool will:
1. Detect outdated sidecars automatically
2. Offer to regenerate them
3. Preserve backward compatibility where possible

## Technical Details

### Hash Collision

XXH3 is a 64-bit hash, so collisions are theoretically possible but extremely unlikely:
- **Probability**: ~1 in 18 quintillion (2^64)
- **Practical risk**: Zero for typical photo collections

### Thread Safety

Sidecar operations are designed to be safe for concurrent access:
- Each sidecar file is independent
- Writes are atomic (write to temp file, then rename)
- No locking needed across different images

### Performance

- **Hash computation**: ~0.5ms for 64KB read
- **Sidecar write**: ~1-2ms (mostly disk I/O)
- **Sidecar read**: ~1ms for image, ~3-5ms for video

## Troubleshooting

### "Failed to read sidecar"

- Corrupted `.msgpack` file - delete and re-scan
- Permission issues - check file ownership
- Disk errors - run filesystem check

### "Outdated sidecar" warnings

- Normal after Scout updates
- Run `scout scan -f` to regenerate
- Safe to ignore if search still works

### Orphaned sidecars

- Sidecars left after deleting images
- Run `scout clean` to remove
- Harmless but wastes disk space

---

**For more information**, see:
- [README.md](../README.md) - General usage guide
- [BUILD.md](BUILD.md) - Building from source
- [Models.md](../models/Models.md) - Model details and alternatives
