# Building Scout

Scout supports two build variants to accommodate different use cases and environments:

## 🖼️ Image-Only Build (Default)

The default build supports image search only and has minimal dependencies.

**Use case:** Local development and testing on systems where FFmpeg compilation is impractical.

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

The video build includes FFmpeg support for extracting and indexing video frames via rsmpeg.

**Use case:** CI/CD environments (GitHub Actions) and systems with FFmpeg installed or build tools available.

**Build command:**
```bash
cargo build --release --features video
```

**Requirements:**
- ✅ Rust 1.81.0+ (required by rsmpeg)
- ✅ FFmpeg libraries (static or system)

**Features:**
- ✅ Image scanning and indexing
- ✅ Text and image-based search  
- ✅ Video frame extraction (10 evenly-spaced frames)
- ✅ Video search with timestamp display

### rsmpeg: Modern FFmpeg Bindings

Scout uses [rsmpeg](https://github.com/larksuite/rsmpeg) for video support, which provides:
- ✅ **Actively maintained** (unlike ffmpeg-next)
- ✅ **Windows support** (via vcpkg or system FFmpeg)
- ✅ **FFmpeg 7.x support** (latest stable)
- ✅ **System linking** (uses installed FFmpeg via pkg-config)

### FFmpeg Installation

rsmpeg uses the `link_system_ffmpeg` feature, which links against system-installed FFmpeg libraries. You must install FFmpeg development packages before building.

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install -y libavcodec-dev libavformat-dev libavutil-dev \
                        libswscale-dev libswresample-dev libavdevice-dev \
                        libavfilter-dev pkg-config
```

**macOS:**
```bash
brew install ffmpeg pkg-config
```

**Windows:**
```bash
# Option 1: vcpkg (recommended)
vcpkg install ffmpeg:x64-windows

# Option 2: Chocolatey
choco install ffmpeg
```

Then build with:
```bash
cargo build --release --features video
```

### Why System FFmpeg?

- ✅ **Fast builds** (~2-5 minutes vs 30+ for compiling FFmpeg)
- ✅ **Smaller binaries** (dynamically linked)
- ✅ **OS-optimized** (uses system codecs and hardware acceleration)
- ✅ **Easy updates** (update FFmpeg separately via package manager)
- ⚠️ **Runtime dependency** (users need FFmpeg installed)

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

- **`video`** (opt-in): Enables video support via rsmpeg
  - Links against system FFmpeg via `link_system_ffmpeg` feature
  - Requires FFmpeg development libraries installed
  - All video code wrapped in `#[cfg(feature = "video")]`

By default, no optional features are enabled, resulting in a fast-building image-only binary.
