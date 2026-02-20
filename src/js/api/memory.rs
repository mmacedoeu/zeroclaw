// Memory API bridge - connects JS plugins to ZeroClaw's Memory backend

use crate::js::error::JsPluginError;
use crate::memory::{Memory, MemoryCategory};
use serde_json::Value;
use std::sync::Arc;

/// Bridge between JS plugins and ZeroClaw's memory backend
///
/// This bridge provides namespaced key isolation - each plugin gets its own
/// key prefix to prevent interference between plugins.
pub struct JsMemoryBridge {
    /// Reference to the configured memory backend
    backend: Arc<dyn Memory>,

    /// Namespace prefix for all keys (isolates plugins from each other)
    key_prefix: String,

    /// Default memory category for plugin data
    category: MemoryCategory,
}

impl JsMemoryBridge {
    /// Create a new memory bridge for a plugin
    ///
    /// # Arguments
    ///
    /// * `backend` - The ZeroClaw memory backend
    /// * `plugin_id` - The plugin identifier used for namespacing
    pub fn new(backend: Arc<dyn Memory>, plugin_id: &str) -> Self {
        Self {
            backend,
            key_prefix: format!("js_plugin:{}", plugin_id),
            category: MemoryCategory::Custom("js_plugin".to_string()),
        }
    }

    /// Get a value from memory
    ///
    /// Keys are namespaced: `js_plugin:<plugin_id>:<key>`
    pub async fn get(&self, key: &str) -> Result<Value, JsPluginError> {
        let namespaced = format!("{}:{}", self.key_prefix, key);

        self.backend
            .get(&namespaced)
            .await
            .map_err(|e| JsPluginError::Memory(format!("Get failed: {}", e)))?
            .ok_or_else(|| JsPluginError::Memory(format!("Key not found: {}", key)))
            .and_then(|entry| {
                serde_json::from_str(&entry.content)
                    .map_err(|e| JsPluginError::Memory(format!("JSON parse failed: {}", e)))
            })
    }

    /// Set a value in memory
    ///
    /// Keys are namespaced: `js_plugin:<plugin_id>:<key>`
    pub async fn set(&self, key: &str, value: &Value) -> Result<(), JsPluginError> {
        let namespaced = format!("{}:{}", self.key_prefix, key);
        let content = serde_json::to_string(value)
            .map_err(|e| JsPluginError::Memory(format!("JSON serialize failed: {}", e)))?;

        self.backend
            .store(&namespaced, &content, self.category.clone(), None)
            .await
            .map_err(|e| JsPluginError::Memory(format!("Set failed: {}", e)))
    }

    /// Delete a value from memory
    ///
    /// Keys are namespaced: `js_plugin:<plugin_id>:<key>`
    pub async fn delete(&self, key: &str) -> Result<bool, JsPluginError> {
        let namespaced = format!("{}:{}", self.key_prefix, key);

        self.backend
            .forget(&namespaced)
            .await
            .map_err(|e| JsPluginError::Memory(format!("Delete failed: {}", e)))
    }

