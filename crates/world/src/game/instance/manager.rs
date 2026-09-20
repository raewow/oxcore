// Instance manager - handles all instance operations

use anyhow::{Context, Result};
use oxcore_db::database::Databases;
use oxcore_shared::game::instance::{
    InstanceBinding, InstanceResetMethod, InstanceResetWarningType, InstanceState,
};
use oxcore_shared::protocol::ObjectGuid;
use parking_lot::RwLock;
use sqlx::{PgConnection, Row};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const PLAYER_BINDINGS_QUERY: &str = "SELECT i.map, b.instance, b.permanent, i.reset_time
     FROM characters.character_instance b JOIN characters.instance i ON i.id = b.instance
     WHERE b.guid = $1";
const GROUP_BINDINGS_QUERY: &str = "SELECT i.map, b.instance, b.permanent, i.reset_time
     FROM characters.group_instance b JOIN characters.instance i ON i.id = b.instance
     WHERE b.leader_guid = $1";

/// Instance manager - handles all instance operations
pub struct InstanceMgr {
    /// Active instances by (map_id, instance_id)
    instances: Arc<RwLock<HashMap<(u32, u32), Arc<RwLock<InstanceState>>>>>,
    /// Player bindings (player_guid -> map_id -> binding)
    player_bindings: Arc<RwLock<HashMap<ObjectGuid, HashMap<u32, InstanceBinding>>>>,
    /// Group bindings (leader_guid -> map_id -> binding), separate from personal saves.
    group_bindings: Arc<RwLock<HashMap<ObjectGuid, HashMap<u32, InstanceBinding>>>>,
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
        map_id: u32,
        instance_id: u32,
        permanent: bool,
        reset_time: u64,
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

