// Plugin manifest parsing (plugin.toml + SKILL.md)

use serde::Deserialize;
use std::path::PathBuf;

/// Parsed plugin.toml manifest
#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginMetadata,
    pub runtime: RuntimeManifestConfig,
    #[serde(default)]
    pub permissions: PluginPermissions,
    #[serde(default)]
    pub tools: ToolDefinitions,
    #[serde(default)]
    pub skills: SkillDefinitions,
}

/// Plugin metadata section
#[derive(Debug, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub openclaw_compatible: bool,
    #[serde(default)]
    pub openclaw_skill_id: Option<String>,
}

/// Runtime configuration section
#[derive(Debug, Deserialize)]
pub struct RuntimeManifestConfig {
    /// Entry point file (e.g., "src/index.ts")
    pub entry: PathBuf,
    /// Optional SDK version requirement
    #[serde(default)]
    pub sdk_version: Option<String>,
}

/// Plugin permissions section
#[derive(Debug, Deserialize, Default)]
pub struct PluginPermissions {
    /// Allowed network hosts
    #[serde(default)]
    pub network: Vec<String>,
    /// Allowed file read paths (glob patterns)
    #[serde(default)]
    pub file_read: Vec<String>,
    /// Allow file write
    #[serde(default)]
    pub file_write: bool,
    /// Allowed environment variables
    #[serde(default)]
    pub env_vars: Vec<String>,
}

/// Tool definitions section
#[derive(Debug, Deserialize, Default)]
pub struct ToolDefinitions {
    /// Tool definitions
    #[serde(default)]
    pub definitions: Vec<ToolDefinition>,
}

/// Individual tool definition
#[derive(Debug, Deserialize, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Skill definitions section
#[derive(Debug, Deserialize, Default)]
pub struct SkillDefinitions {
    /// Skill definitions
    #[serde(default)]
    pub definitions: Vec<SkillDefinition>,
}

/// Individual skill definition
#[derive(Debug, Deserialize, Clone)]
pub struct SkillDefinition {
    pub name: String,
    pub description: String,
    /// Intent patterns for matching
    #[serde(default)]
    pub patterns: Option<Vec<String>>,
    /// Example queries that trigger this skill
    #[serde(default)]
    pub examples: Option<Vec<String>>,
}

impl PluginManifest {
    /// Parse plugin.toml from a file path
    pub fn from_file(path: &PathBuf) -> Result<Self, ParseError> {
        let content = std::fs::read_to_string(path).map_err(|e| ParseError::Io {
            path: path.clone(),
            error: e.to_string(),
        })?;

        let manifest: PluginManifest = toml::from_str(&content).map_err(|e| ParseError::Toml {
            path: path.clone(),
            error: e.to_string(),
        })?;

        Ok(manifest)
    }

    /// Parse plugin.toml from a string
    pub fn from_str(content: &str) -> Result<Self, ParseError> {
        toml::from_str(content).map_err(|e| ParseError::Toml {
            path: "<string>".into(),
            error: e.to_string(),
        })
    }

    /// Validate the manifest
    pub fn validate(&self) -> Result<(), ParseError> {
        if self.plugin.name.is_empty() {
            return Err(ParseError::Validation("Plugin name cannot be empty".into()));
        }
        if self.plugin.name.contains(' ') || self.plugin.name.contains('/') {
            return Err(ParseError::Validation(
                "Plugin name must not contain spaces or slashes".into(),
            ));
        }
        if self.runtime.entry.as_os_str().is_empty() {
            return Err(ParseError::Validation(
                "Runtime entry point cannot be empty".into(),
            ));
        }
        Ok(())
    }
}

/// Manifest parsing errors
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("I/O error reading {path}: {error}")]
    Io { path: PathBuf, error: String },

    #[error("TOML parsing error in {path}: {error}")]
    Toml { path: PathBuf, error: String },

    #[error("Validation error: {0}")]
    Validation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_MANIFEST: &str = r#"
[plugin]
name = "test-plugin"
version = "1.0.0"
description = "A test plugin"
author = "Test Author"
license = "MIT"

[runtime]
entry = "src/index.ts"
sdk_version = "^3.0.0"

[permissions]
network = ["api.example.com"]
file_read = ["./data/**"]
file_write = false
env_vars = ["API_KEY"]

[[tools.definitions]]
name = "test_tool"
description = "A test tool"

[tools.definitions.parameters]
type = "object"
properties.query = { type = "string" }
required = ["query"]
"#;

    #[test]
    fn parse_valid_manifest() {
        let manifest =
            PluginManifest::from_str(VALID_MANIFEST).expect("Should parse valid manifest");

        assert_eq!(manifest.plugin.name, "test-plugin");
        assert_eq!(manifest.plugin.version, "1.0.0");
        assert_eq!(manifest.runtime.entry, PathBuf::from("src/index.ts"));
        assert_eq!(manifest.permissions.network.len(), 1);
        assert_eq!(manifest.tools.definitions.len(), 1);
    }

    #[test]
    fn validate_accepts_valid_manifest() {
        let manifest =
            PluginManifest::from_str(VALID_MANIFEST).expect("Should parse valid manifest");
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn validate_rejects_empty_name() {
        let invalid = r#"
[plugin]
name = ""
version = "1.0.0"
description = "A test plugin"
author = "Test Author"

[runtime]
entry = "src/index.ts"
"#;
        let manifest = PluginManifest::from_str(invalid).unwrap();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn validate_rejects_name_with_spaces() {
        let invalid = r#"
[plugin]
name = "test plugin"
version = "1.0.0"
description = "A test plugin"
author = "Test Author"

[runtime]
entry = "src/index.ts"
"#;
        let manifest = PluginManifest::from_str(invalid).unwrap();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn parse_default_permissions() {
        let minimal = r#"
[plugin]
name = "test"
version = "1.0.0"
description = "Test"
author = "Test"

[runtime]
entry = "index.ts"
"#;

        let manifest = PluginManifest::from_str(minimal).unwrap();
        assert!(manifest.permissions.network.is_empty());
        assert!(!manifest.permissions.file_write);
        assert!(manifest.tools.definitions.is_empty());
    }

    #[test]
    fn parse_invalid_toml() {
        let invalid = r#"
[plugin
name = "test
"#;

        let result = PluginManifest::from_str(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn tool_definition_serialization() {
        let manifest = PluginManifest::from_str(VALID_MANIFEST).unwrap();
        let tool = &manifest.tools.definitions[0];

        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, "A test tool");

        // Verify parameters are valid JSON
        let params = &tool.parameters;
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["query"]["type"] == "string");
    }
}
