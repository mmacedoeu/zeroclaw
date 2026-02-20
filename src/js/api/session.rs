// Session API bridge - connects JS plugins to ZeroClaw's session context

use crate::channels::traits::{Channel, SendMessage};
use crate::js::error::JsPluginError;
use crate::js::sandbox::context::ExecutionContext;
use crate::memory::{Memory, MemoryCategory};
use serde_json::Value;
use std::sync::Arc;

/// Bridge between JS plugins and ZeroClaw's session context
///
/// This bridge provides:
/// - Session metadata (session_id, channel_id, channel_type, user_id)
/// - Optional channel reference for reply/typing operations
/// - Optional memory backend for session-scoped storage
/// - Context extraction from ExecutionContext
pub struct JsSessionBridge {
    /// Current session identifier
    session_id: String,

    /// Channel identifier (if applicable)
    channel_id: Option<String>,

    /// Channel type (telegram, discord, cli, etc.)
    channel_type: String,

    /// User identifier (if authenticated)
    user_id: Option<String>,

    /// Optional channel reference for reply operations
    channel: Option<Arc<dyn Channel>>,

    /// Optional memory backend for session-scoped storage
    memory: Option<Arc<dyn Memory>>,
}

impl JsSessionBridge {
    /// Create a new session bridge from execution context
    ///
    /// # Arguments
    ///
    /// * `ctx` - The execution context containing session information
    pub fn from_context(ctx: &ExecutionContext) -> Self {
        Self {
            session_id: ctx.session_id.clone(),
            channel_id: None,
            channel_type: ctx.channel_type.clone(),
            user_id: ctx.user_id.clone(),
            channel: None,
            memory: None,
        }
    }

    /// Register this session bridge in a JavaScript context value
    ///
    /// This converts the bridge to a serde_json::Value suitable for
    /// injection into QuickJS execution contexts.
    ///
    /// # Returns
    ///
    /// A JSON Value representing the session bridge
    pub fn register_in_ctx(&self) -> Value {
        serde_json::json!({
            "session_id": self.session_id,
            "channel_id": self.channel_id,
            "channel_type": self.channel_type,
            "user_id": self.user_id,
        })
    }

    /// Set the channel reference for this session
    ///
    /// Enables reply() and typing operations
    pub fn with_channel(mut self, channel: Arc<dyn Channel>, channel_id: String) -> Self {
        self.channel = Some(channel);
        self.channel_id = Some(channel_id);
        self
    }

    /// Set the memory backend for this session
    ///
    /// Enables get() and set() operations
    pub fn with_memory(mut self, memory: Arc<dyn Memory>) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Reply to the current channel
    ///
    /// Sends a message to the channel associated with this session.
    /// Requires a channel to be set via with_channel().
    ///
    /// # Arguments
    ///
    /// * `content` - The message content to send
    ///
    /// # Returns
    ///
    /// Ok(()) if the message was sent successfully
    pub async fn reply(&self, content: &str) -> Result<(), JsPluginError> {
        let channel = self.channel.as_ref().ok_or_else(|| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(
                "Channel not set for this session".to_string(),
            ))
        })?;

        let channel_id = self.channel_id.as_ref().ok_or_else(|| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(
                "Channel ID not set for this session".to_string(),
            ))
        })?;

        let message = SendMessage::new(content, channel_id);
        channel.send(&message).await.map_err(|e| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(format!(
                "Failed to send message: {}",
                e
            )))
        })
    }

    /// Start typing indicator
    ///
    /// Signals that the bot is processing a response.
    /// Requires a channel to be set via with_channel().
    pub async fn start_typing(&self) -> Result<(), JsPluginError> {
        let channel = self.channel.as_ref().ok_or_else(|| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(
                "Channel not set for this session".to_string(),
            ))
        })?;

        let channel_id = self.channel_id.as_ref().ok_or_else(|| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(
                "Channel ID not set for this session".to_string(),
            ))
        })?;

        channel.start_typing(channel_id).await.map_err(|e| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(format!(
                "Failed to start typing: {}",
                e
            )))
        })
    }

    /// Stop typing indicator
    ///
    /// Stops any active typing indicator.
    /// Requires a channel to be set via with_channel().
    pub async fn stop_typing(&self) -> Result<(), JsPluginError> {
        let channel = self.channel.as_ref().ok_or_else(|| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(
                "Channel not set for this session".to_string(),
            ))
        })?;

        let channel_id = self.channel_id.as_ref().ok_or_else(|| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(
                "Channel ID not set for this session".to_string(),
            ))
        })?;

        channel.stop_typing(channel_id).await.map_err(|e| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(format!(
                "Failed to stop typing: {}",
                e
            )))
        })
    }

    /// Execute a function with typing indicator
    ///
    /// Starts typing before the function, stops after completion.
    /// Automatically handles typing errors to avoid interrupting the main operation.
    ///
    /// # Arguments
    ///
    /// * `f` - Async function to execute
    pub async fn with_typing<F, Fut, T>(&self, f: F) -> Result<T, JsPluginError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, JsPluginError>>,
    {
        // Start typing (ignore errors)
        let _ = self.start_typing().await;

        // Execute the function
        let result = f().await;

        // Stop typing (ignore errors)
        let _ = self.stop_typing().await;

        result
    }

    /// Get a value from session-scoped memory
    ///
    /// Keys are namespaced: `session:<session_id>:<key>`
    /// Requires memory to be set via with_memory().
    pub async fn get(&self, key: &str) -> Result<Value, JsPluginError> {
        let memory = self
            .memory
            .as_ref()
            .ok_or_else(|| JsPluginError::Memory("Memory backend not set".to_string()))?;

        let namespaced = format!("session:{}:{}", self.session_id, key);

        memory
            .get(&namespaced)
            .await
            .map_err(|e| JsPluginError::Memory(format!("Get failed: {}", e)))?
            .ok_or_else(|| JsPluginError::Memory(format!("Key not found: {}", key)))
            .and_then(|entry| {
                serde_json::from_str(&entry.content)
                    .map_err(|e| JsPluginError::Memory(format!("JSON parse failed: {}", e)))
            })
    }

    /// Set a value in session-scoped memory
    ///
    /// Keys are namespaced: `session:<session_id>:<key>`
    /// Requires memory to be set via with_memory().
    pub async fn set(&self, key: &str, value: &Value) -> Result<(), JsPluginError> {
        let memory = self
            .memory
            .as_ref()
            .ok_or_else(|| JsPluginError::Memory("Memory backend not set".to_string()))?;

        let namespaced = format!("session:{}:{}", self.session_id, key);
        let content = serde_json::to_string(value)
            .map_err(|e| JsPluginError::Memory(format!("JSON serialize failed: {}", e)))?;

        memory
            .store(
                &namespaced,
                &content,
                MemoryCategory::Conversation,
                Some(&self.session_id),
            )
            .await
            .map_err(|e| JsPluginError::Memory(format!("Set failed: {}", e)))
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the channel ID
    pub fn channel_id(&self) -> Option<&str> {
        self.channel_id.as_deref()
    }

    /// Get the channel type
    pub fn channel_type(&self) -> &str {
        &self.channel_type
    }

    /// Get the user ID
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }
}

