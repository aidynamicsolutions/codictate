pub mod audio;
pub mod correction;
pub mod history;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod mlx;
pub mod model;
pub mod transcription;
