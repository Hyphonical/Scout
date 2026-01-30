# Scout Feature Checklist

## Core Features
- [ ] Text-based image search (semantic)
- [ ] Image-based search (reverse image search)
- [ ] Hybrid search (text + image combined with weight)
- [ ] Video frame extraction and indexing
- [ ] Recursive directory scanning
- [ ] Sidecar-based embedding storage (.scout files)

## Search Features
- [ ] Similarity scoring (cosine similarity)
- [ ] Result limit parameter
- [ ] Minimum score threshold filtering
- [ ] Auto-open best result (`--open`)
- [ ] Include reference image in results (`--include-ref`)

## Scanning Features
- [ ] Force re-indexing (`--force`)
- [ ] Minimum width/height filtering
- [ ] Minimum file size filtering (KB)
- [ ] Maximum file size filtering (MB)
- [ ] Exclude patterns (glob patterns)
- [ ] Outdated sidecar upgrade detection
- [ ] File hash verification

## Video Features
- [ ] Video file support (mp4, avi, mkv, mov, etc.)
- [ ] Frame extraction (N evenly-spaced frames)
- [ ] Timestamp tracking for video search results
- [ ] FFmpeg runtime detection
- [ ] Video disable flag (`--disable-video`)

## UI Features
- [ ] Live interactive TUI mode (ratatui)
- [ ] Real-time search-as-you-type
- [ ] Verbose logging mode
- [ ] Colored output
- [ ] Hyperlinked file paths (terminal support)
- [ ] Progress indicators
- [ ] Error reporting with context

## Model/Runtime Features
- [ ] ONNX Runtime integration
- [ ] Auto provider detection (CPU/GPU)
- [ ] Manual provider selection (cpu, xnnpack, cuda, tensorrt, coreml)
- [ ] SigLIP2 vision-language model
- [ ] Batch image processing
- [ ] Model embedding caching

## Utilities
- [ ] Clean command (remove orphaned sidecars)
- [ ] Help command with subcommand help
- [ ] Version information
- [ ] Custom CLI styling (colors)

## File Format Support
### Images
- [ ] JPEG/JPG
- [ ] PNG
- [ ] WebP
- [ ] GIF
- [ ] BMP
- [ ] TIFF

### Videos
- [ ] MP4
- [ ] AVI
- [ ] MKV
- [ ] MOV
- [ ] WMV
- [ ] FLV
- [ ] WebM

## Platform Support
- [ ] Windows (x86_64)
- [ ] Linux (x86_64)
- [ ] macOS (x86_64, aarch64)

## Code Organization
- [ ] cli.rs - Command-line interface
- [ ] config.rs - Configuration handling
- [ ] live.rs - TUI interactive mode
- [ ] logger.rs - Logging and output formatting
- [ ] models.rs - ML model loading and inference
- [ ] runtime.rs - ONNX Runtime provider management
- [ ] scanner.rs - Directory scanning and filtering
- [ ] search.rs - Search logic and ranking
- [ ] sidecar.rs - Embedding storage/loading
- [ ] types.rs - Shared types and enums
- [ ] video.rs - Video frame extraction

## Bloat / Consider Removal
- [ ] Live TUI mode (complex, possibly unnecessary)
- [ ] Custom CLI styling (standard clap is fine)
- [ ] Hyperlink support (not all terminals support it)
- [ ] Multiple execution providers (just auto + cpu?)
- [ ] Exclude patterns (rarely used?)
- [ ] Force re-index (just delete sidecars?)
- [ ] Include reference image flag (niche use case)