/// Mock Memory backend for testing
#[cfg(test)]
pub struct MockMemory {
    data: std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, crate::memory::MemoryEntry>>,
    >,
}

#[cfg(test)]
impl MockMemory {
    pub fn new() -> Self {
        Self {
            data: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }
}

#[cfg(test)]
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
        let entry = crate::memory::MemoryEntry {
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
    ) -> anyhow::Result<Vec<crate::memory::MemoryEntry>> {
        Ok(vec![])
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<crate::memory::MemoryEntry>> {
        let data = self.data.lock().unwrap();
        Ok(data.get(key).cloned())
    }

    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<crate::memory::MemoryEntry>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_bridge_from_context() {
        let ctx = ExecutionContext::new("session-123".to_string(), "telegram".to_string())
            .with_user("user-456".to_string());

        let bridge = JsSessionBridge::from_context(&ctx);

        assert_eq!(bridge.session_id(), "session-123");
        assert_eq!(bridge.channel_type(), "telegram");
        assert_eq!(bridge.user_id(), Some("user-456"));
        assert!(bridge.channel_id().is_none());
    }

    #[test]
    fn session_bridge_register_in_ctx() {
        let ctx = ExecutionContext::new("session-abc".to_string(), "discord".to_string())
            .with_user("user-xyz".to_string());

        let bridge = JsSessionBridge::from_context(&ctx);
        let value = bridge.register_in_ctx();

        assert_eq!(value["session_id"], "session-abc");
        assert_eq!(value["channel_type"], "discord");
        assert_eq!(value["user_id"], "user-xyz");
    }

    #[tokio::test]
    async fn session_bridge_with_memory_get_set() {
        let ctx = ExecutionContext::new("session-memory".to_string(), "test".to_string());
        let memory: Arc<dyn Memory> = Arc::new(MockMemory::new());

        let bridge = JsSessionBridge::from_context(&ctx).with_memory(memory);

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
    async fn session_bridge_memory_without_backend_errors() {
        let ctx = ExecutionContext::new("session-no-mem".to_string(), "test".to_string());
        let bridge = JsSessionBridge::from_context(&ctx);

        let result = bridge.get("key").await;
        assert!(result.is_err());
        match result {
            Err(JsPluginError::Memory(msg)) if msg.contains("not set") => {
                // Expected
            }
            _ => panic!("Expected memory backend error"),
        }
    }

    #[tokio::test]
    async fn session_bridge_channel_without_channel_errors() {
        let ctx = ExecutionContext::new("session-no-chan".to_string(), "test".to_string());
        let bridge = JsSessionBridge::from_context(&ctx);

        let result = bridge.reply("hello").await;
        assert!(result.is_err());

        let result = bridge.start_typing().await;
        assert!(result.is_err());

        let result = bridge.stop_typing().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn session_bridge_with_typing_wraps_function() {
        let ctx = ExecutionContext::new("session-typing".to_string(), "test".to_string());
        let bridge = JsSessionBridge::from_context(&ctx);

        let result = bridge
            .with_typing(|| async {
                // Simulate some work
                Ok::<_, JsPluginError>("done")
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "done");
    }

    #[test]
    fn session_bridge_with_channel_sets_channel_id() {
        let ctx = ExecutionContext::new("session-chan".to_string(), "test".to_string());
        let bridge = JsSessionBridge::from_context(&ctx);

        // Create a mock channel (we'll use a simple test double)
        struct TestChannel;
        #[async_trait::async_trait]
        impl Channel for TestChannel {
            fn name(&self) -> &str {
                "test"
            }
            async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
                Ok(())
            }
            async fn listen(
                &self,
                _tx: tokio::sync::mpsc::Sender<crate::channels::traits::ChannelMessage>,
            ) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let bridge = bridge.with_channel(Arc::new(TestChannel), "channel-123".to_string());
        assert_eq!(bridge.channel_id(), Some("channel-123"));
    }

    #[tokio::test]
    async fn session_bridge_json_types() {
        let ctx = ExecutionContext::new("session-json".to_string(), "test".to_string());
        let memory: Arc<dyn Memory> = Arc::new(MockMemory::new());

        let bridge = JsSessionBridge::from_context(&ctx).with_memory(memory);

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
}
