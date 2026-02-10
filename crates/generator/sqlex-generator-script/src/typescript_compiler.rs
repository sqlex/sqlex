use anyhow::{Context, Result};
use oxc::{
    allocator::Allocator,
    codegen::Codegen,
    parser::Parser,
    semantic::SemanticBuilder,
    span::SourceType,
    transformer::{TransformOptions, Transformer},
};

/// TypeScript compiler that converts TypeScript code to JavaScript
pub struct TypeScriptCompiler;

impl TypeScriptCompiler {
    /// Creates a new TypeScript compiler instance
    pub fn new() -> Self {
        Self
    }

    /// Compiles TypeScript code to JavaScript
    ///
    /// # Arguments
    /// * `typescript_code` - The TypeScript source code to compile
    /// * `filename` - The name of the file being compiled (for error messages)
    ///
    /// # Returns
    /// The compiled JavaScript code
    pub fn compile(&self, typescript_code: &str, filename: &str) -> Result<String> {
        // Create allocator for memory management
        let allocator = Allocator::default();

        // Determine source type from filename
        let source_type = SourceType::from_path(filename)
            .unwrap_or_else(|_| SourceType::default().with_typescript(true));

        // Parse TypeScript code
        let parser_result = Parser::new(&allocator, typescript_code, source_type).parse();

        if !parser_result.errors.is_empty() {
            let error_messages: Vec<String> = parser_result
                .errors
                .iter()
                .map(|e| format!("{:?}", e))
                .collect();
            return Err(anyhow::anyhow!(
                "Failed to parse TypeScript: {}",
                error_messages.join(", ")
            ))
            .context(format!("Error parsing TypeScript file: {}", filename));
        }

        let mut program = parser_result.program;

        // Build semantic information
        let semantic_result = SemanticBuilder::new()
            .with_excess_capacity(2.0)
            .build(&program);

        if !semantic_result.errors.is_empty() {
            let error_messages: Vec<String> = semantic_result
                .errors
                .iter()
                .map(|e| format!("{:?}", e))
                .collect();
            return Err(anyhow::anyhow!(
                "Semantic analysis failed: {}",
                error_messages.join(", ")
            ))
            .context(format!("Error analyzing TypeScript file: {}", filename));
        }

        let (symbols, scopes) = semantic_result.semantic.into_symbol_table_and_scope_tree();

        // Configure transformation options to strip TypeScript types
        let transform_options = TransformOptions::enable_all();

        // Execute transformation
        let transform_result = Transformer::new(
            &allocator,
            std::path::Path::new(filename),
            &transform_options,
        )
        .build_with_symbols_and_scopes(symbols, scopes, &mut program);

        if !transform_result.errors.is_empty() {
            let error_messages: Vec<String> = transform_result
                .errors
                .iter()
                .map(|e| format!("{:?}", e))
                .collect();
            return Err(anyhow::anyhow!(
                "Transformation failed: {}",
                error_messages.join(", ")
            ))
            .context(format!("Error transforming TypeScript file: {}", filename));
        }

        // Generate JavaScript code
        let codegen_result = Codegen::new().build(&program);

        Ok(codegen_result.code)
    }
}
