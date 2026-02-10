use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlex_common::{ir::CompilationUnit, project_root::ProjectRoot};
use sqlex_generator::{FileWriter, Generator};

mod example;
mod script_type;
mod type_definitions;
mod typescript_compiler;

pub use example::generate_example_script;
use script_type::ScriptType;
pub use type_definitions::generate_type_definitions;
use typescript_compiler::TypeScriptCompiler;

#[derive(Debug, Serialize, Deserialize)]
struct ScriptGeneratorConfig {
    script: PathBuf,
}

pub struct ScriptGenerator {
    project_root: ProjectRoot,
    script_path: PathBuf,
    output: PathBuf,
    ts_compiler: TypeScriptCompiler,
}

impl ScriptGenerator {
    pub fn new(
        project_root: &ProjectRoot,
        output: &Path,
        config: serde_json::Value,
    ) -> Result<Self> {
        let config: ScriptGeneratorConfig = serde_json::from_value(config)?;
        Ok(Self {
            project_root: project_root.clone(),
            script_path: config.script,
            output: output.to_path_buf(),
            ts_compiler: TypeScriptCompiler::new(),
        })
    }

    fn compile_if_needed(&self, script_content: &str, script_path: &Path) -> Result<String> {
        let script_type = ScriptType::from_path(script_path)
            .ok_or_else(|| anyhow::anyhow!("Unable to determine script type from path"))?;

        if script_type.is_typescript() {
            let filename = script_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("script.ts");
            self.ts_compiler.compile(script_content, filename)
        } else {
            Ok(script_content.to_string())
        }
    }

    fn execute_script(
        &self,
        script: &str,
        input: &CompilationUnit,
        file_writer: Arc<Mutex<FileWriter>>,
    ) -> Result<()> {
        use rquickjs::{Context, Runtime};

        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;

        context.with(|ctx| {
            self.inject_project(&ctx, input)?;
            self.inject_writer(&ctx, file_writer.clone())?;
            self.inject_string_utils(&ctx)?;
            self.inject_console(&ctx)?;

            // Wrap script to capture detailed error information
            let wrapped_script = format!(
                r#"
                (function() {{
                    try {{
                        {}
                    }} catch (e) {{
                        globalThis.__scriptError = {{
                            name: e.name || 'Error',
                            message: e.message || String(e),
                            stack: e.stack || 'No stack trace'
                        }};
                        throw e;
                    }}
                }})();
                "#,
                script
            );

            if let Err(e) = ctx.eval::<(), _>(wrapped_script.as_str()) {
                // Try to get detailed error information
                let error_details: Result<String, _> = ctx.eval(
                    r#"
                    (function() {{
                        if (globalThis.__scriptError) {{
                            const e = globalThis.__scriptError;
                            return e.name + ': ' + e.message + '\n\n' + e.stack;
                        }}
                        return null;
                    }})()
                    "#,
                );

                if let Ok(details) = error_details {
                    return Err(anyhow::anyhow!("Failed to execute script:\n{}", details));
                } else {
                    return Err(anyhow::anyhow!("Failed to execute script: {:?}", e));
                }
            }

            Ok::<_, anyhow::Error>(())
        })?;

        Ok(())
    }

    fn inject_project(&self, ctx: &rquickjs::Ctx, input: &CompilationUnit) -> Result<()> {
        use serde::Serialize;

        #[derive(Serialize)]
        struct ProjectData<'a> {
            tables: &'a [sqlex_common::types::Table],
            queries: &'a [sqlex_common::ir::QueryDescriptor],
        }

        let project_data = ProjectData {
            tables: &input.tables,
            queries: &input.queries,
        };

