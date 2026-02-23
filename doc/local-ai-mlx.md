# Local AI with MLX

Codictate supports **on-device AI refinement** using Apple's MLX framework on Apple Silicon Macs. This feature allows you to enhance transcriptions locally without sending data to external APIs.

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
│                             REFINE FLOW                                       │
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
          │  │ HTTP over localhost:11400-11500│     │
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
          │           "max_tokens": -1,             │  (-1 = auto-calculate)
          │           "system_ram_gb": 16,          │
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
| **Qwen 3 4B Instruct (2507)** | ~2.26 GB | ~2 GB min, ~4-5 GB typical | 16GB Macs (recommended 9-16GB) |
| **Qwen 3 Base 8B** | ~4.7 GB | ~7-8 GB | 24GB+ Macs (recommended >16GB) |
| Gemma 3 Base 1B | ~0.8 GB | ~1 GB | Multi-language support |
| Gemma 3 Base 4B | ~2.3 GB | ~3 GB | Strong multi-language |
| SmolLM 3 Base 3B | ~1.8 GB | ~2 GB | HuggingFace alternative |

**Recommendation is automatic based on system RAM.**

> Note: Official Qwen `2507` updates are available for 4B (and larger sparse variants), but not currently for 0.6B/1.7B/8B.

## Generation Settings

The sidecar uses optimized settings for translation/enhancement tasks:

| Setting | Value | Purpose |
|---------|-------|---------|
| `max_tokens` | Dynamic | Auto-calculated: input×1.3, capped by RAM tier |
| `temperature` | 0.7 | Balanced creativity/consistency |
| `top_p` | 0.8 | Nucleus sampling for quality |
| `repetition_penalty` | 1.15 | Prevent output loops |
| `enable_thinking` | false | Disable verbose reasoning |

### Dynamic Token Limits by RAM

| RAM | Token Ceiling | Max Recording |
|-----|--------------|---------------|
| ≤8GB | 1536 | ~6 min |
| 9-16GB | 2048 | ~8 min |
| >16GB | 3072 | ~12 min |

### Recording Time Limits

Recordings are automatically limited based on system RAM to prevent memory exhaustion:

- **Visual countdown:** Pink border depletes clockwise during recording
- **30s warning:** Toast notification before auto-stop
- **Auto-stop:** Recording stops and proceeds to transcription

| RAM | Max Duration |
|-----|--------------|
| ≤8GB | 6 minutes |
| 9-16GB | 8 minutes |
| >16GB | 12 minutes |

## Key Files

```
Codictate/
├── python-backend/
│   └── server.py              # FastAPI sidecar (mlx-lm)
├── src-tauri/
│   ├── binaries/
│   │   └── uv-aarch64-apple-darwin  # Bundled Python manager
│   └── src/
│       ├── managers/
│       │   └── mlx/
│       │       ├── catalog.rs       # Canonical IDs, aliases, model sources
│       │       ├── downloader.rs    # Source dispatch (HF now, mirror later)
│       │       ├── provider.rs      # Catalog provider abstraction
│       │       └── manager.rs       # Rust MLX manager runtime
│       └── actions.rs               # Integration point
└── src/
    ├── components/settings/
    │   └── MlxModelSelector.tsx     # UI component
    └── hooks/
        └── useMlxModels.ts          # React state hook
```

## How It Works

1. **Model Selection**: User selects a model in Settings → Refine → Local (MLX)
2. **Disk Space Check**: System verifies sufficient space (model size + 100MB buffer)
3. **Model Download**: Model files are downloaded directly from Hugging Face Hub (~1-5 GB)
   - Public, ungated models download without requiring a user API key/token.
   - Gated/private models require Hugging Face authentication.
4. **Sidecar Startup**: On first use, `uv` spawns the Python sidecar on an available port (11400-11500)
5. **Model Loading**: Model loads into GPU memory (~5-15 seconds)
6. **Text Processing**: Transcriptions are enhanced via the sidecar
7. **Auto-Unload**: Model unloads after idle timeout to free memory
8. **Graceful Shutdown**: Sidecar receives SIGTERM on app exit

## Troubleshooting

### Sidecar won't start
- Ports 11400-11500 are scanned automatically; ensure at least one is available
- Ensure `uv` binary exists in `src-tauri/binaries/`
- Check console for Python errors

### "Insufficient disk space" error
- Model requires download size + 100MB buffer
- Free up disk space or choose a smaller model

### Model output is verbose/rambling
- Ensure chat template is applied (`enable_thinking=False`)
- Verify dynamic `max_tokens` is enabled (`-1` sentinel passed by Rust sidecar client)
- Verify repetition penalty is active

### Generation is slow
- First inference includes model loading (5-15s)
- Subsequent inferences are much faster
- Consider using a smaller model

## Memory Management

On model unload, the sidecar performs:
1. `gc.collect()` — Free Python objects
2. `mx.clear_cache()` — Release Metal GPU buffers

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
