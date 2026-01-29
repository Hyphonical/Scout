# Building Scout

Scout supports two build variants to accommodate different use cases and environments:

## 🖼️ Image-Only Build (Default)

The default build supports image search only and has minimal dependencies.

**Use case:** Local development and testing on systems where FFmpeg compilation is impractical (e.g., Windows laptops).

**Build command:**
```bash
cargo build --release
```

**Features:**
- ✅ Image scanning and indexing
- ✅ Text and image-based search
- ✅ Fast compilation
- ✅ No FFmpeg dependency
- ❌ Video files are detected but show an error message

## 🎥 Video Build (Optional)

The video build includes FFmpeg support for extracting and indexing video frames.

**Use case:** CI/CD environments (GitHub Actions) and systems with FFmpeg build tools pre-installed.

**Build command:**
```bash
cargo build --release --features video
```

**Features:**
- ✅ Image scanning and indexing
- ✅ Text and image-based search  
- ✅ Video frame extraction (10 evenly-spaced frames)
- ✅ Video search with timestamp display
- ⚠️ Requires FFmpeg build dependencies (see below)

### FFmpeg Build Requirements

The video feature compiles FFmpeg from source with static linking. This requires:

**Linux/macOS:**
- Build essentials (`gcc`, `make`, `cmake`)
- `nasm` or `yasm` (assembly compiler)
- `pkg-config`
- `git` (for fetching FFmpeg source)
- `sh` (shell, required by FFmpeg build system)
- ~1-2 GB disk space for build artifacts
- 10-30 minutes build time (depending on CPU)

**Windows:**
- MSYS2 or similar Unix-like environment
- Same tools as Linux/macOS (available via MSYS2)
- full Visual Studio + MSYS2 toolchain
- Not recommended for local development

### Installing FFmpeg Build Dependencies

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install -y build-essential cmake nasm pkg-config git
```

**macOS (Homebrew):**
```bash
brew install cmake nasm pkg-config git
```

**Windows:**
Not recommended. Use GitHub Actions for video builds instead.

## 🤖 CI/CD Builds

GitHub Actions automatically builds both variants for all supported platforms:

**Artifacts produced:**
- `scout-{platform}-x64.{tar.gz|zip}` - Image-only build
- `scout-{platform}-x64-video.{tar.gz|zip}` - Video build with FFmpeg

Supported platforms: Linux (x64), macOS (x64, ARM64), Windows (x64)

## 🧪 Testing Locally

**Check image-only build compiles:**
```bash
cargo check
```

**Check video build compiles (will fail on Windows without proper setup):**
```bash
cargo check --features video
```

**Run image-only build:**
```bash
cargo run --release -- scan /path/to/images
cargo run --release -- search "your query"
```

## 📦 Feature Flags

Scout uses Cargo feature flags for optional functionality:

- **`video`** (opt-in): Enables video support via FFmpeg
  - Adds `ffmpeg-next` dependency
  - Compiles FFmpeg from source with static linking
  - Wraps all video code in `#[cfg(feature = "video")]`

By default, no optional features are enabled, resulting in a fast-building image-only binary.
