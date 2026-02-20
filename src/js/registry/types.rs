// Registry API types - ClawHub plugin metadata structures

use serde::{Deserialize, Serialize};

/// Registry plugin information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryPlugin {
    /// Plugin name (e.g., "@user/plugin" or "plugin")
    pub name: String,

    /// Plugin version
    pub version: String,

    /// Human-readable description
    pub description: String,

    /// Plugin author
    pub author: String,

    /// Plugin metadata
    pub metadata: PluginMetadata,

    /// Download URL for the plugin package
    pub download_url: String,

    /// SHA256 checksum for integrity verification
    pub sha256: String,

    /// Homepage URL
    pub homepage: Option<String>,

    /// Repository URL
    pub repository: Option<String>,

    /// License
    pub license: Option<String>,

    /// Plugin tags/categories
    pub tags: Vec<String>,

    /// Number of downloads
    pub downloads: u64,

    /// Plugin rating (0-5)
    pub rating: Option<f32>,

    /// When the plugin was last updated
    pub updated_at: String,
}

/// Plugin metadata from plugin.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin display name
    pub display_name: String,

    /// Minimum ZeroClaw version required
    pub min_zc_version: Option<String>,

    /// Required permissions
    pub permissions: serde_json::Value,

    /// Defined tools
    pub tools: serde_json::Value,
}

/// Search result from registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Plugin name
    pub name: String,

    /// Short description
    pub description: String,

    /// Author
    pub author: String,

    /// Version
    pub version: String,

    /// Match relevance score
    pub score: f32,

    /// Download count
    pub downloads: u64,

    /// Tags
    pub tags: Vec<String>,
}

/// Registry API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Additional details
    pub details: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_plugin_serialization() {
        let plugin = RegistryPlugin {
            name: "@test/plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Test plugin".to_string(),
            author: "Test Author".to_string(),
            metadata: PluginMetadata {
                display_name: "Test Plugin".to_string(),
                min_zc_version: Some("0.1.0".to_string()),
                permissions: serde_json::json!({}),
                tools: serde_json::json!([]),
            },
            download_url: "https://example.com/plugin.zip".to_string(),
            sha256: "abc123".to_string(),
            homepage: None,
            repository: None,
            license: Some("MIT".to_string()),
            tags: vec!["test".to_string()],
            downloads: 100,
            rating: Some(4.5),
            updated_at: "2024-01-01".to_string(),
        };

        let json = serde_json::to_string(&plugin).unwrap();
        let parsed: RegistryPlugin = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "@test/plugin");
        assert_eq!(parsed.version, "1.0.0");
        assert_eq!(parsed.downloads, 100);
    }

    #[test]
    fn search_result_serialization() {
        let result = SearchResult {
            name: "@test/plugin".to_string(),
            description: "Test plugin".to_string(),
            author: "Test Author".to_string(),
            version: "1.0.0".to_string(),
            score: 0.95,
            downloads: 100,
            tags: vec!["test".to_string()],
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: SearchResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "@test/plugin");
        assert!((parsed.score - 0.95).abs() < 0.01);
    }

    #[test]
    fn registry_error_serialization() {
        let error = RegistryError {
            code: "NOT_FOUND".to_string(),
            message: "Plugin not found".to_string(),
            details: None,
        };

        let json = serde_json::to_string(&error).unwrap();
        let parsed: RegistryError = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.code, "NOT_FOUND");
        assert_eq!(parsed.message, "Plugin not found");
    }
}
