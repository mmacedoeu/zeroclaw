// CLI commands for JS plugin management

use crate::js::{
    install::{InstallOptions, PluginInstaller, PluginSource},
    registry::ClawHubClient,
};
use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

/// Plugin management subcommands
#[derive(Subcommand, Debug)]
pub enum PluginCommands {
    /// Install a plugin from a source (local path, git URL, or registry name)
    Install {
        /// Plugin source (e.g., ./my-plugin, https://github.com/user/plugin, @user/plugin)
        source: String,

        /// Installation directory (default: ~/.zeroclaw/plugins)
        #[arg(short, long)]
        dir: Option<PathBuf>,

        /// Skip transpilation
        #[arg(long)]
        no_transpile: bool,

        /// Skip bundling
        #[arg(long)]
        no_bundle: bool,

        /// Specific version (for registry packages)
        #[arg(long)]
        version: Option<String>,
    },

    /// List installed plugins
    List {
        /// Installation directory (default: ~/.zeroclaw/plugins)
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },

    /// Remove an installed plugin
    Remove {
        /// Plugin name to remove
        name: String,

        /// Installation directory (default: ~/.zeroclaw/plugins)
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },

    /// Search for plugins in the registry
    Search {
        /// Search query
        query: String,

        /// Maximum results to return
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
}

impl PluginCommands {
    /// Execute the plugin command
    pub async fn run(self) -> Result<()> {
        match self {
            PluginCommands::Install {
                source,
                dir,
                no_transpile,
                no_bundle,
                version,
            } => Self::install_cmd(source, dir, no_transpile, no_bundle, version).await,
            PluginCommands::List { dir } => Self::list_cmd(dir),
            PluginCommands::Remove { name, dir } => Self::remove_cmd(name, dir),
            PluginCommands::Search { query, limit } => Self::search_cmd(query, limit).await,
        }
    }

    async fn install_cmd(
        source: String,
        dir: Option<PathBuf>,
        no_transpile: bool,
        no_bundle: bool,
        version: Option<String>,
    ) -> Result<()> {
        let install_dir = dir.unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".zeroclaw")
                .join("plugins")
        });

        let source_str = source.clone();
        let _source = if let Some(ver) = version {
            PluginSource::parse_with_version(&source_str, Some(ver))
        } else {
            PluginSource::parse(&source_str)
        };

        println!("Installing plugin from: {}", source_str);

        let options = InstallOptions {
            transpile: !no_transpile,
            bundle: !no_bundle,
            ..Default::default()
        };

        let installer = PluginInstaller::new();
        let result = installer
            .install(&source_str, &install_dir, options)
            .await?;

        println!(
            "✓ Plugin '{}' v{} installed successfully",
            result.name, result.version
        );
        println!("  Location: {}", result.install_path.display());

        if result.transpiled {
            println!("  Transpiled: yes");
        }
        if result.bundled {
            println!("  Bundled: yes");
        }

        Ok(())
    }

    fn list_cmd(dir: Option<PathBuf>) -> Result<()> {
        let install_dir = dir.unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".zeroclaw")
                .join("plugins")
        });

        if !install_dir.exists() {
            println!("No plugins installed.");
            return Ok(());
        }

        let entries = std::fs::read_dir(&install_dir)?;

        let mut plugins: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        if plugins.is_empty() {
            println!("No plugins installed.");
        } else {
            println!("Installed plugins ({}):", plugins.len());
            for entry in &mut plugins {
                let name = entry.file_name().to_string_lossy().to_string();
                println!("  - {}", name);
            }
        }

        Ok(())
    }

    fn remove_cmd(name: String, dir: Option<PathBuf>) -> Result<()> {
        let install_dir = dir.unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".zeroclaw")
                .join("plugins")
        });

        let plugin_path = install_dir.join(&name);

        if !plugin_path.exists() {
            println!("Plugin '{}' not found.", name);
            return Ok(());
        }

        std::fs::remove_dir_all(&plugin_path)?;
        println!("✓ Plugin '{}' removed successfully.", name);

        Ok(())
    }

    async fn search_cmd(query: String, limit: usize) -> Result<()> {
        let client = ClawHubClient::new();
        let results: Vec<crate::js::registry::SearchResult> = client.search(&query).await?;

        if results.is_empty() {
            println!("No plugins found for '{}'.", query);
        } else {
            let count = results.len().min(limit);
            println!("Found {} plugin(s) for '{}':", count, query);

            for result in results.iter().take(count) {
                println!(
                    "  {} - {} (by {}, downloads: {})",
                    result.name, result.description, result.author, result.downloads
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_source_parsing() {
        let source = PluginSource::parse("@user/plugin");
        assert!(matches!(source, PluginSource::Registry { .. }));

        let source = PluginSource::parse("./local-plugin");
        assert!(matches!(source, PluginSource::Local { .. }));

        let source = PluginSource::parse("https://github.com/user/plugin");
        assert!(matches!(source, PluginSource::Git { .. }));
    }
}
