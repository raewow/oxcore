// Instance manager - handles all instance operations

use anyhow::{Context, Result};
use oxcore_db::database::Databases;
use oxcore_shared::game::instance::{
    InstanceBinding, InstanceResetMethod, InstanceResetWarningType, InstanceState,
};
use oxcore_shared::protocol::ObjectGuid;
use parking_lot::RwLock;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Instance manager - handles all instance operations
pub struct InstanceMgr {
    /// Active instances by (map_id, instance_id)
    instances: Arc<RwLock<HashMap<(u32, u32), Arc<RwLock<InstanceState>>>>>,
    /// Player bindings (player_guid -> map_id -> binding)
    player_bindings: Arc<RwLock<HashMap<ObjectGuid, HashMap<u32, InstanceBinding>>>>,
    /// Group bindings (group_id -> map_id -> binding)
    group_bindings: Arc<RwLock<HashMap<u32, HashMap<u32, InstanceBinding>>>>,
    /// Next instance ID per map
    next_instance_ids: Arc<RwLock<HashMap<u32, u32>>>,
    /// Instances that need reset warnings (map_id, instance_id) -> last warning sent
    reset_warnings: Arc<RwLock<HashMap<(u32, u32), InstanceResetWarningType>>>,
    /// Instances pending reset (empty instances that should be reset after delay)
    pending_resets: Arc<RwLock<HashMap<(u32, u32), u64>>>, // (map_id, instance_id) -> reset time
}

impl InstanceMgr {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            player_bindings: Arc::new(RwLock::new(HashMap::new())),
            group_bindings: Arc::new(RwLock::new(HashMap::new())),
            next_instance_ids: Arc::new(RwLock::new(HashMap::new())),
            reset_warnings: Arc::new(RwLock::new(HashMap::new())),
            pending_resets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize instance manager (load next instance IDs from database)
    pub async fn initialize(&self, databases: &Databases) -> Result<()> {
        // Load next instance IDs per map
        let rows = sqlx::query::<sqlx::Postgres>(
            r#"SELECT map, MAX(id) FROM characters.instance GROUP BY map"#,
        )
        .fetch_all(&databases.character)
        .await
        .context("Failed to query max instance IDs")?;

        let mut next_ids = self.next_instance_ids.write();
        for row in rows {
            let map_id = row.try_get::<i64, _>(0).unwrap_or(0) as u32;
            let max_id = row.try_get::<Option<i64>, _>(1).unwrap_or(None);
            if let Some(max) = max_id {
                next_ids.insert(map_id, (max + 1) as u32);
            } else {
                next_ids.insert(map_id, 1);
            }
        }

        Ok(())
    }

