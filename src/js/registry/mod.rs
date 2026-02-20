// Registry client - ClawHub plugin registry integration

pub mod client;
pub mod types;

pub use client::{ClawHubClient, RegistryConfig};
pub use types::{PluginMetadata, RegistryPlugin, SearchResult};

use crate::js::error::JsPluginError;

/// Search for plugins in the registry
///
/// This is a convenience function that uses the default registry configuration.
pub async fn search_plugins(query: &str) -> Result<Vec<SearchResult>, JsPluginError> {
    let client = ClawHubClient::new();
    client.search(query).await
}

/// Get plugin metadata from the registry
///
/// This is a convenience function that uses the default registry configuration.
pub async fn get_plugin(name: &str) -> Result<RegistryPlugin, JsPluginError> {
    let client = ClawHubClient::new();
    client.get_plugin(name).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_module_exports_types() {
        // Test that our public API is accessible
        let config = RegistryConfig::default();
        assert_eq!(config.base_url, "https://clawhub.dev");
    }
}
