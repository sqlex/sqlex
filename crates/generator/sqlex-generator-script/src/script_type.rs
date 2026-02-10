use std::path::Path;

/// Represents the type of script file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptType {
    JavaScript,
    TypeScript,
}

impl ScriptType {
    /// Determines the script type from a file path based on its extension
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext.to_lowercase().as_str() {
                "ts" => ScriptType::TypeScript,
                "js" => ScriptType::JavaScript,
                _ => ScriptType::JavaScript, // Default to JavaScript for unknown extensions
            })
    }

    /// Returns true if this is a TypeScript script
    pub fn is_typescript(&self) -> bool {
        matches!(self, ScriptType::TypeScript)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_from_path_typescript() {
        let path = PathBuf::from("script.ts");
        assert_eq!(ScriptType::from_path(&path), Some(ScriptType::TypeScript));
    }

    #[test]
    fn test_from_path_javascript() {
        let path = PathBuf::from("script.js");
        assert_eq!(ScriptType::from_path(&path), Some(ScriptType::JavaScript));
    }

    #[test]
    fn test_from_path_no_extension() {
        let path = PathBuf::from("script");
        assert_eq!(ScriptType::from_path(&path), None);
    }

    #[test]
    fn test_is_typescript() {
        assert!(ScriptType::TypeScript.is_typescript());
        assert!(!ScriptType::JavaScript.is_typescript());
    }
}