    /// Check if a key exists in memory
    pub async fn exists(&self, key: &str) -> Result<bool, JsPluginError> {
        match self.get(key).await {
            Ok(_) => Ok(true),
            Err(JsPluginError::Memory(msg)) if msg.contains("not found") => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Recall memories by keyword search
    ///
    /// Searches within the plugin's namespace
    pub async fn recall(&self, query: &str, limit: usize) -> Result<Vec<Value>, JsPluginError> {
        let namespaced_query = format!("{}:{}", self.key_prefix, query);

        self.backend
            .recall(&namespaced_query, limit, None)
            .await
            .map_err(|e| JsPluginError::Memory(format!("Recall failed: {}", e)))
            .and_then(|entries| {
                entries
                    .iter()
                    .map(|entry| {
                        serde_json::from_str(&entry.content)
                            .map_err(|e| JsPluginError::Memory(format!("JSON parse failed: {}", e)))
                    })
                    .collect()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryEntry;

    // Mock Memory backend for testing
    struct MockMemory {
        data: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, MemoryEntry>>>,
    }

    impl MockMemory {
        fn new() -> Self {
            Self {
                data: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl Memory for MockMemory {
        fn name(&self) -> &str {
            "mock"
        }

        async fn store(
            &self,
            key: &str,
            content: &str,
            _category: MemoryCategory,
            _session_id: Option<&str>,
        ) -> anyhow::Result<()> {
            let mut data = self.data.lock().unwrap();
            let entry = MemoryEntry {
                id: uuid::Uuid::new_v4().to_string(),
                key: key.to_string(),
                content: content.to_string(),
                category: MemoryCategory::Core,
                timestamp: chrono::Utc::now().to_rfc3339(),
                session_id: None,
                score: None,
            };
            data.insert(key.to_string(), entry);
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _limit: usize,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(vec![])
        }

        async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
            let data = self.data.lock().unwrap();
            Ok(data.get(key).cloned())
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(vec![])
        }

        async fn forget(&self, key: &str) -> anyhow::Result<bool> {
            let mut data = self.data.lock().unwrap();
            Ok(data.remove(key).is_some())
        }

        async fn count(&self) -> anyhow::Result<usize> {
            let data = self.data.lock().unwrap();
            Ok(data.len())
        }

        async fn health_check(&self) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn memory_bridge_get_set() {
        let backend: Arc<dyn Memory> = Arc::new(MockMemory::new());
        let bridge = JsMemoryBridge::new(backend, "test-plugin");

        // Set a value
        bridge
            .set("key", &Value::String("value".to_string()))
            .await
            .expect("Set should succeed");

        // Get the value back
        let result = bridge.get("key").await.expect("Get should succeed");
        assert_eq!(result, Value::String("value".to_string()));
    }

    #[tokio::test]
    async fn memory_bridge_namespacing() {
        let backend: Arc<dyn Memory> = Arc::new(MockMemory::new());
        let bridge1 = JsMemoryBridge::new(backend.clone(), "plugin1");
        let bridge2 = JsMemoryBridge::new(backend.clone(), "plugin2");

        // Set value in plugin1's namespace
        bridge1
            .set("key", &Value::String("value1".to_string()))
            .await
            .expect("Set should succeed");

        // plugin2 should not see plugin1's value
        let result = bridge2.get("key").await;
        assert!(result.is_err(), "plugin2 should not see plugin1's value");
    }

    #[tokio::test]
    async fn memory_bridge_delete() {
        let backend: Arc<dyn Memory> = Arc::new(MockMemory::new());
        let bridge = JsMemoryBridge::new(backend, "test-plugin");

        bridge
            .set("key", &Value::String("value".to_string()))
            .await
            .expect("Set should succeed");

        // Delete the key
        let deleted = bridge.delete("key").await.expect("Delete should succeed");
        assert!(deleted, "Key should exist");

        // Verify it's gone
        let result = bridge.get("key").await;
        assert!(result.is_err(), "Key should be deleted");
    }

    #[tokio::test]
    async fn memory_bridge_json_types() {
        let backend: Arc<dyn Memory> = Arc::new(MockMemory::new());
        let bridge = JsMemoryBridge::new(backend, "test-plugin");

        // Store different JSON types
        let obj = serde_json::json!({
            "string": "hello",
            "number": 42,
            "bool": true,
            "nested": { "key": "value" }
        });

        bridge.set("data", &obj).await.expect("Set should succeed");

        // Retrieve and verify
        let result = bridge.get("data").await.expect("Get should succeed");
        assert_eq!(result["string"], "hello");
        assert_eq!(result["number"], 42);
        assert_eq!(result["bool"], true);
    }

    #[tokio::test]
    async fn memory_bridge_exists() {
        let backend: Arc<dyn Memory> = Arc::new(MockMemory::new());
        let bridge = JsMemoryBridge::new(backend, "test-plugin");

        // Key doesn't exist initially
        assert!(!bridge
            .exists("key")
            .await
            .expect("Exists check should succeed"));

        bridge
            .set("key", &Value::String("value".to_string()))
            .await
            .expect("Set should succeed");

        // Now it exists
        assert!(bridge
            .exists("key")
            .await
            .expect("Exists check should succeed"));
    }
}
