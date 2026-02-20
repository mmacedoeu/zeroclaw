// Source map registry for JS error stack trace remapping

use regex::Regex;
use std::collections::HashMap;

/// Registry for mapping plugin source maps
///
/// This component stores source maps and provides remapping functionality
/// to translate generated JavaScript stack traces back to original TypeScript sources.
pub struct SourceMapRegistry {
    /// Maps plugin IDs to their source map JSON strings
    maps: HashMap<String, sourcemap::SourceMap>,
}

impl SourceMapRegistry {
    /// Create a new empty source map registry
    pub fn new() -> Self {
        Self {
            maps: HashMap::new(),
        }
    }

    /// Register a source map for a plugin
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin identifier
    /// * `map_json` - The source map as a JSON string
    pub fn register(&mut self, plugin_id: &str, map_json: String) {
        match sourcemap::SourceMap::from_slice(map_json.as_bytes()) {
            Ok(sm) => {
                self.maps.insert(plugin_id.to_string(), sm);
                tracing::debug!("Registered source map for plugin '{}'", plugin_id);
            }
            Err(e) => {
                tracing::warn!(
                    plugin = plugin_id,
                    error = %e,
                    "Failed to parse source map"
                );
            }
        }
    }

    /// Remap a stack trace for a plugin
    ///
    /// Transforms stack frame locations from generated JavaScript
    /// back to original TypeScript source locations.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin identifier
    /// * `raw_stack` - The raw stack trace string
    ///
    /// # Returns
    ///
    /// Returns the remapped stack trace, or the original if no source map is available.
    pub fn remap_stack(&self, plugin_id: &str, raw_stack: &str) -> String {
        self.maps
            .get(plugin_id)
            .map(|sm| self.remap_stack_with_map(raw_stack, sm))
            .unwrap_or_else(|| raw_stack.to_string())
    }

    /// Check if a plugin has a registered source map
    pub fn has_map(&self, plugin_id: &str) -> bool {
        self.maps.contains_key(plugin_id)
    }

    /// Remove a source map from the registry
    pub fn unregister(&mut self, plugin_id: &str) -> bool {
        self.maps.remove(plugin_id).is_some()
    }

    /// Remap stack trace using a specific source map
    fn remap_stack_with_map(&self, stack: &str, sm: &sourcemap::SourceMap) -> String {
        let line_re = Regex::new(r":(\d+):(\d+)").expect("Invalid regex");
        let mut result = String::new();

        for line in stack.lines() {
            let remapped = self.remap_frame(line, &line_re, sm);
            result.push_str(&remapped);
            result.push('\n');
        }

        // Remove trailing newline
        if result.ends_with('\n') {
            result.pop();
        }

        result
    }

    /// Remap a single stack frame
    fn remap_frame(&self, frame: &str, line_re: &Regex, sm: &sourcemap::SourceMap) -> String {
        let Some(caps) = line_re.captures(frame) else {
            return frame.to_string();
        };

        let Ok(line) = caps[1].parse::<u32>() else {
            return frame.to_string();
        };
        let Ok(col) = caps[2].parse::<u32>() else {
            return frame.to_string();
        };

        // sourcemap uses 0-based indexing
        let token = match sm.lookup_token(line.saturating_sub(1), col.saturating_sub(1)) {
            Some(t) => t,
            None => return frame.to_string(),
        };

        let src = token.get_source().unwrap_or("<unknown>");
        let sline = token.get_src_line() + 1; // Convert back to 1-based
        let scol = token.get_src_col() + 1; // Convert back to 1-based

        // Replace the line:col in the frame
        line_re
            .replace(frame, format!("  ({}:{}:{})", src, sline, scol))
            .to_string()
    }
}

impl Default for SourceMapRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_starts_empty() {
        let registry = SourceMapRegistry::new();
        assert!(!registry.has_map("test-plugin"));
    }

    #[test]
    fn register_and_has_map() {
        let mut registry = SourceMapRegistry::new();
        let map_json = r#"{"version":3,"sources":["test.ts"],"mappings":"AAAA","names":[]}"#;
        registry.register("test-plugin", map_json.to_string());
        assert!(registry.has_map("test-plugin"));
    }

    #[test]
    fn unregister_removes_map() {
        let mut registry = SourceMapRegistry::new();
        let map_json = r#"{"version":3,"sources":["test.ts"],"mappings":"AAAA","names":[]}"#;
        registry.register("test-plugin", map_json.to_string());
        assert!(registry.unregister("test-plugin"));
        assert!(!registry.has_map("test-plugin"));
    }

    #[test]
    fn remap_stack_without_map_returns_original() {
        let registry = SourceMapRegistry::new();
        let stack = "Error: test\n    at test.ts:10:5";
        let result = registry.remap_stack("unknown-plugin", stack);
        assert_eq!(result, stack);
    }

    #[test]
    fn remap_stack_preserves_multiline() {
        let registry = SourceMapRegistry::new();
        let stack = "Error: test\n    at frame1 (file.js:1:1)\n    at frame2 (file.js:2:2)";
        let result = registry.remap_stack("unknown-plugin", stack);
        assert_eq!(result, stack);
    }
}