        let json_str = serde_json::to_string(&project_data)?;
        let code = format!(
            "globalThis.project = JSON.parse({});",
            serde_json::to_string(&json_str)?
        );
        ctx.eval::<(), _>(code)?;
        Ok(())
    }

    fn inject_writer(
        &self,
        ctx: &rquickjs::Ctx,
        file_writer: Arc<Mutex<FileWriter>>,
    ) -> Result<()> {
        use rquickjs::{Function, Object};

        let write_fn = Function::new(
            ctx.clone(),
            move |path: String, content: String| -> Result<(), rquickjs::Error> {
                let mut writer = file_writer.lock().map_err(|_| {
                    rquickjs::Error::new_from_js("Error", "FileWriter mutex poisoned")
                })?;
                writer
                    .write(path, content)
                    .map_err(|_| rquickjs::Error::new_from_js("Error", "Write failed"))?;
                Ok(())
            },
        )?;

        let writer_obj = Object::new(ctx.clone())?;
        writer_obj.set("write", write_fn)?;
        ctx.globals().set("writer", writer_obj)?;

        Ok(())
    }

    fn inject_string_utils(&self, ctx: &rquickjs::Ctx) -> Result<()> {
        use heck::{ToKebabCase, ToLowerCamelCase, ToPascalCase, ToShoutySnakeCase, ToSnakeCase};
        use rquickjs::Function;

        let to_snake = Function::new(
            ctx.clone(),
            |s: String| -> Result<String, rquickjs::Error> { Ok(s.to_snake_case()) },
        )?;
        ctx.globals().set("toSnakeCase", to_snake)?;

        let to_camel = Function::new(
            ctx.clone(),
            |s: String| -> Result<String, rquickjs::Error> { Ok(s.to_lower_camel_case()) },
        )?;
        ctx.globals().set("toCamelCase", to_camel)?;

        let to_pascal = Function::new(
            ctx.clone(),
            |s: String| -> Result<String, rquickjs::Error> { Ok(s.to_pascal_case()) },
        )?;
        ctx.globals().set("toPascalCase", to_pascal)?;

        let to_kebab = Function::new(
            ctx.clone(),
            |s: String| -> Result<String, rquickjs::Error> { Ok(s.to_kebab_case()) },
        )?;
        ctx.globals().set("toKebabCase", to_kebab)?;

        let to_screaming = Function::new(
            ctx.clone(),
            |s: String| -> Result<String, rquickjs::Error> { Ok(s.to_shouty_snake_case()) },
        )?;
        ctx.globals().set("toScreamingSnakeCase", to_screaming)?;

        Ok(())
    }

    fn inject_console(&self, ctx: &rquickjs::Ctx) -> Result<()> {
        use rquickjs::{Function, Object};

        let log_fn = Function::new(ctx.clone(), |msg: String| -> Result<(), rquickjs::Error> {
            log::info!("[script] {}", msg);
            Ok(())
        })?;

        let info_fn = Function::new(ctx.clone(), |msg: String| -> Result<(), rquickjs::Error> {
            log::info!("[script] {}", msg);
            Ok(())
        })?;

        let warn_fn = Function::new(ctx.clone(), |msg: String| -> Result<(), rquickjs::Error> {
            log::warn!("[script] {}", msg);
            Ok(())
        })?;

        let error_fn = Function::new(ctx.clone(), |msg: String| -> Result<(), rquickjs::Error> {
            log::error!("[script] {}", msg);
            Ok(())
        })?;

        let console = Object::new(ctx.clone())?;
        console.set("log", log_fn)?;
        console.set("info", info_fn)?;
        console.set("warn", warn_fn)?;
        console.set("error", error_fn)?;

        ctx.globals().set("console", console)?;

        Ok(())
    }
}

