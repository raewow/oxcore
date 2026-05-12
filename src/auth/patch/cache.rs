use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// MD5 hash of a patch file
pub type PatchHash = [u8; 16];

/// Patch information structure
#[derive(Debug, Clone)]
pub struct PatchInfo {
    pub md5: PatchHash,
}

/// Cache for patch file MD5 hashes
/// Scans the patches directory and caches MD5 hashes of .mpq files
pub struct PatchCache {
    patches: Arc<RwLock<HashMap<String, PatchInfo>>>,
    patches_dir: PathBuf,
}

impl PatchCache {
    /// Create a new patch cache and load patches from the directory
    pub async fn new(patches_dir: impl AsRef<Path>) -> Result<Self> {
        let patches_dir = patches_dir.as_ref().to_path_buf();
        let cache = Self {
            patches: Arc::new(RwLock::new(HashMap::new())),
            patches_dir,
        };

        cache.load_patches_info().await?;
        Ok(cache)
    }

    /// Load patch information from the patches directory
    async fn load_patches_info(&self) -> Result<()> {
        info!("Loading patch info from folder: {:?}", self.patches_dir);

        let mut entries = match fs::read_dir(&self.patches_dir).await {
            Ok(entries) => entries,
            Err(e) => {
                warn!("Failed to read patches directory: {}", e);
                return Ok(()); // Directory doesn't exist, that's okay
            }
        };

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if let Some(ext) = path.extension() {
                if ext == "mpq" {
                    let file_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    if file_name.len() >= 8 {
                        debug!("Found patch file: {:?}", path);
                        self.load_patch_md5(&path).await?;
                    }
                }
            }
        }

        let count = self.patches.read().await.len();
        info!("Loaded {} patch files", count);
        Ok(())
    }

    /// Load MD5 hash for a specific patch file
    async fn load_patch_md5(&self, file_path: &Path) -> Result<()> {
        let path_str = file_path.to_string_lossy().to_string();
        debug!("Loading patch MD5 from file: {}", path_str);

        let mut file = match fs::File::open(file_path).await {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to open patch file {}: {}", path_str, e);
                return Ok(());
            }
        };

        // Calculate MD5 hash using md5 0.7's Context API for incremental hashing
        let mut hasher = md5::Context::new();
        let mut buffer = vec![0u8; 4 * 1024]; // 4KB chunks

        loop {
            let n = match tokio::io::AsyncReadExt::read(&mut file, &mut buffer).await {
                Ok(0) => break, // EOF
                Ok(n) => n,
                Err(e) => {
                    warn!("Error reading patch file {}: {}", path_str, e);
                    return Ok(());
                }
            };
            hasher.consume(&buffer[..n]);
        }

        let hash = hasher.compute();
        // md5::Digest is a [u8; 16] array, access it directly
        let md5_hash = hash.0;

        // Store in cache
        let patch_info = PatchInfo { md5: md5_hash };
        self.patches.write().await.insert(path_str, patch_info);

        Ok(())
    }

    /// Get MD5 hash for a patch file
    /// Returns None if the patch is not in cache
    pub async fn get_hash(&self, file_path: &str) -> Option<PatchHash> {
        let patches = self.patches.read().await;

        // Try exact match first
        if let Some(info) = patches.get(file_path) {
            return Some(info.md5);
        }

        // Try case-insensitive match
        for (key, info) in patches.iter() {
            if key.eq_ignore_ascii_case(file_path) {
                return Some(info.md5);
            }
        }

        None
    }

    /// Reload patch MD5 for a specific file (useful when patch is added at runtime)
    pub async fn reload_patch(&self, file_path: impl AsRef<Path>) -> Result<()> {
        self.load_patch_md5(file_path.as_ref()).await
    }

    /// Get all cached patch file paths
    pub async fn get_patch_files(&self) -> Vec<String> {
        self.patches.read().await.keys().cloned().collect()
    }
}
