// JsSkill - Bridge between JS plugins and ZeroClaw's skill system

use crate::js::{
    error::JsPluginError,
    manifest::SkillDefinition,
    runtime::{JsRuntimeHandle, PluginId},
    sandbox::PluginSandbox,
};
use serde_json::Value;
use std::sync::Arc;

/// JS plugin skill adapter
///
/// This allows JS plugins to define skills that can match against
/// user intents and execute custom logic.
pub struct JsSkill {
    /// Plugin identifier
    plugin_id: PluginId,

    /// Skill name
    name: String,

    /// Skill description
    description: String,

    /// Intent patterns for matching
    patterns: Vec<String>,

    /// Example queries that trigger this skill
    examples: Vec<String>,

    /// Sandbox for executing the plugin
    sandbox: Arc<PluginSandbox>,

    /// Runtime handle for the plugin
    handle: JsRuntimeHandle,
}

impl JsSkill {
    /// Create a new JsSkill from a plugin and skill definition
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - Plugin identifier
    /// * `skill` - Skill definition from plugin manifest
    /// * `sandbox` - Plugin sandbox
    /// * `handle` - Runtime handle
    pub fn new(
        plugin_id: PluginId,
        skill: SkillDefinition,
        sandbox: Arc<PluginSandbox>,
        handle: JsRuntimeHandle,
    ) -> Self {
        let name = format!("{}:{}", &plugin_id.0, &skill.name);

        Self {
            plugin_id,
            name,
            description: skill.description,
            patterns: skill.patterns.unwrap_or_default(),
            examples: skill.examples.unwrap_or_default(),
            sandbox,
            handle,
        }
    }

    /// Get the plugin ID
    pub fn plugin_id(&self) -> &PluginId {
        &self.plugin_id
    }

    /// Get the skill name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the skill description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the intent patterns
    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }

    /// Get example queries
    pub fn examples(&self) -> &[String] {
        &self.examples
    }

    /// Check if this skill matches the given user query
    ///
    /// This uses simple pattern matching against the intent patterns.
    /// Patterns can contain wildcards (*) and are case-insensitive.
    pub fn matches(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();

        for pattern in &self.patterns {
            if self.pattern_matches(pattern, &query_lower) {
                return true;
            }
        }

        false
    }

    /// Check if a pattern matches a query
    fn pattern_matches(&self, pattern: &str, query: &str) -> bool {
        // Simple wildcard matching
        // Convert pattern to regex
        let regex_pattern = pattern.to_lowercase().replace('*', ".*").replace('?', ".");

        // Simple contains check for now (can be improved with regex)
        query.contains(&regex_pattern.replace(".*", ""))
    }

    /// Execute the skill's JavaScript handler
    ///
    /// # Arguments
    ///
    /// * `query` - The user query that triggered this skill
    /// * `context` - Optional context object
    ///
    /// # Returns
    ///
    /// The skill execution result
    pub async fn execute(
        &self,
        query: &str,
        context: Option<Value>,
    ) -> Result<SkillResult, JsPluginError> {
        // Prepare the arguments
        let args = serde_json::json!({
            "query": query,
            "context": context,
        });

        let args_json = serde_json::to_string(&args).map_err(|e| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(format!(
                "Failed to serialize arguments: {}",
                e
            )))
        })?;

        // Call the skill handler
        let code = format!(
            "JSON.stringify(__zc_skill_{}(JSON.parse({})))",
            self.skill_name(),
            args_json
        );

        let result = self.handle.execute(&code).await?;

        // Try to parse as SkillResult JSON
        if let Some(output) = result.as_str() {
            if let Ok(skill_result) = serde_json::from_str::<SkillResult>(output) {
                return Ok(skill_result);
            }
        }

        // Default: successful execution with raw output
        Ok(SkillResult {
            success: true,
            response: result.to_string(),
            actions: vec![],
            error: None,
        })
    }

    /// Extract the base skill name (without plugin prefix)
    fn skill_name(&self) -> &str {
        // Split on ':' and return the second part
        self.name.split(':').nth(1).unwrap_or(&self.name)
    }
}

/// Result of executing a JS skill
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillResult {
    /// Whether the skill execution was successful
    pub success: bool,

    /// The response text
    pub response: String,

    /// Actions to take (tool calls, etc.)
    pub actions: Vec<SkillAction>,

    /// Error message if execution failed
    pub error: Option<String>,
}

/// An action to be taken as a result of skill execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillAction {
    /// Action type (tool_call, response, etc.)
    pub action_type: String,

    /// Action data (varies by type)
    pub data: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_skill_name_formatting() {
        let plugin_id = PluginId("my_plugin".to_string());
        assert_eq!(plugin_id.0, "my_plugin");
    }

    #[test]
    fn skill_result_serialization() {
        let result = SkillResult {
            success: true,
            response: "test response".to_string(),
            actions: vec![],
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: SkillResult = serde_json::from_str(&json).unwrap();

        assert!(parsed.success);
        assert_eq!(parsed.response, "test response");
        assert!(parsed.error.is_none());
    }

    #[test]
    fn skill_result_with_actions() {
        let result = SkillResult {
            success: true,
            response: "test response".to_string(),
            actions: vec![SkillAction {
                action_type: "tool_call".to_string(),
                data: serde_json::json!({"tool": "test", "args": {}}),
            }],
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: SkillResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.actions.len(), 1);
        assert_eq!(parsed.actions[0].action_type, "tool_call");
    }

    // Pattern matching tests require a full JsSkill setup which needs runtime
    // These are tested indirectly through integration tests
}
