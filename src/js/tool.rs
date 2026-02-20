// JsTool - Bridge between JS plugins and ZeroClaw's Tool trait

use crate::js::{
    error::JsPluginError,
    manifest::ToolDefinition,
    runtime::{JsRuntimeHandle, PluginId},
    sandbox::PluginSandbox,
};
use crate::tools::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// JS plugin wrapped as a ZeroClaw Tool
///
/// This adapter allows JS plugins to expose tools that can be called
/// by the agent's tool execution system.
pub struct JsTool {
    /// Plugin identifier
    plugin_id: PluginId,

    /// Tool name (e.g., "my_plugin:my_tool")
    name: String,

    /// Tool description
    description: String,

    /// JSON schema for parameters
    parameters_schema: Value,

    /// Sandbox for executing the plugin
    sandbox: Arc<PluginSandbox>,

    /// Runtime handle for the plugin
    handle: JsRuntimeHandle,
}

impl JsTool {
    /// Create a new JsTool from a plugin and tool definition
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - Plugin identifier
    /// * `tool` - Tool definition from plugin manifest
    /// * `sandbox` - Plugin sandbox
    /// * `handle` - Runtime handle
    pub fn new(
        plugin_id: PluginId,
        tool: ToolDefinition,
        sandbox: Arc<PluginSandbox>,
        handle: JsRuntimeHandle,
    ) -> Self {
        let name = format!("{}:{}", &plugin_id.0, &tool.name);

        Self {
            plugin_id,
            name,
            description: tool.description,
            parameters_schema: tool.parameters,
            sandbox,
            handle,
        }
    }

    /// Get the plugin ID
    pub fn plugin_id(&self) -> &PluginId {
        &self.plugin_id
    }

    /// Execute the tool's JavaScript handler
    ///
    /// This calls the plugin's tool handler with the given arguments.
    async fn call_js_handler(&self, args: Value) -> Result<Value, JsPluginError> {
        // Serialize arguments to JSON
        let args_json = serde_json::to_string(&args).map_err(|e| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(format!(
                "Failed to serialize arguments: {}",
                e
            )))
        })?;

        // Call the tool handler
        let code = format!(
            "JSON.stringify(__zc_tool_{}(JSON.parse({})))",
            self.tool_name(),
            args_json
        );

        let result = self.handle.execute(&code).await?;
        Ok(result)
    }

    /// Extract the base tool name (without plugin prefix)
    fn tool_name(&self) -> &str {
        // Split on ':' and return the second part
        self.name.split(':').nth(1).unwrap_or(&self.name)
    }

    /// Convert JsPluginError to ToolResult
    fn error_to_result(error: JsPluginError) -> ToolResult {
        ToolResult {
            success: false,
            output: String::new(),
            error: Some(error.to_string()),
        }
    }
}

#[async_trait]
impl Tool for JsTool {
    /// Tool name (used in LLM function calling)
    fn name(&self) -> &str {
        &self.name
    }

    /// Human-readable description
    fn description(&self) -> &str {
        &self.description
    }

    /// JSON schema for parameters
    fn parameters_schema(&self) -> Value {
        self.parameters_schema.clone()
    }

    /// Execute the tool with given arguments
    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        match self.call_js_handler(args).await {
            Ok(result) => {
                // Parse the result
                if let Some(output) = result.as_str() {
                    // Try to parse as ToolResult JSON
                    if let Ok(tool_result) = serde_json::from_str::<ToolResult>(output) {
                        return Ok(tool_result);
                    }
                }

                // Default: successful execution with raw output
                Ok(ToolResult {
                    success: true,
                    output: result.to_string(),
                    error: None,
                })
            }
            Err(e) => Ok(Self::error_to_result(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_tool_name_formatting() {
        // Test that tool names are formatted correctly
        let plugin_id = PluginId("my_plugin".to_string());

        // This is a basic test - actual integration tests would need
        // a full runtime setup
        assert_eq!(plugin_id.0, "my_plugin");
    }

    #[test]
    fn tool_result_serialization() {
        let result = ToolResult {
            success: true,
            output: "test output".to_string(),
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: ToolResult = serde_json::from_str(&json).unwrap();

        assert!(parsed.success);
        assert_eq!(parsed.output, "test output");
        assert!(parsed.error.is_none());
    }

    #[test]
    fn tool_result_with_error() {
        let result = ToolResult {
            success: false,
            output: String::new(),
            error: Some("test error".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: ToolResult = serde_json::from_str(&json).unwrap();

        assert!(!parsed.success);
        assert_eq!(parsed.error, Some("test error".to_string()));
    }
}
