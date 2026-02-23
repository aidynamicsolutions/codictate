use anyhow::Result;

use super::catalog::{build_embedded_catalog, MlxCatalog};

#[allow(dead_code)]
pub const MLX_MIRROR_BASE_URL: &str = "MLX_MIRROR_BASE_URL";
#[allow(dead_code)]
pub const MLX_CATALOG_URL: &str = "MLX_CATALOG_URL";
#[allow(dead_code)]
pub const MLX_CATALOG_PUBKEY: &str = "MLX_CATALOG_PUBKEY";

pub trait CatalogProvider: Send + Sync {
    fn load_catalog(&self, system_ram_gb: u64) -> Result<MlxCatalog>;
}

#[derive(Debug, Default)]
pub struct EmbeddedCatalogProvider;

impl CatalogProvider for EmbeddedCatalogProvider {
    fn load_catalog(&self, system_ram_gb: u64) -> Result<MlxCatalog> {
        build_embedded_catalog(system_ram_gb)
    }
}
