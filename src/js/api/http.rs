// HTTP API bridge - proxied fetch for JS plugins

use crate::js::error::{JsPluginError, SandboxViolation};
use serde_json::Value;

/// Proxied HTTP bridge for JS plugins
///
/// This bridge provides a `fetch()` function that is:
/// - Validated against a host allowlist
/// - Proxied through reqwest
/// - Blocked by default unless explicitly permitted
pub struct JsHttpBridge {
    /// Allowed network hosts (e.g., "api.example.com", "api.openai.com")
    allowed_hosts: Vec<String>,

    /// reqwest HTTP client
    client: reqwest::Client,
}

impl JsHttpBridge {
    /// Create a new HTTP bridge with an allowlist
    ///
    /// # Arguments
    ///
    /// * `allowed_hosts` - List of allowed hostnames (subdomain matching)
    pub fn new(allowed_hosts: Vec<String>) -> Self {
        Self {
            allowed_hosts,
            client: reqwest::Client::new(),
        }
    }

    /// Create a bridge with no network access (fetch is blocked)
    pub fn blocked() -> Self {
        Self {
            allowed_hosts: vec![],
            client: reqwest::Client::new(),
        }
    }

    /// Perform an HTTP GET request
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to fetch
    ///
    /// # Returns
    ///
    /// Returns the response body parsed as JSON
    pub async fn fetch(&self, url: &str) -> Result<Value, JsPluginError> {
        // Validate URL and check allowlist
        let parsed = reqwest::Url::parse(url).map_err(|_| {
            JsPluginError::Sandbox(SandboxViolation::NetworkBlocked {
                host: url.to_string(),
            })
        })?;

        let host = parsed.host_str().ok_or_else(|| {
            JsPluginError::Sandbox(SandboxViolation::NetworkBlocked {
                host: url.to_string(),
            })
        })?;

        // Check allowlist
        let allowed = self.is_host_allowed(host);
        if !allowed {
            return Err(JsPluginError::Sandbox(SandboxViolation::NetworkBlocked {
                host: host.to_string(),
            }));
        }

        // Perform fetch
        let response = self.client.get(url).send().await.map_err(|e| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(format!(
                "HTTP request failed: {}",
                e
            )))
        })?;

        let status = response.status();
        if !status.is_success() {
            return Err(JsPluginError::Runtime(
                crate::js::error::JsRuntimeError::Execution(format!(
                    "HTTP error: {}",
                    status.as_u16()
                )),
            ));
        }

        let text = response.text().await.map_err(|e| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(format!(
                "Failed to read response: {}",
                e
            )))
        })?;

        serde_json::from_str(&text).map_err(|_| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(
                "Response was not valid JSON".to_string(),
            ))
        })
    }

    /// Check if a host is in the allowlist
    ///
    /// Supports subdomain matching: "example.com" allows "api.example.com"
    fn is_host_allowed(&self, host: &str) -> bool {
        self.allowed_hosts
            .iter()
            .any(|allowed| host == allowed || host.ends_with(&format!(".{}", allowed)))
    }

    /// Get the list of allowed hosts
    pub fn allowed_hosts(&self) -> &[String] {
        &self.allowed_hosts
    }
}

impl Default for JsHttpBridge {
    fn default() -> Self {
        Self::blocked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_bridge_blocked_by_default() {
        let bridge = JsHttpBridge::default();
        assert!(bridge.allowed_hosts.is_empty());
    }

    #[test]
    fn http_bridge_with_allowed_hosts() {
        let bridge = JsHttpBridge::new(vec![
            "api.example.com".to_string(),
            "api.openai.com".to_string(),
        ]);

        assert_eq!(bridge.allowed_hosts().len(), 2);
        assert!(bridge.is_host_allowed("api.example.com"));
        assert!(bridge.is_host_allowed("sub.api.example.com"));
        assert!(!bridge.is_host_allowed("evil.com"));
    }

    #[test]
    fn http_bridge_exact_match() {
        let bridge = JsHttpBridge::new(vec!["api.example.com".to_string()]);

        assert!(bridge.is_host_allowed("api.example.com"));
        assert!(!bridge.is_host_allowed("other.com"));
    }

    #[tokio::test]
    async fn http_bridge_blocks_unallowed_url() {
        let bridge = JsHttpBridge::new(vec!["api.example.com".to_string()]);

        let result = bridge.fetch("https://evil.com/data").await;
        assert!(result.is_err());
        match result {
            Err(JsPluginError::Sandbox(SandboxViolation::NetworkBlocked { .. })) => {
                // Expected
            }
            _ => panic!("Expected NetworkBlocked error"),
        }
    }
}
