# JavaScript/TypeScript Plugins - User Guide

This guide covers installing, managing, and using JavaScript/TypeScript plugins with ZeroClaw.

## Table of Contents

- [Installation](#installation)
- [CLI Reference](#cli-reference)
- [Plugin Sources](#plugin-sources)
- [Troubleshooting](#troubleshooting)
- [Security](#security)

## Installation

### Installing from ClawHub Registry

Search and install plugins from the official registry:

```bash
# Search for plugins
zeroclaw plugin search weather

# Install a plugin
zeroclaw plugin install @user/weather-plugin

# Install specific version
zeroclaw plugin install @user/weather-plugin --version 2.0.0
```

### Installing from Git

Install plugins directly from Git repositories:

```bash
# Install from GitHub
zeroclaw plugin install https://github.com/user/plugin.git

# Install from Git with branch
zeroclaw plugin install git+https://github.com/user/plugin.git#main
```

### Installing from Local Path

Install plugins from a local directory:

```bash
# From relative path
zeroclaw plugin install ./my-plugin

# From absolute path
zeroclaw plugin install /home/user/plugins/my-plugin
```

## CLI Reference

### Install Command

```bash
zeroclaw plugin install [OPTIONS] <source>
```

**Options:**
- `-d, --dir <dir>` - Installation directory (default: ~/.zeroclaw/plugins)
- `--no-transpile` - Skip TypeScript transpilation
- `--no-bundle` - Skip NPM dependency bundling
- `--version <ver>` - Specific version (registry packages only)

**Examples:**

```bash
# Install with defaults
zeroclaw plugin install @user/my-plugin

# Install to custom directory
zeroclaw plugin install ./my-plugin --dir ./custom-plugins

# Install without bundling
zeroclaw plugin install @user/my-plugin --no-bundle
```

### List Command

List installed plugins:

```bash
zeroclaw plugin list

# List from custom directory
zeroclaw plugin list --dir ./custom-plugins
```

**Output:**

```
Installed plugins (3):
  - @user/weather-plugin
  - @user/time-plugin
  - my-local-plugin
```

### Remove Command

Remove an installed plugin:

```bash
zeroclaw plugin remove <name>

# Remove from custom directory
zeroclaw plugin remove my-plugin --dir ./custom-plugins
```

**Output:**

```
✓ Plugin 'my-plugin' removed successfully.
```

### Search Command

Search the ClawHub registry:

```bash
zeroclaw plugin search <query> [OPTIONS]
```

**Options:**
- `-l, --limit <n>` - Maximum results (default: 10)

**Examples:**

```bash
# Search for weather plugins
zeroclaw plugin search weather

# Search with more results
zeroclaw plugin search api --limit 20
```

**Output:**

```
Found 3 plugin(s) for 'weather':
  @user/weather-plugin - Get weather information (by User, downloads: 1234)
  @user/forecast-plugin - Weather forecasts (by User, downloads: 567)
  @org/meteorology - Professional weather data (by Org, downloads: 890)
```

## Plugin Sources

### Registry Plugins

Registry plugins use scoped naming:

- **Scoped**: `@user/plugin` or `@org/plugin`
- **Unscoped**: `user/plugin` (deprecated)

```bash
zeroclaw plugin install @user/plugin
```

### Git Plugins

Git repositories are cloned and installed:

```bash
# GitHub
zeroclaw plugin install https://github.com/user/plugin.git

# GitLab
zeroclaw plugin install https://gitlab.com/user/plugin.git

# With branch
zeroclaw plugin install https://github.com/user/plugin.git#develop
```

### Local Plugins

Local paths can be relative or absolute:

```bash
# Relative path
zeroclaw plugin install ./my-plugin

# Absolute path
zeroclaw plugin install /home/user/my-plugin

# Parent directory
zeroclaw plugin install ../plugins/my-plugin
```

## Troubleshooting

### Plugin Not Found

**Error:** `Plugin 'my-plugin' not found`

**Solutions:**
1. Check the plugin name matches exactly
2. Verify installation directory with `--dir`
3. List installed plugins: `zeroclaw plugin list`

### Permission Denied

**Error:** `Network access to 'api.example.com' not in allowlist`

**Solutions:**
1. Check plugin's permissions in `plugin.toml`
2. Contact plugin author to add required host
3. Use alternative plugin with proper permissions

### Transpilation Failed

**Error:** `Transpilation failed: Syntax errors...`

**Solutions:**
1. Check TypeScript syntax is valid
2. Verify `tsconfig.json` configuration
3. Try `--no-transpile` if using pure JavaScript

### Bundling Failed

**Error:** `Bundle failed: esbuild not found`

**Solutions:**
1. Install esbuild: `npm install -g esbuild`
2. Use `--no-bundle` flag to skip bundling
3. Ensure plugin has no NPM dependencies

### Runtime Errors

**Error:** `Execution error: __zc_tool_my_tool is not defined`

**Solutions:**
1. Verify tool function is exported as `__zc_tool_<name>`
2. Check entry point in `plugin.toml` is correct
3. Enable logging with `console.log()` for debugging

## Security

### Permission Model

Each plugin declares required permissions in `plugin.toml`:

```toml
[permissions]
network = ["api.example.com"]   # Allowed hosts
file_read = ["./data/**"]       # Allowed file paths
file_write = false               # Write permission
env_vars = ["API_KEY"]          # Environment variables
```

### Security Best Practices

1. **Review Permissions**: Always check permissions before installing
   ```bash
   # View plugin permissions
   cat ~/.zeroclaw/plugins/@user/plugin/plugin.toml
   ```

2. **Use Scoped Names**: Prefer scoped packages from trusted authors
   ```bash
   # Good: scoped with trusted author
   zeroclaw plugin install @verified-user/plugin

   # Risky: unscoped or unknown author
   zeroclaw plugin install random-plugin
   ```

3. **Verify Source**: Prefer registry or official Git repositories
   ```bash
   # Good: official registry
   zeroclaw plugin install @user/plugin

   # Good: official GitHub
   zeroclaw plugin install https://github.com/user/plugin.git

   # Risky: unverified URL
   zeroclaw plugin install https://suspicious-site.com/plugin.zip
   ```

4. **Sandbox Isolation**: Plugins run in sandboxed QuickJS runtime
   - Memory limits enforced (64MB default)
   - CPU quota enforced (30s default)
   - Network access controlled by allowlist
   - File access restricted to permitted paths

5. **Checksum Verification**: Registry plugins are verified with SHA256
   ```bash
   # Automatic verification during install
   zeroclaw plugin install @user/plugin
   # ✓ SHA256 checksum verified
   ```

### Updating Plugins

To update a plugin to the latest version:

```bash
# Remove old version
zeroclaw plugin remove @user/plugin

# Install latest version
zeroclaw plugin install @user/plugin
```

### Uninstalling Plugins

Completely remove a plugin:

```bash
# Remove plugin
zeroclaw plugin remove @user/plugin

# Verify removal
zeroclaw plugin list
```

### Plugin Data

Plugins store data in namespaced memory:

```bash
# Memory is stored under "js_plugin:<plugin-id>:<key>"
# Plugin data is removed when plugin is removed
```

## Getting Help

- **Documentation**: [Plugin Authoring Guide](js-plugin-authoring-guide.md)
- **Examples**: [ZeroClaw Plugins](https://github.com/zeroclaw/plugins)
- **Issues**: [GitHub Issues](https://github.com/zeroclaw/zeroclaw/issues)
- **Registry**: [ClawHub](https://clawhub.dev)

## Environment Variables

- `ZEROCLAW_PLUGIN_DIR` - Default plugin installation directory
- `ZEROCLAW_PLUGIN_PATH` - Additional plugin search paths (colon-separated)
- `ZEROCLAW_NO_BUNDLE` - Skip bundling for all plugins (1 = skip)
- `ZEROCLAW_NO_TRANSPILE` - Skip transpilation for all plugins (1 = skip)

**Example:**

```bash
# Set custom plugin directory
export ZEROCLAW_PLUGIN_DIR=/opt/zeroclaw/plugins

# Add additional search paths
export ZEROCLAW_PLUGIN_PATH=/usr/local/plugins:./plugins

# Disable bundling globally
export ZEROCLAW_NO_BUNDLE=1
```

## See Also

- [Plugin Authoring Guide](js-plugin-authoring-guide.md) - Create your own plugins
- [Architecture Design](../plans/2026-02-19-js-plugin-design-v4.md) - Technical details
- [Implementation Plan](../plans/2026-02-19-js-plugin-implementation.md) - Development roadmap
