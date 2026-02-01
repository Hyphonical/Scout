# Scout 🔍

**Semantic image search that actually works locally.**

Find images by what's _in_ them, not what you named the file. No cloud, no API keys, no privacy concerns. Just you, your images, and some clever AI running on your machine.

## Table of Contents

- [What's This About?](#whats-this-about)
- [Why Does This Exist?](#why-does-this-exist)
- [Features](#features-)
- [Installation](#installation-)
  - [Prerequisites](#prerequisites)
  - [Build from Source](#build-from-source)
  - [Download Pre-built Binaries](#download-pre-built-binaries)
- [Model Setup](#model-setup)
- [Quick Start](#quick-start-)
- [Usage](#usage)
  - [`scan` - Index Images](#scan---index-images)
  - [`search` - Find Images](#search---find-images)
  - [`clean` - Remove Orphaned Sidecars](#clean---remove-orphaned-sidecars)
  - [Global Options](#global-options)
- [Hardware Support](#hardware-support-)
- [How It Works](#how-it-works-)
- [Configuration](#configuration)
- [Video Support](#video-support-)
- [Troubleshooting](#troubleshooting-)
- [Contributing](#contributing)
- [License](#license)

## What's This About?

You know how you take a photo of a cool sunset, name it `IMG_2847.jpg`, and then spend 20 minutes hunting through folders trying to find "that one sunset pic"? Yeah, Scout fixes that.

Type `scout search "sunset over mountains"` and boom - every sunset you've ever photographed, ranked by relevance. Or feed it a reference image with `-i photo.jpg` for reverse image search. Or do _both_ simultaneously and weight them however you want. It's weirdly satisfying.

Scout uses [SigLIP2](https://huggingface.co/onnx-community/siglip2-large-patch16-512-ONNX) vision-language models (the same tech that powers modern AI image understanding) to generate embeddings for your images. These embeddings capture semantic meaning, so "golden retriever" will find your dog photos even if you never tagged them.

## Why Does This Exist?

Look, I'm not going to pretend this is revolutionary. There are probably hundreds of semantic image search tools out there. Some are definitely fancier. Some have better UIs. Some have VC funding and marketing budgets.

But here's the thing: I wanted one that was **fast**, **private**, and **didn't require a PhD to set up**. Something I could point at a folder and just... use. No Docker containers, no Elasticsearch clusters, no "please sign up for our SaaS platform."

Also, I think Rust is neat and wanted an excuse to mess with ONNX Runtime. ✨

So if you're the kind of person who has 10,000+ photos scattered across folders and external drives, and you're tired of macOS Spotlight failing to find anything useful, maybe Scout is for you too.

## Features 🎯

- **Text-based search** 📝: Find images by natural language descriptions
- **Image-based search** 🖼️: Reverse image search using a reference photo
- **Hybrid search** 🔀: Combine text + image queries with adjustable weighting
- **Negative prompts** 🚫: Exclude unwanted content with `--not` flag
- **Parallel processing** ⚡: Multi-threaded scanning with `--threads` (default: 2)
- **Multiple output formats** 📊: Pretty (default), JSON, or plain text output
- **Video support** 🎬: Index video files by extracting key frames (requires FFmpeg)
- **Recursive scanning** 📁: Index entire directory trees in one go
- **Smart filtering** ⚙️: Set minimum dimensions, file sizes, exclude patterns
- **Custom model paths** 🔧: Specify custom ONNX models via CLI or config file
- **Configuration file** ⚙️: Persistent settings in `~/.scout/config.toml`
- **Multiple backends** 🚀: Auto-detects best hardware (CUDA, TensorRT, CoreML, XNNPACK, or CPU)
- **Sidecar storage** 💾: Embeddings stored alongside images, no central database to corrupt
- **Offline everything** 🔒: No internet required after initial model download
- **Fast** ⚡: Batch processing and optimized inference (~50-200ms per image depending on hardware)
- **Version tracking** 🔄: Automatically handles model upgrades across your index
- **Cross platform** 🌐: Works on Linux, macOS, Windows
- **Portability remains** 📦: Move/copy your images and their `.scout/` sidecars go with them

## Installation 📦

### Prerequisites

1. **Rust** (1.70+): Install from [rustup.rs](https://rustup.rs)
2. **ONNX Runtime models**: See [models/Models.md](models/Models.md) for download instructions
3. **FFmpeg** (Optional, for video support): Install from [FFmpeg website](https://ffmpeg.org) or package manager
4. **CUDA/TensorRT DLLs** (Optional, NVIDIA GPU only): [ONNX Runtime GPU setup](https://onnxruntime.ai/docs/execution-providers/CUDA-ExecutionProvider.html)

> **Video Support**: Scout uses FFmpeg via subprocess for video processing. If FFmpeg is not in your PATH, video features will be disabled with a clear message. Image search works without FFmpeg.

### Build from source

```bash
git clone https://github.com/Hyphonical/Scout.git
cd scout
cargo build --release
```

> **Note:** Video support works automatically if FFmpeg is installed and in PATH. No build flags needed.

Binary will be at `target/release/scout` (or `scout.exe` on Windows).

### Download pre-built binaries

Check the [Releases](https://github.com/Hyphonical/Scout/releases) page for pre-built binaries:
- `scout-linux-x64.tar.gz` - Linux (x64)
- `scout-macos-arm64.tar.gz` - macOS (Apple Silicon)
- `scout-windows-x64.zip` - Windows (x64)

**Video support**: Install FFmpeg on your system for video processing. See [FFmpeg Installation Guide](docs/INSTALL_FFMPEG.md).

## Quick Start 🚀

```bash
git clone https://github.com/Hyphonical/Scout.git
cd scout
cargo build --release
```

The binary will be at `target/release/scout` (or `scout.exe` on Windows).

### Model setup

Download the three required model files into the `models/` directory:
- `vision_model_q4f16.onnx` (175 MB)
- `text_model_q4f16.onnx` (665 MB)  
- `tokenizer.json` (33 MB)

**→ Full instructions with alternative models: [docs/MODELS.md](docs/MODELS.md)**

### Quick Commands

```bash
# Index all images in a folder (recursively)
scout scan -d ~/Photos -r

# Search by text
scout search "cat sleeping on keyboard" -d ~/Photos

# Search by reference image
scout search -i reference.jpg -d ~/Photos

# Combine text + image (70% text, 30% image)
scout search "vintage car" -i car.jpg -w 0.7 -d ~/Photos
```

## Usage

### `scan` - Index images

```bash
scout scan [OPTIONS]

Options:
  -d, --dir <PATH>              Directory to scan [default: .]
  -r, --recursive               Scan subdirectories
  -f, --force                   Reprocess already-indexed images
  -t, --threads <N>             Number of parallel threads [default: 2] (1 = sequential)
  --min-width <PIXELS>          Minimum image width [default: 64]
  --min-height <PIXELS>         Minimum image height [default: 64]
  --min-size <KB>               Minimum file size in KB [default: 0]
  --max-size <MB>               Maximum file size in MB
  --exclude <PATTERNS>          Comma-separated exclude patterns
```

**Examples:**
```bash
# Scan with 4 parallel threads
scout scan -d ./photos -r --threads 4

# Sequential processing (no parallelism, lower VRAM usage)
scout scan -d ./photos -r --threads 1

# With filtering
scout scan -d ./photos -r --min-width 512 --exclude "thumbnails,cache"
```

**Note:** With multiple threads, each worker loads its own model instance. This speeds up processing but uses more VRAM on GPUs. Use `--threads 1` if you have limited VRAM.

Scout creates a `.scout/` folder next to each directory containing images. Inside are `.msgpack` files (one per image) containing embeddings and metadata. This means:
- No centralized database that can get corrupted
- Embeddings travel with your images when you move/copy folders
- Easy to delete (just remove `.scout/` folders)

### `search` - Find images

```bash
scout search [QUERY] [OPTIONS]

Options:
  -i, --image <PATH>            Reference image for similarity search
  -w, --weight <0.0-1.0>        Text weight in combined search [default: 0.5]
  -d, --dir <PATH>              Directory to search [default: .]
  -r, --recursive               Search subdirectories
  -n, --limit <N>               Max results to show [default: 10]
  -s, --score <FLOAT>           Minimum similarity score [default: 0.0]
  -o, --open                    Open top result in default viewer
  --include-ref                 Include reference image in results
  -f, --format <FORMAT>         Output format: pretty, json, plain [default: pretty]
  --not <QUERY>                 Negative prompt to exclude content
```

**Examples:**

```bash
# Text search with high threshold
scout search "beach sunset" -s 0.15 -n 5

# Reverse image search
scout search -i ~/reference.jpg -d ~/Photos -r

# Combined: 30% text, 70% image
scout search "red sports car" -i ferrari.jpg -w 0.3

# Search with negative prompt
scout search "woman on beach" --not "dog with frisbee"

# JSON output for scripting
scout search "cat" --format json > results.json

# Plain text output
scout search "sunset" --format plain | head -5

# Search and open top result
scout search "dog" -o
```

**Similarity scores** range from -1 to 1 (higher = better match):
- `0.15+`: Excellent matches
- `0.08-0.15`: Good matches  
- `<0.08`: Weak matches

> [!TIP]
> The better and more elaborate your query, the higher the scores you'll see. Negative prompts reduce scores for matching content using semantic subtraction (score = positive - 0.7×negative).

### `clean` - Remove orphaned sidecars

```bash
scout clean [OPTIONS]

Options:
  -d, --dir <PATH>     Directory to clean [default: .]
  -r, --recursive      Clean subdirectories
  -y, --yes            Skip confirmation prompt
```

Deletes `.scout/` sidecar files for images that no longer exist. Useful after moving/deleting photos.

### `config` - Manage configuration

```bash
scout config [OPTIONS]

Options:
  --init    Create config file with defaults
  --show    Show config file path
```

**Examples:**
```bash
# Create config file at ~/.scout/config.toml
scout config --init

# Show where config file should be
scout config --show
```

### Global options

```bash
-v, --verbose                  Show debug output
-p, --provider <TYPE>          Force execution provider [auto,cpu,cuda,tensorrt,coreml,xnnpack]
--vision-model <PATH>          Custom vision model ONNX file
--text-model <PATH>            Custom text model ONNX file
--tokenizer <PATH>             Custom tokenizer JSON file
--disable-video                Skip video files during scanning
```

**Examples:**

```bash
# Use custom models
scout search "cat" --vision-model ./my_model.onnx --text-model ./my_text.onnx

# Force CPU execution
scout scan -d ./photos --provider cpu

# Verbose output with CUDA
scout scan -d ./photos --provider cuda --verbose
```

## Hardware Support ⚡

Scout auto-detects the best execution provider for your hardware:

| Provider | Hardware | Performance |
|----------|----------|-------------|
| **TensorRT** | NVIDIA GPUs (RTX series) | Fastest (50-100ms/image) |
| **CUDA** | NVIDIA GPUs | Very fast (80-150ms/image) |
| **CoreML** | Apple Silicon (M1/M2/M3) | Very fast (80-150ms/image) |
| **XNNPACK** | ARM/x64 CPUs | Moderate (200-400ms/image) |
| **CPU** | Fallback | Slower (500-1000ms/image) |

Override with `--provider <type>` if needed. Use `--verbose` to see which provider is active.

## How It Works 🧠

1. **Scanning**: Scout reads images, resizes them to 512×512, and generates 1024-dimensional embeddings using a vision model
2. **Storage**: Embeddings are saved as `.msgpack` files in `.scout/` folders next to your images  
   → See [docs/SIDECAR.md](docs/SIDECAR.md) for detailed information about the sidecar format
3. **Searching**: Your query (text/image/both) is converted to an embedding, then compared against all indexed images using cosine similarity
4. **Ranking**: Results are sorted by similarity score (dot product of normalized vectors)

The models are quantized to Q4F16 (4-bit weights, FP16 activations) for a good balance of speed, size, and accuracy. See [docs/MODELS.md](docs/MODELS.md) for alternatives.

## Configuration

Scout supports a configuration file at `~/.scout/config.toml` for persistent settings.

Create it with: `scout config --init`

```toml
[scan]
# Number of parallel threads (1 = sequential)
threads = 2
# Scan subdirectories by default
recursive = false
# Force reprocessing
force = false

[models]
# Execution provider: "auto", "cuda", "tensorrt", "coreml", "xnnpack", or "cpu"
# provider = "cuda"

# Custom model paths (optional, defaults to models/ directory)
# vision_model = "./models/vision_model_q4f16.onnx"
# text_model = "./models/text_model_q4f16.onnx"
# tokenizer = "./models/tokenizer.json"

[search]
# Default number of search results
default_limit = 10
# Minimum similarity score threshold
min_score = 0.0
# Negative prompt weight multiplier (0.0-1.0)
negative_weight = 0.7
```

**Priority**: CLI arguments > config file > defaults

Run `scout config --show` to see the config file path.

## Video Support 🎬

Scout can process video files by extracting key frames and indexing them. This requires FFmpeg to be installed on your system.

### Installing FFmpeg

**Windows:**
```powershell
choco install ffmpeg-full
# or download from https://www.gyan.dev/ffmpeg/builds/
```

**macOS:**
```bash
brew install ffmpeg
```

**Linux:**
```bash
sudo apt install ffmpeg  # Debian/Ubuntu
sudo dnf install ffmpeg  # Fedora
sudo pacman -S ffmpeg    # Arch
```

### Verification

```bash
ffmpeg -version  # Should show FFmpeg 4.4 or newer
scout scan -d ~/Videos -r  # Will process videos if FFmpeg found
```

If FFmpeg is not installed, Scout will skip video files and only process images. You'll see a one-time warning message.

### Supported Formats

`.mp4`, `.mkv`, `.avi`, `.mov`, `.webm`, `.wmv`, `.flv`, `.m4v`

## Troubleshooting 🔧

**"Vision model not found"**  
→ Download models to `models/` directory. See [docs/MODELS.md](docs/MODELS.md)

**"No results found"**  
→ Run `scout scan -d <dir> -r` first to index images  
→ Try lowering `--score` threshold or using broader search terms

**Slow performance**  
→ Check execution provider with `--verbose` flag  
→ Ensure CUDA/CoreML/TensorRT is properly installed  
→ Consider using smaller model variant (see docs/MODELS.md)
**"X outdated sidecars found"**  
→ Run `scout scan -f` to regenerate embeddings with current model version

## Contributing

This is a weekend project that got slightly out of hand. If you find bugs or have ideas, PRs are welcome! The codebase is intentionally kept simple and readable.

Things I'd love help with:
- Better test coverage
- More export formats (JSON, CSV, HTML gallery)
- Performance optimizations

## License

MIT - do whatever you want with it. If you build something cool on top of this, let me know!

---

Made with ☕ and questionable life choices (It's Rust and ONNX, what did you expect?). If you have 10,000 photos named `IMG_XXXX.jpg`, this is for you.