        self.group_bindings
            .write()
            .entry(leader_guid)
            .or_default()
            .insert(
                map_id,
                InstanceBinding {
                    map_id,
                    instance_id,
                    permanent,
                    reset_time,
                },
            );

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
        self.group_bindings
            .read()
            .get(&leader_guid)?
            .get(&map_id)
            .cloned()
    }

    /// Remove a group save without touching any member's personal binding.
    pub async fn unbind_group_from_instance(
        &self,
        databases: &Databases,
        leader_guid: ObjectGuid,
        map_id: u32,
        instance_id: u32,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM characters.group_instance WHERE leader_guid = $1 AND instance = $2",
        )
        .bind(i64::from(leader_guid.low()))
        .bind(i64::from(instance_id))
        .execute(&databases.character)
        .await
        .context("Failed to unbind group from instance")?;
        if let Some(bindings) = self.group_bindings.write().get_mut(&leader_guid) {
            if bindings
                .get(&map_id)
                .is_some_and(|binding| binding.instance_id == instance_id)
            {
                bindings.remove(&map_id);
            }
        }
        Ok(())
    }

    /// Forget a disbanded group's cache after its database rows have been removed.
    pub fn unload_group_bindings(&self, leader_guid: ObjectGuid) {
        self.group_bindings.write().remove(&leader_guid);
    }

    /// Transfer a group's instance bindings from the old leader to the new one,
    /// mirroring `Group::_setLeader` (reference `Group.cpp:1710-1786`).
    ///
    /// Publish the new group cache only after the database transfer commits. Personal
    /// saves remain with their owners, except for the reference's explicit handoff
    /// of a replaced group save and conversion of the new leader's temporary saves.
    pub async fn transfer_group_bindings(
        &self,
        databases: &Databases,
        old_leader: ObjectGuid,
        new_leader: ObjectGuid,
    ) -> Result<()> {
        if old_leader == new_leader {
            return Ok(());
        }
        let old_low = i64::from(old_leader.low());
        let new_low = i64::from(new_leader.low());
        let mut transaction = databases.character.begin().await?;
        let group_id: i64 = sqlx::query_scalar(
            "SELECT group_id FROM characters.groups WHERE leader_guid = $1 FOR UPDATE",
        )
        .bind(old_low)
        .fetch_one(&mut *transaction)
        .await
        .context("Failed to find group for instance binding transfer")?;
        let old_group =
            Self::read_bindings(&mut transaction, GROUP_BINDINGS_QUERY, old_leader).await?;
        let mut old_personal =
            Self::read_bindings(&mut transaction, PLAYER_BINDINGS_QUERY, old_leader).await?;
        let new_personal =
            Self::read_bindings(&mut transaction, PLAYER_BINDINGS_QUERY, new_leader).await?;
        let mut new_group = HashMap::new();

        for (map_id, binding) in old_group {
            // The new leader supplies permanent group saves. Uncontested
            // temporary group saves follow the group to its new leader.
            if binding.permanent {
                continue;
            }
            if !new_personal.contains_key(&map_id) {
                new_group.insert(map_id, binding);
            } else if !old_personal.contains_key(&map_id) {
                // A personal save for this map replaces the group copy, even
                // when it refers to a different instance of the same map.
                sqlx::query(
                    "INSERT INTO characters.character_instance (guid, instance, permanent, extend) VALUES ($1, $2, 0, 0)",
                )
                .bind(old_low)
                .bind(i64::from(binding.instance_id))
                .execute(&mut *transaction)
                .await
                .context("Failed to hand group save to old leader")?;
                old_personal.insert(map_id, binding);
            }
        }
        new_group.extend(new_personal.clone());

        // group_instance references groups.leader_guid without ON UPDATE CASCADE.
        // Remove the old rows, change the leader, then restore the group's saves
        // in the same transaction so the foreign key is valid throughout.
        sqlx::query("DELETE FROM characters.group_instance WHERE leader_guid = $1")
            .bind(old_low)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("UPDATE characters.groups SET leader_guid = $1 WHERE group_id = $2")
            .bind(new_low)
            .bind(group_id)
            .execute(&mut *transaction)
            .await
            .context("Failed to change instance binding owner")?;
        for binding in new_group.values() {
            sqlx::query(
                "INSERT INTO characters.group_instance (leader_guid, instance, permanent) VALUES ($1, $2, $3)",
            )
            .bind(new_low)
            .bind(i64::from(binding.instance_id))
            .bind(i16::from(binding.permanent))
            .execute(&mut *transaction)
            .await?;
        }
        // ConvertInstancesToGroup preserves permanent personal locks, while
        // temporary personal saves become group-owned.
        sqlx::query("DELETE FROM characters.character_instance WHERE guid = $1 AND permanent = 0")
            .bind(new_low)
            .execute(&mut *transaction)
            .await?;
        transaction
            .commit()
            .await
            .context("Failed to commit group binding transfer")?;

        let mut groups = self.group_bindings.write();
        let mut players = self.player_bindings.write();
        groups.remove(&old_leader);
        groups.insert(new_leader, new_group);
        players.insert(old_leader, old_personal);
        players.insert(
            new_leader,
            new_personal
                .into_iter()
                .filter(|(_, binding)| binding.permanent)
                .collect(),
        );

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
        let mut connection = databases.character.acquire().await?;
        let bindings =
            Self::read_bindings(&mut connection, PLAYER_BINDINGS_QUERY, player_guid).await?;

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
        let mut connection = databases.character.acquire().await?;
        let bindings =
            Self::read_bindings(&mut connection, GROUP_BINDINGS_QUERY, leader_guid).await?;

        self.group_bindings
            .write()
            .insert(leader_guid, bindings.clone());

        Ok(bindings)
    }

    async fn read_bindings(
        connection: &mut PgConnection,
        query: &str,
        owner: ObjectGuid,
    ) -> Result<HashMap<u32, InstanceBinding>> {
        let rows = sqlx::query_as::<_, (i64, i64, i16, i64)>(query)
            .bind(i64::from(owner.low()))
            .fetch_all(connection)
            .await
            .context("Failed to query instance bindings")?;
        rows.into_iter()
            .map(|(map_id, instance_id, permanent, reset_time)| {
                let map_id = u32::try_from(map_id)?;
                Ok((
                    map_id,
                    InstanceBinding {
                        map_id,
                        instance_id: u32::try_from(instance_id)?,
                        permanent: permanent != 0,
                        reset_time: reset_time as u64,
                    },
                ))
            })
            .collect()
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
        // Get all player bindings
        let bindings = {
            let player_bindings = self.player_bindings.read();
            player_bindings
                .get(&player_guid)
                .cloned()
                .unwrap_or_default()
        };

        self.reset_bound_instances(databases, player_guid, bindings, get_players_in_instance)
            .await
    }

    async fn reset_bound_instances(
        &self,
        databases: &Databases,
        requester_guid: ObjectGuid,
        bindings: HashMap<u32, InstanceBinding>,
        get_players_in_instance: impl Fn(u32, u32) -> Vec<ObjectGuid>,
    ) -> (Vec<u32>, Vec<(u32, super::InstanceResetFailReason)>) {
        use super::InstanceResetFailReason;

        let mut success_map_ids = Vec::new();
        let mut failed_map_ids = Vec::new();

        for (map_id, binding) in bindings {
            // Skip permanent instances (raids)
            if binding.permanent {
                continue;
            }

            // Check if anyone is in the instance
            let players = get_players_in_instance(map_id, binding.instance_id);
            if !players.is_empty() {
                // Check if it's the player themselves zoning
                if players.len() == 1 && players[0] == requester_guid {
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
                    "Failed to reset instance {} for requester {}: {}",
                    binding.instance_id,
                    requester_guid,
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
        let bindings = self
            .group_bindings
            .read()
            .get(&leader_guid)
            .cloned()
            .unwrap_or_default();
        self.reset_bound_instances(databases, leader_guid, bindings, get_players_in_instance)
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

#[cfg(test)]
mod tests;
