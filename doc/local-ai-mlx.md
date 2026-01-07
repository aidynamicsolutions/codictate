# Local AI with MLX

Handy supports **on-device AI post-processing** using Apple's MLX framework on Apple Silicon Macs. This feature allows you to enhance transcriptions locally without sending data to external APIs.

## Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              HANDY APPLICATION                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐    ┌──────────────────┐    ┌──────────────────────────┐  │
│  │   Frontend   │───▶│   Tauri Backend  │───▶│    Python Sidecar        │  │
│  │  (React/TS)  │    │      (Rust)      │    │    (FastAPI + mlx-lm)    │  │
│  └──────────────┘    └──────────────────┘    └──────────────────────────┘  │
│         │                    │                          │                   │
│         ▼                    ▼                          ▼                   │
│  ┌──────────────┐    ┌──────────────────┐    ┌──────────────────────────┐  │
│  │ Model Select │    │ MlxModelManager  │    │   Qwen3 via mlx-lm      │  │
│  │ Download UI  │    │ HTTP Client      │    │   Chat Template         │  │
│  │ Progress Bar │    │ Sidecar Mgmt     │    │   Text Generation       │  │
│  └──────────────┘    └──────────────────┘    └──────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Features

- **Zero API Costs** — Run AI locally, no external API keys needed
- **Privacy First** — Transcriptions never leave your device
- **Offline Capable** — Works without internet after model download
- **Apple Silicon Optimized** — Uses Metal GPU acceleration via MLX

## Requirements

- macOS on Apple Silicon (M1/M2/M3/M4)
- ~1-5 GB disk space depending on model choice
- 8GB+ RAM recommended

## Architecture

### Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           POST-PROCESSING FLOW                               │
└─────────────────────────────────────────────────────────────────────────────┘

  Speech Input          Transcription         Post-Processing        Output
       │                     │                      │                  │
       ▼                     ▼                      ▼                  ▼
  ┌─────────┐         ┌───────────┐         ┌─────────────┐      ┌─────────┐
  │  Audio  │────────▶│  Whisper  │────────▶│  MLX Local  │─────▶│ Enhanced│
  │ Recording│         │Transcribe │         │     AI      │      │  Text   │
  └─────────┘         └───────────┘         └─────────────┘      └─────────┘
                                                   │
                                                   ▼
                                            ┌─────────────┐
                                            │   Sidecar   │
                                            │ (Python)    │
                                            │             │
                                            │ • Load model│
                                            │ • Generate  │
                                            │ • Cleanup   │
                                            └─────────────┘
```

### Python Sidecar Architecture

The MLX inference runs in a separate Python process for best model compatibility:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          SIDECAR COMMUNICATION                               │
└─────────────────────────────────────────────────────────────────────────────┘

     Rust Backend                              Python Sidecar
    (MlxModelManager)                         (FastAPI Server)
          │                                         │
          │  ┌───────────────────────────────┐      │
          │  │  HTTP over localhost:5000-5100│      │
          │  └───────────────────────────────┘      │
          │                                         │
          ├──────────GET /status───────────────────▶│  Health check
          │◀─────────{"status": "running"}──────────┤
          │                                         │
          ├──────────POST /load────────────────────▶│  Load model
          │          {"model_path": "..."}          │  (5-15s first time)
          │◀─────────{"status": "loaded"}───────────┤
          │                                         │
          ├──────────POST /generate────────────────▶│  Generate text
          │          {"prompt": "...",              │
          │           "max_tokens": 150,            │
          │           "temperature": 0.7}           │
          │◀─────────{"response": "..."}────────────┤
          │                                         │
          ├──────────POST /unload──────────────────▶│  Free memory
          │◀─────────{"status": "unloaded"}─────────┤
          │                                         │
```

### Model Lifecycle

```
                         ┌──────────────────┐
                         │  Not Downloaded  │
                         └────────┬─────────┘
                                  │ User clicks "Download"
                                  ▼
                         ┌──────────────────┐
                    ┌───▶│   Downloading    │───┐
                    │    └────────┬─────────┘   │ Failed/Cancelled
                    │             │ Complete    │
                    │             ▼             ▼
                    │    ┌──────────────────┐  ┌──────────────────┐
                    │    │    Downloaded    │◀─│  Download Failed │
           Delete   │    └────────┬─────────┘  └──────────────────┘
                    │             │ First inference
                    │             ▼ requested
                    │    ┌──────────────────┐
                    │    │     Loading      │
                    │    └────────┬─────────┘
                    │             │ Loaded to GPU
                    │             ▼
                    │    ┌──────────────────┐
                    └────│      Ready       │◀──────────────────────┐
                         └────────┬─────────┘                       │
                                  │ Idle timeout                    │
                                  ▼                                 │
                         ┌──────────────────┐  Next inference       │
                         │    Unloaded      │───────────────────────┘
                         └──────────────────┘
```

