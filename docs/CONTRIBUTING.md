# Contributing to Scout

Thank you for your interest in contributing to Scout!

## Code of Conduct

Be respectful, inclusive, and constructive.

## Getting Started

### Development Setup

1. **Clone the repository:**
   ```bash
   git clone https://github.com/Hyphonical/Scout.git
   cd Scout
   ```

2. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. **Build the project:**
   ```bash
   cargo build
   ```

4. **Run tests:**
   ```bash
   cargo test
   ```

5. **Download models**

## Code Style

### Rust Conventions

Scout follows standard Rust conventions:

- Use `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Add documentation comments for public APIs
- Use descriptive variable names
- Keep functions focused and small

### Module Organization

```
src/
  cli.rs          # Command-line interface (clap)
  main.rs         # Entry point
  config.rs       # Constants and configuration
  
  core/           # Domain types
    embedding.rs  # Embedding vector type
    hash.rs       # File hashing (xxHash)
    media.rs      # Media type detection
  
  models/         # ML model management
    vision.rs     # Vision model (SigLIP2)
    text.rs       # Text model (SigLIP2)
    manager.rs    # Lazy loading, model lifecycle
  
  runtime/        # ONNX Runtime integration
    providers.rs  # Execution provider selection
  
  storage/        # Persistence layer
    sidecar.rs    # Sidecar format (MessagePack)
    index.rs      # Sidecar discovery
  
  processing/     # Media processing
    scan.rs       # Directory scanning
    image.rs      # Image loading and encoding
    video.rs      # FFmpeg integration
  
  commands/       # CLI command implementations
    scan.rs       # scan command
    search.rs     # search command
    clean.rs      # clean command
  
  ui/             # User interface
    log.rs        # Logging and output formatting
```

### Naming Conventions

- **Files:** `snake_case.rs`
- **Types:** `PascalCase`
- **Functions:** `snake_case()`
- **Constants:** `SCREAMING_SNAKE_CASE`
- **Modules:** `snake_case`

## How to Add Features

### Adding a New Command

1. **Create command file:**
   ```bash
   touch src/commands/my_command.rs
   ```

2. **Implement command:**
   ```rust
   //! My command - does something useful
   
   use anyhow::Result;
   use std::path::Path;
   use crate::ui;
   
   pub fn run(dir: &Path) -> Result<()> {
       ui::info("Running my command");
       // ... implementation
       Ok(())
   }
   ```

3. **Add to CLI (`src/cli.rs`):**
   ```rust
   #[derive(Subcommand)]
   pub enum Command {
       // ... existing commands
       
       /// My new command
       MyCommand {
           #[arg(short, long)]
           dir: PathBuf,
       },
   }
   ```

4. **Wire up in `src/main.rs`:**
   ```rust
   cli::Command::MyCommand { dir } => {
       commands::my_command::run(&dir)
   }
   ```

5. **Export in `src/commands/mod.rs`:**
   ```rust
   pub mod my_command;
   ```

### Adding Search Filters

To add a new search filter:

1. **Add CLI argument (`src/cli.rs`):**
   ```rust
   Search {
       // ... existing fields
       
       #[arg(long)]
       my_filter: bool,
   }
   ```

2. **Update search function signature (`src/commands/search.rs`):**
   ```rust
   pub fn run(
       // ... existing params
       my_filter: bool,
   ) -> Result<()>
   ```

3. **Implement filter logic:**
   ```rust
   if my_filter && !meets_condition(&result) {
       continue;
   }
   ```

4. **Update main.rs call:**
   ```rust
   cli::Command::Search { /* ... */, my_filter } => {
       commands::search::run(/* ... */, my_filter)
   }
   ```

### Adding a New Execution Provider

1. **Add to Cargo.toml** (platform-specific):
   ```toml
   [target.'cfg(target_os = "linux")'.dependencies]
   ort = { version = "2.0.0-rc.11", features = ["std", "my-provider"] }
   ```

2. **Add to Provider enum (`src/cli.rs`):**
   ```rust
   pub enum Provider {
       // ... existing
       MyProvider,
   }
   ```

3. **Add to macro (`src/runtime/providers.rs`):**
   ```rust
   create_provider_fn! {
       // ... existing
       (MyProvider, "MyProvider", my_provider),
   }
   ```

## Testing

### Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_embedding_similarity

# With output
cargo test -- --nocapture
```