#[async_trait]
impl Generator for ScriptGenerator {
    async fn generate(&self, input: &CompilationUnit) -> Result<()> {
        let script_full_path = self.project_root.resolve(&self.script_path);
        let script_content = tokio::fs::read_to_string(&script_full_path)
            .await
            .context(format!("Failed to read script: {:?}", script_full_path))?;

        // Compile TypeScript to JavaScript if needed
        let js_content = self.compile_if_needed(&script_content, &script_full_path)?;

        let file_writer = Arc::new(Mutex::new(FileWriter::new(&self.output).await?));

        self.execute_script(&js_content, input, file_writer.clone())?;

        let mut writer = Arc::try_unwrap(file_writer)
            .map_err(|_| anyhow::anyhow!("Failed to unwrap Arc: multiple references still exist"))?
            .into_inner()
            .map_err(|e| anyhow::anyhow!("FileWriter mutex poisoned: {}", e))?;
        writer.flush().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sqlex_common::types::{ColumnInfo, DataType, Table};
    use tempfile::TempDir;

    use super::*;

    fn create_test_compilation_unit() -> CompilationUnit {
        CompilationUnit {
            tables: vec![Table {
                name: "users".to_string(),
                columns: vec![
                    ColumnInfo {
                        name: "id".to_string(),
                        data_type: DataType::Int(false),
                        nullability: false,
                    },
                    ColumnInfo {
                        name: "name".to_string(),
                        data_type: DataType::Text,
                        nullability: false,
                    },
                ],
            }],
            queries: vec![],
        }
    }

    #[tokio::test]
    async fn test_basic_script_execution() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = ProjectRoot::from_dir(temp_dir.path());
        let output_dir = temp_dir.path().join("output");

        // Create script file
        let script_path = temp_dir.path().join("generate.js");
        tokio::fs::write(
            &script_path,
            r#"
                writer.write("test.txt", "Hello, World!");
            "#,
        )
        .await
        .unwrap();

        let config = serde_json::json!({
            "script": "generate.js"
        });

        let generator = ScriptGenerator::new(&project_root, &output_dir, config).unwrap();
        let input = create_test_compilation_unit();

        generator.generate(&input).await.unwrap();

        // Verify output
        let output_file = output_dir.join("test.txt");
        assert!(output_file.exists());
        let content = tokio::fs::read_to_string(&output_file).await.unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_access_project_data() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = ProjectRoot::from_dir(temp_dir.path());
        let output_dir = temp_dir.path().join("output");

        let script_path = temp_dir.path().join("generate.js");
        tokio::fs::write(
            &script_path,
            r#"
                const tableName = project.tables[0].name;
                const columnName = project.tables[0].columns[0].name;
                writer.write("data.txt", `Table: ${tableName}, Column: ${columnName}`);
            "#,
        )
        .await
        .unwrap();

        let config = serde_json::json!({ "script": "generate.js" });
        let generator = ScriptGenerator::new(&project_root, &output_dir, config).unwrap();
        let input = create_test_compilation_unit();

        generator.generate(&input).await.unwrap();

        let output_file = output_dir.join("data.txt");
        let content = tokio::fs::read_to_string(&output_file).await.unwrap();
        assert_eq!(content, "Table: users, Column: id");
    }

    #[tokio::test]
    async fn test_string_utils() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = ProjectRoot::from_dir(temp_dir.path());
        let output_dir = temp_dir.path().join("output");

        let script_path = temp_dir.path().join("generate.js");
        tokio::fs::write(
            &script_path,
            r#"
                const snake = toSnakeCase("HelloWorld");
                const camel = toCamelCase("hello_world");
                const pascal = toPascalCase("hello_world");
                const kebab = toKebabCase("HelloWorld");
                const screaming = toScreamingSnakeCase("helloWorld");
                writer.write("names.txt", `${snake},${camel},${pascal},${kebab},${screaming}`);
            "#,
        )
        .await
        .unwrap();

        let config = serde_json::json!({ "script": "generate.js" });
        let generator = ScriptGenerator::new(&project_root, &output_dir, config).unwrap();
        let input = create_test_compilation_unit();

        generator.generate(&input).await.unwrap();

