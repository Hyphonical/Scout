# Scout 🔍

**Semantic image search that actually works locally.**

Find images by what's _in_ them, not what you named the file. No cloud, no API keys, no privacy concerns. Just you, your images, and some clever AI running on your machine.

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

- **Text-based search**: Find images by natural language descriptions
- **Image-based search**: Reverse image search using a reference photo
- **Hybrid search**: Combine text + image queries with adjustable weighting
- **Live interactive mode**: TUI interface with real-time search-as-you-type
- **Recursive scanning**: Index entire directory trees in one go
- **Smart filtering**: Set minimum dimensions, file sizes, exclude patterns
- **Multiple backends**: Auto-detects best hardware (CUDA, TensorRT, CoreML, XNNPACK, or CPU)
- **Sidecar storage**: Embeddings stored alongside images, no central database to corrupt
- **Offline everything**: No internet required after initial model download
- **Fast**: Batch processing and optimized inference (~50-200ms per image depending on hardware)
- **Version tracking**: Automatically handles model upgrades across your index
- **Cross platform**: Works on Linux, macOS, Windows
- **Portabilty remains**: Move/copy your images and their `.scout/` sidecars go with them

## Installation 📦

### Prerequisites

1. **Rust** (1.70+): Install from [rustup.rs](https://rustup.rs)
2. **ONNX Runtime models**: See [models/Models.md](models/Models.md) for download instructions
3. **CUDA/TensorRT DLLs** (NVIDIA GPU only): [ONNX Runtime GPU setup](https://onnxruntime.ai/docs/execution-providers/CUDA-ExecutionProvider.html)

### Build from source

```bash
git clone https://github.com/yourusername/scout.git
cd scout
cargo build --release
```

The binary will be at `target/release/scout` (or `scout.exe` on Windows).

### Model setup

Download the three required model files into the `models/` directory:
- `vision_model_q4f16.onnx` (175 MB)
- `text_model_q4f16.onnx` (665 MB)  
- `tokenizer.json` (33 MB)

**→ Full instructions with alternative models: [models/Models.md](models/Models.md)**

## Quick Start 🚀

```bash
# Index all images in a folder (recursively)
scout scan -d ~/Photos -r

# Search by text
scout search "cat sleeping on keyboard" -d ~/Photos

# Search by reference image
scout search -i reference.jpg -d ~/Photos

# Combine text + image (70% text, 30% image)
scout search "vintage car" -i car.jpg -w 0.7 -d ~/Photos

# Live interactive search
scout live -d ~/Photos -r
```

## Usage

### `scan` - Index images

```bash
scout scan [OPTIONS]

Options:
  -d, --dir <PATH>              Directory to scan [default: .]
  -r, --recursive               Scan subdirectories
  -f, --force                   Reprocess already-indexed images
  --min-width <PIXELS>          Minimum image width [default: 64]
  --min-height <PIXELS>         Minimum image height [default: 64]
  --min-size <KB>               Minimum file size in KB [default: 0]
  --max-size <MB>               Maximum file size in MB
  --exclude <PATTERNS>          Comma-separated exclude patterns
```

**Example:**
```bash
scout scan -d ./photos -r --min-width 512 --exclude "thumbnails,cache"
```

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
```

**Examples:**

```bash
# Text search with high threshold
scout search "beach sunset" -s 0.15 -n 5

# Reverse image search
scout search -i ~/reference.jpg -d ~/Photos -r

# Combined: 30% text, 70% image
scout search "red sports car" -i ferrari.jpg -w 0.3

# Search and open top result
scout search "dog" -o
```

**Similarity scores** range from -1 to 1 (higher = better match):
- `0.15+`: Excellent matches
- `0.08-0.15`: Good matches  
- `<0.08`: Weak matches

> [!TIP]
> The better and more elaborate your query, the higher the scores you'll see.

### `live` - Interactive search

```bash
scout live -d ~/Photos -r
```

Launches a terminal UI with real-time search:
- Type to filter results instantly (400ms debounce)
- `↑`/`↓` to navigate results
- `Enter` to open selected image
- `Esc` to quit

Great for exploratory search when you're not sure exactly what you're looking for.

### `clean` - Remove orphaned sidecars

```bash
scout clean [OPTIONS]

Options:
  -d, --dir <PATH>     Directory to clean [default: .]
  -r, --recursive      Clean subdirectories
  -y, --yes            Skip confirmation prompt
```

Deletes `.scout/` sidecar files for images that no longer exist. Useful after moving/deleting photos.

### Global options

```bash
-v, --verbose          Show debug output
-p, --provider <TYPE>  Force execution provider [auto,cpu,cuda,tensorrt,coreml,xnnpack]
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
3. **Searching**: Your query (text/image/both) is converted to an embedding, then compared against all indexed images using cosine similarity
4. **Ranking**: Results are sorted by similarity score (dot product of normalized vectors)

The models are quantized to Q4F16 (4-bit weights, FP16 activations) for a good balance of speed, size, and accuracy. See [models/Models.md](models/Models.md) for alternatives.

## Configuration

Models and constants are defined in `src/config.rs`. If you use different model variants, update:

```rust
pub const VISION_MODEL: &str = "vision_model_q4f16.onnx";
pub const TEXT_MODEL: &str = "text_model_q4f16.onnx";
pub const INPUT_SIZE: u32 = 512;  // Must match model
pub const EMBEDDING_DIM: usize = 1024;  // Must match model
```

Then rebuild: `cargo build --release`

## Troubleshooting 🔧

**"Vision model not found"**  
→ Download models to `models/` directory. See [models/Models.md](models/Models.md)

**"No results found"**  
→ Run `scout scan -d <dir> -r` first to index images  
→ Try lowering `--score` threshold or using broader search terms

**Slow performance**  
→ Check execution provider with `--verbose` flag  
→ Ensure CUDA/CoreML/TensorRT is properly installed  
→ Consider using smaller model variant (see Models.md)

**"X outdated sidecars found"**  
→ Run `scout scan -f` to regenerate embeddings with current model version

## Contributing

This is a weekend project that got slightly out of hand. If you find bugs or have ideas, PRs are welcome! The codebase is intentionally kept simple and readable.

Things I'd love help with:
- Better test coverage
- More export formats (JSON, CSV, HTML gallery)
- Automatic binary releases
- Performance optimizations (workflow)
- Video support (extract N amount of frames and index them)

## License

MIT - do whatever you want with it. If you build something cool on top of this, let me know!

---

Made with ☕ and questionable life choices (It's Rust and ONNX, what did you expect?). If you have 10,000 photos named `IMG_XXXX.jpg`, this is for you.
