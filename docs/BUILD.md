# Building Scout

Scout uses system FFmpeg for video support. One binary per platform that handles both images and videos.

## Prerequisites

### All Platforms
- **Rust 1.81.0+**: Install from [rustup.rs](https://rustup.rs)
- **FFmpeg development libraries**: Required for compilation

### Windows

```powershell
# Option 1: vcpkg (recommended)
git clone https://github.com/microsoft/vcpkg.git
cd vcpkg
.\bootstrap-vcpkg.bat
.\vcpkg install ffmpeg:x64-windows
# Set environment variables:
$env:VCPKG_ROOT = "C:\path\to\vcpkg"
$env:FFMPEG_DIR = "$env:VCPKG_ROOT\installed\x64-windows"

# Option 2: Chocolatey
choco install ffmpeg-full
# Then set FFMPEG_DIR to the FFmpeg installation path
```

### macOS

```bash
brew install ffmpeg pkg-config
```

### Linux

```bash
# Debian/Ubuntu
sudo apt-get install ffmpeg libavcodec-dev libavformat-dev libavutil-dev \
                     libavfilter-dev libswscale-dev libswresample-dev pkg-config

# Fedora
sudo dnf install ffmpeg-devel

# Arch
sudo pacman -S ffmpeg
```

## Building

```bash
git clone https://github.com/Hyphonical/Scout.git
cd scout
cargo build --release
```

Binary will be at `target/release/scout` (or `scout.exe` on Windows).

## CI/CD Builds

GitHub Actions builds for all platforms with FFmpeg pre-installed:

**Artifacts produced:**
- `scout-linux-x64.tar.gz`
- `scout-macos-arm64.tar.gz`
- `scout-macos-x64.tar.gz`
- `scout-windows-x64.zip`

## Runtime Behavior

- **FFmpeg installed**: Full functionality (images + videos)
- **FFmpeg not installed**: Videos are skipped with a warning, images work normally

The binary checks for FFmpeg availability at runtime and degrades gracefully.

## Testing

```bash
# Check build compiles
cargo check

# Run tests
cargo test

# Run locally
cargo run --release -- scan /path/to/media
cargo run --release -- search "your query"
```

## Troubleshooting

**"No linking method set!" during build:**  
FFmpeg development libraries are not found. Install them per the instructions above.

**"pkg-config not found":**  
Install pkg-config (`brew install pkg-config` on macOS, `apt install pkg-config` on Linux).

**Video files skipped at runtime:**  
FFmpeg is not installed or not in PATH. Install FFmpeg and verify with `ffmpeg -version`.
