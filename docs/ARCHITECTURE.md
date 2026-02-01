# Scout Architecture

Technical overview of Scout's internals.

## Table of Contents

- [High-Level Overview](#high-level-overview)
- [Module Breakdown](#module-breakdown)
- [Data Flow](#data-flow)
- [Sidecar Format](#sidecar-format)
- [Model Management](#model-management)
- [Video Pipeline](#video-pipeline)
- [Execution Providers](#execution-providers)

---

## High-Level Overview

Scout is a semantic search engine for images and videos. It works in two phases:

1. **Indexing (scan):** Generate embeddings for media files and store in sidecars
2. **Searching (search/repl):** Encode query and compare against stored embeddings

### Technology Stack

- **Rust** - Systems language for performance and safety
- **ONNX Runtime** - Cross-platform ML inference (ort crate)
- **SigLIP2** - Vision-language model (dual encoder)
- **MessagePack** - Efficient binary serialization
- **FFmpeg** - Video frame extraction (optional)
- **xxHash** - Fast non-cryptographic hashing

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         CLI Layer                           │
│  (clap parser, command routing, global options)             │
└──────────────────┬──────────────────────────────────────────┘
                   │
    ┌──────────────┼──────────────┐
    │              │              │
    v              v              v
┌─────────┐  ┌─────────┐  ┌──────────┐  ┌─────────┐
│  scan   │  │ search  │  │  repl    │  │ clean   │
└────┬────┘  └────┬────┘  └─────┬────┘  └─────┬───┘
     │            │             │             │
     │  ┌─────────┴─────────────┘             │
     │  │                                     │
     v  v                                     v
┌──────────────────┐                   ┌──────────┐
│   Processing     │                   │ Storage  │
│  - scan.rs       │                   │  Index   │
│  - image.rs      │                   └──────────┘
│  - video.rs      │
└────────┬─────────┘
         │
         v
┌──────────────────┐        ┌──────────────────┐
│   Models         │<───────│  ONNX Runtime    │
│  - manager.rs    │        │  - providers.rs  │
│  - vision.rs     │        └──────────────────┘
│  - text.rs       │
└────────┬─────────┘
         │
         v
┌──────────────────┐
│   Storage        │
│  - sidecar.rs    │
│  - index.rs      │
└──────────────────┘
```

---

## Module Breakdown

### `cli.rs` - Command-Line Interface

**Purpose:** Parse CLI arguments and options

**Key types:**
- `Cli` - Top-level struct with global options
- `Command` - Enum of subcommands (Scan, Search, Repl, Clean)
- `Provider` - Execution provider enum

**Responsibilities:**
- Parse arguments with clap
- Validate inputs
- Route to command implementations

### `main.rs` - Entry Point

**Purpose:** Bootstrap and coordinate

**Flow:**
1. Parse CLI arguments
2. Show random slogan
3. Set verbose mode
4. Configure models and FFmpeg paths
5. Select execution provider
6. Route to command
7. Handle errors

### `config.rs` - Configuration

**Purpose:** Global constants and paths

**Key constants:**
- `VISION_MODEL`, `TEXT_MODEL`, `TOKENIZER` - Model filenames
- `INPUT_SIZE`, `EMBEDDING_DIM` - Model parameters
- `SIDECAR_DIR`, `SIDECAR_EXT` - Storage settings
- `IMAGE_EXTENSIONS`, `VIDEO_EXTENSIONS` - Supported formats
- `DEFAULT_LIMIT`, `DEFAULT_MIN_SCORE` - Search defaults

**Dynamic configuration:**
- `models_dir()` - Resolve model directory (env var, custom, default)
- `set_model_dir()` - Set custom model path

### `core/` - Domain Types

#### `embedding.rs`

**Purpose:** Embedding vector type

```rust
pub struct Embedding {
    data: Vec<f32>,
}

impl Embedding {
    pub fn new(data: Vec<f32>) -> Self { ... }
    pub fn raw(data: Vec<f32>) -> Self { ... }  // Skip normalization
    pub fn similarity(&self, other: &Self) -> f32 { ... }  // Cosine sim
    pub fn blend(a: &Self, b: &Self, weight: f32) -> Self { ... }
}
```

**Key operations:**
- Normalization (L2 norm)
- Cosine similarity (dot product of normalized vectors)
- Blending (weighted average)

#### `hash.rs`

**Purpose:** Fast file hashing

```rust
pub struct FileHash(u64);

impl FileHash {
    pub fn from_path(path: &Path) -> Result<Self>
    pub fn as_str(&self) -> String  // Base32 encoded
}
```

Uses xxHash (xxh3_64) for speed.

#### `media.rs`

**Purpose:** Media type detection

```rust
pub enum MediaType {
    Image,
    Video,
}

impl MediaType {
    pub fn detect(path: &Path) -> Option<Self>
}
```

Based on file extension.

### `models/` - ML Model Management

#### `manager.rs`

**Purpose:** Lazy model loading and lifecycle

```rust
pub struct Models {
    vision: OnceLock<VisionModel>,
    text: OnceLock<TextModel>,
}

impl Models {
    pub fn new() -> Result<Self>
    pub fn encode_image(&mut self, img: &DynamicImage) -> Result<Embedding>
    pub fn encode_text(&mut self, text: &str) -> Result<Embedding>
}
```

**Lazy loading:**
- Models loaded on first use
- Shared across multiple operations
- Memory efficient (load what you need)

#### `vision.rs`

**Purpose:** Image encoding

```rust
pub struct VisionModel {
    session: Session,
}

impl VisionModel {
    pub fn load(path: &Path) -> Result<Self>
    pub fn encode(&self, img: &DynamicImage) -> Result<Embedding>
}
```

**Pipeline:**
1. Resize to 512x512 (bilinear)
2. Convert to RGB
3. Normalize (ImageNet stats)
4. CHW layout (channels, height, width)
5. Run ONNX inference
6. Extract embedding from output

#### `text.rs`

**Purpose:** Text encoding

```rust
pub struct TextModel {
    session: Session,
    tokenizer: Tokenizer,
}

impl TextModel {
    pub fn load(model_path: &Path, tokenizer_path: &Path) -> Result<Self>
    pub fn encode(&self, text: &str) -> Result<Embedding>
}
```

**Pipeline:**
1. Tokenize text (HuggingFace tokenizers)
2. Add special tokens ([CLS], [SEP])
3. Pad/truncate to max length
4. Run ONNX inference
5. Extract [CLS] token embedding

### `runtime/` - ONNX Runtime Integration

#### `providers.rs`

**Purpose:** Execution provider selection

**Macro-based design:**
```rust
create_provider_fn! {
    (Cpu, "CPU", cpu),
    (Cuda, "CUDA", cuda),
    (Tensorrt, "TensorRT", tensorrt),
    (CoreML, "CoreML", coreml),
    (Xnnpack, "XNNPACK", xnnpack),
}
```

**Selection logic:**
1. User specifies provider (or Auto)
2. Try to create session with provider
3. Fall back to CPU if unavailable
4. Show user which provider is active

### `storage/` - Persistence Layer

#### `sidecar.rs`

**Purpose:** Sidecar file format

**Types:**
```rust
pub struct ImageSidecar {
    version: String,
    filename: String,
    hash: String,
    embedding: Vec<f32>,
}

pub struct VideoSidecar {
    version: String,
    filename: String,
    hash: String,
    frames: Vec<VideoFrame>,
}

pub struct VideoFrame {
    timestamp: f64,
    embedding: Vec<f32>,
}

pub enum Sidecar {
    Image(ImageSidecar),
    Video(VideoSidecar),
}
```

**Operations:**
- `save_image()`, `save_video()` - Serialize to MessagePack
- `load()` - Deserialize (auto-detect type)
- Version checking for migrations

**Path:** `.scout/<hash>.msgpack`

#### `index.rs`

**Purpose:** Sidecar discovery

```rust
pub fn scan(root: &Path, recursive: bool) -> Vec<(PathBuf, PathBuf)>
pub fn find(media_dir: &Path, hash: &FileHash) -> Option<PathBuf>
```

Walks directory tree looking for `.scout/` folders.

### `processing/` - Media Processing

#### `scan.rs`

**Purpose:** Directory scanning with filters

```rust
pub struct ScanResult {
    pub to_process: Vec<MediaFile>,
    pub already_indexed: usize,
    pub filtered: usize,
    pub outdated: usize,
}

pub fn scan_directory(
    dir: &Path,
    recursive: bool,
    force: bool,
    min_resolution: Option<u32>,
    max_size: Option<u64>,
) -> ScanResult
```

**Pipeline:**
1. Walk directory tree (optional recursive)
2. Filter by extension (image/video)
3. Apply resolution filter (if image)
4. Apply size filter
5. Compute file hash
6. Check if sidecar exists and is current
7. Check `.scoutignore` patterns

**Filters:**
- Extension whitelist
- Minimum resolution (shortest side)
- Maximum file size
- .scoutignore patterns (gitignore syntax)
- Sidecar freshness (version check)

#### `image.rs`

**Purpose:** Image loading and encoding

```rust
pub fn encode(models: &mut Models, path: &Path) -> Result<Embedding>
pub fn encode_image(models: &mut Models, img: &DynamicImage) -> Result<Embedding>
```

Wrapper around `Models::encode_image()`.

#### `video.rs`

**Purpose:** FFmpeg integration

```rust
pub fn is_available() -> bool
pub fn extract_frames(path: &Path, count: usize) -> Result<Vec<(f64, RgbImage)>>
pub fn format_timestamp(seconds: f64) -> String
pub fn set_ffmpeg_path(path: PathBuf)
```

**Pipeline:**
1. Check FFmpeg/ffprobe availability
2. Probe video metadata (duration, dimensions, fps)
3. Calculate frame timestamps (evenly spaced)
4. Build FFmpeg filter expression
5. Extract frames as raw RGB24 data
6. Parse into RgbImage structs

**Frame selection:**
- Evenly distribute `count` frames across video duration
- Use ffprobe to get accurate metadata
- Use FFmpeg select filter for efficient extraction
- One FFmpeg invocation for all frames

### `commands/` - Command Implementations

#### `scan.rs`

**Purpose:** Index media files

**Flow:**
1. Check FFmpeg availability
2. Scan directory with filters
3. Initialize models (lazy)
4. For each file:
   - Load and encode (image or video)
   - Create and save sidecar
5. Show progress bar
6. Report statistics

#### `search.rs`

**Purpose:** Search indexed files

**Flow:**
1. Load models
2. Encode query (text, image, or combined)
3. Encode negative query (if provided)
4. Scan for sidecars
5. For each sidecar:
   - Load and check version
   - Compute similarity
   - Apply negative penalty (if any)
   - Filter by score threshold
6. Sort by score (descending)
7. Filter reference image (unless --include-ref)
8. Truncate to limit
9. Display results with timestamps (for videos)

#### `repl.rs`

**Purpose:** Interactive search mode

**Flow:**
1. Load models once
2. Pre-scan sidecars
3. Enter REPL loop:
   - Read query
   - Encode and search
   - Display results
   - Repeat
4. Exit on "exit"/"quit"/"q"

**Optimization:**
- Models loaded once
- Index cached in memory
- No startup overhead per query

#### `clean.rs`

**Purpose:** Remove orphaned sidecars

**Flow:**
1. Scan for sidecars
2. For each sidecar:
   - Check if media file exists
   - If not, mark for deletion
3. Delete orphaned sidecars
4. Report count

### `ui/` - User Interface

#### `log.rs`

**Purpose:** Logging and output

```rust
pub fn info(msg: &str)
pub fn success(msg: &str)
pub fn warn(msg: &str)
pub fn error(msg: &str)
pub fn debug(msg: &str)  // Only if --verbose
pub fn header(text: &str)
pub fn path_link(path: &Path) -> String
pub fn path_link_truncated(path: &Path, max_len: usize) -> String
pub fn random_slogan() -> &'static str
```

**Features:**
- Colored output (bright blue theme)
- Icons (✓, ✗, ⚠, ℹ)
- Clickable file paths (OSC 8 hyperlinks)
- Verbose mode filtering
- Random startup slogans

---

## Data Flow

### Scan Flow

```
User Input
    │
    v
scan_directory()  ─────> MediaFile[]
    │                        │
    │                        v
    │                    For each file:
    │                        │
    │                        ├─> Load image/video
    │                        │       │
    │                        │       v
    │                        ├─> encode_image() ──> Embedding
    │                        │   or extract_frames() + encode
    │                        │
    │                        └─> Create Sidecar
    │                                │
    │                                v
    v                            save() ──> .scout/<hash>.msgpack
Progress bar ────────────────────────────────────────┘
```

### Search Flow

```
User Query (text/image)
    │
    v
encode_text() or encode_image()
    │
    v
Query Embedding
    │
    v
scan() ──> Sidecar paths
    │
    v
For each sidecar:
    │
    ├─> load() ──> ImageSidecar or VideoSidecar
    │       │
    │       v
    │   Extract embedding(s)
    │       │
    │       v
    └─> similarity() ──> Score
            │
            v
        Filter by threshold
            │
            v
        Sort by score
            │
            v
        Display results
```

### REPL Flow

```
Initialize
    │
    ├─> Load models (once)
    │
    └─> Scan sidecars (once)
        │
        v
    ┌─> Read query
    │   │
    │   v
    │   encode_text()
    │   │
    │   v
    │   Search cached sidecars
    │   │
    │   v
    │   Display results
    │   │
    └───┘ (loop)
```

---

## Sidecar Format

### MessagePack Structure

**Image Sidecar:**
```json
{
  "version": "2.0.0",
  "filename": "IMG_4523.jpg",
  "hash": "ABC123XYZ",
  "embedding": [0.123, -0.456, 0.789, ...]
}
```

**Video Sidecar:**
```json
{
  "version": "2.0.0",
  "filename": "vacation.mp4",
  "hash": "DEF456UVW",
  "frames": [
    {"timestamp": 0.0, "embedding": [...]},
    {"timestamp": 12.5, "embedding": [...]},
    {"timestamp": 25.0, "embedding": [...]},
    ...
  ]
}
```

### Binary Format

MessagePack is a binary format (not human-readable):
- Compact (smaller than JSON)
- Fast to serialize/deserialize
- Schema-free (self-describing)

### File Naming

**Pattern:** `.scout/<hash>.msgpack`

Where `<hash>` is the base32-encoded xxHash of the media file.

**Example:**
```
photos/
  IMG_4523.jpg
  .scout/
    7S1P3CG6KKZHRV40A8KKZXCAX0.msgpack
```

### Version Management

Each sidecar stores Scout's version. On load:
- If version matches: Use sidecar
- If version differs: Mark as outdated
- User can force re-index with `--force`

---

## Model Management

### Lazy Loading

Models are expensive to load (100-300 MB each). Scout uses lazy loading:

```rust
pub struct Models {
    vision: OnceLock<VisionModel>,  // Loaded on first image encode
    text: OnceLock<TextModel>,      // Loaded on first text encode
}
```

**Benefits:**
- Scan images only? Don't load text model
- Search by text only? Don't load vision model
- Fast startup

### Model Resolution

Priority order:
1. Custom path (--model-dir)
2. Environment variable (SCOUT_MODELS_DIR)
3. Next to executable (./models/)

### Model Format

**ONNX (Open Neural Network Exchange):**
- Cross-platform
- Hardware-agnostic
- Optimized inference

**Quantization:**
- `q4f16` = 4-bit weights, 16-bit activations
- ~75% size reduction
- Minimal accuracy loss
- Faster inference

### Memory Management

- Models stay loaded for duration of command
- Dropped when command exits
- REPL mode keeps models loaded (efficiency)

---

## Video Pipeline

### Architecture

```
Video File
    │
    v
ffprobe ──> Metadata (duration, dimensions, fps)
    │
    v
Calculate frame timestamps
    │
    v
ffmpeg ──> Extract frames (raw RGB24)
    │
    v
Parse into RgbImage[]
    │
    v
For each frame:
    │
    ├─> Convert to DynamicImage
    │
    └─> encode_image() ──> Embedding
            │
            v
        VideoSidecar with (timestamp, embedding) pairs
```

### Frame Selection

**Evenly spaced:**
```
Video duration: 60s
Frame count: 10
Interval: 60/10 = 6s
Timestamps: [3, 9, 15, 21, 27, 33, 39, 45, 51, 57]
```

Centered in each interval (offset by 0.5 * interval).

### FFmpeg Commands

**Probe:**
```bash
ffprobe -v error -print_format json -show_format -show_streams video.mp4
```

**Extract:**
```bash
ffmpeg -i video.mp4 \
  -vf "select='eq(n,30)+eq(n,60)+eq(n,90)...'" \
  -vsync 0 \
  -f rawvideo -pix_fmt rgb24 \
  pipe:1
```

**Optimizations:**
- Single FFmpeg invocation for all frames
- Select filter (no decode of unwanted frames)
- Raw output (no encoding overhead)
- Pipe output (no disk I/O)

### Graceful Degradation

If FFmpeg not found:
- Show warning
- Skip all video files
- Continue with images
- No error exit

---

## Execution Providers

### Provider Selection

**Auto mode:**
```rust
fn select_provider() -> ExecutionProvider {
    if tensorrt_available() {
        TensorRT
    } else if cuda_available() {
        CUDA
    } else if coreml_available() {
        CoreML
    } else if xnnpack_available() {
        XNNPACK
    } else {
        CPU
    }
}
```

### Provider Characteristics

| Provider | Hardware | Speed | Setup |
|----------|----------|-------|-------|
| CPU | Any | 1x | None |
| XNNPACK | Modern CPU | 2-3x | None |
| CUDA | NVIDIA GPU | 5-10x | CUDA 11+ |
| TensorRT | NVIDIA GPU | 8-15x | TensorRT 8+ |
| CoreML | Apple Silicon | 5-8x | macOS 11+ |

### Platform Availability

Controlled by Cargo.toml features:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
ort = { features = ["coreml", "xnnpack"] }

[target.'cfg(target_os = "linux")'.dependencies]
ort = { features = ["cuda", "tensorrt", "xnnpack"] }

[target.'cfg(target_os = "windows")'.dependencies]
ort = { features = ["cuda", "tensorrt", "xnnpack"] }
```

### Error Handling

If requested provider unavailable:
- Log warning
- Fall back to CPU
- Continue execution

---

## Performance Considerations

### Bottlenecks

1. **Model inference** - GPU acceleration helps most
2. **File I/O** - SSD recommended for large collections
3. **Image decoding** - CPU-bound, multi-threading possible
4. **Video decoding** - FFmpeg handles efficiently

### Optimizations

**Scan:**
- Lazy model loading
- Skip already-indexed files
- Progress bar (no UI blocking)
- Efficient hashing (xxHash)

**Search:**
- Pre-computed embeddings (no re-encoding)
- Fast similarity (dot product)
- REPL caches models and index

**Storage:**
- MessagePack (compact, fast)
- Sidecars (no database overhead)
- Per-directory (locality)

### Scaling

**Small collection (< 1000 files):**
- CPU sufficient
- Seconds to index
- Instant search

**Medium collection (1000-10,000 files):**
- GPU recommended
- Minutes to index
- Fast search (< 1s)

**Large collection (> 10,000 files):**
- GPU essential
- Hours to index (one-time)
- Search still fast (linear scan acceptable)
- Consider filtering by directory

---

## Future Architecture Considerations

### Potential Enhancements

1. **Parallel processing** - Batch encoding on GPU
2. **Vector database** - For massive collections (> 100K files)
3. **Incremental indexing** - Watch filesystem changes
4. **Distributed scanning** - Multi-machine indexing
5. **Model caching** - Persistent model memory
6. **Multi-modal search** - Audio, documents, etc.

### Design Principles

- **Simplicity** - No external services required
- **Performance** - GPU-accelerated where possible
- **Portability** - Cross-platform, single binary
- **Privacy** - Fully local, no cloud dependencies
- **Reliability** - Graceful degradation, clear errors

---

This architecture enables Scout to be fast, efficient, and easy to use while remaining hackable and extensible.