    /// Create a new instance
    pub async fn create_instance(
        &self,
        databases: &Databases,
        map_id: u32,
        difficulty: u8,
        permanent: bool,
    ) -> Result<u32> {
        // Generate instance ID
        let instance_id = {
            let mut next_ids = self.next_instance_ids.write();
            let id = next_ids.get(&map_id).copied().unwrap_or(1);
            next_ids.insert(map_id, id + 1);
            id
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Calculate reset time (default: 7 days for raids, 1 day for dungeons)
        // TODO: Load from DBC data
        let reset_time = if permanent {
            u64::MAX
        } else {
            // Default reset time: 7 days for raids, 1 day for dungeons
            // This should be loaded from Map.dbc
            now + (7 * 24 * 60 * 60) // 7 days default
        };

        let instance = InstanceState {
            map_id,
            instance_id,
            difficulty,
            permanent,
            reset_time,
            created_time: now,
            completed_encounters: Vec::new(),
        };

        // Save to database (note: difficulty column doesn't exist in vanilla schema)
        sqlx::query::<sqlx::Postgres>(r#"INSERT INTO characters.instance (id, map, reset_time, data) VALUES ($1, $2, $3, '')"#)
            .bind(i64::from(instance_id))
            .bind(i64::from(map_id))
            .bind(reset_time as i64)
            .execute(&databases.character)
            .await
            .context("Failed to create instance")?;

        // Add to cache
        {
            let mut instances = self.instances.write();
            instances.insert((map_id, instance_id), Arc::new(RwLock::new(instance)));
        }

        Ok(instance_id)
    }

    /// Load instance from database
    pub async fn load_instance(
        &self,
        databases: &Databases,
        map_id: u32,
        instance_id: u32,
    ) -> Result<Option<Arc<RwLock<InstanceState>>>> {
        // Check cache first
        {
            let instances = self.instances.read();
            if let Some(instance) = instances.get(&(map_id, instance_id)) {
                return Ok(Some(instance.clone()));
            }
        }

        // Load from database (including data field for encounter state)
        let row = sqlx::query::<sqlx::Postgres>(
            r#"SELECT id, map, reset_time, data FROM characters.instance WHERE id = $1 AND map = $2"#,
        )
        .bind(i64::from(instance_id))
        .bind(i64::from(map_id))
        .fetch_optional(&databases.character)
        .await
        .context("Failed to query instance")?;

        if let Some(row) = row {
            let reset_time: i64 = row.get(2);
            let data: Option<String> = row.get(3);
            let difficulty: u8 = 0; // Difficulty not in vanilla schema

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // Parse completed encounters from data field
            // Format: comma-separated encounter IDs (e.g., "1,5,10")
            let completed_encounters = Self::parse_instance_data(&data.unwrap_or_default());

            let instance = InstanceState {
                map_id,
                instance_id,
                difficulty,
                permanent: reset_time == 0 || reset_time as u64 == u64::MAX,
                reset_time: reset_time as u64,
                created_time: now,
                completed_encounters,
            };

            let instance = Arc::new(RwLock::new(instance));

            // Add to cache
            {
                let mut instances = self.instances.write();
                instances.insert((map_id, instance_id), instance.clone());
            }

            Ok(Some(instance))
        } else {
            Ok(None)
        }
    }

    /// Parse instance data string to extract completed encounters
    /// Format: comma-separated encounter IDs (e.g., "1,5,10")
    fn parse_instance_data(data: &str) -> Vec<u32> {
        if data.is_empty() {
            return Vec::new();
        }

        data.split(',')
            .filter_map(|s| s.trim().parse::<u32>().ok())
            .collect()
    }

    /// Serialize completed encounters to data string
    /// Format: comma-separated encounter IDs (e.g., "1,5,10")
    fn serialize_instance_data(completed_encounters: &[u32]) -> String {
        completed_encounters
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Save instance data to database
    pub async fn save_instance_data(
        &self,
        databases: &Databases,
        map_id: u32,
        instance_id: u32,
    ) -> Result<()> {
        let data = {
            let instances = self.instances.read();
            if let Some(instance) = instances.get(&(map_id, instance_id)) {
                let instance = instance.read();
                Self::serialize_instance_data(&instance.completed_encounters)
            } else {
                return Ok(()); // Instance not in cache
            }
        };

        sqlx::query::<sqlx::Postgres>(
            r#"UPDATE characters.instance SET data = $1 WHERE id = $2 AND map = $3"#,
        )
        .bind(&data)
        .bind(i64::from(instance_id))
        .bind(i64::from(map_id))
        .execute(&databases.character)
        .await
        .context("Failed to save instance data")?;

        Ok(())
    }

    /// Mark an encounter as completed for an instance
    pub fn mark_encounter_completed(&self, map_id: u32, instance_id: u32, encounter_id: u32) {
        let instances = self.instances.read();
        if let Some(instance) = instances.get(&(map_id, instance_id)) {
            let mut instance = instance.write();
            if !instance.completed_encounters.contains(&encounter_id) {
                instance.completed_encounters.push(encounter_id);
            }
        }
    }

    /// Check if an encounter is completed for an instance
    pub fn is_encounter_completed(&self, map_id: u32, instance_id: u32, encounter_id: u32) -> bool {
        let instances = self.instances.read();
        if let Some(instance) = instances.get(&(map_id, instance_id)) {
            let instance = instance.read();
            instance.completed_encounters.contains(&encounter_id)
        } else {
            false
        }
    }

    /// Bind player to instance
    pub async fn bind_player_to_instance(
        &self,
        databases: &Databases,
        player_guid: ObjectGuid,
        map_id: u32,
        instance_id: u32,
        permanent: bool,
        reset_time: u64,
    ) -> Result<()> {
        // Save to database
        sqlx::query::<sqlx::Postgres>(
            r#"INSERT INTO characters.character_instance (guid, instance, permanent, extend) VALUES ($1, $2, $3, 0) ON CONFLICT (guid, instance) DO UPDATE SET permanent = EXCLUDED.permanent, extend = EXCLUDED.extend"#
        )
        .bind(i64::from(player_guid.low()))
        .bind(i64::from(instance_id))
        .bind(if permanent { 1i16 } else { 0i16 })
        .execute(&databases.character)
        .await
        .context("Failed to bind player to instance")?;

        // Update cache
        {
            let mut bindings = self.player_bindings.write();
            let player_bindings = bindings.entry(player_guid).or_insert_with(HashMap::new);
            player_bindings.insert(
                map_id,
                InstanceBinding {
                    map_id,
                    instance_id,
                    permanent,
                    reset_time,
                },
            );
        }

        Ok(())
    }

    /// Bind group to instance (uses leader GUID as per vanilla schema)
    pub async fn bind_group_to_instance(
        &self,
        databases: &Databases,
        leader_guid: ObjectGuid,
        _map_id: u32,
        instance_id: u32,
        permanent: bool,
        _reset_time: u64,
    ) -> Result<()> {
        // Save to database (uses leader_guid as per vanilla schema)
        sqlx::query::<sqlx::Postgres>(
            r#"INSERT INTO characters.group_instance (leader_guid, instance, permanent) VALUES ($1, $2, $3) ON CONFLICT (leader_guid, instance) DO UPDATE SET permanent = EXCLUDED.permanent"#,
        )
        .bind(i64::from(leader_guid.low()))
        .bind(i64::from(instance_id))
        .bind(if permanent { 1i16 } else { 0i16 })
        .execute(&databases.character)
        .await
        .context("Failed to bind group to instance")?;

        // Note: We don't cache group bindings by group_id since schema uses leader_guid
        // Groups will load bindings from their leader's bindings

        Ok(())
    }

    /// Get player's instance binding for a map
    pub fn get_player_binding(
        &self,
        player_guid: ObjectGuid,
        map_id: u32,
    ) -> Option<InstanceBinding> {
        let bindings = self.player_bindings.read();
        bindings.get(&player_guid)?.get(&map_id).cloned()
    }

    /// Get group's instance binding for a map (via leader GUID)
    pub fn get_group_binding(
        &self,
        leader_guid: ObjectGuid,
        map_id: u32,
    ) -> Option<InstanceBinding> {
        // Group bindings are stored by leader GUID in vanilla schema
        self.get_player_binding(leader_guid, map_id)
    }

    /// Transfer a group's instance bindings from the old leader to the new one,
    /// mirroring `Group::_setLeader` (reference `Group.cpp:1710-1786`).
    ///
    /// Group bindings are keyed by leader GUID in the vanilla schema, and the in-memory
    /// `player_bindings` cache stores them under that same key, so the cache entry is
    /// re-keyed here too.
    pub async fn transfer_group_bindings(
        &self,
        databases: &Databases,
        old_leader: ObjectGuid,
        new_leader: ObjectGuid,
    ) -> Result<()> {
        let old_low = i64::from(old_leader.low());
        let new_low = i64::from(new_leader.low());

        // 1. Delete the group's permanent binds and any bind the new leader already holds
        //    personally (those get replaced by the new leader's permanent binds below).
        sqlx::query::<sqlx::Postgres>(
            r#"DELETE FROM characters.group_instance
               WHERE leader_guid = $1 AND (permanent = 1 OR instance IN
                 (SELECT instance FROM characters.character_instance WHERE guid = $2))"#,
        )
        .bind(old_low)
        .bind(new_low)
        .execute(&databases.character)
        .await
        .context("Failed to delete group bindings on leader change")?;

        // 2. For each remaining non-permanent group bind the new leader already holds
        //    personally, give the group's save to the old leader if they have none, then
        //    drop the group row.
        let rows = sqlx::query::<sqlx::Postgres>(
            r#"SELECT instance FROM characters.group_instance WHERE leader_guid = $1"#,
        )
        .bind(old_low)
        .fetch_all(&databases.character)
        .await
        .context("Failed to query remaining group bindings")?;

        for row in rows {
            let instance_id: i64 = row.get(0);

            let new_has_personal = sqlx::query::<sqlx::Postgres>(
                r#"SELECT 1 FROM characters.character_instance WHERE guid = $1 AND instance = $2"#,
            )
            .bind(new_low)
            .bind(instance_id)
            .fetch_optional(&databases.character)
            .await
            .context("Failed to check new leader personal bind")?;

            if new_has_personal.is_none() {
                continue;
            }

            let old_has_personal = sqlx::query::<sqlx::Postgres>(
                r#"SELECT 1 FROM characters.character_instance WHERE guid = $1 AND instance = $2"#,
            )
            .bind(old_low)
            .bind(instance_id)
            .fetch_optional(&databases.character)
            .await
            .context("Failed to check old leader personal bind")?;

            if old_has_personal.is_none() {
                sqlx::query::<sqlx::Postgres>(
                    r#"INSERT INTO characters.character_instance (guid, instance, permanent, extend)
                       SELECT $1, instance, permanent, 0 FROM characters.group_instance
                       WHERE leader_guid = $2 AND instance = $3"#,
                )
                .bind(old_low)
                .bind(old_low)
                .bind(instance_id)
                .execute(&databases.character)
                .await
                .context("Failed to hand group save to old leader")?;
            }

            sqlx::query::<sqlx::Postgres>(
                r#"DELETE FROM characters.group_instance WHERE leader_guid = $1 AND instance = $2"#,
            )
            .bind(old_low)
            .bind(instance_id)
            .execute(&databases.character)
            .await
            .context("Failed to delete replaced group bind")?;
        }

        // 3. Move the remaining group binds to the new leader.
        sqlx::query::<sqlx::Postgres>(
            r#"UPDATE characters.group_instance SET leader_guid = $1 WHERE leader_guid = $2"#,
        )
        .bind(new_low)
        .bind(old_low)
        .execute(&databases.character)
        .await
        .context("Failed to transfer group bindings to new leader")?;

        // 4. Copy the new leader's permanent personal binds into the group.
        sqlx::query::<sqlx::Postgres>(
            r#"INSERT INTO characters.group_instance (leader_guid, instance, permanent)
               SELECT $1, instance, permanent FROM characters.character_instance
               WHERE guid = $1 AND permanent = 1
               ON CONFLICT (leader_guid, instance) DO UPDATE SET permanent = EXCLUDED.permanent"#,
        )
        .bind(new_low)
        .execute(&databases.character)
        .await
        .context("Failed to copy new leader permanent binds to group")?;

        // Cache detail: group binds live in player_bindings keyed by the leader GUID.
        // Re-key (merge, so a cached personal entry for the new leader is preserved).
        {
            let mut player_bindings = self.player_bindings.write();
            if let Some(bindings) = player_bindings.remove(&old_leader) {
                let entry = player_bindings.entry(new_leader).or_insert_with(HashMap::new);
                for (map_id, binding) in bindings {
                    entry.insert(map_id, binding);
                }
            }
        }

        Ok(())
    }

    /// Reset instance
    pub async fn reset_instance(
        &self,
        databases: &Databases,
        map_id: u32,
        instance_id: u32,
        _method: InstanceResetMethod,
    ) -> Result<()> {
        // Delete instance from database
        sqlx::query::<sqlx::Postgres>(
            r#"DELETE FROM characters.instance WHERE id = $1 AND map = $2"#,
        )
        .bind(i64::from(instance_id))
        .bind(i64::from(map_id))
        .execute(&databases.character)
        .await
        .context("Failed to delete instance")?;

        // Remove from cache
        {
            let mut instances = self.instances.write();
            instances.remove(&(map_id, instance_id));
        }

        // Unbind all players and groups
        self.unbind_instance(databases, map_id, instance_id).await?;

        Ok(())
    }

    /// Check if instance has expired
    pub fn is_instance_expired(&self, map_id: u32, instance_id: u32) -> bool {
        let instances = self.instances.read();
        if let Some(instance) = instances.get(&(map_id, instance_id)) {
            let instance = instance.read();
            if instance.permanent {
                return false;
            }
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            now >= instance.reset_time
        } else {
            true // Instance doesn't exist, consider it expired
        }
    }

    /// Get instance reset time remaining (in seconds)
    pub fn get_reset_time_remaining(&self, map_id: u32, instance_id: u32) -> Option<u64> {
        let instances = self.instances.read();
        if let Some(instance) = instances.get(&(map_id, instance_id)) {
            let instance = instance.read();
            if instance.permanent {
                return None; // Never resets
            }
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if now >= instance.reset_time {
                Some(0) // Already expired
            } else {
                Some(instance.reset_time - now)
            }
        } else {
            None
        }
    }

    /// Load player bindings from database
    pub async fn load_player_bindings(
        &self,
        databases: &Databases,
        player_guid: ObjectGuid,
    ) -> Result<HashMap<u32, InstanceBinding>> {
        let rows = sqlx::query::<sqlx::Postgres>(
            r#"SELECT instance, permanent FROM characters.character_instance WHERE guid = $1"#,
        )
        .bind(i64::from(player_guid.low()))
        .fetch_all(&databases.character)
        .await
        .context("Failed to query player instance bindings")?;

        let mut bindings = HashMap::new();
        for row in rows {
            let instance_id = row.get::<i64, _>(0) as u32;
            let permanent = row.get::<i16, _>(1);

            // Load instance to get map_id and reset_time
            // TODO: This is inefficient, should join with instance table
            // For now, we'll need to query instance table separately
            let instance_row = sqlx::query::<sqlx::Postgres>(
                r#"SELECT map, reset_time FROM characters.instance WHERE id = $1"#,
            )
            .bind(i64::from(instance_id))
            .fetch_optional(&databases.character)
            .await
            .context("Failed to query instance")?;

            if let Some(inst_row) = instance_row {
                let map_id = inst_row.get::<i64, _>(0) as u32;
                let reset_time: i64 = inst_row.get(1);

                bindings.insert(
                    map_id,
                    InstanceBinding {
                        map_id,
                        instance_id,
                        permanent: permanent != 0,
                        reset_time: reset_time as u64,
                    },
                );
            }
        }

        // Update cache
        {
            let mut player_bindings = self.player_bindings.write();
            player_bindings.insert(player_guid, bindings.clone());
        }

        Ok(bindings)
    }

    /// Load group bindings from database (via leader GUID)
    pub async fn load_group_bindings(
        &self,
        databases: &Databases,
        leader_guid: ObjectGuid,
    ) -> Result<HashMap<u32, InstanceBinding>> {
        // Group bindings use leader_guid in vanilla schema
        let rows = sqlx::query::<sqlx::Postgres>(
            r#"SELECT instance, permanent FROM characters.group_instance WHERE leader_guid = $1"#,
        )
        .bind(i64::from(leader_guid.low()))
        .fetch_all(&databases.character)
        .await
        .context("Failed to query group instance bindings")?;

        let mut bindings = HashMap::new();
        for row in rows {
            let instance_id = row.get::<i64, _>(0) as u32;
            let permanent = row.get::<i16, _>(1);

            // Load instance to get map_id and reset_time
            let instance_row = sqlx::query::<sqlx::Postgres>(
                r#"SELECT map, reset_time FROM characters.instance WHERE id = $1"#,
            )
            .bind(i64::from(instance_id))
            .fetch_optional(&databases.character)
            .await
            .context("Failed to query instance")?;

            if let Some(inst_row) = instance_row {
                let map_id = inst_row.get::<i64, _>(0) as u32;
                let reset_time: i64 = inst_row.get(1);

                bindings.insert(
                    map_id,
                    InstanceBinding {
                        map_id,
                        instance_id,
                        permanent: permanent != 0,
                        reset_time: reset_time as u64,
                    },
                );
            }
        }

        // Update cache (store by leader GUID)
        {
            let mut player_bindings = self.player_bindings.write();
            player_bindings.insert(leader_guid, bindings.clone());
        }

        Ok(bindings)
    }

    /// Get or create instance for a player/group
    /// Returns the instance ID that should be used
    pub async fn get_or_create_instance(
        &self,
        databases: &Databases,
        map_id: u32,
        player_guid: Option<ObjectGuid>,
        group_leader_guid: Option<ObjectGuid>,
        is_raid: bool,
    ) -> Result<u32> {
        // Check player binding first
        if let Some(guid) = player_guid {
            if let Some(binding) = self.get_player_binding(guid, map_id) {
                // Check if instance still exists and is valid
                if let Some(instance) = self
                    .load_instance(databases, map_id, binding.instance_id)
                    .await?
                {
                    let _instance_guard = instance.read();
                    if !self.is_instance_expired(map_id, binding.instance_id) {
                        return Ok(binding.instance_id);
                    }
                }
            }
        }

        // Check group binding
        if let Some(leader_guid) = group_leader_guid {
            if let Some(binding) = self.get_group_binding(leader_guid, map_id) {
                // Check if instance still exists and is valid
                if let Some(instance) = self
                    .load_instance(databases, map_id, binding.instance_id)
                    .await?
                {
                    let _instance_guard = instance.read();
                    if !self.is_instance_expired(map_id, binding.instance_id) {
                        return Ok(binding.instance_id);
                    }
                }
            }
        }

        // Create new instance
        let permanent = is_raid; // Raids are permanent, dungeons are temporary
        self.create_instance(databases, map_id, 0, permanent).await
    }

    /// Check if player can enter instance
    pub fn can_enter_instance(
        &self,
        player_guid: ObjectGuid,
        map_id: u32,
        instance_id: u32,
        group_leader_guid: Option<ObjectGuid>,
    ) -> bool {
        // Check player binding
        if let Some(binding) = self.get_player_binding(player_guid, map_id) {
            if binding.instance_id == instance_id {
                return true;
            }
            // Player is bound to different instance
            return false;
        }

        // Check group binding
        if let Some(leader_guid) = group_leader_guid {
            if let Some(binding) = self.get_group_binding(leader_guid, map_id) {
                if binding.instance_id == instance_id {
                    return true;
                }
            }
        }

        // No binding, can enter new instance
        true
    }

    /// Update instance system (check for resets, warnings, etc.)
    /// Called from world update loop
    pub async fn update(
        &self,
        databases: &Databases,
        _diff: u32,
        get_players_in_instance: impl Fn(u32, u32) -> Vec<ObjectGuid> + Send + Sync,
    ) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check for expired instances and send warnings
        {
            let instances = self.instances.read().clone();
            for ((map_id, instance_id), instance) in instances.iter() {
                let map_id_copy = *map_id;
                let instance_id_copy = *instance_id;
                let should_reset = {
                    let instance_guard = instance.read();

                    // Skip permanent instances (raids reset on schedule, not by expiration)
                    if instance_guard.permanent {
                        continue;
                    }

                    // Check if instance is expired
                    now >= instance_guard.reset_time
                };

                if should_reset {
                    // Instance expired - check if empty
                    let players = get_players_in_instance(map_id_copy, instance_id_copy);

                    if players.is_empty() {
                        // Instance is empty and expired - reset it
                        if let Err(e) = self
                            .reset_instance(
                                databases,
                                map_id_copy,
                                instance_id_copy,
                                InstanceResetMethod::Expire,
                            )
                            .await
                        {
                            tracing::warn!(
                                "Failed to reset expired instance {}:{}: {}",
                                map_id_copy,
                                instance_id_copy,
                                e
                            );
                        }
                    } else {
                        // Instance expired but has players - send warning
                        let warning_type = InstanceResetWarningType::Expired;
                        let mut warnings = self.reset_warnings.write();
                        let last_warning = warnings.get(&(map_id_copy, instance_id_copy));

                        if last_warning != Some(&warning_type) {
                            // Send expired warning to players
                            // Note: In vanilla, warnings are sent via system messages
                            // We'll track this for now - actual packet sending happens in world update
                            warnings.insert((map_id_copy, instance_id_copy), warning_type);
                        }
                    }
                } else {
                    // Check for reset warnings (15 min, 10 min, 5 min, 1 min before reset)
                    let instance_guard = instance.read();
                    let time_remaining = instance_guard.reset_time - now;
                    let warning_type = if time_remaining <= 60 {
                        InstanceResetWarningType::Hours15Min // 1 minute
                    } else if time_remaining <= 300 {
                        InstanceResetWarningType::Hours30Min // 5 minutes
                    } else if time_remaining <= 600 {
                        InstanceResetWarningType::Hours1 // 10 minutes
                    } else if time_remaining <= 3600 {
                        InstanceResetWarningType::Hours1 // 1 hour
                    } else {
                        continue; // No warning needed
                    };

                    let mut warnings = self.reset_warnings.write();
                    let last_warning = warnings.get(&(*map_id, *instance_id));

                    if last_warning != Some(&warning_type) {
                        // Send warning to players
                        // Note: In vanilla, warnings are sent via system messages
                        // We'll track this for now - actual packet sending happens in world update
                        warnings.insert((*map_id, *instance_id), warning_type);
                    }
                }
            }
        }

        // Check pending resets (empty instances that should reset after delay)
        {
            let mut to_reset = Vec::new();
            {
                let pending = self.pending_resets.read();
                for ((map_id, instance_id), reset_time) in pending.iter() {
                    if now >= *reset_time {
                        to_reset.push((*map_id, *instance_id));
                    }
                }
            }

            for (map_id, instance_id) in to_reset {
                {
                    let mut pending = self.pending_resets.write();
                    pending.remove(&(map_id, instance_id));
                }
                if let Err(e) = self
                    .reset_instance(databases, map_id, instance_id, InstanceResetMethod::Expire)
                    .await
                {
                    tracing::warn!(
                        "Failed to reset pending instance {}:{}: {}",
                        map_id,
                        instance_id,
                        e
                    );
                }
            }
        }

        Ok(())
    }

    /// Mark instance as empty and schedule reset (for normal dungeons)
    /// Normal dungeons reset after a delay when empty
    pub fn schedule_instance_reset(&self, map_id: u32, instance_id: u32, delay_seconds: u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let reset_time = now + delay_seconds;
        let mut pending = self.pending_resets.write();
        pending.insert((map_id, instance_id), reset_time);
    }

    /// Get instances that need reset warnings
    pub fn get_reset_warnings(&self) -> Vec<((u32, u32), InstanceResetWarningType)> {
        let warnings = self.reset_warnings.read();
        warnings.iter().map(|(k, v)| (*k, *v)).collect()
    }

    /// Clear reset warning for instance
    pub fn clear_reset_warning(&self, map_id: u32, instance_id: u32) {
        let mut warnings = self.reset_warnings.write();
        warnings.remove(&(map_id, instance_id));
    }

    /// Enter instance - validates and binds player/group to instance
    /// Returns the instance ID to use
    pub async fn enter_instance(
        &self,
        databases: &Databases,
        map_id: u32,
        player_guid: ObjectGuid,
        group_leader_guid: Option<ObjectGuid>,
        is_raid: bool,
    ) -> Result<u32> {
        // Get or create instance
        let instance_id = self
            .get_or_create_instance(
                databases,
                map_id,
                Some(player_guid),
                group_leader_guid,
                is_raid,
            )
            .await?;

        // Load instance to get reset time
        let instance = self
            .load_instance(databases, map_id, instance_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to load instance"))?;

        let reset_time = {
            let instance_guard = instance.read();
            instance_guard.reset_time
        };

        // Bind player to instance
        let permanent = is_raid;
        self.bind_player_to_instance(
            databases,
            player_guid,
            map_id,
            instance_id,
            permanent,
            reset_time,
        )
        .await?;

        // Bind group to instance if in group
        if let Some(leader_guid) = group_leader_guid {
            self.bind_group_to_instance(
                databases,
                leader_guid,
                map_id,
                instance_id,
                permanent,
                reset_time,
            )
            .await?;
        }

        Ok(instance_id)
    }

    /// Check if instance is empty (no players)
    pub fn is_instance_empty(
        &self,
        _map_id: u32,
        _instance_id: u32,
        get_players: impl Fn() -> Vec<ObjectGuid>,
    ) -> bool {
        let players = get_players();
        players.is_empty()
    }

    /// Unbind all players and groups from instance (called on reset)
    pub async fn unbind_instance(
        &self,
        databases: &Databases,
        map_id: u32,
        instance_id: u32,
    ) -> Result<()> {
        // Remove all player bindings for this instance
        sqlx::query::<sqlx::Postgres>(
            r#"DELETE FROM characters.character_instance WHERE instance = $1"#,
        )
        .bind(i64::from(instance_id))
        .execute(&databases.character)
        .await
        .context("Failed to unbind players from instance")?;

        // Remove all group bindings for this instance
        sqlx::query::<sqlx::Postgres>(
            r#"DELETE FROM characters.group_instance WHERE instance = $1"#,
        )
        .bind(i64::from(instance_id))
        .execute(&databases.character)
        .await
        .context("Failed to unbind groups from instance")?;

        // Clear from cache
        {
            let mut player_bindings = self.player_bindings.write();
            for bindings in player_bindings.values_mut() {
                bindings.remove(&map_id);
            }
        }

        {
            let mut group_bindings = self.group_bindings.write();
            for bindings in group_bindings.values_mut() {
                bindings.remove(&map_id);
            }
        }

        Ok(())
    }

    /// Reset all non-permanent instances for a player (manual reset via UI)
    /// Returns (success_map_ids, failed_map_ids_with_reason)
    /// An instance can only be reset if:
    /// 1. It's not permanent (not a raid lockout)
    /// 2. No players are currently inside the instance
    pub async fn reset_player_instances(
        &self,
        databases: &Databases,
        player_guid: ObjectGuid,
        get_players_in_instance: impl Fn(u32, u32) -> Vec<ObjectGuid>,
    ) -> (Vec<u32>, Vec<(u32, super::InstanceResetFailReason)>) {
        use super::InstanceResetFailReason;

        let mut success_map_ids = Vec::new();
        let mut failed_map_ids = Vec::new();

        // Get all player bindings
        let bindings = {
            let player_bindings = self.player_bindings.read();
            player_bindings
                .get(&player_guid)
                .cloned()
                .unwrap_or_default()
        };

        for (map_id, binding) in bindings {
            // Skip permanent instances (raids)
            if binding.permanent {
                continue;
            }

            // Check if anyone is in the instance
            let players = get_players_in_instance(map_id, binding.instance_id);
            if !players.is_empty() {
                // Check if it's the player themselves zoning
                if players.len() == 1 && players[0] == player_guid {
                    failed_map_ids.push((map_id, InstanceResetFailReason::Zoning));
                } else {
                    failed_map_ids.push((map_id, InstanceResetFailReason::Offline));
                }
                continue;
            }

            // Reset the instance
            if let Err(e) = self
                .reset_instance(
                    databases,
                    map_id,
                    binding.instance_id,
                    InstanceResetMethod::Manual,
                )
                .await
            {
                tracing::warn!(
                    "Failed to reset instance {} for player {}: {}",
                    binding.instance_id,
                    player_guid,
                    e
                );
                failed_map_ids.push((map_id, InstanceResetFailReason::General));
                continue;
            }

            success_map_ids.push(map_id);
        }

        (success_map_ids, failed_map_ids)
    }

    /// Reset all non-permanent instances for a group (called by group leader)
    /// Same as reset_player_instances but checks group bindings
    pub async fn reset_group_instances(
        &self,
        databases: &Databases,
        leader_guid: ObjectGuid,
        get_players_in_instance: impl Fn(u32, u32) -> Vec<ObjectGuid>,
    ) -> (Vec<u32>, Vec<(u32, super::InstanceResetFailReason)>) {
        // In vanilla schema, group bindings are stored by leader GUID
        // So we can reuse the player instance reset logic
        self.reset_player_instances(databases, leader_guid, get_players_in_instance)
            .await
    }

    /// Check if a player can reset instances (not in an instance themselves)
    pub fn can_player_reset_instances(&self, player_guid: ObjectGuid, current_map_id: u32) -> bool {
        // Check if the player is currently in any of their bound instances
        let bindings = {
            let player_bindings = self.player_bindings.read();
            player_bindings
                .get(&player_guid)
                .cloned()
                .unwrap_or_default()
        };

        // If the player's current map matches any of their bound instance maps,
        // they can't reset (they're inside an instance)
        for (map_id, _binding) in bindings {
            if map_id == current_map_id {
                return false;
            }
        }

        true
    }
}

impl Default for InstanceMgr {
    fn default() -> Self {
        Self::new()
    }
}
