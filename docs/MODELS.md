# Model Configuration

Scout uses SigLIP2 vision-language models for semantic image search. Download these three files and place them in the `models/` directory:

## Recommended (Large, Q4F16)
**Tested and optimized for Scout:**
- [vision_model_q4f16.onnx](https://huggingface.co/onnx-community/siglip2-large-patch16-512-ONNX/resolve/main/onnx/vision_model_q4f16.onnx) (174.8MB)
- [text_model_q4f16.onnx](https://huggingface.co/onnx-community/siglip2-large-patch16-512-ONNX/resolve/main/onnx/text_model_q4f16.onnx) (665.2MB)
- [tokenizer.json](https://huggingface.co/onnx-community/siglip2-large-patch16-512-ONNX/resolve/main/tokenizer.json) (32.7MB)

## Alternative Models

### Base (Lower specs)
For systems with limited RAM or GPU memory:
- [siglip2-base-patch16-512-ONNX](https://huggingface.co/onnx-community/siglip2-base-patch16-512-ONNX)
- Uses same config values (INPUT_SIZE: 512, EMBEDDING_DIM: 1024). Change only model file names.
- Faster inference, slightly lower accuracy

### Giant (High performance)
For powerful systems requiring maximum accuracy:
- [siglip2-giant-opt-patch16-384-ONNX](https://huggingface.co/onnx-community/siglip2-giant-opt-patch16-384-ONNX)
- **Untested** - may require config changes (INPUT_SIZE: 384, EMBEDDING_DIM: TBD)
- Check model card for exact dimensions

## Quantization Notes
- **Q4F16** (Recommended): 4-bit weights + FP16 activations - High performance, greater accuracy loss
- **INT8**: 8-bit quantization - good balance
- **FP16**: Half precision - smaller than FP32, faster on modern GPUs
- **FP32**: Full precision - largest, slowest, highest accuracy

Always use matching quantization for both vision and text models. E.g., if using Q4F16 for vision, use Q4F16 for text.

## Config Updates
If using non-large models, update `src/config.rs`:
```rust
pub const INPUT_SIZE: u32 = 512;  // Or 384 for giant
pub const EMBEDDING_DIM: usize = 1024;  // Check model card

pub const VISION_MODEL: &str = "vision_model_q4f16.onnx";
pub const TEXT_MODEL: &str = "text_model_q4f16.onnx";
pub const TOKENIZER: &str = "tokenizer.json";
```

## Performance Comparison

| Format        | File Size (Relative) | Accuracy     | Best Hardware                          |
|---------------|----------------------|--------------|----------------------------------------|
| FP32          | 100% (Huge)          | 100%         | Legacy Systems / Debugging             |
| FP16 / BF16   | 50%                  | ~99.9%       | NVIDIA GPUs, Apple Silicon             |
| INT8          | 25%                  | ~98–99%      | Intel/AMD CPUs, Edge Devices           |
| Q4F16         | ~15–20%              | ~95–97%      | Consumer GPUs, MacBooks, Mobile        |