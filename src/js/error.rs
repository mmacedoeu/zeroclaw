// Error types for JS plugins

use thiserror::Error;

/// Unified error type for JS plugin operations
///
/// This is the primary error type returned by JS plugin operations.
/// Individual error types are exposed through `From` conversions.
#[derive(Debug, Error)]
pub enum JsPluginError {
    #[error("Transpilation failed: {0}")]
    Transpile(#[from] TranspileError),

    #[error("Bundling failed: {0}")]
    Bundle(#[from] BundleError),

    #[error("Runtime error: {0}")]
    Runtime(#[from] JsRuntimeError),

    #[error("Sandbox violation: {0}")]
    Sandbox(#[from] SandboxViolation),

    #[error("Registry error: {0}")]
    Registry(#[from] RegistryError),

    #[error("Plugin '{name}' not found")]
    NotFound { name: String },

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors during JavaScript code execution
#[derive(Debug, Error)]
pub enum JsRuntimeError {
    #[error("CPU quota exceeded")]
    CpuQuotaExceeded,

    #[error("Memory limit exceeded")]
    OutOfMemory,

    #[error("Worker thread died")]
    WorkerShutdown,

    #[error("Execution error: {0}")]
    Execution(String),
}

/// Errors during TypeScript → JavaScript transpilation
#[derive(Debug, Error)]
pub enum TranspileError {
    #[error("Syntax errors:\n{0}")]
    Syntax(String),

    #[error("Transform errors:\n{0}")]
    Transform(String),
}

/// Errors during NPM dependency bundling with esbuild
#[derive(Debug, Error)]
pub enum BundleError {
    #[error(
        "esbuild not found. Install it with: npm install -g esbuild\n\
         esbuild is required to bundle plugins with NPM dependencies."
    )]
    EsbuildNotFound,

    #[error("Bundle failed: {0}")]
    BundleFailed(String),
}

/// Sandbox security violations
#[derive(Debug, Error)]
pub enum SandboxViolation {
    #[error("CPU quota exceeded")]
    CpuQuotaExceeded,

    #[error("Memory limit exceeded")]
    MemoryExceeded,

    #[error("Network access to '{host}' not in allowlist")]
    NetworkBlocked { host: String },

    #[error("File access to '{path}' not allowed")]
    FileBlocked { path: String },
}

/// Registry-related errors
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Registry request failed: {0}")]
    RequestFailed(String),

    #[error("Invalid registry response: {0}")]
    InvalidResponse(String),

    #[error("Integrity check failed: {0}")]
    IntegrityCheckFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        assert_eq!(
            JsPluginError::NotFound {
                name: "test".to_string()
            }
            .to_string(),
            "Plugin 'test' not found"
        );

        assert_eq!(
            JsRuntimeError::CpuQuotaExceeded.to_string(),
            "CPU quota exceeded"
        );

        assert_eq!(
            TranspileError::Syntax("line 1: unexpected token".to_string()).to_string(),
            "Syntax errors:\nline 1: unexpected token"
        );

        assert_eq!(
            BundleError::EsbuildNotFound
                .to_string()
                .contains("npm install -g esbuild"),
            true
        );
    }

    #[test]
    fn from_conversions_work() {
        let transpile_err: JsPluginError = TranspileError::Syntax("test".to_string()).into();
        assert!(matches!(transpile_err, JsPluginError::Transpile(_)));

        let bundle_err: JsPluginError = BundleError::EsbuildNotFound.into();
        assert!(matches!(bundle_err, JsPluginError::Bundle(_)));

        let runtime_err: JsPluginError = JsRuntimeError::CpuQuotaExceeded.into();
        assert!(matches!(runtime_err, JsPluginError::Runtime(_)));

        let sandbox_err: JsPluginError = SandboxViolation::CpuQuotaExceeded.into();
        assert!(matches!(sandbox_err, JsPluginError::Sandbox(_)));
    }

    #[test]
    fn memory_error_from_string() {
        let err = JsPluginError::Memory("key not found".to_string());
        assert_eq!(err.to_string(), "Memory error: key not found");
    }
}
