use anyhow::{Context, Result};
use bytes::{BufMut, BytesMut};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::shared::database::auth::repositories::RealmRepository;

// Realm flag constants
const REALM_FLAG_OFFLINE: u8 = 0x02;
const REALM_FLAG_SPECIFYBUILD: u8 = 0x04;
const REALM_FLAG_NEW_PLAYERS: u8 = 0x20;
const REALM_FLAG_RECOMMENDED: u8 = 0x40;
const REALM_FLAG_VALID_MASK: u8 = 0x67; // OFFLINE | SPECIFYBUILD | NEW_PLAYERS | RECOMMENDED

/// Realm information structure
#[derive(Debug, Clone)]
pub struct Realm {
    pub id: u32,
    pub name: String,
    pub address: String,
    pub local_address: String,
    pub local_subnet_mask: u32,
    pub port: u16,
    pub icon: u8,
    pub flag: u8,
    pub timezone: u8,
    pub allowed_security_level: u8,
    pub population: f32,
    pub realm_builds: String,
    pub realm_builds_set: std::collections::HashSet<u32>, // Parsed realm builds
    pub realm_build_info: Option<crate::auth::realm::build_info::RealmBuildInfo>,
    pub last_seen: Option<DateTime<Utc>>, // Last heartbeat timestamp from world server
}

pub struct RealmList {
    realm_repo: Arc<RealmRepository>,
    realms: Arc<RwLock<Vec<Realm>>>,
    last_update: Arc<RwLock<DateTime<Utc>>>,
    update_interval_secs: u64,
    offline_threshold_secs: u64,
}

impl RealmList {
    pub fn new(
        realm_repo: Arc<RealmRepository>,
        update_interval_secs: u64,
        offline_threshold_secs: u64,
    ) -> Self {
        Self {
            realm_repo,
            realms: Arc::new(RwLock::new(Vec::new())),
            last_update: Arc::new(RwLock::new(DateTime::UNIX_EPOCH)),
            update_interval_secs,
            offline_threshold_secs,
        }
    }

    pub async fn load_realms(&self) -> Result<()> {
        info!("Loading realms from database...");

        let rows = self.realm_repo.find_all_active_realms().await?;

        let mut realms = Vec::new();

        for row in rows {
            let port: u16 = row.port.max(0) as u16;
            let last_seen_utc = row.last_seen;

            let valid_flags = row.realmflags & REALM_FLAG_VALID_MASK;
            let mut flag = if row.realmflags != valid_flags {
                error!(
                    "Realm {} has invalid flags, masking to valid flags",
                    row.name
                );
                valid_flags
            } else {
                row.realmflags
            };

            let now = Utc::now();
            let is_stale = match last_seen_utc {
                Some(ls) => {
                    let elapsed = (now - ls).num_seconds();
                    elapsed > self.offline_threshold_secs as i64
                }
                None => true, // No last_seen means never started, treat as offline
            };
            if is_stale {
                flag |= REALM_FLAG_OFFLINE;
                info!(
                    "Realm '{}' (id={}) marked offline: last_seen={:?}, threshold={}s",
                    row.name, row.id, last_seen_utc, self.offline_threshold_secs
                );
            }

            let realm_builds_set: std::collections::HashSet<u32> = row
                .realmbuilds
                .split_whitespace()
                .filter_map(|s| s.parse::<u32>().ok())
                .collect();

            let local_subnet_mask_u32 = row.local_subnet_mask.parse::<u32>().unwrap_or(0);

            let first_build = realm_builds_set.iter().next().copied().unwrap_or(0);
            let realm_build_info = if first_build > 0 { None } else { None };

            let realm_address = format!("{}:{}", row.address, port);
            info!(
                "Loaded realm '{}' (id={}): address='{}', port={}, final_address='{}'",
                row.name, row.id, row.address, port, realm_address
            );

            realms.push(Realm {
                id: row.id,
                name: row.name.clone(),
                address: realm_address,
                local_address: format!("{}:{}", row.local_address, port),
                local_subnet_mask: local_subnet_mask_u32,
                port,
                icon: row.icon,
                flag,
                timezone: row.timezone,
                allowed_security_level: row.allowed_security_level,
                population: row.population,
                realm_builds: row.realmbuilds,
                realm_builds_set,
                realm_build_info,
                last_seen: last_seen_utc,
            });
        }

        if realms.is_empty() {
            warn!("No realms found in database. Clients will see an empty realm list.");
        }

        *self.realms.write().await = realms.clone();
        *self.last_update.write().await = Utc::now();

        info!("Loaded {} realms", realms.len());
        Ok(())
    }

