use std::error::Error;
use std::fmt::{Display, Formatter};

use super::catalog::{MlxCatalogModel, ModelSource, ModelSourceKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadSourceTarget {
    HuggingFaceRepo(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceDispatchErrorKind {
    Disabled,
    NotConfigured,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDispatchError {
    pub kind: SourceDispatchErrorKind,
    pub allow_fallback: bool,
    pub message: String,
}

impl SourceDispatchError {
    /// Disabled sources always allow fallback; not-configured sources only do
    /// when explicitly marked with `allow_fallback`.
    pub fn can_fallback(&self) -> bool {
        self.allow_fallback || self.kind != SourceDispatchErrorKind::NotConfigured
    }
}

impl Display for SourceDispatchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for SourceDispatchError {}

pub struct ModelDownloader;

impl ModelDownloader {
    pub fn ordered_sources(model: &MlxCatalogModel) -> Vec<&ModelSource> {
        let mut sources: Vec<&ModelSource> = model.sources.iter().collect();
        sources.sort_by_key(|source| source.priority);
        sources
    }

    pub fn dispatch_download_target(
        source: &ModelSource,
    ) -> Result<DownloadSourceTarget, SourceDispatchError> {
        if !source.enabled {
            return Err(SourceDispatchError {
                kind: SourceDispatchErrorKind::Disabled,
                allow_fallback: true,
                message: "Download source is disabled".to_string(),
            });
        }

        match source.kind {
            ModelSourceKind::HuggingfaceRepo => {
                if source.value.trim().is_empty() {
                    return Err(SourceDispatchError {
                        kind: SourceDispatchErrorKind::NotConfigured,
                        allow_fallback: source.allow_fallback,
                        message: "Hugging Face source is missing repo value".to_string(),
                    });
                }

                Ok(DownloadSourceTarget::HuggingFaceRepo(source.value.clone()))
            }
            ModelSourceKind::HttpMirror => Err(SourceDispatchError {
                kind: SourceDispatchErrorKind::NotConfigured,
                allow_fallback: source.allow_fallback,
                message: "HTTP mirror source is not configured in this release".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::mlx::catalog::{MlxCatalogModel, ModelSource, ModelSourceKind};

    #[test]
    fn ordered_sources_are_sorted_by_priority() {
        let model = MlxCatalogModel {
            canonical_id: "test_v1".to_string(),
            aliases: Vec::new(),
            display_name: "Test".to_string(),
            description: "Test".to_string(),
            size_bytes: 1,
            parameters: "~1 GB".to_string(),
            sources: vec![
                ModelSource {
                    kind: ModelSourceKind::HttpMirror,
                    value: "https://mirror".to_string(),
                    priority: 20,
                    enabled: true,
                    allow_fallback: true,
                },
                ModelSource {
                    kind: ModelSourceKind::HuggingfaceRepo,
                    value: "mlx-community/test".to_string(),
                    priority: 10,
                    enabled: true,
                    allow_fallback: false,
                },
            ],
        };

        let ordered = ModelDownloader::ordered_sources(&model);
        assert_eq!(ordered[0].priority, 10);
        assert_eq!(ordered[1].priority, 20);
    }

    #[test]
    fn dispatch_hf_source_returns_hf_target() {
        let source = ModelSource {
            kind: ModelSourceKind::HuggingfaceRepo,
            value: "mlx-community/test".to_string(),
            priority: 10,
            enabled: true,
            allow_fallback: false,
        };

        let target = ModelDownloader::dispatch_download_target(&source)
            .expect("hf source should resolve to a target");
        assert_eq!(
            target,
            DownloadSourceTarget::HuggingFaceRepo("mlx-community/test".to_string())
        );
    }

    #[test]
    fn dispatch_mirror_source_returns_non_fatal_error_when_fallback_allowed() {
        let source = ModelSource {
            kind: ModelSourceKind::HttpMirror,
            value: "https://mirror".to_string(),
            priority: 10,
            enabled: true,
            allow_fallback: true,
        };

        let error = ModelDownloader::dispatch_download_target(&source)
            .expect_err("mirror source should be deferred");
        assert_eq!(error.kind, SourceDispatchErrorKind::NotConfigured);
        assert!(error.can_fallback());
    }
}