        let output_file = output_dir.join("names.txt");
        let content = tokio::fs::read_to_string(&output_file).await.unwrap();
        assert_eq!(
            content,
            "hello_world,helloWorld,HelloWorld,hello-world,HELLO_WORLD"
        );
    }

    #[tokio::test]
    async fn test_script_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = ProjectRoot::from_dir(temp_dir.path());
        let output_dir = temp_dir.path().join("output");

        let config = serde_json::json!({ "script": "nonexistent.js" });
        let generator = ScriptGenerator::new(&project_root, &output_dir, config).unwrap();
        let input = create_test_compilation_unit();

        let result = generator.generate(&input).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to read script")
        );
    }

    #[tokio::test]
    async fn test_script_syntax_error() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = ProjectRoot::from_dir(temp_dir.path());
        let output_dir = temp_dir.path().join("output");

        let script_path = temp_dir.path().join("invalid.js");
        tokio::fs::write(
            &script_path,
            r#"
                // Invalid JavaScript syntax
                const x = {
            "#,
        )
        .await
        .unwrap();

        let config = serde_json::json!({ "script": "invalid.js" });
        let generator = ScriptGenerator::new(&project_root, &output_dir, config).unwrap();
        let input = create_test_compilation_unit();

        let result = generator.generate(&input).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to execute script")
        );
    }

    #[tokio::test]
    async fn test_script_runtime_error() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = ProjectRoot::from_dir(temp_dir.path());
        let output_dir = temp_dir.path().join("output");

        let script_path = temp_dir.path().join("runtime_error.js");
        tokio::fs::write(
            &script_path,
            r#"
                // Try to access non-existent property
                const value = project.nonExistentField.someProperty;
            "#,
        )
        .await
        .unwrap();

        let config = serde_json::json!({ "script": "runtime_error.js" });
        let generator = ScriptGenerator::new(&project_root, &output_dir, config).unwrap();
        let input = create_test_compilation_unit();

        let result = generator.generate(&input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_console_logging() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = ProjectRoot::from_dir(temp_dir.path());
        let output_dir = temp_dir.path().join("output");

        let script_path = temp_dir.path().join("logging.js");
        tokio::fs::write(
            &script_path,
            r#"
                console.log("Log message");
                console.info("Info message");
                console.warn("Warning message");
                console.error("Error message");
                writer.write("output.txt", "Done");
            "#,
        )
        .await
        .unwrap();

        let config = serde_json::json!({ "script": "logging.js" });
        let generator = ScriptGenerator::new(&project_root, &output_dir, config).unwrap();
        let input = create_test_compilation_unit();

        // Should not error even with console calls
        generator.generate(&input).await.unwrap();

        let output_file = output_dir.join("output.txt");
        assert!(output_file.exists());
    }

    #[tokio::test]
    async fn test_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = ProjectRoot::from_dir(temp_dir.path());
        let output_dir = temp_dir.path().join("output");

        let script_path = temp_dir.path().join("multi.js");
        tokio::fs::write(
            &script_path,
            r#"
                writer.write("file1.txt", "Content 1");
                writer.write("file2.txt", "Content 2");
                writer.write("subdir/file3.txt", "Content 3");
            "#,
        )
        .await
        .unwrap();

        let config = serde_json::json!({ "script": "multi.js" });
        let generator = ScriptGenerator::new(&project_root, &output_dir, config).unwrap();
        let input = create_test_compilation_unit();

        generator.generate(&input).await.unwrap();

        assert!(output_dir.join("file1.txt").exists());
        assert!(output_dir.join("file2.txt").exists());
        assert!(output_dir.join("subdir/file3.txt").exists());

        let content1 = tokio::fs::read_to_string(output_dir.join("file1.txt"))
            .await
            .unwrap();
        assert_eq!(content1, "Content 1");

        let content3 = tokio::fs::read_to_string(output_dir.join("subdir/file3.txt"))
            .await
            .unwrap();
        assert_eq!(content3, "Content 3");
    }

    #[tokio::test]
    async fn test_detailed_error_message() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = ProjectRoot::from_dir(temp_dir.path());
        let output_dir = temp_dir.path().join("output");

        let script_path = temp_dir.path().join("error.js");
        tokio::fs::write(
            &script_path,
            r#"
                // This will cause a ReferenceError
                undefinedVariable.someMethod();
            "#,
        )
        .await
        .unwrap();

        let config = serde_json::json!({ "script": "error.js" });
        let generator = ScriptGenerator::new(&project_root, &output_dir, config).unwrap();
        let input = create_test_compilation_unit();

        let result = generator.generate(&input).await;
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        println!("Error message:\n{}", error_msg);

        // The error message should contain detailed information
        assert!(
            error_msg.contains("ReferenceError") || error_msg.contains("not defined"),
            "Error message should contain error type or description, got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_typescript_basic_compilation() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = ProjectRoot::from_dir(temp_dir.path());
        let output_dir = temp_dir.path().join("output");

        let script_path = temp_dir.path().join("generate.ts");
        tokio::fs::write(
            &script_path,
            r#"
                const message: string = "Hello from TypeScript!";
                writer.write("test.txt", message);
            "#,
        )
        .await
        .unwrap();

        let config = serde_json::json!({ "script": "generate.ts" });
        let generator = ScriptGenerator::new(&project_root, &output_dir, config).unwrap();
        let input = create_test_compilation_unit();

        generator.generate(&input).await.unwrap();

        let output_file = output_dir.join("test.txt");
        assert!(output_file.exists());
        let content = tokio::fs::read_to_string(&output_file).await.unwrap();
        assert_eq!(content, "Hello from TypeScript!");
    }

    #[tokio::test]
    async fn test_typescript_with_types() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = ProjectRoot::from_dir(temp_dir.path());
        let output_dir = temp_dir.path().join("output");

        let script_path = temp_dir.path().join("generate.ts");
        tokio::fs::write(
            &script_path,
            r#"
                interface User {
                    id: number;
                    name: string;
                }

                const user: User = { id: 1, name: "Alice" };
                writer.write("user.txt", `${user.id}: ${user.name}`);
            "#,
        )
        .await
        .unwrap();

        let config = serde_json::json!({ "script": "generate.ts" });
        let generator = ScriptGenerator::new(&project_root, &output_dir, config).unwrap();
        let input = create_test_compilation_unit();

        generator.generate(&input).await.unwrap();

        let output_file = output_dir.join("user.txt");
        let content = tokio::fs::read_to_string(&output_file).await.unwrap();
        assert_eq!(content, "1: Alice");
    }

    #[tokio::test]
    async fn test_typescript_syntax_error() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = ProjectRoot::from_dir(temp_dir.path());
        let output_dir = temp_dir.path().join("output");

        let script_path = temp_dir.path().join("invalid.ts");
        tokio::fs::write(
            &script_path,
            r#"
                // Invalid TypeScript syntax
                const x: string = 123;
                const y = {
            "#,
        )
        .await
        .unwrap();

        let config = serde_json::json!({ "script": "invalid.ts" });
        let generator = ScriptGenerator::new(&project_root, &output_dir, config).unwrap();
        let input = create_test_compilation_unit();

        let result = generator.generate(&input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_typescript_advanced_features() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = ProjectRoot::from_dir(temp_dir.path());
        let output_dir = temp_dir.path().join("output");

        let script_path = temp_dir.path().join("advanced.ts");
        tokio::fs::write(
            &script_path,
            r#"
                // Test generics and optional chaining
                function identity<T>(value: T): T {
                    return value;
                }

                const result = identity<string>("test");
                const optional = project.tables[0]?.name ?? "default";
                writer.write("output.txt", `${result},${optional}`);
            "#,
        )
        .await
        .unwrap();

        let config = serde_json::json!({ "script": "advanced.ts" });
        let generator = ScriptGenerator::new(&project_root, &output_dir, config).unwrap();
        let input = create_test_compilation_unit();

        generator.generate(&input).await.unwrap();

        let output_file = output_dir.join("output.txt");
        let content = tokio::fs::read_to_string(&output_file).await.unwrap();
        assert_eq!(content, "test,users");
    }
}
