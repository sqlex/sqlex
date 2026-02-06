//! Container pool management for reusing Docker containers.

use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    future::Future,
    io::{ErrorKind, Read, Write},
    path::PathBuf,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sqlex_analyzer::{AnalyzerError, Result};
use sqlex_common::dialect::Dialect;
use tokio::sync::Mutex as AsyncMutex;

use crate::docker_raw;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub container_id: String,
    pub dialect: Dialect,
    pub host: String,
    pub port: u16,
    pub password: String,
}

struct ContainerEntry {
    info: ContainerInfo,
    _lock_file: File,
    lock_path: PathBuf,
}

pub struct ContainerPool {
    containers: Mutex<HashMap<Dialect, ContainerEntry>>,
    create_lock: AsyncMutex<()>,
    orphan_cleanup_lock: AsyncMutex<()>,
    orphan_cleaned: AtomicBool,
}

static GLOBAL_POOL: OnceLock<ContainerPool> = OnceLock::new();

impl ContainerPool {
    pub fn global() -> &'static Self {
        GLOBAL_POOL.get_or_init(|| {
            unsafe {
                libc::atexit(cleanup_on_exit);
            }
            Self {
                containers: Mutex::new(HashMap::new()),
                create_lock: AsyncMutex::new(()),
                orphan_cleanup_lock: AsyncMutex::new(()),
                orphan_cleaned: AtomicBool::new(false),
            }
        })
    }

    pub async fn get_or_create_container<F, Fut>(
        &self,
        dialect: Dialect,
        creator: F,
    ) -> Result<ContainerInfo>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<ContainerInfo>>,
    {
        self.cleanup_orphans_once().await;

        let existing = self
            .containers
            .lock()
            .map_err(|e| {
                AnalyzerError::ExecutionError(format!("Failed to lock container pool: {}", e))
            })?
            .get(&dialect)
            .map(|entry| entry.info.clone());
        if let Some(container) = existing {
            return Ok(container);
        }

        let _guard = self.create_lock.lock().await;
        let existing = self
            .containers
            .lock()
            .map_err(|e| {
                AnalyzerError::ExecutionError(format!("Failed to lock container pool: {}", e))
            })?
            .get(&dialect)
            .map(|entry| entry.info.clone());
        if let Some(container) = existing {
            return Ok(container);
        }

        let info = creator().await?;
        if info.dialect != dialect {
            return Err(AnalyzerError::ExecutionError(format!(
                "Container dialect mismatch: expected {}, got {}",
                dialect, info.dialect
            )));
        }

        let dir = std::env::temp_dir().join("sqlex").join("containers");
        fs::create_dir_all(&dir)
            .map_err(|e| AnalyzerError::ExecutionError(format!("Failed to create dir: {}", e)))?;

        let short_id: String = info.container_id.chars().take(12).collect();
        let lock_path = dir.join(format!("{}_{}.json", info.dialect, short_id));
        let mut file = File::create(&lock_path).map_err(|e| {
            AnalyzerError::ExecutionError(format!("Failed to create lock file: {}", e))
        })?;

        file.try_lock_exclusive()
            .map_err(|e| AnalyzerError::ExecutionError(format!("Failed to lock file: {}", e)))?;

        let json = serde_json::to_string(&info)
            .map_err(|e| AnalyzerError::ExecutionError(format!("Failed to serialize: {}", e)))?;
        file.write_all(json.as_bytes())
            .map_err(|e| AnalyzerError::ExecutionError(format!("Failed to write: {}", e)))?;
        file.sync_all().map_err(|e| {
            AnalyzerError::ExecutionError(format!("Failed to sync lock file: {}", e))
        })?;

        let entry = ContainerEntry {
            info: info.clone(),
            _lock_file: file,
            lock_path,
        };

        let mut guard = self.containers.lock().map_err(|e| {
            AnalyzerError::ExecutionError(format!("Failed to lock container pool: {}", e))
        })?;
        guard.insert(info.dialect, entry);

        Ok(info)
    }

    async fn cleanup_orphans_once(&self) {
        if self.orphan_cleaned.load(Ordering::Acquire) {
            return;
        }

        let _guard = self.orphan_cleanup_lock.lock().await;
        if self.orphan_cleaned.load(Ordering::Relaxed) {
            return;
        }

        self.cleanup_orphans();
        self.orphan_cleaned.store(true, Ordering::Release);
    }

    fn cleanup_orphans(&self) {
        let dir = std::env::temp_dir().join("sqlex").join("containers");
        let mut protected_container_ids = HashSet::new();

        if dir.exists() {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !matches!(path.extension().and_then(|ext| ext.to_str()), Some("json")) {
                        continue;
                    }

                    let mut file = match OpenOptions::new().read(true).write(true).open(&path) {
                        Ok(file) => file,
                        Err(_) => continue,
                    };

                    if let Err(err) = file.try_lock_exclusive() {
                        if err.kind() == ErrorKind::WouldBlock {
                            if let Ok(mut protected_file) = File::open(&path) {
                                let mut protected_content = String::new();
                                if protected_file
                                    .read_to_string(&mut protected_content)
                                    .is_ok()
                                {
                                    if let Ok(info) =
                                        serde_json::from_str::<ContainerInfo>(&protected_content)
                                    {
                                        protected_container_ids.insert(info.container_id);
                                    }
                                }
                            }
                            continue;
                        }
                        continue;
                    }

                    let mut content = String::new();
                    if file.read_to_string(&mut content).is_ok() {
                        if let Ok(info) = serde_json::from_str::<ContainerInfo>(&content) {
                            let _ = file.unlock();
                            self.cleanup_container_sync(&info.container_id, &path);
                            continue;
                        }
                    }

                    let _ = file.unlock();
                    let _ = fs::remove_file(&path);
                }
            }
        }

        self.cleanup_untracked_managed_containers(&protected_container_ids);
    }

    fn cleanup_untracked_managed_containers(&self, protected_container_ids: &HashSet<String>) {
        let managed_container_ids = docker_raw::list_managed_container_ids();

        for container_id in managed_container_ids {
            let protected = protected_container_ids
                .iter()
                .any(|id| Self::container_id_matches(id, &container_id));
            if !protected {
                docker_raw::cleanup_container_only_sync(&container_id);
            }
        }
    }

    fn cleanup_container_sync(&self, container_id: &str, lock_path: &PathBuf) {
        docker_raw::cleanup_container_only_sync(container_id);
        let _ = fs::remove_file(lock_path);
    }

    fn container_id_matches(left: &str, right: &str) -> bool {
        left == right || left.starts_with(right) || right.starts_with(left)
    }
}

extern "C" fn cleanup_on_exit() {
    let _ = std::panic::catch_unwind(|| {
        if let Some(pool) = GLOBAL_POOL.get() {
            let containers = if let Ok(mut guard) = pool.containers.lock() {
                let containers: Vec<(String, PathBuf)> = guard
                    .values()
                    .map(|entry| (entry.info.container_id.clone(), entry.lock_path.clone()))
                    .collect();
                guard.clear();
                containers
            } else {
                Vec::new()
            };

            for (container_id, lock_path) in containers {
                pool.cleanup_container_sync(&container_id, &lock_path);
            }
        }
    });
}
