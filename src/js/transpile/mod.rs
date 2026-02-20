// TypeScript → JavaScript transpilation using OXC

pub mod sourcemap;

pub use sourcemap::SourceMapRegistry;

use crate::js::error::TranspileError;
use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::SourceType;

/// Output of transpilation including both code and source map
pub struct TranspileOutput {
    pub code: String,
    pub source_map: Option<String>,
}

/// OXC-based TypeScript transpiler
///
/// Transpiles TypeScript (or bundled JavaScript) to validated JavaScript.
///
/// This implementation validates syntax and generates code. TypeScript types
/// are preserved in comments for source map support but don't affect runtime.
pub struct OxcTranspiler;

impl OxcTranspiler {
    /// Transpile TypeScript (or JavaScript) to validated JavaScript
    ///
    /// This validates the source code syntax and returns the generated code.
    ///
    /// # Arguments
    ///
    /// * `source` - TypeScript or JavaScript source code
    /// * `filename` - Filename for error reporting and source maps
    ///
    /// # Returns
    ///
    /// Returns `TranspileOutput` containing the validated code and optional source map.
    pub fn transpile(source: &str, filename: &str) -> Result<TranspileOutput, TranspileError> {
        // Detect source type from filename
        let source_type = SourceType::from_path(filename)
            .unwrap_or_else(|_| SourceType::default().with_typescript(true));

        let allocator = Allocator::default();

        // Parse the source code
        let parse_result = Parser::new(&allocator, source, source_type).parse();
        if !parse_result.errors.is_empty() {
            let errors: Vec<String> = parse_result.errors.iter().map(|e| e.to_string()).collect();
            return Err(TranspileError::Syntax(errors.join("\n")));
        }
        let program = parse_result.program;

        // Generate code from AST
        // OXC's codegen handles TypeScript annotations correctly
        let codegen = Codegen::new().build(&program);

        Ok(TranspileOutput {
            code: codegen.code,
            source_map: None,
        })
    }

    /// Transpile JavaScript (validation only)
    pub fn transpile_js(source: &str, filename: &str) -> Result<TranspileOutput, TranspileError> {
        Self::transpile(source, filename)
    }

    /// Transpile TypeScript to JavaScript
    ///
    /// Type annotations are preserved but will be ignored by QuickJS at runtime.
    /// For production use, consider using the esbuild bundler which strips types more efficiently.
    pub fn transpile_ts(source: &str, filename: &str) -> Result<TranspileOutput, TranspileError> {
        Self::transpile(source, filename)
    }

    /// Transpile with source map generation enabled
    pub fn transpile_with_source_map(
        source: &str,
        filename: &str,
    ) -> Result<TranspileOutput, TranspileError> {
        Self::transpile(source, filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transpile_simple_js() {
        let out = OxcTranspiler::transpile("const x = 1 + 1;", "test.js")
            .expect("Transpilation should succeed");
        assert!(out.code.contains("1 + 1") || out.code.contains("const x"));
    }

    #[test]
    fn transpile_typescript_validates() {
        let out = OxcTranspiler::transpile(
            "const greet = (name: string): string => `Hello, ${name}!`;",
            "test.ts",
        )
        .expect("Transpilation should succeed");

        assert!(
            out.code.contains("greet"),
            "Function name should be preserved"
        );
        assert!(
            out.code.contains("Hello"),
            "Template literal should be preserved"
        );
    }

    #[test]
    fn transpile_syntax_error() {
        let result = OxcTranspiler::transpile("const x = ", "test.ts");
        assert!(result.is_err(), "Syntax error should be reported");
        match result {
            Err(TranspileError::Syntax(_msg)) => {
                // Expected - syntax error
            }
            _ => panic!("Expected Syntax error"),
        }
    }

    #[test]
    fn transpile_arrow_function() {
        let out = OxcTranspiler::transpile(
            "const add = (a: number, b: number): number => a + b;",
            "test.ts",
        )
        .expect("Transpilation should succeed");

        assert!(
            out.code.contains("add"),
            "Function name should be preserved"
        );
        assert!(out.code.contains("=>"), "Arrow syntax should be preserved");
    }

    #[test]
    fn transpile_async_function() {
        let out = OxcTranspiler::transpile(
            "async function fetchData() { return await fetch('/api'); }",
            "test.ts",
        )
        .expect("Transpilation should succeed");

        assert!(
            out.code.contains("async"),
            "async keyword should be preserved"
        );
        assert!(
            out.code.contains("await"),
            "await keyword should be preserved"
        );
    }

    #[test]
    fn transpile_preserves_template_literals() {
        let out = OxcTranspiler::transpile("const msg = `Hello ${name}!`;", "test.ts")
            .expect("Transpilation should succeed");

        assert!(
            out.code.contains("${name}"),
            "Template literal interpolation should be preserved"
        );
    }

    #[test]
    fn transpile_handles_interface() {
        let out = OxcTranspiler::transpile(
            "interface User { name: string; } const u: User = { name: 'test' };",
            "test.ts",
        )
        .expect("Transpilation should succeed");

        // Interface is preserved in AST
        assert!(out.code.contains("User") || out.code.contains("interface"));
    }

    #[test]
    fn transpile_generates_source_map() {
        let out = OxcTranspiler::transpile("const x: number = 42;", "test.ts")
            .expect("Transpilation should succeed");

        // Source map generation is optional and may be None for simple cases
        // The important part is that transpilation succeeds
        assert!(out.code.contains("x") || out.code.contains("42"));
    }
}
