// Sandbox permissions - maps JS plugin permissions to ZeroClaw Security Policy

use crate::js::config::JsPluginPermissions;

/// Convert JS plugin permissions to ZeroClaw security policy
///
/// This maps the plugin's declared permissions to ZeroClaw's unified
/// security model, enabling consistent enforcement across all extensions.
///
/// Note: This is currently a stub implementation that returns the permissions
/// as-is. Full integration with ZeroClaw's security module will be added later.
impl From<JsPluginPermissions> for SandboxPermissions {
    fn from(perm: JsPluginPermissions) -> Self {
        SandboxPermissions {
            network: perm.network.clone(),
            file_read: perm.file_read.clone(),
            file_write: perm.file_write,
            env_vars: perm.env_vars.clone(),
        }
    }
}

/// Sandbox permissions (mirrors JsPluginPermissions)
#[derive(Debug, Clone)]
pub struct SandboxPermissions {
    pub network: Vec<String>,
    pub file_read: Vec<std::path::PathBuf>,
    pub file_write: bool,
    pub env_vars: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_with_network() {
        let js_perms = JsPluginPermissions {
            network: vec!["api.example.com".to_string(), "api.openai.com".to_string()],
            file_read: vec![],
            file_write: false,
            env_vars: vec![],
        };

        let sandbox_perms: SandboxPermissions = js_perms.into();
        assert_eq!(sandbox_perms.network.len(), 2);
        assert!(sandbox_perms.file_read.is_empty());
        assert!(!sandbox_perms.file_write);
    }

    #[test]
    fn permissions_no_write() {
        let js_perms = JsPluginPermissions {
            network: vec![],
            file_read: vec![],
            file_write: false,
            env_vars: vec![],
        };

        let sandbox_perms: SandboxPermissions = js_perms.into();
        assert!(!sandbox_perms.file_write);
    }

    #[test]
    fn permissions_with_env_vars() {
        let js_perms = JsPluginPermissions {
            network: vec![],
            file_read: vec![],
            file_write: false,
            env_vars: vec!["API_KEY".to_string(), "DATABASE_URL".to_string()],
        };

        let sandbox_perms: SandboxPermissions = js_perms.into();
        assert_eq!(sandbox_perms.env_vars.len(), 2);
    }
}
