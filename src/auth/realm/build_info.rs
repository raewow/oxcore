use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::shared::database::auth::repositories::RealmRepository;

/// Information about a client build
#[derive(Debug, Clone)]
pub struct RealmBuildInfo {
    pub major_version: u8,
    pub minor_version: u8,
    pub bugfix_version: u8,
    pub hotfix_version: char,
    pub build: u16,
    pub os: u32,
    pub platform: u32,
    pub integrity_hash: [u8; 20],
}

/// Manager for allowed client builds
pub struct AllowedBuilds {
    builds: Arc<RwLock<Vec<RealmBuildInfo>>>,
    builds_by_id: Arc<RwLock<HashMap<u16, Vec<usize>>>>, // build -> indices
}

impl AllowedBuilds {
    pub fn new() -> Self {
        Self {
            builds: Arc::new(RwLock::new(Vec::new())),
            builds_by_id: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load allowed client builds from database
    pub async fn load_from_db(realm_repo: &RealmRepository) -> Result<Self> {
        info!("Loading allowed client builds from database...");

        let rows = realm_repo.find_allowed_clients().await?;

        let mut builds = Vec::new();
        let mut builds_by_id = HashMap::new();

        for row in rows {
            let build = row.build as u16;
            let os_str = row.os;
            let platform_str = row.platform;
            let integrity_hash_str = row.integrity_hash;

            // Parse OS (3-byte string stored as u32)
            // C++: MANGOS_ASSERT(os.size() == 3); memcpy(&buildInfo.os, os.data(), 4);
            let os = if os_str.len() >= 3 {
                let os_bytes = os_str.as_bytes();
                u32::from_le_bytes([
                    os_bytes[0],
                    os_bytes.get(1).copied().unwrap_or(0),
                    os_bytes.get(2).copied().unwrap_or(0),
                    0,
                ])
            } else {
                0
            };

            // Parse platform (3-byte string stored as u32)
            // C++: MANGOS_ASSERT(platform.size() == 3); memcpy(&buildInfo.platform, platform.data(), 4);
            let platform = if platform_str.len() >= 3 {
                let platform_bytes = platform_str.as_bytes();
                u32::from_le_bytes([
                    platform_bytes[0],
                    platform_bytes.get(1).copied().unwrap_or(0),
                    platform_bytes.get(2).copied().unwrap_or(0),
                    0,
                ])
            } else {
                0
            };

            // Parse integrity hash (40 hex chars = 20 bytes)
            // C++: if (!integrityHash.empty()) { MANGOS_ASSERT(integrityHash.size() == (20 * 2)); HexStrToByteArray(...); }
            let mut integrity_hash = [0u8; 20];
            if !integrity_hash_str.is_empty() {
                let hash_str = integrity_hash_str.trim();
                // Only parse if it's exactly 40 hex characters (20 bytes * 2 for hex)
                if hash_str.len() == 40 {
                    if let Ok(hash_bytes) = hex::decode(hash_str) {
                        if hash_bytes.len() == 20 {
                            integrity_hash.copy_from_slice(&hash_bytes);
                        }
                    }
                }
            }

            let build_info = RealmBuildInfo {
                major_version: row.major_version,
                minor_version: row.minor_version,
                bugfix_version: row.bugfix_version,
                hotfix_version: row.hotfix_version.chars().next().unwrap_or(' '),
                build,
                os,
                platform,
                integrity_hash,
            };

            let index = builds.len();
            builds.push(build_info);
            builds_by_id
                .entry(build)
                .or_insert_with(Vec::new)
                .push(index);
        }

        info!("Loaded {} allowed client builds", builds.len());

        Ok(Self {
            builds: Arc::new(RwLock::new(builds)),
            builds_by_id: Arc::new(RwLock::new(builds_by_id)),
        })
    }

    /// Find build info by build number only
    pub async fn find_build(&self, build: u16) -> Option<RealmBuildInfo> {
        let builds = self.builds.read().await;
        let builds_by_id = self.builds_by_id.read().await;

        // First build is low bound of always accepted range
        if let Some(first) = builds.first() {
            if build >= first.build {
                return Some(first.clone());
            }
        }

        // Check for exact match
        if let Some(indices) = builds_by_id.get(&build) {
            if let Some(&index) = indices.first() {
                return builds.get(index).cloned();
            }
        }

        None
    }

    /// Find build info by build, OS, and platform
    pub async fn find_build_with_os_platform(
        &self,
        build: u16,
        os: u32,
        platform: u32,
    ) -> Vec<RealmBuildInfo> {
        let builds = self.builds.read().await;
        let mut matching = Vec::new();

        for build_info in builds.iter() {
            if build_info.build == build && build_info.os == os && build_info.platform == platform {
                matching.push(build_info.clone());
            }
        }

        matching
    }

    /// Get all builds (for iteration)
    pub async fn get_all_builds(&self) -> Vec<RealmBuildInfo> {
        self.builds.read().await.clone()
    }
}

impl Default for AllowedBuilds {
    fn default() -> Self {
        Self::new()
    }
}
