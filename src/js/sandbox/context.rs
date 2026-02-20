// Execution context for JS plugins

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Execution context passed to JS plugins
///
/// This provides read-only ambient information about the current
/// execution environment (session, user, channel, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Current session ID
    pub session_id: String,

    /// Optional user ID (if authenticated)
    pub user_id: Option<String>,

    /// Channel type (telegram, discord, cli, etc.)
    pub channel_type: String,

    /// Additional configuration
    pub config: Value,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            session_id: "default".to_string(),
            user_id: None,
            channel_type: "cli".to_string(),
            config: Value::Object(serde_json::Map::new()),
        }
    }
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(session_id: String, channel_type: String) -> Self {
        Self {
            session_id,
            user_id: None,
            channel_type,
            config: Value::Object(serde_json::Map::new()),
        }
    }

    /// Set the user ID
    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Add a configuration value
    pub fn with_config(mut self, key: String, value: Value) -> Self {
        if let Value::Object(ref mut map) = self.config {
            map.insert(key, value);
        }
        self
    }

    /// Convert to a value suitable for injection into JS
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| Value::Null)
    }

    /// Generate JavaScript code to inject the global Zeroclaw object
    ///
    /// This creates a global `Zeroclaw` object with:
    /// - `pluginId`: The ID of the current plugin
    /// - `on(event, handler)`: Method to register event handlers
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The identifier for the current plugin
    ///
    /// # Returns
    ///
    /// JavaScript code that sets up the global Zeroclaw object
    pub fn inject_zeroclaw_global(plugin_id: &str) -> String {
        format!(
            r#"
// Inject global Zeroclaw object for plugin: {}
globalThis.Zeroclaw = {{
    // The unique identifier for this plugin
    pluginId: "{}",

    // Register an event handler
    // Usage: Zeroclaw.on('event.name', (data) => {{ ... }})
    on: function(eventName, handler) {{
        if (typeof handler !== 'function') {{
            throw new Error('Handler must be a function');
        }}
        // Store the handler for later execution
        // The actual hook registration happens via the native bridge
        if (!typeof globalThis.__zeroclaw_hooks === 'object') {{
            globalThis.__zeroclaw_hooks = {{}};
        }}
        if (!globalThis.__zeroclaw_hooks[eventName]) {{
            globalThis.__zeroclaw_hooks[eventName] = [];
        }}
        globalThis.__zeroclaw_hooks[eventName].push(handler);
    }}
}};
"#,
            plugin_id, plugin_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_default() {
        let ctx = ExecutionContext::default();
        assert_eq!(ctx.session_id, "default");
        assert_eq!(ctx.channel_type, "cli");
        assert!(ctx.user_id.is_none());
    }

    #[test]
    fn context_with_user() {
        let ctx = ExecutionContext::default().with_user("user123".to_string());

        assert_eq!(ctx.user_id, Some("user123".to_string()));
    }

    #[test]
    fn context_with_config() {
        let ctx = ExecutionContext::default()
            .with_config("key".to_string(), Value::String("value".to_string()));

        if let Value::Object(map) = ctx.config {
            assert_eq!(map.get("key"), Some(&Value::String("value".to_string())));
        }
    }

    #[test]
    fn context_to_value() {
        let ctx = ExecutionContext::new("session-123".to_string(), "telegram".to_string())
            .with_user("user-456".to_string());

        let value = ctx.to_value();
        assert_eq!(value["session_id"], "session-123");
        assert_eq!(value["channel_type"], "telegram");
        assert_eq!(value["user_id"], "user-456");
    }

    #[test]
    fn inject_zeroclaw_global_contains_plugin_id() {
        let code = ExecutionContext::inject_zeroclaw_global("test-plugin");
        assert!(code.contains("pluginId: \"test-plugin\""));
    }

    #[test]
    fn inject_zeroclaw_global_creates_global_object() {
        let code = ExecutionContext::inject_zeroclaw_global("my-plugin");
        assert!(code.contains("globalThis.Zeroclaw"));
    }

    #[test]
    fn inject_zeroclaw_global_has_on_method() {
        let code = ExecutionContext::inject_zeroclaw_global("plugin-a");
        assert!(code.contains("on: function"));
    }

    #[test]
    fn inject_zeroclaw_global_validates_handler_is_function() {
        let code = ExecutionContext::inject_zeroclaw_global("plugin-b");
        assert!(code.contains("typeof handler !== 'function'"));
    }
}
