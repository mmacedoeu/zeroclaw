// Plugin installer - unified install workflow

use super::source::PluginSource;
use crate::js::{
    bundle::EsbuildBundler, error::JsPluginError, manifest::PluginManifest,
    registry::ClawHubClient, transpile::OxcTranspiler,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Plugin installation options
#[derive(Debug, Clone)]
pub struct InstallOptions {
    /// Whether to transpile TypeScript to JavaScript
    pub transpile: bool,

    /// Whether to bundle NPM dependencies
    pub bundle: bool,

    /// Whether to verify checksums (for registry downloads)
    pub verify_checksum: bool,

    /// Target directory for installation
    pub target_dir: Option<PathBuf>,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            transpile: true,
            bundle: true,
            verify_checksum: true,
            target_dir: None,
        }
    }
}

/// Result of a plugin installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    /// Plugin name
    pub name: String,

    /// Plugin version
    pub version: String,

    /// Installation path
    pub install_path: PathBuf,

    /// Whether transpilation was performed
    pub transpiled: bool,

    /// Whether bundling was performed
    pub bundled: bool,

    /// Installation metadata
    pub metadata: InstallMetadata,
}

/// Installation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallMetadata {
    /// Source type
    pub source_type: String,

    /// Original source
    pub source: String,

    /// When the plugin was installed
    pub installed_at: String,

    /// Size of installed plugin in bytes
    pub size_bytes: u64,
}

/// Plugin installer
///
/// This installer provides a unified interface for installing plugins from:
/// - Local file paths
/// - Git repositories
/// - ClawHub registry
pub struct PluginInstaller {
    /// Registry client
    registry_client: ClawHubClient,

    /// Transpiler
    transpiler: OxcTranspiler,

    /// Bundler
    bundler: Option<EsbuildBundler>,
}

impl PluginInstaller {
    /// Create a new plugin installer
    pub fn new() -> Self {
        Self {
            registry_client: ClawHubClient::new(),
            transpiler: OxcTranspiler,
            bundler: EsbuildBundler::new().ok(),
        }
    }

    /// Create a new plugin installer with a custom registry client
    pub fn with_registry_client(client: ClawHubClient) -> Self {
        Self {
            registry_client: client,
            transpiler: OxcTranspiler,
            bundler: EsbuildBundler::new().ok(),
        }
    }

    /// Install a plugin from a source
    ///
    /// # Arguments
    ///
    /// * `source` - Plugin source string (path, URL, or registry name)
    /// * `install_dir` - Base directory for installations
    /// * `options` - Installation options
    ///
    /// # Returns
    ///
    /// Installation result with metadata
    pub async fn install(
        &self,
        source: &str,
        install_dir: &Path,
        options: InstallOptions,
    ) -> Result<InstallResult, JsPluginError> {
        let parsed_source = PluginSource::parse(source);

        match parsed_source {
            PluginSource::Local { path } => self.install_local(&path, install_dir, &options).await,
            PluginSource::Git { url, branch } => {
                self.install_git(&url, branch.as_deref(), install_dir, &options)
                    .await
            }
            PluginSource::Registry { name, version } => {
                self.install_registry(&name, version.as_deref(), install_dir, &options)
                    .await
            }
        }
    }

    /// Install from a local path
    async fn install_local(
        &self,
        path: &str,
        install_dir: &Path,
        options: &InstallOptions,
    ) -> Result<InstallResult, JsPluginError> {
        let source_path = PathBuf::from(path);

        if !source_path.exists() {
            return Err(JsPluginError::NotFound {
                name: path.to_string(),
            });
        }

        // Read plugin manifest
        let manifest = self.load_manifest(&source_path)?;

        // Create installation directory
        let plugin_dir = install_dir.join(&manifest.plugin.name);
        std::fs::create_dir_all(&plugin_dir)?;

        // Process entry point
        let entry_point = source_path.join(&manifest.runtime.entry);
        let output_path = plugin_dir.join("index.js");

        self.process_plugin(&entry_point, &output_path, &manifest.plugin.name, options)
            .await?;

        Ok(InstallResult {
            name: manifest.plugin.name.clone(),
            version: manifest.plugin.version.clone(),
            install_path: plugin_dir.clone(),
            transpiled: options.transpile,
            bundled: options.bundle,
            metadata: InstallMetadata {
                source_type: "local".to_string(),
                source: path.to_string(),
                installed_at: chrono::Utc::now().to_rfc3339(),
                size_bytes: self.dir_size(&plugin_dir)?,
            },
        })
    }