    pub async fn update_if_needed(&self) -> Result<()> {
        let last_update = *self.last_update.read().await;
        let now = Utc::now();
        let elapsed = (now - last_update).num_seconds() as u64;

        if elapsed >= self.update_interval_secs {
            self.load_realms().await?;
        }

        Ok(())
    }

    pub async fn get_realms(&self) -> Vec<Realm> {
        self.realms.read().await.clone()
    }

    /// Get realm address (local if client is on same subnet, otherwise remote)
    /// Matches C++ GetRealmAddress implementation
    fn get_realm_address(realm: &Realm, _client_ip: Option<&std::net::IpAddr>) -> String {
        // Simplified: always return remote address
        // Full implementation would check if client_ip is in local_subnet_mask
        realm.address.clone()
    }

    /// Build realm list packet for client
    /// Matches the C++ LoadRealmlist implementation
    pub async fn build_packet(
        &self,
        client_build: u16,
        account_id: Option<u32>,
        account_security: &std::collections::HashMap<u32, u8>,
        account_default_security: u8,
        allowed_builds: &crate::auth::realm::AllowedBuilds,
        client_ip: Option<&std::net::IpAddr>,
    ) -> Result<BytesMut> {
        // Update realms if needed
        self.update_if_needed().await?;

        let realms = self.get_realms().await;
        let mut buf = BytesMut::new();

        // For builds < 6299 (before 2.0.3), format is different
        if client_build < 6299 {
            // Count eligible realms (C++: getEligibleRealmCount)
            // Eligible = realms where allowedSecurityLevel <= accountSecurityLevel
            let eligible_count = realms
                .iter()
                .filter(|realm| {
                    let account_security = account_security
                        .get(&realm.id)
                        .copied()
                        .unwrap_or(account_default_security);
                    realm.allowed_security_level <= account_security
                })
                .count();

            buf.put_u32_le(0); // unused value
            buf.put_u8(eligible_count as u8);

            for realm in &realms {
                // Check account security level for this realm
                let account_security = account_security
                    .get(&realm.id)
                    .copied()
                    .unwrap_or(account_default_security);

                // C++: if (!securityLevel && i.second.allowedSecurityLevel > 0) continue;
                // Don't display higher security realms for players with no security level
                if account_security == 0 && realm.allowed_security_level > 0 {
                    continue; // Skip this realm entirely
                }

                // Get character count for this account on this realm
                let char_count = if let Some(acc_id) = account_id {
                    self.realm_repo
                        .find_character_count(realm.id, acc_id as u64)
                        .await
                        .unwrap_or(None)
                        .unwrap_or(0)
                } else {
                    0
                };

                let ok_build = realm.realm_builds_set.contains(&(client_build as u32));

                let build_info = if ok_build {
                    allowed_builds.find_build(client_build).await
                } else {
                    realm.realm_build_info.clone()
                };

                let lock = if realm.allowed_security_level > account_security {
                    1
                } else {
                    0
                };

                let mut realmflags = realm.flag;

                if !ok_build || lock != 0 {
                    realmflags |= REALM_FLAG_OFFLINE;
                }

                let category_id =
                    crate::auth::realm::category::get_realm_category_id_by_build_and_zone(
                        client_build,
                        realm.timezone,
                        build_info.as_ref(),
                    );

                // Realm icon (u32)
                buf.put_u32_le(realm.icon as u32);

                // Realm flags (u8)
                buf.put_u8(realmflags);

                // Realm name (null-terminated string)
                // For older clients, append version info if available
                let mut realm_name = realm.name.clone();
                if let Some(ref b_info) = build_info {
                    realm_name.push_str(&format!(
                        " ({},{},{})",
                        b_info.major_version, b_info.minor_version, b_info.bugfix_version
                    ));
                }
                buf.put_slice(realm_name.as_bytes());
                buf.put_u8(0);

                // Realm address (null-terminated string) - use GetRealmAddress
                let realm_addr = Self::get_realm_address(realm, client_ip);
                info!(
                    "Sending realm '{}' (id={}) with address: '{}' (vanilla format)",
                    realm.name, realm.id, realm_addr
                );
                buf.put_slice(realm_addr.as_bytes());
                buf.put_u8(0);

                // Population (f32)
                buf.put_f32_le(realm.population);

                // Number of characters (u8)
                buf.put_u8(char_count);

                // Realm category (u8)
                buf.put_u8(category_id);

                // Unknown field (u8)
                buf.put_u8(0x00);
            }

            // Add unused value at the end (matches C++: pkt << uint16(0x0002))
            buf.put_u16_le(0x0002);
        } else {
            // For builds >= 6299, use newer format
            buf.put_u16_le(0); // Placeholder for size
            buf.put_u16_le(0); // Unknown field
            buf.put_u16_le(realms.len() as u16);

            for realm in &realms {
                let ok_build = realm.realm_builds_set.contains(&(client_build as u32));

                // Get build info
                let build_info = if ok_build {
                    allowed_builds.find_build(client_build).await
                } else {
                    realm.realm_build_info.clone()
                };

                // Check if realm is locked
                let account_security = account_security
                    .get(&realm.id)
                    .copied()
                    .unwrap_or(account_default_security);
                let lock = if realm.allowed_security_level > account_security {
                    1
                } else {
                    0
                };

                let mut realmflags = realm.flag;

                // Show offline state for unsupported client builds
                if !ok_build {
                    realmflags |= REALM_FLAG_OFFLINE;
                }

                // Remove SPECIFYBUILD flag if no build info
                if build_info.is_none() {
                    realmflags &= !REALM_FLAG_SPECIFYBUILD;
                }

                // Get realm category ID
                let _category_id =
                    crate::auth::realm::category::get_realm_category_id_by_build_and_zone(
                        client_build,
                        realm.timezone,
                        build_info.as_ref(),
                    );

                // Realm type (u8): icon value
                buf.put_u8(realm.icon);

                // Lock flag (u8): 0x01 if realm locked
                buf.put_u8(lock);

                // Realm flags (u8)
                buf.put_u8(realmflags);

                // Realm name (null-terminated string)
                buf.put_slice(realm.name.as_bytes());
                buf.put_u8(0);

                // Realm address (null-terminated string) - use GetRealmAddress
                let realm_addr = Self::get_realm_address(realm, client_ip);
                info!(
                    "Sending realm '{}' (id={}) with address: '{}' (TBC+ format)",
                    realm.name, realm.id, realm_addr
                );
                buf.put_slice(realm_addr.as_bytes());
                buf.put_u8(0);

                // Population (f32)
                buf.put_f32_le(realm.population);

                // Number of characters (u8)
                let char_count = if let Some(acc_id) = account_id {
                    self.realm_repo
                        .find_character_count(realm.id, acc_id as u64)
                        .await
                        .unwrap_or(None)
                        .unwrap_or(0)
                } else {
                    0
                };
                buf.put_u8(char_count);

                // Timezone (u8)
                buf.put_u8(realm.timezone);

                // Unknown (u8)
                buf.put_u8(0);
            }

            // Update size field at the beginning
            let size = (buf.len() - 2) as u16;
            buf[0] = (size & 0xFF) as u8;
            buf[1] = ((size >> 8) & 0xFF) as u8;
        }

        Ok(buf)
    }

    /// Get character count for a realm (for a specific account)
    pub async fn get_character_count(&self, _account_id: u32, _realm_id: u32) -> Result<u8> {
        // This would query the world database, but for now we'll return 0
        // In a full implementation, this would connect to the world server
        // or query a shared database
        Ok(0)
    }

    pub async fn start_update_task(self: Arc<Self>) -> Result<()> {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(self.update_interval_secs));

        loop {
            interval.tick().await;
            if let Err(e) = self.load_realms().await {
                error!("Failed to update realm list: {}", e);
            }
        }
    }
}
