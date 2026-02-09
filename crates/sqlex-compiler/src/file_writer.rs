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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    /// Helper function to create a temporary directory for testing
    fn create_temp_dir() -> TempDir {
        TempDir::new().expect("Failed to create temp dir")
    }

    /// Helper function to check if a file exists and has the expected content
    fn assert_file_content(root: &Path, rel_path: &str, expected: &str) {
        let full_path = root.join(rel_path);
        assert!(full_path.exists(), "File {:?} should exist", full_path);
        let content = fs::read_to_string(&full_path).expect("Failed to read file");
        assert_eq!(
            content, expected,
            "File content mismatch for {:?}",
            full_path
        );
    }

    /// Helper function to check if a file does not exist
    fn assert_file_not_exists(root: &Path, rel_path: &str) {
        let full_path = root.join(rel_path);
        assert!(!full_path.exists(), "File {:?} should not exist", full_path);
    }

    /// Helper function to check if a directory exists
    fn assert_dir_exists(root: &Path, rel_path: &str) {
        let full_path = root.join(rel_path);
        assert!(
            full_path.exists() && full_path.is_dir(),
            "Directory {:?} should exist",
            full_path
        );
    }

    /// Helper function to check if a directory does not exist
    fn assert_dir_not_exists(root: &Path, rel_path: &str) {
        let full_path = root.join(rel_path);
        assert!(
            !full_path.exists() || !full_path.is_dir(),
            "Directory {:?} should not exist",
            full_path
        );
    }

    #[tokio::test]
    async fn test_basic_write_and_flush() {
        let temp_dir = create_temp_dir();
        let root = temp_dir.path();

        let mut writer = FileWriter::new(root)
            .await
            .expect("Failed to create FileWriter");

        // Write some files
        writer
            .write("file1.txt", "content1")
            .expect("Failed to write file1");
        writer
            .write("dir/file2.txt", "content2")
            .expect("Failed to write file2");

        // Flush to disk
        writer.flush().await.expect("Failed to flush");

        // Verify files exist with correct content
        assert_file_content(root, "file1.txt", "content1");
        assert_file_content(root, "dir/file2.txt", "content2");
        assert_file_content(root, MARKER_FILE, "");
    }

    #[tokio::test]
    async fn test_marker_file_created() {
        let temp_dir = create_temp_dir();
        let root = temp_dir.path();

        FileWriter::new(root)
            .await
            .expect("Failed to create FileWriter");

        // Marker file should be created
        assert_file_content(root, MARKER_FILE, "");
    }

    #[tokio::test]
    async fn test_unchanged_content_not_written() {
        let temp_dir = create_temp_dir();
        let root = temp_dir.path();

        let mut writer = FileWriter::new(root)
            .await
            .expect("Failed to create FileWriter");
        writer
            .write("file.txt", "content")
            .expect("Failed to write");
        writer.flush().await.expect("Failed to flush");

        // Get the modification time
        let file_path = root.join("file.txt");
        let metadata1 = fs::metadata(&file_path).expect("Failed to get metadata");
        let mtime1 = metadata1.modified().expect("Failed to get mtime");

        // Wait a bit to ensure time difference
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Write the same content again
        let mut writer = FileWriter::new(root)
            .await
            .expect("Failed to create FileWriter");
        writer
            .write("file.txt", "content")
            .expect("Failed to write");
        writer.flush().await.expect("Failed to flush");

        // Modification time should not change
        let metadata2 = fs::metadata(&file_path).expect("Failed to get metadata");
        let mtime2 = metadata2.modified().expect("Failed to get mtime");

        assert_eq!(
            mtime1, mtime2,
            "File should not be rewritten when content is unchanged"
        );
    }

    #[tokio::test]
    async fn test_file_deletion() {
        let temp_dir = create_temp_dir();
        let root = temp_dir.path();

        // First flush: create two files
        let mut writer = FileWriter::new(root)
            .await
            .expect("Failed to create FileWriter");
        writer
            .write("file1.txt", "content1")
            .expect("Failed to write file1");
        writer
            .write("file2.txt", "content2")
            .expect("Failed to write file2");
        writer.flush().await.expect("Failed to flush");

        assert_file_content(root, "file1.txt", "content1");
        assert_file_content(root, "file2.txt", "content2");

        // Second flush: only write file1, file2 should be deleted
        let mut writer = FileWriter::new(root)
            .await
            .expect("Failed to create FileWriter");
        writer
            .write("file1.txt", "content1")
            .expect("Failed to write file1");
        writer.flush().await.expect("Failed to flush");

        assert_file_content(root, "file1.txt", "content1");
        assert_file_not_exists(root, "file2.txt");
    }

    #[tokio::test]
    async fn test_empty_directory_removal() {
        let temp_dir = create_temp_dir();
        let root = temp_dir.path();

        // First flush: create files in subdirectories
        let mut writer = FileWriter::new(root)
            .await
            .expect("Failed to create FileWriter");
        writer
            .write("dir1/file1.txt", "content1")
            .expect("Failed to write");
        writer
            .write("dir2/file2.txt", "content2")
            .expect("Failed to write");
        writer.flush().await.expect("Failed to flush");

        assert_dir_exists(root, "dir1");
        assert_dir_exists(root, "dir2");

        // Second flush: only write to dir1, dir2 should be removed
        let mut writer = FileWriter::new(root)
            .await
            .expect("Failed to create FileWriter");
        writer
            .write("dir1/file1.txt", "content1")
            .expect("Failed to write");
        writer.flush().await.expect("Failed to flush");

        assert_dir_exists(root, "dir1");
        assert_dir_not_exists(root, "dir2");
    }

    #[tokio::test]
    async fn test_reject_absolute_path() {
        let temp_dir = create_temp_dir();
        let root = temp_dir.path();

        let mut writer = FileWriter::new(root)
            .await
            .expect("Failed to create FileWriter");
        let result = writer.write("/etc/passwd", "malicious");

        assert!(result.is_err(), "Should reject absolute path");
        assert!(result.unwrap_err().to_string().contains("must be relative"));
    }

    #[tokio::test]
    async fn test_reject_parent_dir_path() {
        let temp_dir = create_temp_dir();
        let root = temp_dir.path();

        let mut writer = FileWriter::new(root)
            .await
            .expect("Failed to create FileWriter");
        let result = writer.write("../etc/passwd", "malicious");

        assert!(result.is_err(), "Should reject path with '..'");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("cannot contain '..'")
        );
    }

    #[tokio::test]
    async fn test_reject_non_generated_directory() {
        let temp_dir = create_temp_dir();
        let root = temp_dir.path();

        // Create a file in the directory (without marker file)
        fs::create_dir_all(root).expect("Failed to create dir");
        fs::write(root.join("existing_file.txt"), "existing content")
            .expect("Failed to write file");

        // Should fail because directory exists but is not marked
        let result = FileWriter::new(root).await;

        assert!(result.is_err(), "Should reject non-generated directory");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not marked as a sqlex-generated directory")
        );
    }

    #[tokio::test]
    async fn test_marker_file_not_deleted() {
        let temp_dir = create_temp_dir();
        let root = temp_dir.path();

        // First flush: create a file
        let mut writer = FileWriter::new(root)
            .await
            .expect("Failed to create FileWriter");
        writer
            .write("file.txt", "content")
            .expect("Failed to write");
        writer.flush().await.expect("Failed to flush");

        // Second flush: don't write any files
        let mut writer = FileWriter::new(root)
            .await
            .expect("Failed to create FileWriter");
        writer.flush().await.expect("Failed to flush");

        // Marker file should still exist
        assert_file_content(root, MARKER_FILE, "");
        // But the other file should be deleted
        assert_file_not_exists(root, "file.txt");
    }
}