    /// Install from a git repository
    async fn install_git(
        &self,
        url: &str,
        branch: Option<&str>,
        install_dir: &Path,
        options: &InstallOptions,
    ) -> Result<InstallResult, JsPluginError> {
        // Create a temporary directory for cloning
        let temp_dir = tempfile::TempDir::new().map_err(|e| JsPluginError::Io(e))?;

        // Check if git is available
        let git_available = std::process::Command::new("git")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);

        if !git_available {
            return Err(JsPluginError::Runtime(
                crate::js::error::JsRuntimeError::Execution(
                    "git not found. Install git to install plugins from git repositories."
                        .to_string(),
                ),
            ));
        }

        // Clone the repository
        let clone_dir = temp_dir.path().join("repo");
        let mut clone_cmd = std::process::Command::new("git");
        clone_cmd.arg("clone").arg(url).arg(&clone_dir);

        if let Some(b) = branch {
            clone_cmd.arg("--branch").arg(b);
        }

        clone_cmd.arg("--depth").arg("1"); // Shallow clone for faster download

        let clone_output = clone_cmd.output().map_err(|e| JsPluginError::Io(e))?;

        if !clone_output.status.success() {
            let stderr = String::from_utf8_lossy(&clone_output.stderr);
            return Err(JsPluginError::Runtime(
                crate::js::error::JsRuntimeError::Execution(format!(
                    "git clone failed: {}",
                    stderr
                )),
            ));
        }

        // Read plugin manifest
        let manifest = self.load_manifest(&clone_dir)?;

        // Create installation directory
        let plugin_dir = install_dir.join(&manifest.plugin.name);
        std::fs::create_dir_all(&plugin_dir)?;

        // Process entry point
        let entry_point = clone_dir.join(&manifest.runtime.entry);
        let output_path = plugin_dir.join("index.js");

        self.process_plugin(&entry_point, &output_path, &manifest.plugin.name, options)
            .await?;

        // Copy plugin.toml to install directory
        let manifest_path = clone_dir.join("plugin.toml");
        if manifest_path.exists() {
            let target_manifest = plugin_dir.join("plugin.toml");
            std::fs::copy(&manifest_path, &target_manifest)?;
        }

        // Copy package.json if it exists (for reference)
        let package_json_path = clone_dir.join("package.json");
        if package_json_path.exists() {
            let target_package_json = plugin_dir.join("package.json");
            std::fs::copy(&package_json_path, &target_package_json)?;

            // Install npm dependencies if needed
            self.install_npm_dependencies(&plugin_dir)?;
        }

        // Calculate final size
        let size_bytes = self.dir_size(&plugin_dir)?;