## Supported Models

All models are **4-bit quantized** for efficient Apple Silicon inference.

| Model | Download | RAM Usage | Best For |
|-------|----------|-----------|----------|
| Qwen 3 Base 0.6B | ~0.4 GB | ~1 GB | Ultra-fast, simple corrections |
| **Qwen 3 Base 1.7B** | ~1.0 GB | ~2-3 GB | 8GB Macs (recommended ≤8GB) |
| **Qwen 3 Base 4B** | ~2.3 GB | ~4-5 GB | 16GB Macs (recommended 9-16GB) |
| **Qwen 3 Base 8B** | ~4.7 GB | ~7-8 GB | 24GB+ Macs (recommended >16GB) |
| Gemma 3 Base 1B | ~0.8 GB | ~1 GB | Multi-language support |
| Gemma 3 Base 4B | ~2.3 GB | ~3 GB | Strong multi-language |
| SmolLM 3 Base 3B | ~1.8 GB | ~2 GB | HuggingFace alternative |

**Recommendation is automatic based on system RAM.**

## Generation Settings

The sidecar uses optimized settings for translation/enhancement tasks:

| Setting | Value | Purpose |
|---------|-------|---------|
| `max_tokens` | 150 | Limit output length for efficiency |
| `temperature` | 0.7 | Balanced creativity/consistency |
| `top_p` | 0.8 | Nucleus sampling for quality |
| `repetition_penalty` | 1.15 | Prevent output loops |
| `enable_thinking` | false | Disable verbose reasoning |

## Key Files

```
Handy/
├── python-backend/
│   └── server.py              # FastAPI sidecar (mlx-lm)
├── src-tauri/
│   ├── binaries/
│   │   └── uv-aarch64-apple-darwin  # Bundled Python manager
│   └── src/
│       ├── managers/
│       │   └── mlx/
│       │       └── manager.rs       # Rust MLX manager
│       └── actions.rs               # Integration point
└── src/
    ├── components/settings/
    │   └── MlxModelSelector.tsx     # UI component
    └── hooks/
        └── useMlxModels.ts          # React state hook
```

## How It Works

1. **Model Selection**: User selects a model in Settings → Post-Processing → Local (MLX)
2. **Disk Space Check**: System verifies sufficient space (model size + 100MB buffer)
3. **Model Download**: Model files are downloaded from Hugging Face Hub (~1-5 GB)
4. **Sidecar Startup**: On first use, `uv` spawns the Python sidecar on an available port (5000-5100)
5. **Model Loading**: Model loads into GPU memory (~5-15 seconds)
6. **Text Processing**: Transcriptions are enhanced via the sidecar
7. **Auto-Unload**: Model unloads after idle timeout to free memory
8. **Graceful Shutdown**: Sidecar receives SIGTERM on app exit

## Troubleshooting

### Sidecar won't start
- Ports 5000-5100 are scanned automatically; ensure at least one is available
- Ensure `uv` binary exists in `src-tauri/binaries/`
- Check console for Python errors

### "Insufficient disk space" error
- Model requires download size + 100MB buffer
- Free up disk space or choose a smaller model

### Model output is verbose/rambling
- Ensure chat template is applied (`enable_thinking=False`)
- Check `max_tokens` is set to 150
- Verify repetition penalty is active

### Generation is slow
- First inference includes model loading (5-15s)
- Subsequent inferences are much faster
- Consider using a smaller model

## Memory Management

On model unload, the sidecar performs:
1. `gc.collect()` — Free Python objects
2. `mx.metal.clear_cache()` — Release Metal GPU buffers

This ensures GPU memory is fully released when the model is unloaded.

## Platform Support

| Platform | Support |
|----------|---------|
| macOS Apple Silicon | ✅ Full support |
| macOS Intel | ❌ Not supported (no MLX) |
| Windows | ❌ Not supported |
| Linux | ❌ Not supported |

---

*MLX Local AI is only available on Apple Silicon Macs (M1/M2/M3/M4).*
