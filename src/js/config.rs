// Configuration types for JS plugins

use std::path::PathBuf;
use std::time::Duration;

/// Thread pool configuration for JS runtime
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of worker threads to spawn
    pub max_workers: usize,

    /// Memory limit per QuickJS context in bytes
    pub memory_limit: usize,

    /// CPU time limit per execution call
    pub cpu_time_limit: Duration,

    /// Default permissions for plugins loaded into this pool
    pub default_permissions: JsPluginPermissions,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_workers: 4,
            memory_limit: 64 * 1024 * 1024, // 64MB
            cpu_time_limit: Duration::from_secs(30),
            default_permissions: JsPluginPermissions::default(),
        }
    }
}

/// Plugin-specific permissions
#[derive(Debug, Clone, Default)]
pub struct JsPluginPermissions {
    /// Network hosts this plugin is allowed to contact
    pub network: Vec<String>,

    /// Filesystem paths this plugin is allowed to read (glob patterns)
    pub file_read: Vec<PathBuf>,

    /// Whether this plugin can write to files
    pub file_write: bool,

    /// Environment variables this plugin is allowed to read
    pub env_vars: Vec<String>,
}

impl JsPluginPermissions {
    /// Create a new permissions object with no access
    pub fn none() -> Self {
        Self::default()
    }

    /// Check if any permissions are granted
    pub fn is_empty(&self) -> bool {
        self.network.is_empty()
            && self.file_read.is_empty()
            && !self.file_write
            && self.env_vars.is_empty()
    }
}

/// Configuration for a single plugin runtime instance
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Memory limit in bytes
    pub memory_limit: usize,

    /// CPU time limit per execution
    pub cpu_time_limit: Duration,

    /// Permissions granted to this plugin
    pub permissions: JsPluginPermissions,

    /// Plugin identifier
    pub plugin_id: String,
}

impl RuntimeConfig {
    /// Create a RuntimeConfig from PoolConfig for a specific plugin
    pub fn from_pool_config(pool_config: &PoolConfig, plugin_id: String) -> Self {
        Self {
            memory_limit: pool_config.memory_limit,
            cpu_time_limit: pool_config.cpu_time_limit,
            permissions: pool_config.default_permissions.clone(),
            plugin_id,
        }
    }
}

// Helper function for the implementation plan - this creates a default RuntimeConfig
// for worker initialization
impl RuntimeConfig {
    pub(crate) fn from_default(pool_config: &PoolConfig) -> Self {
        Self {
            memory_limit: pool_config.memory_limit,
            cpu_time_limit: pool_config.cpu_time_limit,
            permissions: pool_config.default_permissions.clone(),
            plugin_id: "<worker>".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pool_config() {
        let config = PoolConfig::default();
        assert_eq!(config.max_workers, 4);
        assert_eq!(config.memory_limit, 64 * 1024 * 1024);
        assert_eq!(config.cpu_time_limit, Duration::from_secs(30));
        assert!(config.default_permissions.is_empty());
    }

    #[test]
    fn permissions_none_is_empty() {
        let perms = JsPluginPermissions::none();
        assert!(perms.is_empty());
    }

    #[test]
    fn permissions_with_network_not_empty() {
        let mut perms = JsPluginPermissions::default();
        perms.network.push("api.example.com".to_string());
        assert!(!perms.is_empty());
    }

    #[test]
    fn runtime_config_from_pool() {
        let pool_config = PoolConfig::default();
        let runtime_config =
            RuntimeConfig::from_pool_config(&pool_config, "test-plugin".to_string());

        assert_eq!(runtime_config.plugin_id, "test-plugin");
        assert_eq!(runtime_config.memory_limit, 64 * 1024 * 1024);
    }
}