### Adding Tests

Add tests in the same file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_my_feature() {
        let result = my_function();
        assert_eq!(result, expected);
    }
}
```

Or in a separate `tests/` directory for integration tests.

### Test Coverage

Focus on:
- Core logic (embeddings, similarity)
- Data structures (serialization)
- Edge cases (empty inputs, invalid data)
- Error handling

## Documentation

### Code Documentation

Use doc comments:

```rust
/// Computes similarity between two embeddings.
///
/// Returns a score between 0.0 (completely different) and 1.0 (identical).
///
/// # Example
///
/// ```
/// let e1 = Embedding::new(vec![1.0, 0.0]);
/// let e2 = Embedding::new(vec![0.0, 1.0]);
/// let sim = e1.similarity(&e2);
/// assert!(sim >= 0.0 && sim <= 1.0);
/// ```
pub fn similarity(&self, other: &Embedding) -> f32 {
    // ...
}
```

### User Documentation

Update docs when adding features:
- `docs/ARCHITECTURE.md` - Technical details
- `README.md` - Quick reference

## Pull Request Process

1. **Fork the repository**

2. **Create a feature branch:**
   ```bash
   git checkout -b feature/my-feature
   ```

3. **Make your changes:**
   - Write clear, focused commits
   - Add tests for new features
   - Update documentation

4. **Verify quality:**
   ```bash
   cargo fmt
   cargo clippy
   cargo test
   cargo build --release
   ```

5. **Push and create PR:**
   ```bash
   git push origin feature/my-feature
   ```
   
   Then create PR on GitHub with:
   - Clear description of changes
   - Motivation and context
   - Testing done
   - Screenshots (if UI changes)

6. **Address review feedback**

7. **Merge!** (after approval)

## Code Review Guidelines

### For Authors

- Keep PRs focused and small
- Write clear commit messages
- Respond to feedback promptly
- Be open to suggestions

### For Reviewers

- Be constructive and specific
- Focus on correctness, clarity, maintainability
- Suggest improvements, don't demand perfection
- Approve when ready

## Common Tasks

### Adding a Dependency

1. Add to `Cargo.toml`:
   ```toml
   [dependencies]
   my-crate = "1.0"
   ```

2. Use in code:
   ```rust
   use my_crate::Thing;
   ```

3. Document why it's needed in PR

### Updating Models

Models are external artifacts. To update:

1. Train/export new ONNX models
2. Update model version in docs
3. Test thoroughly
4. Create release with new models

### Profiling Performance

```bash
# Build with profiling symbols
cargo build --release

# Run with profiler
perf record ./target/release/scout scan -d photos/
perf report

# Or use flamegraph
cargo install flamegraph
cargo flamegraph -- scan -d photos/
```

### Debugging

```bash
# Verbose logging
scout --verbose scan -d photos/

# Rust backtrace
RUST_BACKTRACE=1 scout scan -d photos/

# Debug build (with symbols)
cargo build
gdb ./target/debug/scout
```

## Release Process

1. **Update version** in `Cargo.toml`
2. **Update CHANGELOG.md**
3. **Tag release:**
   ```bash
   git tag -a v2.1.0 -m "Release v2.1.0"
   git push origin v2.1.0
   ```
4. **Build binaries** for all platforms
5. **Create GitHub Release** with notes and binaries

## Questions?

- Open an issue for bugs or feature requests
- Start a discussion for questions or ideas

Thank you for contributing! 🎉
