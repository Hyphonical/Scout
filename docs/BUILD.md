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
### rsmpeg

Scout uses [rsmpeg](https://github.com/larksuite/rsmpeg) 0.18 to extract frames from videos for semantic search. rsmpeg is a modern Rust wrapper around FFmpeg 8.x that provides safe, idiomatic bindings.

**FFmpeg 8 is required** - System packages may have older versions, so we build FFmpeg 8 from source in CI to ensure compatibility.

### Building FFmpeg 8 from Source

**Ubuntu/Debian:**
```bash
# Install build dependencies
sudo apt-get update
sudo apt-get install -y build-essential pkg-config yasm nasm \
                        libx264-dev libx265-dev libvpx-dev libfdk-aac-dev \
                        libmp3lame-dev libopus-dev

# Download and build FFmpeg 8
wget https://ffmpeg.org/releases/ffmpeg-8.0.tar.xz
tar xf ffmpeg-8.0.tar.xz
cd ffmpeg-8.0
./configure --prefix=$HOME/ffmpeg-8 \
  --enable-gpl --enable-version3 --enable-nonfree \
  --enable-shared --disable-static \
  --enable-libx264 --enable-libx265 --enable-libvpx \
  --enable-libfdk-aac --enable-libmp3lame --enable-libopus
make -j$(nproc)
make install

# Set environment variables
export PKG_CONFIG_PATH="$HOME/ffmpeg-8/lib/pkgconfig:$PKG_CONFIG_PATH"
export LD_LIBRARY_PATH="$HOME/ffmpeg-8/lib:$LD_LIBRARY_PATH"
export PATH="$HOME/ffmpeg-8/bin:$PATH"
```

**macOS:**
```bash
# Install build dependencies
brew install pkg-config yasm nasm x264 x265 libvpx fdk-aac lame opus

# Download and build FFmpeg 8
wget https://ffmpeg.org/releases/ffmpeg-8.0.tar.xz
tar xf ffmpeg-8.0.tar.xz
cd ffmpeg-8.0
./configure --prefix=$HOME/ffmpeg-8 \
  --enable-gpl --enable-version3 --enable-nonfree \
  --enable-shared --disable-static \
  --enable-libx264 --enable-libx265 --enable-libvpx \
  --enable-libfdk-aac --enable-libmp3lame --enable-libopus
make -j$(sysctl -n hw.ncpu)
make install

# Set environment variables
export PKG_CONFIG_PATH="$HOME/ffmpeg-8/lib/pkgconfig:$PKG_CONFIG_PATH"
export DYLD_LIBRARY_PATH="$HOME/ffmpeg-8/lib:$DYLD_LIBRARY_PATH"
export PATH="$HOME/ffmpeg-8/bin:$PATH"
```

**Windows:**
Video support is optional on Windows. You can use vcpkg or pre-built FFmpeg 8 binaries.

Then build Scout with:
```bash
cargo build --release --features video
```

### Why Build FFmpeg 8 from Source?

- ✅ **Guaranteed compatibility** - rsmpeg 0.18 requires FFmpeg 8.x APIs
- ✅ **CI caching** - Build once, cache for future runs (~2-5 min on cache hit)
- ✅ **Latest features** - FFmpeg 8 has improved codecs and performance
- ⚠️ **Initial build time** - First build takes ~30 minutes (cached afterward)

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
