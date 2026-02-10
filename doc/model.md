# Model Management

The **Models** page in Settings represents a unified hub for managing all AI models used by Handy. This includes both the speech-to-text (ASR) models for transcription and the text-to-text (LLM) models for post-processing.

## Transcription Models

These models convert your spoken audio into text. Handy supports a variety of Whisper and Parakeet models to balance speed and accuracy.

### Features

-   **Grouped View**: Models are organized into "Downloaded" (ready to use) and "Available" (can be downloaded).
-   **Collapsible Sections**: To keep the UI clean, lists of models are collapsible. The active model is always visible.
-   **Language Filtering**: A dropdown menu allows you to filter models based on the languages they support. This helps you quickly find the right model for your needs (e.g., finding models that support "French").
-   **Performance Metrics**: Each card displays:
    -   **Accuracy**: How well the model understands speech (higher is better).
    -   **Speed**: How fast the model transcribes (higher is faster).
    -   **Size**: Disk space required.

### Actions

-   **Select**: Click a downloaded model to make it the active transcriber.
-   **Download**: Click the download icon on an available model to start downloading.
-   **Delete**: Remove a downloaded model to free up disk space. The active model cannot be deleted until another is selected.

## Language Models (LLMs)

*Note: This section is currently available on Apple Silicon Macs.*

These use local Large Language Models (LLMs) to perform post-processing tasks on your transcriptions, such as cleaning up grammar, summarizing, or translating. These models run entirely on-device using the [MLX framework](https://github.com/ml-explore/mlx).

### Features

-   **Local Execution**: Runs 100% offline for privacy and speed.
-   **Optimized Models**: Models (like Gemma 2 2b) are 4-bit quantized for efficient inference on Apple Silicon.
-   **Download Management**:
    -   Real-time download progress with speed and ETA.
    -   ability to cancel and retry downloads.
-   **Integration**: This section is linked directly from the **Post Processing** settings page. If you select "Local (MLX)" as your provider but don't have a model, you'll be redirected here to download one.
-   **Finder Integration**: Downloaded models can be revealed in Finder for manual inspection.

## Technical Details

-   **Storage**: Models are stored in the application's data directory (e.g., `~/Library/Application Support/com.pais.codictate/models/` on macOS).
-   **Format**:
    -   Transcription models use `ggml` or `bin` formats compatible with `whisper.cpp`.
    -   Language models use `mlx` format directories.