        Ok(InstallResult {
            name: manifest.plugin.name.clone(),
            version: manifest.plugin.version.clone(),
            install_path: plugin_dir,
            transpiled: options.transpile && entry_point.extension().map_or(false, |e| e == "ts"),
            bundled: options.bundle && self.bundler.is_some(),
            metadata: InstallMetadata {
                source_type: "git".to_string(),
                source: url.to_string(),
                installed_at: chrono::Utc::now().to_rfc3339(),
                size_bytes,
            },
        })
    }

    /// Install from the registry
    async fn install_registry(
        &self,
        name: &str,
        _version: Option<&str>,
        install_dir: &Path,
        options: &InstallOptions,
    ) -> Result<InstallResult, JsPluginError> {
        // Fetch plugin metadata from registry
        let plugin = self.registry_client.get_plugin(name).await?;

        // Download plugin package
        let bytes = self.registry_client.download_plugin(&plugin).await?;

        // Create temporary directory for extraction
        let temp_dir = tempfile::TempDir::new().map_err(|e| JsPluginError::Io(e))?;

        // Detect archive type and extract
        let extracted_dir = self.extract_plugin_package(&bytes, temp_dir.path())?;

        // Parse plugin manifest
        let manifest_path = extracted_dir.join("plugin.toml");
        if !manifest_path.exists() {
            return Err(JsPluginError::Runtime(
                crate::js::error::JsRuntimeError::Execution(
                    "plugin.toml not found in package".to_string(),
                ),
            ));
        }

        let manifest = self.load_manifest(&extracted_dir)?;

        // Install npm dependencies if package.json exists
        let package_json_path = extracted_dir.join("package.json");
        if package_json_path.exists() {
            self.install_npm_dependencies(&extracted_dir)?;
        }

        // Determine entry point
        let entry_point = extracted_dir.join(&manifest.runtime.entry);

        // Create plugin directory
        let plugin_dir = install_dir.join(&manifest.plugin.name);
        std::fs::create_dir_all(&plugin_dir)?;

        // Transpile and bundle the plugin
        let output_path = plugin_dir.join("index.js");
        self.process_plugin(&entry_point, &output_path, &manifest.plugin.name, options)
            .await?;

        // Copy plugin.toml to install directory
        let target_manifest = plugin_dir.join("plugin.toml");
        std::fs::copy(&manifest_path, &target_manifest)?;

        // Copy package.json if it exists (for reference)
        if package_json_path.exists() {
            let target_package_json = plugin_dir.join("package.json");
            std::fs::copy(&package_json_path, &target_package_json)?;
        }

        // Calculate final size
        let size_bytes = self.dir_size(&plugin_dir)?;

        Ok(InstallResult {
            name: manifest.plugin.name.clone(),
            version: manifest.plugin.version.clone(),
            install_path: plugin_dir,
            transpiled: options.transpile && entry_point.extension().map_or(false, |e| e == "ts"),
            bundled: options.bundle && self.bundler.is_some(),
            metadata: InstallMetadata {
                source_type: "registry".to_string(),
                source: name.to_string(),
                installed_at: chrono::Utc::now().to_rfc3339(),
                size_bytes,
            },
        })
    }

    /// Extract a plugin package (ZIP or tar.gz)
    fn extract_plugin_package(
        &self,
        bytes: &[u8],
        dest_dir: &Path,
    ) -> Result<PathBuf, JsPluginError> {
        // Try to detect the archive type by magic bytes
        let is_zip = bytes.len() > 3
            && bytes[0] == 0x50
            && bytes[1] == 0x4b
            && bytes[2] == 0x03
            && bytes[3] == 0x04;

        let is_targz = bytes.len() > 10 && bytes[0] == 0x1f && bytes[1] == 0x8b;

        if is_zip {
            self.extract_zip(bytes, dest_dir)
        } else if is_targz {
            self.extract_targz(bytes, dest_dir)
        } else {
            // Assume it's a tar.gz even if magic bytes don't match
            // (could be compressed with gzip)
            self.extract_targz(bytes, dest_dir)
        }
    }

    /// Extract a ZIP archive
    fn extract_zip(&self, bytes: &[u8], dest_dir: &Path) -> Result<PathBuf, JsPluginError> {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(format!(
                "ZIP error: {}",
                e
            )))
        })?;

        // Extract all files
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| {
                JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(format!(
                    "ZIP entry error: {}",
                    e
                )))
            })?;

            if file.is_dir() {
                let name = file.name();
                let target_path = dest_dir.join(name);
                std::fs::create_dir_all(&target_path)?;
            } else {
                // Create parent directories if needed
                let name = file.name();
                if let Some(parent) = PathBuf::from(name).parent() {
                    let parent_path = dest_dir.join(parent);
                    std::fs::create_dir_all(&parent_path)?;
                }

                let target_path = dest_dir.join(name);

                let mut writer =
                    std::fs::File::create(&target_path).map_err(|e| JsPluginError::Io(e))?;

                std::io::copy(&mut file, &mut writer).map_err(|e| JsPluginError::Io(e))?;
            }
        }

        Ok(dest_dir.to_path_buf())
    }

    /// Extract a tar.gz archive
    fn extract_targz(&self, bytes: &[u8], dest_dir: &Path) -> Result<PathBuf, JsPluginError> {
        use flate2::read::GzDecoder;

        let cursor = std::io::Cursor::new(bytes);
        let decoder = GzDecoder::new(cursor);
        let mut archive = tar::Archive::new(decoder);

        for entry in archive.entries().map_err(|e| JsPluginError::Io(e))? {
            let mut file = entry.map_err(|e| JsPluginError::Io(e))?;

            // Get the file path - unwrap the Result
            let path = file.path().map_err(|e| {
                JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(format!(
                    "tar path error: {}",
                    e
                )))
            })?;

            // Convert to string path
            let path_str = path.to_string_lossy();
            let target_path = dest_dir.join(&*path_str);

            // Check file type using entry_type()
            let file_type = file.header().entry_type();

            if file_type == tar::EntryType::Directory {
                std::fs::create_dir_all(&target_path)?;
            } else {
                // Create parent directories if needed
                if let Some(parent) = target_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                file.unpack(&target_path)
                    .map_err(|e| JsPluginError::Io(e))?;
            }
        }

        Ok(dest_dir.to_path_buf())
    }

    /// Install npm dependencies for a plugin
    fn install_npm_dependencies(&self, plugin_dir: &Path) -> Result<(), JsPluginError> {
        let package_json_path = plugin_dir.join("package.json");

        if !package_json_path.exists() {
            return Ok(());
        }

        // Check if npm is available
        #[cfg(feature = "js-bundle")]
        let npm_available = which::which("npm").is_ok();

        #[cfg(not(feature = "js-bundle"))]
        let npm_available = Self::check_command_exists("npm");

        if !npm_available {
            // If npm is not available, check if node_modules already exists
            let node_modules = plugin_dir.join("node_modules");
            if !node_modules.exists() {
                return Err(JsPluginError::Runtime(
                    crate::js::error::JsRuntimeError::Execution(
                        "npm not found and node_modules doesn't exist. Install npm or install dependencies manually.".to_string()
                    )
                ));
            }
            return Ok(());
        }

        // Run npm install in the plugin directory
        let output = std::process::Command::new("npm")
            .arg("install")
            .arg("--production")
            .current_dir(plugin_dir)
            .output()
            .map_err(|e| JsPluginError::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(JsPluginError::Runtime(
                crate::js::error::JsRuntimeError::Execution(format!(
                    "npm install failed: {}",
                    stderr
                )),
            ));
        }

        Ok(())
    }

    /// Check if a command exists (fallback when `which` crate is not available)
    #[cfg(not(feature = "js-bundle"))]
    fn check_command_exists(cmd: &str) -> bool {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {}", cmd))
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Process a plugin entry point (transpile and optionally bundle)
    async fn process_plugin(
        &self,
        entry_point: &Path,
        output_path: &Path,
        _plugin_name: &str,
        options: &InstallOptions,
    ) -> Result<(), JsPluginError> {
        let source = std::fs::read_to_string(entry_point)?;

        // Transpile if needed
        let code = if options.transpile && entry_point.extension().map_or(false, |e| e == "ts") {
            let result =
                OxcTranspiler::transpile(&source, entry_point.to_str().unwrap_or("plugin.ts"))?;
            result.code
        } else {
            source
        };

        // Write the output
        std::fs::write(output_path, code)?;

        // Bundle if needed and bundler is available
        if options.bundle {
            if let Some(bundler) = &self.bundler {
                let bundle_path = output_path.with_extension("bundled.js");
                let _: crate::js::bundle::BundleOutput =
                    bundler.bundle(output_path, &bundle_path).await?;

                // Replace the output with the bundled version
                std::fs::rename(&bundle_path, output_path)?;
            }
        }

        Ok(())
    }

    /// Load plugin manifest from a directory
    fn load_manifest(&self, plugin_dir: &Path) -> Result<PluginManifest, JsPluginError> {
        let manifest_path = plugin_dir.join("plugin.toml");

        if !manifest_path.exists() {
            return Err(JsPluginError::Runtime(
                crate::js::error::JsRuntimeError::Execution("plugin.toml not found".to_string()),
            ));
        }

        let content = std::fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&content).map_err(|e| {
            JsPluginError::Runtime(crate::js::error::JsRuntimeError::Execution(format!(
                "Failed to parse plugin.toml: {}",
                e
            )))
        })?;

        Ok(manifest)
    }

    /// Calculate directory size
    fn dir_size(&self, path: &Path) -> Result<u64, JsPluginError> {
        fn visit_dirs(
            dir: &Path,
            cb: &mut dyn FnMut(&Path) -> std::io::Result<u64>,
        ) -> std::io::Result<u64> {
            let mut total = 0;
            if dir.is_dir() {
                for entry in std::fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() {
                        total += visit_dirs(&path, cb)?;
                    } else {
                        total += cb(&path)?;
                    }
                }
            }
            Ok(total)
        }

        let size = visit_dirs(path, &mut |p| Ok(std::fs::metadata(p)?.len()))
            .map_err(JsPluginError::Io)?;

        Ok(size)
    }
}

impl Default for PluginInstaller {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_options_default() {
        let options = InstallOptions::default();
        assert!(options.transpile);
        assert!(options.bundle);
        assert!(options.verify_checksum);
        assert!(options.target_dir.is_none());
    }

    #[test]
    fn plugin_installer_can_be_created() {
        let installer = PluginInstaller::new();
        // Just verify it can be created without panicking
        let _ = installer;
    }

    #[test]
    fn install_result_serialization() {
        let result = InstallResult {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            install_path: PathBuf::from("/test/plugins/test-plugin"),
            transpiled: true,
            bundled: true,
            metadata: InstallMetadata {
                source_type: "local".to_string(),
                source: "./test".to_string(),
                installed_at: "2024-01-01T00:00:00Z".to_string(),
                size_bytes: 1000,
            },
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: InstallResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "test-plugin");
        assert_eq!(parsed.version, "1.0.0");
        assert!(parsed.transpiled);
        assert!(parsed.bundled);
    }
}
