// Plugin installer - unified install workflow for JS plugins

pub mod installer;
pub mod source;

pub use installer::{InstallOptions, InstallResult, PluginInstaller};
pub use source::{InstallSource, PluginSource};

use crate::js::error::JsPluginError;
use std::path::Path;

/// Install a plugin from a source
///
/// This is a convenience function that uses the default installer configuration.
///
/// # Arguments
///
/// * `source` - The plugin source (local path, git URL, or registry name)
/// * `install_dir` - Directory where the plugin should be installed
///
/// # Returns
///
/// The installed plugin metadata.
pub async fn install_plugin(
    source: &str,
    install_dir: &Path,
) -> Result<InstallResult, JsPluginError> {
    let installer = PluginInstaller::new();
    installer
        .install(source, install_dir, InstallOptions::default())
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_module_exports_types() {
        // Test that our public API is accessible
        let options = InstallOptions::default();
        assert_eq!(options.transpile, true);
        assert_eq!(options.bundle, true);
    }
}
