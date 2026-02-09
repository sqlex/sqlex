use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};

/// Represents the root directory of a sqlex project
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRoot {
    path: PathBuf,
}

impl ProjectRoot {
    /// Create from config file path (parent directory is the project root)
    pub fn from_config_path(config_path: &Path) -> Result<Self> {
        let root = config_path
            .parent()
            .ok_or_else(|| anyhow!("Invalid config path: {}", config_path.display()))?
            .to_path_buf();
        Ok(Self { path: root })
    }

    /// Create directly from directory path
    pub fn from_dir(dir: impl AsRef<Path>) -> Self {
        Self {
            path: dir.as_ref().to_path_buf(),
        }
    }

    /// Get the absolute path of the root directory
    pub fn as_path(&self) -> &Path {
        &self.path
    }

    /// Resolve a relative path to an absolute path
    pub fn resolve(&self, relative_path: impl AsRef<Path>) -> PathBuf {
        self.path.join(relative_path)
    }

    /// Convert an absolute path to a relative path
    pub fn relativize(&self, absolute_path: &Path) -> Result<PathBuf> {
        absolute_path
            .strip_prefix(&self.path)
            .context(format!(
                "Path {} is not under project root {}",
                absolute_path.display(),
                self.path.display()
            ))
            .map(|p| p.to_path_buf())
    }

    /// Check if a path is under the project root
    pub fn contains(&self, path: &Path) -> bool {
        path.starts_with(&self.path)
    }
}
