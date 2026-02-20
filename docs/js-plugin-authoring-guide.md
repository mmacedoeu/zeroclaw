# JavaScript/TypeScript Plugin Authoring Guide

This guide covers how to create, test, and distribute plugins for ZeroClaw using JavaScript or TypeScript.

## Table of Contents

- [Quick Start](#quick-start)
- [Plugin Structure](#plugin-structure)
- [Manifest Format](#manifest-format)
- [SDK Reference](#sdk-reference)
- [Tools](#tools)
- [Skills](#skills)
- [Testing](#testing)
- [Debugging](#debugging)
- [Publishing](#publishing)

## Quick Start

Create a minimal plugin in 5 minutes:

```bash
# Create plugin directory
mkdir my-plugin
cd my-plugin

# Create manifest
cat > plugin.toml << 'EOF'
[plugin]
name = "my-plugin"
version = "1.0.0"
description = "My first ZeroClaw plugin"
author = "Your Name"
license = "MIT"

[runtime]
entry = "index.js"

[permissions]
network = []
file_read = []
file_write = false
env_vars = []
EOF

# Create entry point
cat > index.js << 'EOF'
// Export a tool
function hello(name) {
    return {
        success: true,
        output: "Hello, " + name + "!",
        error: null
    };
}

// Register with ZeroClaw
__zc_tool_hello = hello;
EOF

# Install locally
zeroclaw plugin install .
```

## Plugin Structure

A typical plugin:

```
my-plugin/
├── plugin.toml          # Plugin manifest
├── index.ts             # Entry point (TypeScript)
├── package.json         # NPM dependencies
├── tsconfig.json        # TypeScript config
└── src/
    ├── tools.ts         # Tool implementations
    └── skills.ts        # Skill implementations
```

## Manifest Format

The `plugin.toml` file defines your plugin's metadata:

```toml
[plugin]
name = "my-plugin"              # Required: lowercase alphanumeric + hyphens
version = "1.0.0"               # Required: semver
description = "Plugin description"
author = "Your Name"
license = "MIT"                 # Recommended

[runtime]
entry = "index.js"              # Required: entry point after transpilation
sdk_version = "^1.0.0"          # Optional: SDK version requirement

[permissions]
network = ["api.example.com"]   # Allowed network hosts
file_read = ["./data/**"]       # Allowed file read paths (glob patterns)
file_write = false               # Allow file writes
env_vars = ["API_KEY"]          # Allowed environment variables

[[tools.definitions]]
name = "my_tool"
description = "Does something useful"

[tools.definitions.parameters]
type = "object"
properties.input = { type = "string" }
required = ["input"]

[[skills.definitions]]
name = "my_skill"
description = "Responds to user intents"
patterns = ["hello *", "greet *"]
examples = ["hello world", "greet the user"]
```

## SDK Reference

### Global Objects

ZeroClaw injects these globals into your plugin:

- `__zc_tool_<name>` - Tool handler functions
- `__zc_skill_<name>` - Skill handler functions
- `console.log()` - Logging (sandboxed)
- `fetch()` - HTTP requests (if permitted)

### Memory API

Store and retrieve data:

```javascript
// Set a value
await memory.set("key", { value: "data" });

// Get a value
const data = await memory.get("key");

// Delete a value
await memory.delete("key");

// Check existence
const exists = await memory.exists("key");

// Search
const results = await memory.recall("query", 10);
```

### HTTP API

Make HTTP requests (if network permission granted):

```javascript
// Allowed hosts are configured in plugin.toml
const response = await fetch("https://api.example.com/data");
const data = await response.json();
```

## Tools

Tools are functions that the LLM can call. Define them in `plugin.toml`:

```toml
[[tools.definitions]]
name = "search"
description = "Search the web for information"

[tools.definitions.parameters]
type = "object"
properties.query = { type = "string", description = "Search query" }
properties.limit = { type = "number", default = 10 }
required = ["query"]
```

Implement the tool in JavaScript:

```javascript
async function __zc_tool_search(args) {
    const { query, limit = 10 } = JSON.parse(args);

    // Your implementation here
    const results = await performSearch(query, limit);

    return {
        success: true,
        output: JSON.stringify(results),
        error: null
    };
}
```

Return format:

```javascript
{
    success: boolean,    // Required: whether execution succeeded
    output: string,      // Required: result output
    error: string | null // Error message if failed
}
```

## Skills

Skills respond to user intents with natural language. Define them in `plugin.toml`:

```toml
[[skills.definitions]]
name = "greeting"
description = "Greets the user"
patterns = ["hello", "hi *", "greet *"]
examples = ["hello", "hi there", "greet the world"]
```

Implement the skill:

```javascript
async function __zc_skill_greeting(args) {
    const { query, context } = JSON.parse(args);

    return {
        success: true,
        response: "Hello! How can I help you today?",
        actions: [],
        error: null
    };
}
```

Return format:

```javascript
{
    success: boolean,
    response: string,           // Natural language response
    actions: [                  // Optional actions to take
        {
            action_type: string,  // "tool_call", "response", etc.
            data: any
        }
    ],
    error: string | null
}
```

## Testing

### Unit Testing

Use standard JavaScript testing frameworks:

```javascript
// test/tools.test.js
import { describe, it, expect } from 'vitest';
import { myTool } from '../src/tools';

describe('myTool', () => {
    it('should return success', () => {
        const result = myTool({ input: 'test' });
        expect(result.success).toBe(true);
    });
});
```

### Integration Testing

Test with ZeroClaw sandbox:

```bash
# Install plugin locally
zeroclaw plugin install .

# Test tool execution
zeroclaw agent "Use my-tool with input 'test'"

# Test skill matching
zeroclaw agent "hello"
```

## Debugging

### Enable Logging

```javascript
console.log("Debug info:", data);
console.error("Error:", error);
```

### Check Permissions

Ensure your `plugin.toml` has required permissions:

```toml
[permissions]
network = ["api.example.com"]  # For fetch()
file_read = ["./data/**"]       # For file access
```

### Common Issues

#### "Plugin not found"

- Check `plugin.name` matches directory name
- Verify `plugin.toml` exists

#### "Permission denied"

- Add required permissions to `[permissions]`
- Check host allowlist for network requests

#### "Tool not found"

- Verify `__zc_tool_<name>` is defined
- Check tool name matches manifest

## Publishing

### Prepare for Distribution

```bash
# Build your plugin
npm run build

# Test installation
zeroclaw plugin install .

# Verify tools work
zeroclaw agent "List available tools"
```

### Publish to ClawHub

```bash
# Login to ClawHub
zeroclaw plugin login

# Publish
zeroclaw plugin publish

# Search for your plugin
zeroclaw plugin search my-plugin
```

### Git Distribution

Users can install from Git:

```bash
zeroclaw plugin install https://github.com/user/plugin.git
```

## Advanced Topics

### TypeScript Support

Use TypeScript for type safety:

```typescript
interface ToolArgs {
    input: string;
    options?: {
        limit?: number;
    };
}

async function myTool(args: ToolArgs): Promise<ToolResult> {
    // Implementation
}
```

### Async Operations

All tool/skill handlers are async:

```javascript
async function __zc_tool_fetch(args) {
    const { url } = JSON.parse(args);
    const response = await fetch(url);
    const data = await response.json();

    return {
        success: true,
        output: JSON.stringify(data),
        error: null
    };
}
```

### Error Handling

Return structured errors:

```javascript
try {
    const result = await operation();
    return { success: true, output: result, error: null };
} catch (error) {
    return {
        success: false,
        output: "",
        error: error.message
    };
}
```

## Security Considerations

- **Principle of least privilege**: Only request necessary permissions
- **Validate inputs**: Never trust user input without validation
- **Sanitize outputs**: Escape HTML/special characters in outputs
- **Avoid eval**: Never use `eval()` or `Function()` on user input
- **Secure secrets**: Use `env_vars` for sensitive data, never hardcode

## Resources

- [ZeroClaw Documentation](../README.md)
- [Plugin Examples](https://github.com/zeroclaw/plugins)
- [ClawHub Registry](https://clawhub.dev)
- [Issue Tracker](https://github.com/zeroclaw/zeroclaw/issues)
