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
- ✅ **Actively maintained**
- ✅ **Windows static builds** (works out of the box)
- ✅ **FFmpeg 7.x support** (latest stable)
- ✅ **Flexible linking** (static, system, vcpkg)

### FFmpeg Build/Installation Options

**Option 1: Let rsmpeg compile FFmpeg (CI/Linux/macOS)**

Default behavior when building with `--features video`:
```bash
cargo build --release --features video
```

Requirements:
- Build essentials: `gcc`, `make`, `cmake`
- `nasm` or `yasm` (assembly optimizer)
- `pkg-config`
- ~10-30 minutes build time (grab a coffee!)

**Install on Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install -y build-essential cmake nasm pkg-config
```

**Install on macOS:**
```bash
brew install cmake nasm pkg-config
```

**Option 2: Use system FFmpeg (fastest for development)**

If you have FFmpeg 4.4+ installed:
```bash
# Install FFmpeg first
# Ubuntu/Debian:
sudo apt-get install -y libavcodec-dev libavformat-dev libavutil-dev libswscale-dev

# macOS:
brew install ffmpeg

# Then build Scout
cargo build --release --features video
```

rsmpeg will automatically detect and use system FFmpeg.

**Option 3: Use vcpkg (Windows)**

```bash
vcpkg install ffmpeg:x64-windows
cargo build --release --features video
```

**Windows Note:**
Static FFmpeg builds work on Windows with rsmpeg. However, it's still easier to use CI for Windows builds or install FFmpeg via vcpkg/chocolatey.

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
  - Compiles FFmpeg from source with static linking
  - Wraps all video code in `#[cfg(feature = "video")]`

By default, no optional features are enabled, resulting in a fast-building image-only binary.
