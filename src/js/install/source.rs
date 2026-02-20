// Plugin source types - local files, git URLs, registry names

use serde::{Deserialize, Serialize};

/// Plugin installation source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginSource {
    /// Local file path (e.g., "./plugins/my-plugin")
    Local { path: String },

    /// Git repository URL (e.g., "https://github.com/user/plugin")
    Git { url: String, branch: Option<String> },

    /// Registry name (e.g., "@user/plugin" or "plugin")
    Registry {
        name: String,
        version: Option<String>,
    },
}

impl PluginSource {
    /// Parse a source string into a PluginSource
    ///
    /// # Arguments
    ///
    /// * `source` - Source string to parse
    ///
    /// # Returns
    ///
    /// A parsed PluginSource
    ///
    /// # Examples
    ///
    /// ```
    /// # use zeroclaw::js::install::PluginSource;
    ///
    /// // Local path
    /// let source = PluginSource::parse("./my-plugin");
    /// assert!(matches!(source, PluginSource::Local { .. }));
    ///
    /// // Git URL
    /// let source = PluginSource::parse("https://github.com/user/plugin");
    /// assert!(matches!(source, PluginSource::Git { .. }));
    ///
    /// // Registry name
    /// let source = PluginSource::parse("@user/plugin");
    /// assert!(matches!(source, PluginSource::Registry { .. }));
    /// ```
    pub fn parse(source: &str) -> Self {
        // Check for absolute or relative local paths first
        // Paths starting with /, ./, ../, or ~ are local
        if source.starts_with('/')
            || source.starts_with("./")
            || source.starts_with("../")
            || source.starts_with("~/")
        {
            return PluginSource::Local {
                path: source.to_string(),
            };
        }

        // Check if it's a scoped registry package (@user/plugin)
        if source.starts_with('@') {
            return PluginSource::Registry {
                name: source.to_string(),
                version: None,
            };
        }

        // Check if it's a git URL
        if source.starts_with("git://")
            || source.starts_with("https://github.com/")
            || source.starts_with("git+https://")
            || source.ends_with(".git")
        {
            return PluginSource::Git {
                url: source.to_string(),
                branch: None,
            };
        }

        // Check if it contains a / and has no extension - likely a registry package
        if source.contains('/') && !source.contains('.') {
            return PluginSource::Registry {
                name: source.to_string(),
                version: None,
            };
        }

        // Default to local path
        PluginSource::Local {
            path: source.to_string(),
        }
    }

    /// Parse a source string with a specific version (for registry packages)
    pub fn parse_with_version(source: &str, version: Option<String>) -> Self {
        let base = Self::parse(source);

        match base {
            PluginSource::Registry { name, .. } => PluginSource::Registry { name, version },
            _ => base,
        }
    }
}

/// Shorthand type for PluginSource
pub type InstallSource = PluginSource;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_local_path() {
        let source = PluginSource::parse("./my-plugin");
        assert!(matches!(source, PluginSource::Local { .. }));

        if let PluginSource::Local { path } = source {
            assert_eq!(path, "./my-plugin");
        }
    }

    #[test]
    fn parse_absolute_path() {
        let source = PluginSource::parse("/home/user/plugins/my-plugin");
        assert!(matches!(source, PluginSource::Local { .. }));
    }

    #[test]
    fn parse_git_url() {
        let source = PluginSource::parse("https://github.com/user/plugin");
        assert!(matches!(source, PluginSource::Git { .. }));

        if let PluginSource::Git { url, .. } = source {
            assert_eq!(url, "https://github.com/user/plugin");
        }
    }

    #[test]
    fn parse_git_url_with_git_extension() {
        let source = PluginSource::parse("https://example.com/repo.git");
        assert!(matches!(source, PluginSource::Git { .. }));
    }

    #[test]
    fn parse_registry_scoped() {
        let source = PluginSource::parse("@user/plugin");
        assert!(matches!(source, PluginSource::Registry { .. }));

        if let PluginSource::Registry { name, .. } = source {
            assert_eq!(name, "@user/plugin");
        }
    }

    #[test]
    fn parse_registry_unscoped() {
        let source = PluginSource::parse("user/plugin");
        assert!(matches!(source, PluginSource::Registry { .. }));

        if let PluginSource::Registry { name, .. } = source {
            assert_eq!(name, "user/plugin");
        }
    }

    #[test]
    fn parse_with_version() {
        let source = PluginSource::parse_with_version("@user/plugin", Some("1.0.0".to_string()));
        assert!(matches!(source, PluginSource::Registry { .. }));

        if let PluginSource::Registry { version, .. } = source {
            assert_eq!(version, Some("1.0.0".to_string()));
        }
    }

    #[test]
    fn parse_with_version_preserves_git() {
        let source = PluginSource::parse_with_version(
            "https://github.com/user/plugin",
            Some("main".to_string()),
        );
        // Git sources don't use versions, but branch is specified separately
        assert!(matches!(source, PluginSource::Git { .. }));
    }

    #[test]
    fn plugin_source_serialization() {
        let source = PluginSource::Local {
            path: "./my-plugin".to_string(),
        };

        let json = serde_json::to_string(&source).unwrap();
        let parsed: PluginSource = serde_json::from_str(&json).unwrap();

        assert!(matches!(parsed, PluginSource::Local { .. }));
    }
}
