use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use futures::{FutureExt, future::BoxFuture};

/// Marker file to indicate this directory is managed by sqlex
const MARKER_FILE: &str = ".sqlex-generated";

/// A file writer that collects files to write and synchronizes them in a batch.
///
/// This writer maintains an in-memory cache of existing files and only writes
/// files when their content has changed, avoiding unnecessary file system operations
/// and timestamp updates.
///
/// The writer uses a marker file (`.sqlex-generated`) to ensure it only manages
/// directories explicitly designated for generated code, preventing accidental
/// deletion of user files.
#[derive(Debug)]
pub struct FileWriter {
    root: PathBuf,
    pending_files: HashMap<PathBuf, String>,
    cached_files: HashMap<PathBuf, String>,
}

impl FileWriter {
    /// Creates a new FileWriter for the given root directory.
    ///
    /// This will scan the directory and build a cache of all existing files.
    ///
    /// # Safety
    ///
    /// If the directory exists but does not contain a `.sqlex-generated` marker file,
    /// and the directory is not empty, this function will return an error to prevent
    /// accidental deletion of user files.
    pub async fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let marker_path = root.join(MARKER_FILE);

        if root.exists() {
            if !marker_path.exists() {
                if !Self::is_directory_empty(&root).await? {
                    return Err(anyhow!(
                        "Directory {:?} exists but is not marked as a sqlex-generated directory. \
                        To use this directory, either:\n\
                        1. Create an empty file named '{}' in the directory, or\n\
                        2. Use an empty directory",
                        root,
                        MARKER_FILE
                    ));
                }
                tokio::fs::create_dir_all(&root).await?;
                tokio::fs::write(&marker_path, "").await?;
            }
        } else {
            tokio::fs::create_dir_all(&root).await?;
            tokio::fs::write(&marker_path, "").await?;
        }

        let cached_files = Self::scan_directory(&root).await?;

        Ok(Self {
            root,
            pending_files: HashMap::new(),
            cached_files,
        })
    }

    /// Checks if a directory is empty (ignoring hidden files like .DS_Store).
    async fn is_directory_empty(path: &Path) -> Result<bool> {
        let mut read_dir = tokio::fs::read_dir(path).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // Ignore hidden files (starting with .)
            if !file_name_str.starts_with('.') {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Scans a directory recursively and builds a map of relative paths to file contents.
    async fn scan_directory(root: &Path) -> Result<HashMap<PathBuf, String>> {
        let mut files = HashMap::new();

        if !root.exists() {
            return Ok(files);
        }

        Self::scan_directory_recursive(root, root, &mut files).await?;

        Ok(files)
    }

    /// Recursively scans a directory and collects all file contents.
    ///
    /// Skips the marker file (`.sqlex-generated`) as it's managed separately.
    fn scan_directory_recursive<'a>(
        root: &'a Path,
        current: &'a Path,
        files: &'a mut HashMap<PathBuf, String>,
    ) -> BoxFuture<'a, Result<()>> {
        async move {
            if !current.is_dir() {
                return Ok(());
            }

            let mut read_dir = tokio::fs::read_dir(current).await?;

            while let Some(entry) = read_dir.next_entry().await? {
                let path = entry.path();

                if path.is_file() {
                    let rel_path = path
                        .strip_prefix(root)
                        .map_err(|e| anyhow!("Failed to strip prefix: {}", e))?
                        .to_path_buf();

                    // Skip the marker file
                    if rel_path.to_string_lossy() == MARKER_FILE {
                        continue;
                    }

                    let content = tokio::fs::read_to_string(&path).await?;
                    files.insert(rel_path, content);
                } else if path.is_dir() {
                    Self::scan_directory_recursive(root, &path, files).await?;
                }
            }

            Ok(())
        }
        .boxed()
    }

    /// Collects all directories that should be removed based on pending files.
    ///
    /// A directory should be removed if no files in pending_files are under it.
    fn collect_empty_directories(&self) -> Vec<PathBuf> {
        use std::collections::HashSet;

        // Collect all directories that will have files after flush
        let mut active_dirs = HashSet::new();
        for file_path in self.pending_files.keys() {
            let mut current = file_path.as_path();
            while let Some(parent) = current.parent() {
                if parent == Path::new("") {
                    break;
                }
                active_dirs.insert(parent.to_path_buf());
                current = parent;
            }
        }

        // Collect all directories from cached files
        let mut all_dirs = HashSet::new();
        for file_path in self.cached_files.keys() {
            let mut current = file_path.as_path();
            while let Some(parent) = current.parent() {
                if parent == Path::new("") {
                    break;
                }
                all_dirs.insert(parent.to_path_buf());
                current = parent;
            }
        }

        // Directories to remove = all_dirs - active_dirs
        let mut dirs_to_remove: Vec<_> = all_dirs.difference(&active_dirs).cloned().collect();

        // Sort by depth (deepest first) to ensure we delete children before parents
        dirs_to_remove.sort_by(|a, b| {
            let depth_a = a.components().count();
            let depth_b = b.components().count();
            depth_b.cmp(&depth_a)
        });

        dirs_to_remove
    }

    /// Adds a file to the pending write queue.
    ///
    /// The path must be relative to the root directory.
    /// Paths containing '..' or absolute paths are rejected for security.
    pub fn write(&mut self, path: impl AsRef<Path>, content: impl Into<String>) -> Result<()> {
        let path = path.as_ref();

        if path.is_absolute() {
            return Err(anyhow!("Path must be relative: {:?}", path));
        }

        if path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(anyhow!("Path cannot contain '..': {:?}", path));
        }

        self.pending_files
            .insert(path.to_path_buf(), content.into());
        Ok(())
    }

    /// Flushes all pending files to disk.
    ///
    /// This will:
    /// 1. Write files that are new or have changed content
    /// 2. Delete files that exist in the cache but not in pending files (except marker file)
    /// 3. Remove empty directories
    /// 4. Update the internal cache
    /// 5. Clear the pending files queue
    pub async fn flush(&mut self) -> Result<()> {
        // Write new or modified files
        for (rel_path, new_content) in &self.pending_files {
            let full_path = self.root.join(rel_path);

            let needs_write = match self.cached_files.get(rel_path) {
                Some(old_content) => old_content != new_content,
                None => true,
            };

            if needs_write {
                if let Some(parent) = full_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                tokio::fs::write(&full_path, new_content).await?;
            }
        }

        // Delete files that are no longer needed (but keep the marker file)
        for old_file in self.cached_files.keys() {
            // Skip marker file
            if old_file.to_string_lossy() == MARKER_FILE {
                continue;
            }

            if !self.pending_files.contains_key(old_file) {
                let full_path = self.root.join(old_file);
                if full_path.exists() {
                    tokio::fs::remove_file(&full_path).await?;
                }
            }
        }

        // Remove empty directories
        let empty_dirs = self.collect_empty_directories();
        for dir in empty_dirs {
            let full_path = self.root.join(&dir);
            if full_path.exists() && full_path.is_dir() {
                tokio::fs::remove_dir(&full_path).await?;
            }
        }

        // Update cache and clear pending
        self.cached_files = self.pending_files.clone();
        self.pending_files.clear();

        Ok(())
    }
}
