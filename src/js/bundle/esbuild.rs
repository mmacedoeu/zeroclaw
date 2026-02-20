// esbuild integration - bundles npm dependencies at install time

use crate::js::error::{BundleError, JsPluginError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;

/// esbuild bundler configuration
#[derive(Debug, Clone)]
pub struct EsbuildConfig {
    /// ECMAScript target version
    pub target: String,

    /// Whether to minify the output
    pub minify: bool,

    /// Output format (iife, cjs, esm)
    pub format: String,

    /// External modules to exclude from bundling
    pub external: Vec<String>,

    /// Additional esbuild arguments
    pub extra_args: Vec<String>,
}

impl Default for EsbuildConfig {
    fn default() -> Self {
        Self {
            target: "es2020".to_string(),
            minify: false,
            format: "iife".to_string(),
            external: vec![],
            extra_args: vec![],
        }
    }
}

/// Result of a bundling operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleOutput {
    /// Path to the bundled file
    pub output_path: PathBuf,

    /// Original source file size in bytes
    pub input_size: u64,

    /// Bundled output size in bytes
    pub output_size: u64,

    /// Whether source maps were generated
    pub has_source_map: bool,
}

/// esbuild bundler for JavaScript/TypeScript plugins
///
/// This bundler wraps the esbuild CLI to bundle npm dependencies
/// at plugin install time.
pub struct EsbuildBundler {
    /// Path to the esbuild executable
    esbuild_path: PathBuf,

    /// Bundler configuration
    config: EsbuildConfig,
}

impl EsbuildBundler {
    /// Create a new bundler by detecting esbuild
    ///
    /// # Errors
    ///
    /// Returns `BundleError::EsbuildNotFound` if esbuild is not installed
    /// or not in PATH.
    pub fn new() -> Result<Self, BundleError> {
        Self::with_config(EsbuildConfig::default())
    }

    /// Create a new bundler with a specific configuration
    pub fn with_config(config: EsbuildConfig) -> Result<Self, BundleError> {
        let esbuild_path = which("esbuild").map_err(|_| BundleError::EsbuildNotFound)?;

        Ok(Self {
            esbuild_path,
            config,
        })
    }

    /// Create a bundler with a specific esbuild path
    ///
    /// This is useful for testing or when esbuild is not in PATH.
    pub fn with_esbuild_path(esbuild_path: PathBuf) -> Self {
        Self {
            esbuild_path,
            config: EsbuildConfig::default(),
        }
    }

    /// Bundle a JavaScript/TypeScript entry point
    ///
    /// # Arguments
    ///
    /// * `entry_point` - Path to the input file (e.g., plugin.js)
    /// * `output_path` - Path where the bundle should be written
    ///
    /// # Errors
    ///
    /// Returns `BundleError::BundleFailed` if esbuild exits with a non-zero code.
    pub async fn bundle(
        &self,
        entry_point: &Path,
        output_path: &Path,
    ) -> Result<BundleOutput, JsPluginError> {
        // Get input file size
        let input_size = std::fs::metadata(entry_point).map(|m| m.len()).unwrap_or(0);

        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                BundleError::BundleFailed(format!("Failed to create output directory: {}", e))
            })?;
        }

        // Build esbuild command
        let mut cmd = Command::new(&self.esbuild_path);

        // Add entry point
        cmd.arg(entry_point);

        // Add output path
        cmd.arg("--bundle");
        cmd.arg(&format!("--outfile={}", output_path.display()));

        // Add target
        cmd.arg(&format!("--target={}", self.config.target));

        // Add format
        cmd.arg(&format!("--format={}", self.config.format));

        // Add minify flag if enabled
        if self.config.minify {
            cmd.arg("--minify");
        }

        // Add external modules
        for ext in &self.config.external {
            cmd.arg(&format!("--external:{}", ext));
        }

        // Add extra arguments
        for arg in &self.config.extra_args {
            cmd.arg(arg);
        }

        // Run esbuild
        let output = cmd
            .output()
            .map_err(|e| BundleError::BundleFailed(format!("Failed to execute esbuild: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BundleError::BundleFailed(format!("esbuild failed: {}", stderr)).into());
        }

        // Get output file size
        let output_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

        // Check for source map
        let map_path = PathBuf::from(format!("{}.map", output_path.display()));
        let has_source_map = map_path.exists();

        Ok(BundleOutput {
            output_path: output_path.to_path_buf(),
            input_size,
            output_size,
            has_source_map,
        })
    }

    /// Get a reference to the bundler configuration
    pub fn config(&self) -> &EsbuildConfig {
        &self.config
    }

    /// Set a new configuration
    pub fn set_config(&mut self, config: EsbuildConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esbuild_config_default() {
        let config = EsbuildConfig::default();
        assert_eq!(config.target, "es2020");
        assert_eq!(config.format, "iife");
        assert!(!config.minify);
        assert!(config.external.is_empty());
    }

    #[test]
    fn esbuild_config_with_minify() {
        let config = EsbuildConfig {
            minify: true,
            ..Default::default()
        };
        assert!(config.minify);
    }

    #[test]
    fn esbuild_config_with_external() {
        let config = EsbuildConfig {
            external: vec!["lodash".to_string(), "axios".to_string()],
            ..Default::default()
        };
        assert_eq!(config.external.len(), 2);
    }

    #[test]
    fn bundle_output_serialization() {
        let output = BundleOutput {
            output_path: PathBuf::from("/test/bundle.js"),
            input_size: 1000,
            output_size: 500,
            has_source_map: true,
        };

        let json = serde_json::to_string(&output).unwrap();
        let parsed: BundleOutput = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.output_path, PathBuf::from("/test/bundle.js"));
        assert_eq!(parsed.input_size, 1000);
        assert_eq!(parsed.output_size, 500);
        assert!(parsed.has_source_map);
    }

    #[tokio::test]
    async fn esbuild_bundler_without_esbuild() {
        // Test the error path when esbuild is not available
        // We use a non-existent path to simulate this
        let bundler = EsbuildBundler::with_esbuild_path(PathBuf::from("/nonexistent/esbuild"));

        // Create a temporary input file
        let temp_dir = std::env::temp_dir();
        let input = temp_dir.join("test-input.js");
        let output = temp_dir.join("test-output.js");

        std::fs::write(&input, "console.log('test');").unwrap();

        let result = bundler.bundle(&input, &output).await;
        assert!(result.is_err());
    }
}
