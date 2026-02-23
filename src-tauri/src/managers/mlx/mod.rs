//! MLX Local AI Model Manager for Apple Silicon Macs
//!
//! This module provides local LLM inference using Apple's MLX framework
//! via a Python sidecar process running mlx-lm.

// Model manager
pub mod catalog;
pub mod downloader;
pub mod manager;
pub mod provider;

// Re-export manager types
pub use manager::MlxModelManager;
pub use manager::MlxModelInfo;
