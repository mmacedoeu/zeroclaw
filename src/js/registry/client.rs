// ClawHub registry client - fetches plugin metadata and downloads

use super::types::{RegistryPlugin, SearchResult};
use crate::js::error::{JsPluginError, RegistryError as PluginRegistryError};
use reqwest::Client;
use serde::Deserialize;

/// Registry client configuration
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Base URL for the registry API
    pub base_url: String,

    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            base_url: "https://clawhub.dev".to_string(),
            timeout_secs: 30,
        }
    }
}

/// ClawHub plugin registry client
///
/// This client provides methods to:
/// - Search for plugins by keyword
/// - Get plugin metadata
/// - Download plugin packages with integrity verification
pub struct ClawHubClient {
    /// HTTP client
    client: Client,

    /// Registry configuration
    config: RegistryConfig,
}

impl ClawHubClient {
    /// Create a new registry client with default configuration
    pub fn new() -> Self {
        Self::with_config(RegistryConfig::default())
    }

    /// Create a new registry client with a specific configuration
    pub fn with_config(config: RegistryConfig) -> Self {
        let timeout = std::time::Duration::from_secs(config.timeout_secs);
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client, config }
    }

    /// Search for plugins by keyword
    ///
    /// # Arguments
    ///
    /// * `query` - Search query (keywords)
    ///
    /// # Returns
    ///
    /// A list of search results sorted by relevance.
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>, JsPluginError> {
        if query.is_empty() {
            return Ok(vec![]);
        }

        let url = format!(
            "{}/api/v1/plugins/search?q={}",
            self.config.base_url,
            urlencoding::encode(query)
        );

        let response = self.client.get(&url).send().await.map_err(|e| {
            JsPluginError::Registry(PluginRegistryError::RequestFailed(format!(
                "HTTP request failed: {}",
                e
            )))
        })?;

        if !response.status().is_success() {
            return Err(JsPluginError::Registry(PluginRegistryError::NotFound(
                format!("Search failed: {}", response.status()),
            )));
        }

        #[derive(Deserialize)]
        struct SearchResponse {
            results: Vec<SearchResult>,
        }

        let search_response: SearchResponse = response.json().await.map_err(|e| {
            JsPluginError::Registry(PluginRegistryError::InvalidResponse(format!(
                "Failed to parse response: {}",
                e
            )))
        })?;

        Ok(search_response.results)
    }

    /// Get plugin metadata from the registry
    ///
    /// # Arguments
    ///
    /// * `name` - Plugin name (e.g., "@user/plugin" or "plugin")
    ///
    /// # Returns
    ///
    /// The plugin metadata including download URL and checksum.
    pub async fn get_plugin(&self, name: &str) -> Result<RegistryPlugin, JsPluginError> {
        let encoded_name = urlencoding::encode(name);
        let url = format!("{}/api/v1/plugins/{}", self.config.base_url, encoded_name);

        let response = self.client.get(&url).send().await.map_err(|e| {
            JsPluginError::Registry(PluginRegistryError::RequestFailed(format!(
                "HTTP request failed: {}",
                e
            )))
        })?;

        if response.status() == 404 {
            return Err(JsPluginError::Registry(PluginRegistryError::NotFound(
                format!("Plugin '{}' not found", name),
            )));
        }

        if !response.status().is_success() {
            return Err(JsPluginError::Registry(PluginRegistryError::RequestFailed(
                format!("Registry error: {}", response.status()),
            )));
        }

        response.json().await.map_err(|e| {
            JsPluginError::Registry(PluginRegistryError::InvalidResponse(format!(
                "Failed to parse response: {}",
                e
            )))
        })
    }

    /// Download a plugin package
    ///
    /// # Arguments
    ///
    /// * `plugin` - Plugin metadata containing download URL
    ///
    /// # Returns
    ///
    /// The downloaded plugin package as bytes.
    pub async fn download_plugin(&self, plugin: &RegistryPlugin) -> Result<Vec<u8>, JsPluginError> {
        let response = self
            .client
            .get(&plugin.download_url)
            .send()
            .await
            .map_err(|e| {
                JsPluginError::Registry(PluginRegistryError::RequestFailed(format!(
                    "Download failed: {}",
                    e
                )))
            })?;

        if !response.status().is_success() {
            return Err(JsPluginError::Registry(PluginRegistryError::RequestFailed(
                format!("Download error: {}", response.status()),
            )));
        }

        let bytes = response.bytes().await.map_err(|e| {
            JsPluginError::Registry(PluginRegistryError::RequestFailed(format!(
                "Failed to read response: {}",
                e
            )))
        })?;

        // Verify SHA256 checksum
        self.verify_checksum(&bytes, &plugin.sha256)?;

        Ok(bytes.to_vec())
    }

    /// Verify SHA256 checksum of downloaded data
    fn verify_checksum(&self, data: &[u8], expected: &str) -> Result<(), JsPluginError> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let checksum = format!("{:x}", result);

        if checksum != expected.to_lowercase() {
            return Err(JsPluginError::Registry(
                PluginRegistryError::IntegrityCheckFailed(format!(
                    "SHA256 mismatch: expected {}, got {}",
                    expected, checksum
                )),
            ));
        }

        Ok(())
    }

    /// Get the registry configuration
    pub fn config(&self) -> &RegistryConfig {
        &self.config
    }
}

impl Default for ClawHubClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_config_default() {
        let config = RegistryConfig::default();
        assert_eq!(config.base_url, "https://clawhub.dev");
        assert_eq!(config.timeout_secs, 30);
    }

    #[test]
    fn registry_config_custom_base_url() {
        let config = RegistryConfig {
            base_url: "https://custom.registry".to_string(),
            ..Default::default()
        };
        assert_eq!(config.base_url, "https://custom.registry");
    }

    #[test]
    fn claw_hub_client_can_be_created() {
        let client = ClawHubClient::new();
        assert_eq!(client.config().base_url, "https://clawhub.dev");
    }

    #[test]
    fn claw_hub_client_with_custom_config() {
        let config = RegistryConfig {
            base_url: "https://test.registry".to_string(),
            timeout_secs: 60,
        };
        let client = ClawHubClient::with_config(config);
        assert_eq!(client.config().base_url, "https://test.registry");
        assert_eq!(client.config().timeout_secs, 60);
    }
}
