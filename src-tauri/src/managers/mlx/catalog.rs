use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelSourceKind {
    HuggingfaceRepo,
    HttpMirror,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSource {
    pub kind: ModelSourceKind,
    pub value: String,
    pub priority: u32,
    pub enabled: bool,
    pub allow_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MlxCatalogModel {
    pub canonical_id: String,
    pub aliases: Vec<String>,
    pub display_name: String,
    pub description: String,
    pub size_bytes: u64,
    pub parameters: String,
    pub sources: Vec<ModelSource>,
}

impl MlxCatalogModel {
    pub fn primary_hf_repo(&self) -> Option<&str> {
        self.sources
            .iter()
            .filter(|source| source.enabled && source.kind == ModelSourceKind::HuggingfaceRepo)
            .min_by_key(|source| source.priority)
            .map(|source| source.value.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct MlxCatalog {
    models: HashMap<String, MlxCatalogModel>,
    alias_to_canonical: HashMap<String, String>,
    default_canonical_id: String,
}

impl MlxCatalog {
    pub fn new(models: Vec<MlxCatalogModel>, default_canonical_id: String) -> Result<Self> {
        let mut by_id = HashMap::new();
        let mut alias_to_canonical = HashMap::new();
        let mut seen_aliases = HashSet::new();

        for model in models {
            if by_id.contains_key(&model.canonical_id) {
                return Err(anyhow!(
                    "Duplicate canonical MLX model ID: {}",
                    model.canonical_id
                ));
            }

            if !model.sources.iter().any(|source| source.enabled) {
                return Err(anyhow!(
                    "MLX model {} has no enabled download source",
                    model.canonical_id
                ));
            }

            for alias in &model.aliases {
                if alias == &model.canonical_id {
                    return Err(anyhow!(
                        "MLX model {} has alias equal to canonical ID",
                        model.canonical_id
                    ));
                }

                if !seen_aliases.insert(alias.clone()) {
                    return Err(anyhow!("Duplicate MLX model alias: {}", alias));
                }

                alias_to_canonical.insert(alias.clone(), model.canonical_id.clone());
            }

            by_id.insert(model.canonical_id.clone(), model);
        }

        if !by_id.contains_key(&default_canonical_id) {
            return Err(anyhow!(
                "Default MLX model {} is not in catalog",
                default_canonical_id
            ));
        }

        for alias in alias_to_canonical.keys() {
            if by_id.contains_key(alias) {
                return Err(anyhow!(
                    "MLX alias {} collides with canonical model ID",
                    alias
                ));
            }
        }

        Ok(Self {
            models: by_id,
            alias_to_canonical,
            default_canonical_id,
        })
    }

    pub fn resolve_canonical_id(&self, id: &str) -> Option<String> {
        if self.models.contains_key(id) {
            return Some(id.to_string());
        }

        self.alias_to_canonical.get(id).cloned()
    }

    pub fn get_model(&self, canonical_id: &str) -> Option<&MlxCatalogModel> {
        self.models.get(canonical_id)
    }

    pub fn models(&self) -> impl Iterator<Item = &MlxCatalogModel> {
        self.models.values()
    }

    pub fn default_canonical_id(&self) -> &str {
        &self.default_canonical_id
    }
}

pub fn recommended_model_id_for_ram(system_ram_gb: u64) -> &'static str {
    if system_ram_gb > 16 {
        "qwen3_base_8b_v1"
    } else if system_ram_gb > 8 {
        "qwen3_base_4b_v1"
    } else {
        "qwen3_base_1.7b_v1"
    }
}

pub fn build_embedded_catalog(system_ram_gb: u64) -> Result<MlxCatalog> {
    let default_canonical_id = recommended_model_id_for_ram(system_ram_gb).to_string();

    let models = vec![
        MlxCatalogModel {
            canonical_id: "qwen3_base_0.6b_v1".to_string(),
            aliases: vec!["qwen3_base_0.6b".to_string()],
            display_name: "Qwen 3 Base 0.6B".to_string(),
            description: "Ultra-fast responses. Best for simple corrections.".to_string(),
            size_bytes: 400 * 1024 * 1024,
            parameters: "~1 GB".to_string(),
            sources: vec![ModelSource {
                kind: ModelSourceKind::HuggingfaceRepo,
                value: "mlx-community/Qwen3-0.6B-4bit".to_string(),
                priority: 10,
                enabled: true,
                allow_fallback: false,
            }],
        },
        MlxCatalogModel {
            canonical_id: "qwen3_base_1.7b_v1".to_string(),
            aliases: vec!["qwen3_base_1.7b".to_string()],
            display_name: "Qwen 3 Base 1.7B".to_string(),
            description: "Good speed and quality. Great for 8GB Macs.".to_string(),
            size_bytes: 1024 * 1024 * 1024,
            parameters: "~2-3 GB".to_string(),
            sources: vec![ModelSource {
                kind: ModelSourceKind::HuggingfaceRepo,
                value: "mlx-community/Qwen3-1.7B-4bit".to_string(),
                priority: 10,
                enabled: true,
                allow_fallback: false,
            }],
        },
        MlxCatalogModel {
            canonical_id: "qwen3_base_4b_v1".to_string(),
            aliases: vec!["qwen3_base_4b".to_string()],
            display_name: "Qwen 3 4B Instruct (2507)".to_string(),
            description: "Strong instruction-following and writing quality.".to_string(),
            size_bytes: 2260 * 1024 * 1024,
            parameters: "~2 GB min, ~4-5 GB typical".to_string(),
            sources: vec![ModelSource {
                kind: ModelSourceKind::HuggingfaceRepo,
                value: "mlx-community/Qwen3-4B-Instruct-2507-4bit".to_string(),
                priority: 10,
                enabled: true,
                allow_fallback: false,
            }],
        },
        MlxCatalogModel {
            canonical_id: "qwen3_base_8b_v1".to_string(),
            aliases: vec!["qwen3_base_8b".to_string()],
            display_name: "Qwen 3 Base 8B".to_string(),
            description: "Best quality and complex reasoning.".to_string(),
            size_bytes: 4700 * 1024 * 1024,
            parameters: "~7-8 GB".to_string(),
            sources: vec![ModelSource {
                kind: ModelSourceKind::HuggingfaceRepo,
                value: "mlx-community/Qwen3-8B-4bit".to_string(),
                priority: 10,
                enabled: true,
                allow_fallback: false,
            }],
        },
        MlxCatalogModel {
            canonical_id: "gemma3_base_1b_v1".to_string(),
            aliases: vec!["gemma3_base_1b".to_string()],
            display_name: "Gemma 3 Base 1B".to_string(),
            description: "Lightweight with good multi-language support.".to_string(),
            size_bytes: 800 * 1024 * 1024,
            parameters: "~1 GB".to_string(),
            sources: vec![ModelSource {
                kind: ModelSourceKind::HuggingfaceRepo,
                value: "mlx-community/gemma-3-1b-it-4bit".to_string(),
                priority: 10,
                enabled: true,
                allow_fallback: false,
            }],
        },
        MlxCatalogModel {
            canonical_id: "gemma3_base_4b_v1".to_string(),
            aliases: vec!["gemma3_base_4b".to_string()],
            display_name: "Gemma 3 Base 4B".to_string(),
            description: "Excellent multi-language and translation.".to_string(),
            size_bytes: 2300 * 1024 * 1024,
            parameters: "~3 GB".to_string(),
            sources: vec![ModelSource {
                kind: ModelSourceKind::HuggingfaceRepo,
                value: "mlx-community/gemma-3-4b-it-4bit".to_string(),
                priority: 10,
                enabled: true,
                allow_fallback: false,
            }],
        },
        MlxCatalogModel {
            canonical_id: "smollm3_base_3b_v1".to_string(),
            aliases: vec!["smollm3_base_3b".to_string()],
            display_name: "SmolLM 3 Base 3B".to_string(),
            description: "HuggingFace's efficient small model.".to_string(),
            size_bytes: 1770 * 1024 * 1024,
            parameters: "~2 GB".to_string(),
            sources: vec![ModelSource {
                kind: ModelSourceKind::HuggingfaceRepo,
                value: "mlx-community/SmolLM3-3B-4bit".to_string(),
                priority: 10,
                enabled: true,
                allow_fallback: false,
            }],
        },
    ];

    MlxCatalog::new(models, default_canonical_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_resolves_aliases() {
        let catalog = build_embedded_catalog(16).expect("catalog should build");
        assert_eq!(
            catalog.resolve_canonical_id("qwen3_base_4b"),
            Some("qwen3_base_4b_v1".to_string())
        );
        assert_eq!(
            catalog.resolve_canonical_id("qwen3_base_4b_v1"),
            Some("qwen3_base_4b_v1".to_string())
        );
    }

    #[test]
    fn embedded_catalog_uses_versioned_ids() {
        let catalog = build_embedded_catalog(16).expect("catalog should build");
        assert!(
            catalog
                .models()
                .all(|model| model.canonical_id.ends_with("_v1")),
            "all canonical IDs must be versioned"
        );
    }

    #[test]
    fn recommended_default_is_ram_tiered() {
        assert_eq!(recommended_model_id_for_ram(8), "qwen3_base_1.7b_v1");
        assert_eq!(recommended_model_id_for_ram(12), "qwen3_base_4b_v1");
        assert_eq!(recommended_model_id_for_ram(24), "qwen3_base_8b_v1");
    }

    #[test]
    fn smollm3_entry_points_to_smollm3_repo() {
        let catalog = build_embedded_catalog(16).expect("catalog should build");
        let model = catalog
            .get_model("smollm3_base_3b_v1")
            .expect("smollm3 model should exist");
        assert_eq!(
            model.primary_hf_repo(),
            Some("mlx-community/SmolLM3-3B-4bit")
        );
    }

    #[test]
    fn catalog_rejects_duplicate_aliases() {
        let result = MlxCatalog::new(
            vec![
                MlxCatalogModel {
                    canonical_id: "a_v1".to_string(),
                    aliases: vec!["legacy_a".to_string()],
                    display_name: "A".to_string(),
                    description: "A".to_string(),
                    size_bytes: 1,
                    parameters: "~1 GB".to_string(),
                    sources: vec![ModelSource {
                        kind: ModelSourceKind::HuggingfaceRepo,
                        value: "repo/a".to_string(),
                        priority: 10,
                        enabled: true,
                        allow_fallback: false,
                    }],
                },
                MlxCatalogModel {
                    canonical_id: "b_v1".to_string(),
                    aliases: vec!["legacy_a".to_string()],
                    display_name: "B".to_string(),
                    description: "B".to_string(),
                    size_bytes: 1,
                    parameters: "~1 GB".to_string(),
                    sources: vec![ModelSource {
                        kind: ModelSourceKind::HuggingfaceRepo,
                        value: "repo/b".to_string(),
                        priority: 10,
                        enabled: true,
                        allow_fallback: false,
                    }],
                },
            ],
            "a_v1".to_string(),
        );

        assert!(result.is_err());
    }
}
