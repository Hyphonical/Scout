# Scout 🔍

**Semantic media search that actually works locally.**

Find images and videos by what's _in_ them, not what you named the file. No cloud, no API keys, no privacy concerns. Just you, your media, and some clever AI running on your machine.

## Table of Contents

- [What's This About?](#whats-this-about)
- [Why Does This Exist?](#why-does-this-exist)
- [Features](#features-)
- [Quick Start](#quick-start-)
- [Documentation](#documentation-)
- [Usage](#usage)
  - [`scan` - Index Media](#scan---index-media)
  - [`search` - Find Media](#search---find-media)
  - [`cluster` - Group Media by Visual Similarity](#cluster---group-media-by-visual-similarity)
  - [`clean` - Remove Orphaned Sidecars](#clean---remove-orphaned-sidecars)
  - [`watch` - Auto-Index New Files](#watch---auto-index-new-files)
  - [Global Options](#global-options)
- [Hardware Support](#hardware-support-)
- [Contributing](#contributing)
- [License](#license)

## What's This About?

You know how you take a photo of a cool sunset, name it `IMG_2847.jpg`, and then spend 20 minutes hunting through folders trying to find "that one sunset pic"? Yeah, Scout fixes that.

Type `scout search "sunset over mountains"` and boom - every sunset you've ever photographed, ranked by relevance. Or feed it a reference image with `-i photo.jpg` for reverse image search. Or do _both_ simultaneously and weight them however you want. It's weirdly satisfying.

Scout uses [SigLIP2](https://huggingface.co/onnx-community/siglip2-large-patch16-512-ONNX) vision-language models (the same tech that powers modern AI image understanding) to generate embeddings for your media. These embeddings capture semantic meaning, so "golden retriever" will find your dog photos even if you never tagged them.

## Why Does This Exist?

Look, I'm not going to pretend this is revolutionary. There are probably hundreds of semantic image search tools out there. Some are definitely fancier. Some have better UIs. Some have VC funding and marketing budgets.

But here's the thing: I wanted one that was **fast**, **private**, and **didn't require a PhD to set up**. Something I could point at a folder and just... use. No Docker containers, no Elasticsearch clusters, no "please sign up for our SaaS platform."

Also, I think Rust is neat and wanted an excuse to mess with ONNX Runtime. ✨

So if you're the kind of person who has 10,000+ photos scattered across folders and external drives, and you're tired of macOS Spotlight failing to find anything useful, maybe Scout is for you too.

## Features 🎯

- **📝 Text-based search**: Find media by natural language descriptions
- **🖼️ Image-based search**: Reverse image search using a reference photo
- **🔀 Hybrid search**: Combine text + image queries with adjustable weighting
- **✨ HDBSCAN clustering**: Group media by visual similarity with optional dimensionality reduction
- **🚫 Negative prompts**: Exclude unwanted content with `--not` flag
- **🎬 Video support**: Index video files using intelligent scene detection (requires FFmpeg)
- **📁 Recursive scanning**: Index entire directory trees in one go
- **⚙️ Smart filtering**: Exclude videos, set minimum resolution, file size limits
- **🔧 Custom model paths**: Specify custom ONNX models via CLI or environment
- **🚀 Multiple backends**: Auto-detects best hardware (CUDA, TensorRT, CoreML, XNNPACK, or CPU)
- **💾 Sidecar storage**: Embeddings stored alongside media, no central database
- **🔒 Offline everything**: No internet required after initial model download
- **⚡ Fast**: Optimized inference (~50-200ms per file depending on hardware)
- **🌐 Cross platform**: Works on Linux, macOS, Windows
- **📦 Portability**: Move/copy files and `.scout/` sidecars travel with them
- **👁️ Watch mode**: Monitor directories and auto-index new files as they arrive
- **🏷️ Rename immunity**: Files identified by content hash, not filename - rename freely!
- **📤 Export support**: Export search and cluster results as JSON for scripting
- **🔗 Path output**: Output file paths directly for shell pipelines and automation

## Demo

<img src="assets/scan.gif" width="600" alt="Scout scanning demo">

## Quick Start 🚀

### 1. Install

```bash
git clone https://github.com/Hyphonical/Scout.git
cd scout
cargo build --release
```

Binary at `target/release/scout` (or `scout.exe` on Windows).

### 2. Get Models

Download three model files into `models/` directory:
- `vision_model_q4f16.onnx` (175 MB)
- `text_model_q4f16.onnx` (665 MB)  
- `tokenizer.json` (33 MB)

**→ Full download instructions: [docs/MODELS.md](docs/MODELS.md)**

### 3. Index and Search

```bash
# Index all images (recursive)
scout scan -d ~/Photos -r

# Text search
scout search "cat sleeping on keyboard" -d ~/Photos

# Image search
scout search -i reference.jpg -d ~/Photos

# Cluster images by visual similarity
scout cluster -d ~/Photos
```

## Documentation 📚

- **[User Guide](docs/USER-GUIDE.md)** - Comprehensive feature documentation
- **[Clustering Guide](docs/CLUSTERING.md)** - How the clustering math works (UMAP & HDBSCAN)
- **[Architecture](docs/ARCHITECTURE.md)** - Technical deep dive
- **[Contributing](docs/CONTRIBUTING.md)** - Developer guide
- **[Models](docs/MODELS.md)** - Model download and alternatives
- **[Video Support](docs/INSTALL_FFMPEG.md)** - FFmpeg installation guide
- **[Sidecar Format](docs/SIDECAR.md)** - Storage format details

## Usage

### `scan` - Index media

```bash
scout scan [OPTIONS]

Options:
  -d, --dir <PATH>              Directory to scan [default: .]
  -r, --recursive               Scan subdirectories
  -f, --force                   Reprocess already-indexed files
  --exclude-videos              Skip video files
  --min-resolution <PIXELS>     Minimum resolution (shortest side in pixels)
  --max-size <MB>               Maximum file size in MB
  --max-frames <N>              Maximum frames per video [default: 15]
  --scene-threshold <0.0-1.0>   Scene detection threshold [default: 0.3]
```

> [!TIP]
> **Video frame extraction uses intelligent scene detection**: Instead of extracting frames at fixed intervals, Scout analyzes video content to identify scene changes. Static videos (like a single pose or interview) extract 1-4 frames, while action-packed videos extract up to 15 frames at key moments. Adjust `--scene-threshold` (lower = more sensitive) and `--max-frames` to control behavior.

**Examples:**
```bash
# Basic recursive scan
scout scan -d ./photos -r

# Exclude videos
scout scan -d ./photos -r --exclude-videos

# With filtering
scout scan -d ./photos -r --min-resolution 512 --max-size 50

# Custom video extraction (more sensitive scene detection)
scout scan -d ./videos -r --scene-threshold 0.2 --max-frames 20
```

### `search` - Find media

```bash
scout search [QUERY] [OPTIONS]

Options:
  -i, --image <PATH>            Reference image for similarity search
  -w, --weight <0.0-1.0>        Text weight in combined search [default: 0.5]
  -d, --dir <PATH>              Directory to search [default: .]
  -n, --limit <N>               Max results to show [default: 10]
  -s, --score <FLOAT>           Minimum similarity score [default: 0.0]
  --not <QUERY>                 Negative prompt to exclude content
  --include-ref                 Include reference image in results
  -o, --open                    Open first result
  --exclude-videos              Exclude videos from results
  --paths                       Output only file paths (useful for scripting)
  --export <PATH>               Export results as JSON to file (use '-' for stdout)
```

**Examples:**

```bash
# Text search
scout search "beach sunset" -d ~/Photos

# Image search
scout search -i reference.jpg -d ~/Photos

# Combined (30% text, 70% image)
scout search "red car" -i ferrari.jpg -w 0.3

# With negative prompt
scout search "woman on beach" --not "dog with frisbee"

# Open first result
scout search "sunset" -o

# Export results as JSON
scout search "mountains" --export results.json

# Export to stdout (pipe to jq for processing)
scout search "sunset" --export - | jq '.results[].path'

# Get only file paths (for copying, moving, etc.)
scout search "cat" --paths
```
> [!TIP]
> For best search results, write descriptive captions instead of single keywords. See [SEARCH_TIPS.md](docs/SEARCH_TIPS.md) for detailed guidance on crafting effective queries.

```bash
# Copy search results to a new folder (Windows)
scout search "vacation 2024" --paths > files.txt
foreach ($file in Get-Content files.txt) { Copy-Item $file "C:\Vacation2024\" }

# Copy search results to a new folder (Linux/macOS)
scout search "vacation 2024" --paths | xargs -I {} cp {} /path/to/backup/

# Move low-scoring duplicates (using jq to filter)
scout search "landscape" --export - | jq -r '.results[] | select(.score < 0.5) | .path' | xargs -I {} mv {} ./low_quality/
```

### `cluster` - Group media by visual similarity

```bash
scout cluster [OPTIONS]

Options:
  -d, --dir <PATH>              Directory to cluster [default: .]
  -f, --force                   Force reclustering (ignore cache)
  --min-cluster-size <N>        Minimum media files per cluster [default: 5]
  --min-samples <N>             Minimum samples for core points
  --use-umap                    Use UMAP for dimensionality reduction (experimental)
  --export <PATH>               Export cluster results as JSON (use '-' for stdout)
```

**Examples:**

```bash
# Basic clustering
scout cluster -d ~/Photos

# Force reclustering
scout cluster -d ~/Photos -f

# With UMAP for large collections
scout cluster -d ~/Photos --use-umap

# Stricter clustering (larger clusters)
scout cluster -d ~/Photos --min-cluster-size 10

# Export clusters as JSON
scout cluster -d ~/Photos --export clusters.json

# Export to stdout and process with jq
scout cluster -d ~/Photos --export - | jq '.clusters[0].members[]'

# Organize files by cluster (Windows PowerShell)
scout cluster --export clusters.json
$data = Get-Content clusters.json | ConvertFrom-Json
foreach ($cluster in $data.clusters) {
    New-Item -ItemType Directory -Force -Path "Cluster_$($cluster.id)"
    foreach ($file in $cluster.members) {
        Copy-Item $file "Cluster_$($cluster.id)\"
    }
}

# Organize files by cluster (Linux/macOS with jq)
scout cluster --export - | jq -r '.clusters[] | "mkdir -p cluster_\(.id) && echo \(.members[]) | xargs -I {} cp {} cluster_\(.id)/"' | sh

# Extract only high-cohesion clusters
scout cluster --export - | jq '.clusters[] | select(.cohesion > 0.8)'

# Count files per cluster
scout cluster --export - | jq '.clusters[] | {id, count: .members | length}'

# Find the representative file for each cluster
scout cluster --export - | jq -r '.clusters[] | "\(.id): \(.representative)"'
```

**How it works:**
- Computes embeddings for all media in the collection
- Groups visually similar content using HDBSCAN algorithm
- Displays representative file for each cluster
- Shows cluster cohesion score (how similar items in the cluster are)
- Optional UMAP dimensionality reduction (512D) for large datasets
- Results cached in `.scout/clusters.msgpack` (regenerate with `--force`)

**Example output:**
```
✓ 19 clusters, 1384 media files, 1180 noise (85.3%)

Cluster 0 (33 files, 86.7% cohesion)
  Representative: DA1AWQTKC46JM8D9SF9GJ7MSZ0.jpeg
  [1] FK30SDJAMW44KAPK34JS01JSLQ.jpeg
  [2] 1Q2ASFXAVAT0FAEMQ60F4SZMT0.jpeg
  ... and 31 more
```

### `clean` - Remove orphaned sidecars

```bash
scout clean [OPTIONS]

Options:
  -d, --dir <PATH>     Directory to clean [default: .]
```

Deletes `.scout/` sidecar files for images that no longer exist.

### `watch` - Auto-index new files

```bash
scout watch [OPTIONS]

Options:
  -d, --dir <PATH>              Directory to watch [default: .]
  --exclude-videos              Skip video files
  --min-resolution <PIXELS>     Minimum resolution (shortest side in pixels)
  --max-size <MB>               Maximum file size in MB
  --max-frames <N>              Maximum frames per video [default: 15]
  --scene-threshold <0.0-1.0>   Scene detection threshold [default: 0.3]
```

Monitors a directory for new or modified media files and automatically indexes them in real-time. Perfect for download folders, camera uploads, or ongoing projects.

**How it works:**
- Watches for file system changes (new files, copies, moves)
- Automatically processes qualifying media files
- Hash-based deduplication - files already indexed are skipped
- Queued processing to avoid CPU spikes
- Runs continuously until stopped with `Ctrl+C`

**Examples:**

```bash
# Watch current directory
scout watch

# Watch with filters
scout watch -d ~/Downloads --exclude-videos --min-resolution 512
```

### Global options

```bash
-v, --verbose                  Show debug output
-r, --recursive                Include subdirectories (for all commands)
-p, --provider <TYPE>          Force execution provider [auto,cpu,cuda,tensorrt,coreml,xnnpack]
--model-dir <PATH>             Custom model directory
--ffmpeg-path <PATH>           Custom FFmpeg executable path
```

**Examples:**

```bash
# Scan with recursive enabled and custom models
scout -r scan -d photos/ --model-dir ./my_models

# Force CPU execution for search
scout --provider cpu search "cat"

# Recursive clustering with verbose output
scout -v -r cluster -d ~/Photos
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

> [!TIP]
> Override with `--provider <type>` if needed. Use `--verbose` to see which provider is active.

## How It Works 🧠

1. **Scanning**: Resizes images to 512×512, generates 1024-dimensional embeddings using SigLIP2
2. **Storage**: Embeddings saved as `.msgpack` files in `.scout/` folders (see [docs/SIDECAR.md](docs/SIDECAR.md))
3. **Searching**: Query converted to embedding, compared via cosine similarity
4. **Ranking**: Results sorted by similarity score (dot product of normalized vectors)
5. **Export**: Results can be exported as JSON for further processing and automation

Models are quantized to Q4F16 (4-bit weights, FP16 activations) for speed/size/accuracy balance.

> [!CAUTION]
> ⚠️ Disclaimer
>
> **Scout uses AI models for semantic understanding.** While the technology is powerful, it can produce incorrect or unexpected results. Please be aware:
> 
> - **Always verify search results** before making decisions based on them
> - **Double-check piped commands** and automated workflows, don't blindly pipe results to destructive operations like `rm` or `mv`
> - **Scout does not modify or delete media files**, it only reads them to generate embeddings. However, *you* are responsible for how you use the results
> - **Check the documentation** in [docs/](docs/) before reporting issues
> - **Test scripts on small datasets first** before running them on your entire media library
> 
> Scout is a semantic search tool, not a guarantee. Use it responsibly.

## Contributing

PRs welcome! See [docs/contributing.md](docs/contributing.md) for developer guide.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

---

Made with ☕ and Rust. If you have 10,000 photos named `IMG_XXXX.jpg`, this is for you.
