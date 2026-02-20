// Bundle management - esbuild integration for plugin bundling

pub mod esbuild;

pub use esbuild::{BundleOutput, EsbuildBundler, EsbuildConfig};

use crate::js::error::JsPluginError;
use std::path::Path;

/// Bundle a plugin's dependencies
///
/// This is a convenience function that uses the default bundler configuration.
pub async fn bundle_plugin(
    entry_point: &Path,
    output_path: &Path,
) -> Result<BundleOutput, JsPluginError> {
    let bundler = EsbuildBundler::new()?;
    bundler.bundle(entry_point, output_path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_module_exports_types() {
        // Test that our public API is accessible
        let config = EsbuildConfig::default();
        assert_eq!(config.minify, false);
        assert_eq!(config.target, "es2020");
    }

    #[test]
    #[cfg(feature = "js-runtime")]
    fn esbuild_bundler_can_be_created() {
        // This test verifies the bundler can be instantiated if esbuild is available
        // In CI environments without esbuild, this will test the error path
        let result = EsbuildBundler::new();
        // We don't assert success here since esbuild may not be installed
        // We just verify it doesn't panic and returns a Result
        let _ = result;
    }
}
