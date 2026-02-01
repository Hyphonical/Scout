# Scout User Guide

Complete guide to using Scout's features.

## Table of Contents

- [Commands](#commands)
- [Search Techniques](#search-techniques)
- [Filtering](#filtering)
- [Configuration](#configuration)
- [GPU Acceleration](#gpu-acceleration)
- [Video Support](#video-support)
- [Tips & Best Practices](#tips--best-practices)

---

## Commands

### `scan` - Index Files

Index images and videos in a directory.

```bash
scout scan [OPTIONS]
```

**Options:**
- `-d, --dir <DIR>` - Directory to scan (default: current directory)
- `-r, --recursive` - Scan subdirectories
- `-f, --force` - Re-index already indexed files
- `--min-resolution <PIXELS>` - Skip images smaller than this (shortest side)
- `--max-size <MB>` - Skip files larger than this
- `--exclude-videos` - Skip video files

**Examples:**

```bash
# Index current directory
scout scan

# Recursive scan
scout scan -d ~/Pictures -r

# Force re-index (e.g., after model update)
scout scan -d photos/ -f

# Filter by resolution and size
scout scan -r --min-resolution 512 --max-size 50

# Images only, no videos
scout scan -r --exclude-videos
```

**What happens during scan:**
1. Scout walks the directory tree
2. For each image:
   - Computes file hash (xxHash)
   - Checks if sidecar exists and is current
   - Loads image and generates embedding
   - Saves `.scout/<hash>.msgpack` sidecar
3. For videos (if FFmpeg available):
   - Extracts 10 evenly-spaced frames
   - Generates embeddings for each frame
   - Saves multi-frame sidecar

### `search` - Find Similar Files

Search indexed files by description or reference image.

```bash
scout search [QUERY] [OPTIONS]
```

**Options:**
- `[QUERY]` - Text description (optional if using `--image`)
- `-i, --image <PATH>` - Reference image for similarity search
- `-w, --weight <0.0-1.0>` - Text weight in combined search (default: 0.5)
- `--not <QUERY>` - Negative prompt to exclude
- `-d, --dir <DIR>` - Search directory (default: current)
- `-r, --recursive` - Search subdirectories
- `-n, --limit <NUM>` - Max results (default: 10)
- `-s, --score <0.0-1.0>` - Minimum similarity score (default: 0.0)
- `-o, --open` - Open first result
- `--include-ref` - Include reference image in results
- `--exclude-videos` - Exclude videos from results

**Examples:**

```bash
# Text search
scout search "sunset over ocean"

# Image similarity search
scout search -i reference.jpg

# Combined search (text + image)
scout search "red sports car" -i car.jpg -w 0.7

# Negative prompt
scout search "forest" --not "people"

# Filter and limit
scout search "cat" -n 5 -s 0.3

# Open top result
scout search "mountain landscape" -o
```

### `repl` - Interactive Mode

Start an interactive search session.

```bash
scout repl [OPTIONS]
```

**Options:**
- `-d, --dir <DIR>` - Search directory
- `-r, --recursive` - Include subdirectories
- `-n, --limit <NUM>` - Max results per query
- `-s, --score <0.0-1.0>` - Minimum score
- `--exclude-videos` - No videos

**Example session:**

```bash
$ scout repl -d ~/Photos

ℹ Starting interactive search mode
ℹ Type your queries, or 'exit' to quit

✓ Loaded 1,247 indexed files

scout> beautiful sunset
 1. vacation_2024/IMG_5432.jpg 89% 🔥
 2. landscapes/golden_hour.png 82% 🔥
 3. beach_trip/sunset_1.jpg 76%
✓ Found 3 matches in 18ms

scout> dog playing
 1. pets/charlie_park.jpg 91% 🔥
 2. family/backyard_fun.png 73%
✓ Found 2 matches in 15ms

scout> help
REPL Commands:
  <text>  Enter a search query
  help    Show this help message
  exit    Exit REPL mode

scout> exit
ℹ Goodbye!
```

**Why use REPL:**
- **Faster** - Models loaded once, index cached
- **Interactive** - Refine queries without retyping options
- **Efficient** - No startup overhead per search

### `clean` - Remove Orphaned Sidecars

Remove `.scout` sidecars for deleted/moved files.

```bash
scout clean [OPTIONS]
```

**Options:**
- `-d, --dir <DIR>` - Directory to clean
- `-r, --recursive` - Clean subdirectories

**Example:**

```bash
scout clean -d photos/ -r
```

---

## Search Techniques

### Text Search

Use natural language descriptions:

```bash
# Good queries
scout search "woman with red hair sitting on bench"
scout search "mountain landscape with snow peaks"
scout search "modern architecture glass building"

# Less effective
scout search "IMG_4523"  # filenames don't work
scout search "photo"     # too generic
```

**Tips:**
- Use complete sentences or detailed phrases
- Be specific about colors, objects, settings
- Prefix with "Image of..." or "Photo of..." for better results

### Image Similarity Search

Find images similar to a reference:

```bash
scout search -i reference.jpg -n 20
```

Use cases:
- Find duplicates or near-duplicates
- Find variations of a scene
- Organize similar photos

### Combined Search (Text + Image)

Blend text and image queries:

```bash
scout search "red car" -i sports_car.jpg -w 0.7
```

The `--weight` parameter controls the blend:
- `0.0` - Pure image search
- `0.5` - Equal blend (default)
- `1.0` - Pure text search

**Example use case:**
You have a photo of a car and want to find similar red cars:
```bash
scout search "red sports car" -i my_car.jpg -w 0.6
```

### Negative Prompts

Exclude unwanted content:

```bash
scout search "landscape" --not "people"
scout search "food" --not "meat, dairy"
```

How it works:
- Encodes negative prompt
- Penalizes matches with high negative similarity
- Weight: 70% penalty (configurable in code)

---

## Filtering

### Resolution Filter

Skip small images during scan:

```bash
scout scan -r --min-resolution 1024
```

Useful for:
- Ignoring thumbnails
- Focusing on high-quality images
- Faster indexing

### Size Filter

Skip large files:

```bash
scout scan -r --max-size 50  # Max 50 MB
```

### Score Threshold

Only show high-confidence matches:

```bash
scout search "query" -s 0.3  # Minimum 30% similarity
```

### Exclude Videos

Search images only:

```bash
scout search "sunset" --exclude-videos
```

Or scan images only:

```bash
scout scan -r --exclude-videos
```

---

## Configuration

### Model Paths

Three ways to specify model location:

**1. Default location (next to executable):**
```
scout.exe
models/
  vision_model_q4f16.onnx
  text_model_q4f16.onnx
  tokenizer.json
```

**2. Environment variable:**
```bash
export SCOUT_MODELS_DIR=/path/to/models
scout scan -d photos/
```

**3. Command-line flag:**
```bash
scout --model-dir ./models scan -d photos/
```

### FFmpeg Path

For video support, Scout uses FFmpeg from PATH. To specify a custom location:

```bash
scout --ffmpeg-path /usr/local/bin/ffmpeg scan -d videos/
```

### Ignore Files

Create `.scoutignore` in any directory:

```
# Skip these patterns
temp/
*.tmp
cache/*
node_modules/
```

Syntax same as `.gitignore`.

---

## GPU Acceleration

Scout supports multiple execution providers for faster inference.

### Available Providers

- `auto` - Automatically select best available (default)
- `cpu` - CPU only (slowest, most compatible)
- `cuda` - NVIDIA GPU (CUDA 11+)
- `tensorrt` - NVIDIA GPU (optimized)
- `coreml` - Apple Silicon (M1/M2/M3)
- `xnnpack` - CPU optimized (ARM/x86)

### Usage

```bash
scout --provider cuda scan -d photos/
```

### Performance Comparison

| Provider | Speed | Requirements |
|----------|-------|--------------|
| CPU | 1x (baseline) | Always available |
| XNNPACK | 2-3x | Modern CPU |
| CUDA | 5-10x | NVIDIA GPU, CUDA 11+ |
| TensorRT | 8-15x | NVIDIA GPU, TensorRT |
| CoreML | 5-8x | Apple Silicon |

### Troubleshooting

**"Provider not available":**
- Install required drivers (CUDA, TensorRT)
- Check GPU compatibility
- Fall back to `--provider cpu`

---

## Video Support

Scout can search within videos by extracting and encoding frames.

### Requirements

- FFmpeg installed and in PATH
- Or specify path: `--ffmpeg-path`

### Installation

**Windows:**
```bash
winget install FFmpeg
```

**macOS:**
```bash
brew install ffmpeg
```

**Linux:**
```bash
sudo apt install ffmpeg  # Debian/Ubuntu
sudo dnf install ffmpeg  # Fedora
```

### How It Works

1. FFmpeg extracts 10 evenly-spaced frames
2. Each frame encoded separately
3. Search finds best-matching frame
4. Results show timestamp (MM:SS)

### Example

```bash
scout scan -d videos/ -r
scout search "person walking in park"
```

Output:
```
Results
 1. vacation.mp4 @ 01:23 87% 🔥
 2. family_trip.mkv @ 00:45 76%
```

### Disable Videos

If you don't want video support:

```bash
scout scan -r --exclude-videos
scout search "query" --exclude-videos
```

---

## Tips & Best Practices

### Scan Optimization

**Re-scan only when needed:**
- After model updates: `scout scan -f`
- After adding new files: `scout scan` (skips indexed)
- Outdated sidecars automatically detected

**Filter effectively:**
```bash
# Skip small thumbnails and huge RAW files
scout scan -r --min-resolution 512 --max-size 100
```

### Search Optimization

**Start generic, then refine:**
```bash
scout search "landscape"
scout search "mountain landscape with lake"
scout search "mountain landscape with lake sunset"
```

**Use REPL for exploration:**
```bash
scout repl -d photos/
# Try different queries quickly
```

### Storage Management

**Sidecar size:**
- Image sidecar: ~5 KB
- Video sidecar: ~50 KB (10 frames)
- Total: ~0.5% of original media size

**Clean orphaned sidecars:**
```bash
scout clean -r
```

### Query Quality

**Good queries:**
- "Woman with red hair wearing blue dress"
- "Modern office interior with plants"
- "Golden retriever playing in snow"

**Poor queries:**
- "nice" (too vague)
- "IMG_4523" (filenames don't work)
- "photo" (too generic)

### Performance Tips

1. **Use GPU** for large collections (1000+ files)
2. **Use REPL** for multiple searches
3. **Filter during scan** to reduce index size
4. **Exclude videos** if not needed (faster scans)

### Troubleshooting

**"No matches found":**
- Lower score threshold: `-s 0`
- Try different query phrasing
- Check if files are indexed: `ls .scout/`

**Slow search:**
- Use GPU: `--provider cuda`
- Use REPL mode (cache models)
- Reduce search scope: `-d specific/folder`

**"Vision model not found":**
- Check model location
- Use `--model-dir` to specify path
- Set `SCOUT_MODELS_DIR` environment variable
