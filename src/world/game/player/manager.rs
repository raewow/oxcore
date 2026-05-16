//! Player Manager - owns all online players

use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;

use super::broadcaster::PlayerBroadcaster;
use super::player::Player;
use crate::shared::database::CharacterRepository;
use crate::shared::messages::update::{
    CreateObjectBlock, ObjectType, SmsgUpdateObject, UpdateBlockData,
};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Position};
use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
use crate::world::core::common::position::Position as WorldPosition;
use crate::world::core::common::ObjectGuidGenerator;
use crate::world::game::common::object_type::update_flags;
use crate::world::game::common::object_type::MovementSpeeds;
use crate::world::game::common::object_type::ObjectTypeId;
use crate::world::game::common::player_constants::{get_faction_for_race, get_player_display_id};
use crate::world::game::common::unit_flags as unit_flags_mod;
use crate::world::game::common::update_fields::*;
use crate::world::World;

/// Manages all online players
pub struct PlayerManager {
    /// All online players by GUID
    players: DashMap<ObjectGuid, Player>,
    /// Players by account ID (for session binding)
    by_account: DashMap<u32, ObjectGuid>,
    /// GUID generator for new players
    guid_generator: RwLock<ObjectGuidGenerator>,
}

impl PlayerManager {
    pub fn new() -> Self {
        Self {
            players: DashMap::new(),
            by_account: DashMap::new(),
            guid_generator: RwLock::new(ObjectGuidGenerator::new(1)),
        }
    }

    /// Initialize GUID generator from database MAX query
    pub async fn init_guid_generator(&self, character_db: &sqlx::MySqlPool) -> anyhow::Result<()> {
        let max_guid: Option<u32> = sqlx::query_scalar("SELECT MAX(guid) FROM characters")
            .fetch_optional(character_db)
            .await?;

        let next_guid = max_guid.map(|g| g + 1).unwrap_or(1);
        *self.guid_generator.write() = ObjectGuidGenerator::new(next_guid);

        tracing::debug!(
            "Initialized Player GUID generator - starting at {}",
            next_guid
        );
        Ok(())
    }

    /// Add a player to the manager
    pub fn add_player(&self, player: Player, account_id: u32) {
        let guid = player.guid;
        self.players.insert(guid, player);
        self.by_account.insert(account_id, guid);
    }

    /// Get a player by GUID
    pub fn get_player(
        &self,
        guid: ObjectGuid,
    ) -> Option<dashmap::mapref::one::Ref<'_, ObjectGuid, Player>> {
        self.players.get(&guid)
    }

    /// Get a mutable reference to a player
    pub fn get_player_mut(
        &self,
        guid: ObjectGuid,
    ) -> Option<dashmap::mapref::one::RefMut<'_, ObjectGuid, Player>> {
        self.players.get_mut(&guid)
    }

    /// Execute a closure with mutable access to a player, returning the closure's result
    ///
    /// This is a performance optimization that allows batching multiple player operations
    /// into a single DashMap lookup, reducing overhead from ~600ns to ~200ns per operation.
    ///
    /// # Performance Impact
    ///
    /// **Before (multiple lookups):**
    /// ```rust,ignore
    /// // 3 separate DashMap accesses = 3× hash + 3× lock acquisition
    /// let pos = player_mgr.get_position(guid);          // 200ns
    /// player_mgr.update_movement(guid, |state| {...});  // 200ns
    /// let map_id = player_mgr.get_player(guid).map_id;  // 200ns
    /// // Total: ~600ns overhead
    /// ```
    ///
    /// **After (batched access):**
    /// ```rust,ignore
    /// // 1 DashMap access = 1× hash + 1× lock acquisition
    /// let (pos, map_id) = player_mgr.with_player_mut(guid, |player| {
    ///     let pos = player.movement.position;
    ///     player.movement.position = new_pos;
    ///     (pos, player.map_id)
    /// });
    /// // Total: ~200ns overhead (67% reduction)
    /// ```
    ///
    /// # Concurrency Safety
    ///
    /// The closure holds a mutable guard on the player's DashMap shard. This is safe because:
    /// - Different players hash to different shards (no contention)
    /// - The guard is released immediately after the closure completes
    /// - No async boundaries are crossed while holding the guard
    ///
    /// **Do NOT call async code inside the closure** - it will block other operations on the same shard.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let (old_pos, map_id) = player_mgr.with_player_mut(player_guid, |player| {
    ///     let old_pos = player.movement.position;
    ///
    ///     // Validate and update movement state
    ///     player.movement.position = new_pos;
    ///     player.movement.movement_flags = flags;
    ///     player.movement.timestamp = time;
    ///
    ///     (old_pos, player.map_id)
    /// })?;
    /// ```
    pub fn with_player_mut<F, R>(&self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&mut Player) -> R,
    {
        self.get_player_mut(guid).map(|mut p| f(&mut *p))
    }

    /// Execute a closure with immutable access to a player
    ///
    /// Returns `None` if player not found.
    ///
    /// # Example
    /// ```rust,ignore
    /// let level = player_mgr.with_player(guid, |player| {
    ///     player.level
    /// })?;
    /// ```
    pub fn with_player<F, R>(&self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&Player) -> R,
    {
        self.get_player(guid).map(|p| f(&*p))
    }

    /// Check if a player has had CREATE_OBJECT2 sent for a specific object.
    /// Used by creature broadcast functions to skip players who haven't
    /// been introduced to the object yet (prevents client crashes).
    pub fn has_object_created(&self, player_guid: ObjectGuid, object_guid: ObjectGuid) -> bool {
        self.with_player(player_guid, |player| {
            player.visibility.objects_created.contains(&object_guid)
        })
        .unwrap_or(false)
    }

    /// Get broadcaster for a player
    pub fn get_broadcaster(&self, guid: ObjectGuid) -> Option<Arc<PlayerBroadcaster>> {
        self.get_player(guid).and_then(|p| p.broadcaster())
    }

    /// Get a player's name by GUID
    pub fn get_player_name(&self, guid: ObjectGuid) -> Option<String> {
        self.get_player(guid).map(|p| p.name.clone())
    }

    /// Find a player by name (case-insensitive)
    pub fn find_player_by_name(&self, name: &str) -> Option<ObjectGuid> {
        let lower_name = name.to_lowercase();
        self.players
            .iter()
            .find(|entry| entry.value().name.to_lowercase() == lower_name)
            .map(|entry| *entry.key())
    }

    /// Set player's current selection/target
    pub fn set_selection(&self, player_guid: ObjectGuid, target: ObjectGuid) {
        if let Some(mut player) = self.get_player_mut(player_guid) {
            player.selection = Some(target);
        }
    }

    /// Clear player's current selection
    pub fn clear_selection(&self, player_guid: ObjectGuid) {
        if let Some(mut player) = self.get_player_mut(player_guid) {
            player.selection = None;
        }
    }

    /// Get player's current selection
    pub fn get_selection(&self, player_guid: ObjectGuid) -> Option<ObjectGuid> {
        self.get_player(player_guid).and_then(|p| p.selection)
    }

    /// Get an iterator over all online players
    pub fn iter(&self) -> dashmap::iter::Iter<'_, ObjectGuid, Player> {
        self.players.iter()
    }

    /// Execute a closure for each player (mutable access)
    ///
    /// This is used by systems that need to update all players (like power regeneration)
    pub fn for_each_player<F>(&self, mut f: F)
    where
        F: FnMut(ObjectGuid, &mut Player),
    {
        for mut entry in self.players.iter_mut() {
            let guid = *entry.key();
            f(guid, entry.value_mut());
        }
    }

    /// Collect all online player GUIDs (snapshot for iteration outside lock)
    pub fn collect_online_guids(&self) -> Vec<ObjectGuid> {
        self.players.iter().map(|entry| *entry.key()).collect()
    }

    /// Logout a player - saves to database and removes from manager
    ///
    /// This method:
    /// 1. Saves player position to database
    /// 2. Sets online status to false
    /// 3. Removes player from by_account index
    /// 4. Removes player from players map
    ///
    /// Returns the removed Player object
    pub async fn logout_player(
        &self,
        guid: ObjectGuid,
        account_id: u32,
        character_db: &sqlx::MySqlPool,
        world: &World,
    ) -> anyhow::Result<Option<Player>> {
        // Get player data before removal
        let player_opt = self.get_player(guid);
        if player_opt.is_none() {
            return Ok(None);
        }

        let player = player_opt.unwrap();
        drop(player); // Release DashMap guard before async DB call

        // Set online status to false
        let char_repo = CharacterRepository::new(Arc::new(character_db.clone()));
        char_repo.update_online(guid.counter(), false).await?;

        // Remove from indices
        self.by_account.remove(&account_id);

        // Remove from players map (returns (key, value) tuple)
        let removed = self.remove_player(guid);

        Ok(removed.map(|(_, player)| player))
    }

    /// Remove a player
    pub fn remove_player(&self, guid: ObjectGuid) -> Option<(ObjectGuid, Player)> {
        self.players.remove(&guid)
    }

    /// Get player by account ID
    pub fn get_player_by_account(&self, account_id: u32) -> Option<ObjectGuid> {
        self.by_account.get(&account_id).map(|r| *r)
    }

    /// Number of online players
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// Get online player count (async wrapper for console compatibility)
    pub async fn get_online_count(&self) -> usize {
        self.player_count()
    }

    /// Generate a new player GUID
    pub fn generate_player_guid(&self) -> ObjectGuid {
        let counter = self.guid_generator.write().generate();
        ObjectGuid::new_player(counter)
    }

    /// Build CREATE_OBJECT2 message for a player
    ///
    /// This includes all available player data for visibility updates.
    /// Always uses CREATE_OBJECT2 for initial visibility spawn.
    pub fn build_create_msg(&self, guid: ObjectGuid, world: &World) -> Option<SmsgUpdateObject> {
        let player = self.get_player(guid)?;

        // Collect visible item fields from inventory system (for character model rendering)
        let mut visible_item_fields: Vec<(u32, u32)> = Vec::new();
        world
            .systems
            .inventory
            .populate_visible_items(guid, &mut visible_item_fields);

        // Collect inventory slot GUID fields (need to use set_guid_field)
        let mut inventory_guid_fields: Vec<(u32, WorldObjectGuid)> = Vec::new();
        world
            .systems
            .inventory
            .populate_inventory_slots(guid, &mut inventory_guid_fields);

        let faction_template = get_faction_for_race(player.race);
        let display_id = get_player_display_id(player.race, player.gender);

        let power_type: u8 = match player.class {
            1 => 1, // Warrior = rage
            4 => 3, // Rogue = energy
            _ => 0, // All others = mana
        };

        let update_type = ObjectTypeId::Player;
        let object_type = ObjectType::Player;

        let world_guid = WorldObjectGuid::from_raw(guid.raw());

        // Get position from PlayerManager (sole authority)
        let movement_pos = self.get_position(guid).unwrap_or_else(Position::default);

        let world_position = WorldPosition::new(
            movement_pos.x,
            movement_pos.y,
            movement_pos.z,
            movement_pos.o,
        );

        // Get player's actual current movement flags from PlayerManager
        // This ensures players appear in their correct state (moving, jumping, etc.)
        // when they first become visible to other players
        let movement_flags = self
            .get_movement_state(guid)
            .map(|s| s.movement_flags)
            .unwrap_or(0);

        let mut block = CreateObjectBlock::new(world_guid, update_type, object_type)
            .add_flags(update_flags::UPDATEFLAG_LIVING);

        block = block.with_movement(
            world_position,
            movement_flags,
            Some(MovementSpeeds::default()),
        );

        // Sort visible item fields by index for proper update mask ordering
        visible_item_fields.sort_by_key(|&(idx, _)| idx);

        // Build block with visible item fields set
        block = block.set_fields(visible_item_fields);

        // Set inventory slot GUID fields using set_guid_field (ensures proper GUID handling)
        for (field_index, item_guid) in inventory_guid_fields {
            block = block.set_guid_field(field_index, item_guid);
        }

        // Set remaining fields
        let block = block
            // Object fields
            .set_guid_field(OBJECT_FIELD_GUID, world_guid)
            .set_field(OBJECT_FIELD_TYPE, 0x19) // TYPEMASK_OBJECT | TYPEMASK_UNIT | TYPEMASK_PLAYER
            .set_float_field(OBJECT_FIELD_SCALE_X, 1.0)
            // Unit fields
            .set_field(UNIT_FIELD_HEALTH, player.stats.health)
            .set_field(UNIT_FIELD_MAXHEALTH, player.stats.max_health)
            .set_field(UNIT_FIELD_LEVEL, player.level as u32)
            .set_field(UNIT_FIELD_STAT0, player.stats.strength)
            .set_field(UNIT_FIELD_STAT1, player.stats.agility)
            .set_field(UNIT_FIELD_STAT2, player.stats.stamina)
            .set_field(UNIT_FIELD_STAT3, player.stats.intellect)
            .set_field(UNIT_FIELD_STAT4, player.stats.spirit)
            .set_field(UNIT_FIELD_POWER1, player.stats.mana)
            .set_field(UNIT_FIELD_MAXPOWER1, player.stats.max_mana)
            .set_field(
                UNIT_FIELD_ATTACK_POWER,
                player.stats.melee_attack_power as u32,
            )
            .set_field(
                UNIT_FIELD_RANGED_ATTACK_POWER,
                player.stats.ranged_attack_power as u32,
            )
            // Resistances
            .set_field(UNIT_FIELD_RESISTANCES, player.stats.armor)
            .set_field(UNIT_FIELD_RESISTANCES + 1, player.stats.resistances[1])
            .set_field(UNIT_FIELD_RESISTANCES + 2, player.stats.resistances[2])
            .set_field(UNIT_FIELD_RESISTANCES + 3, player.stats.resistances[3])
            .set_field(UNIT_FIELD_RESISTANCES + 4, player.stats.resistances[4])
            .set_field(UNIT_FIELD_RESISTANCES + 5, player.stats.resistances[5])
            .set_field(UNIT_FIELD_RESISTANCES + 6, player.stats.resistances[6])
            // Damage ranges
            .set_float_field(UNIT_FIELD_MINDAMAGE, player.stats.min_damage)
            .set_float_field(UNIT_FIELD_MAXDAMAGE, player.stats.max_damage)
            .set_float_field(UNIT_FIELD_MINOFFHANDDAMAGE, player.stats.min_offhand_damage)
            .set_float_field(UNIT_FIELD_MAXOFFHANDDAMAGE, player.stats.max_offhand_damage)
            .set_float_field(UNIT_FIELD_MINRANGEDDAMAGE, player.stats.min_ranged_damage)
            .set_float_field(UNIT_FIELD_MAXRANGEDDAMAGE, player.stats.max_ranged_damage)
            // Combat percentages
            .set_float_field(PLAYER_CRIT_PERCENTAGE, player.stats.melee_crit_pct)
            .set_float_field(PLAYER_RANGED_CRIT_PERCENTAGE, player.stats.ranged_crit_pct)
            .set_float_field(PLAYER_DODGE_PERCENTAGE, player.stats.dodge_pct)
            .set_float_field(PLAYER_PARRY_PERCENTAGE, player.stats.parry_pct)
            .set_float_field(PLAYER_BLOCK_PERCENTAGE, player.stats.block_pct)
            .set_field(UNIT_FIELD_FACTIONTEMPLATE, faction_template)
            .set_field(UNIT_FIELD_DISPLAYID, display_id)
            .set_field(UNIT_FIELD_NATIVEDISPLAYID, display_id)
            .set_bytes_field(
                UNIT_FIELD_BYTES_0,
                [player.race, player.class, player.gender, power_type],
            )
            .set_bytes_field(
                UNIT_FIELD_BYTES_1,
                [player.stand_state, 0, player.shapeshift_form, 0],
            )
            .set_field(UNIT_FIELD_FLAGS, 0x00000008_u32) // PLAYER_CONTROLLED
            // Player fields
            .set_field(PLAYER_FLAGS, 0)
            .set_bytes_field(
                PLAYER_BYTES,
                [
                    player.skin,
                    player.face,
                    player.hair_style,
                    player.hair_color,
                ],
            )
            .set_bytes_field(
                PLAYER_BYTES_2,
                [player.facial_hair, 0xEE, 0x00, 0x02], // Normal (not rested)
            )
            .set_field(PLAYER_FIELD_WATCHED_FACTION_INDEX, 0xFFFFFFFF);

        // Guild fields — set PLAYER_GUILDID and PLAYER_GUILDRANK if player is in a guild
        let block = if let Some(guild_state) = world.systems.guild.get_player_guild(guid) {
            if let Some(guild_id) = guild_state.guild_id {
                block
                    .set_field(PLAYER_GUILDID, guild_id)
                    .set_field(PLAYER_GUILDRANK, guild_state.rank_id as u32)
            } else {
                block
            }
        } else {
            block
        };

        Some(SmsgUpdateObject::new().add_block(UpdateBlockData::CreateObject2(block)))
    }

    // ========== Movement State Access ==========

    /// Get player position (read-only)
    pub fn get_position(&self, guid: ObjectGuid) -> Option<Position> {
        self.get_player(guid).map(|p| p.movement.position)
    }

    /// Get player position (alias for get_position, used by AI system)
    pub fn get_player_position(&self, guid: ObjectGuid) -> Option<Position> {
        self.get_position(guid)
    }

    /// Check if player is alive
    pub fn is_player_alive(&self, guid: ObjectGuid) -> bool {
        self.get_player(guid).map(|p| p.is_alive()).unwrap_or(false)
    }

    /// Get full movement state (read-only, cloned)
    pub fn get_movement_state(&self, guid: ObjectGuid) -> Option<super::movement::MovementState> {
        self.get_player(guid).map(|p| p.movement.clone())
    }

    /// Update player movement state
    pub fn update_movement<F>(&self, guid: ObjectGuid, f: F) -> Option<()>
    where
        F: FnOnce(&mut super::movement::MovementState),
    {
        self.get_player_mut(guid).map(|mut p| f(&mut p.movement))
    }

    /// Save player position to database
    pub async fn save_position(
        &self,
        player_guid: ObjectGuid,
        character_db: &sqlx::MySqlPool,
    ) -> anyhow::Result<()> {
        let (position, map_id, instance_id, zone_id) = self
            .get_player(player_guid)
            .map(|p| (p.movement.position, p.map_id, p.instance_id, p.zone_id))
            .ok_or_else(|| anyhow::anyhow!("Player not found"))?;

        let char_repo = CharacterRepository::new(Arc::new(character_db.clone()));
        char_repo
            .update_position(
                player_guid.counter(),
                map_id,
                instance_id,
                zone_id,
                position.x,
                position.y,
                position.z,
                position.o,
            )
            .await?;

        Ok(())
    }

    /// Save player experience and level to database
    pub async fn save_experience(
        &self,
        player_guid: ObjectGuid,
        character_db: &sqlx::MySqlPool,
    ) -> anyhow::Result<()> {
        let (xp, level) = self
            .get_player(player_guid)
            .map(|p| (p.xp, p.level))
            .ok_or_else(|| anyhow::anyhow!("Player not found"))?;

        let char_repo = CharacterRepository::new(Arc::new(character_db.clone()));
        char_repo
            .update_experience(player_guid.counter(), xp, level)
            .await?;

        Ok(())
    }

    /// Save player health and power to database
    pub async fn save_health_and_power(
        &self,
        player_guid: ObjectGuid,
        character_db: &sqlx::MySqlPool,
    ) -> anyhow::Result<()> {
        let (health, power1, power2, power3, power4, power5) = self
            .get_player(player_guid)
            .map(|p| {
                (
                    p.stats.health,
                    p.power.current[0], // Mana
                    p.power.current[1], // Rage
                    p.power.current[2], // Focus
                    p.power.current[3], // Energy
                    p.power.current[4], // Happiness
                )
            })
            .ok_or_else(|| anyhow::anyhow!("Player not found"))?;

        let char_repo = CharacterRepository::new(Arc::new(character_db.clone()));
        char_repo
            .update_health_and_power(
                player_guid.counter(),
                health,
                power1,
                power2,
                power3,
                power4,
                power5,
            )
            .await?;

        Ok(())
    }

    /// Save player rest state to database (rest bonus, logout time, character flags)
    pub async fn save_rest_state(
        &self,
        player_guid: ObjectGuid,
        character_db: &sqlx::MySqlPool,
    ) -> anyhow::Result<()> {
        let (rest_bonus, character_flags) = self
            .get_player(player_guid)
            .map(|p| (p.rest_bonus, p.player_flags))
            .ok_or_else(|| anyhow::anyhow!("Player not found"))?;

        // Get current timestamp for logout_time
        let logout_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let char_repo = CharacterRepository::new(Arc::new(character_db.clone()));
        char_repo
            .update_rest_data(
                player_guid.counter(),
                rest_bonus,
                logout_time,
                character_flags,
            )
            .await?;

        Ok(())
    }

    // ========== Data-collection helpers (pub(crate) for testing) ==========

    /// Collect the spellbook for a player. Returns the list of spell IDs to persist.
    pub(crate) fn collect_spells_for_save(&self, player_guid: ObjectGuid) -> Vec<u32> {
        self.get_player(player_guid)
            .map(|p| p.spells.spellbook.clone())
            .unwrap_or_default()
    }

    /// Collect non-empty action bar slots for a player.
    /// Returns `(slot, action, button_type)` for every occupied slot.
    pub(crate) fn collect_action_buttons_for_save(
        &self,
        player_guid: ObjectGuid,
    ) -> Vec<(u8, u32, u8)> {
        self.get_player(player_guid)
            .map(|p| {
                p.settings
                    .action_buttons
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, opt)| {
                        opt.map(|btn| (slot as u8, btn.action, btn.button_type))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Collect reputation standings for a player.
    /// Returns `(faction_id, standing, flags)` for every tracked faction.
    pub(crate) fn collect_reputation_for_save(
        &self,
        player_guid: ObjectGuid,
    ) -> Vec<(u32, i32, i32)> {
        self.get_player(player_guid)
            .map(|p| {
                p.reputation
                    .factions
                    .values()
                    .map(|s| (s.faction_id, s.standing, s.flags as i32))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Collect skill values for a player, excluding deleted entries.
    /// Returns `(skill_id, current_value, max_value)` for every live skill.
    pub(crate) fn collect_skills_for_save(&self, player_guid: ObjectGuid) -> Vec<(u16, u16, u16)> {
        use crate::world::game::player::skills::state::SkillSaveState;
        self.get_player(player_guid)
            .map(|p| {
                p.skills
                    .skills
                    .values()
                    .filter(|s| s.state != SkillSaveState::Deleted)
                    .map(|s| (s.skill_id, s.current_value, s.max_value))
                    .collect()
            })
            .unwrap_or_default()
    }

    // ========== Save methods ==========

    /// Save learned spells to database.
    pub async fn save_spells(
        &self,
        player_guid: ObjectGuid,
        character_db: &sqlx::MySqlPool,
    ) -> anyhow::Result<()> {
        let spellbook = self.collect_spells_for_save(player_guid);

        let char_repo = CharacterRepository::new(Arc::new(character_db.clone()));
        // Delete all saved spells then re-insert (matches vMaNGOS _SaveSpells pattern)
        sqlx::query("DELETE FROM character_spell WHERE guid = ?")
            .bind(player_guid.counter())
            .execute(character_db)
            .await?;
        for spell_id in spellbook {
            char_repo
                .add_spell(player_guid.counter(), spell_id, 1, 0)
                .await?;
        }

        Ok(())
    }

    /// Save action bar buttons to database.
    pub async fn save_action_buttons(
        &self,
        player_guid: ObjectGuid,
        character_db: &sqlx::MySqlPool,
    ) -> anyhow::Result<()> {
        let buttons = self.collect_action_buttons_for_save(player_guid);
        let char_repo = CharacterRepository::new(Arc::new(character_db.clone()));
        char_repo
            .save_actions(player_guid.counter(), &buttons)
            .await?;
        Ok(())
    }

    /// Save reputation standings to database.
    pub async fn save_reputation(
        &self,
        player_guid: ObjectGuid,
        character_db: &sqlx::MySqlPool,
    ) -> anyhow::Result<()> {
        use crate::shared::database::characters::models::reputation::ReputationRow;
        use crate::shared::database::characters::repositories::ReputationRepository;

        let factions = self.collect_reputation_for_save(player_guid);

        let rep_repo = ReputationRepository::new(Arc::new(character_db.clone()));
        for (faction_id, standing, flags) in factions {
            rep_repo
                .save_reputation(&ReputationRow {
                    guid: player_guid.counter(),
                    faction: faction_id,
                    standing,
                    flags,
                })
                .await?;
        }

        Ok(())
    }

    /// Save skill values to database.
    pub async fn save_skills(
        &self,
        player_guid: ObjectGuid,
        character_db: &sqlx::MySqlPool,
    ) -> anyhow::Result<()> {
        let skills = self.collect_skills_for_save(player_guid);
        let char_repo = CharacterRepository::new(Arc::new(character_db.clone()));
        char_repo
            .save_skills(player_guid.counter(), &skills)
            .await?;
        Ok(())
    }

    /// Save all player data to database (comprehensive save for logout and auto-saves)
    ///
    /// This saves:
    /// - Position (map, instance, zone, coordinates, orientation)
    /// - Experience and level
    /// - Health and power (mana/rage/energy)
    /// - Rest state (rest bonus, logout time, character flags)
    /// - Spells (spellbook)
    /// - Action bar buttons
    /// - Reputation standings
    /// - Skills
    pub async fn save_all_player_data(
        &self,
        player_guid: ObjectGuid,
        character_db: &sqlx::MySqlPool,
    ) -> anyhow::Result<()> {
        // Save all player data in parallel for performance
        let (
            pos_result,
            xp_result,
            health_result,
            rest_result,
            spells_result,
            actions_result,
            rep_result,
            skills_result,
        ) = tokio::join!(
            self.save_position(player_guid, character_db),
            self.save_experience(player_guid, character_db),
            self.save_health_and_power(player_guid, character_db),
            self.save_rest_state(player_guid, character_db),
            self.save_spells(player_guid, character_db),
            self.save_action_buttons(player_guid, character_db),
            self.save_reputation(player_guid, character_db),
            self.save_skills(player_guid, character_db),
        );

        // Check for errors
        pos_result?;
        xp_result?;
        health_result?;
        rest_result?;
        spells_result?;
        actions_result?;
        rep_result?;
        skills_result?;

        Ok(())
    }

    /// Get player's looting target
    pub fn get_looting_target(&self, guid: ObjectGuid) -> Option<ObjectGuid> {
        self.get_player(guid).and_then(|p| p.looting_target)
    }

    /// Set player's looting target
    pub fn set_looting_target(&self, guid: ObjectGuid, target: ObjectGuid) {
        if let Some(mut player) = self.get_player_mut(guid) {
            player.looting_target = Some(target);
        }
    }

    /// Clear player's looting target
    pub fn clear_looting_target(&self, guid: ObjectGuid) {
        if let Some(mut player) = self.get_player_mut(guid) {
            player.looting_target = None;
        }
    }

    /// Add money to player
    pub async fn add_money(&self, guid: ObjectGuid, amount: u32) -> anyhow::Result<()> {
        if let Some(mut player) = self.get_player_mut(guid) {
            player.money = player.money.saturating_add(amount);
            // TODO: Send money update packet to client
            Ok(())
        } else {
            Err(anyhow::anyhow!("Player not found"))
        }
    }

    /// Get player's money
    pub fn get_money(&self, guid: ObjectGuid) -> Option<u32> {
        self.get_player(guid).map(|p| p.money)
    }
}

impl Default for PlayerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::{HighGuid, ObjectGuid};
    use crate::world::game::player::player::Player;
    use crate::world::game::player::reputation::state::FactionStanding;
    use crate::world::game::player::settings::state::{
        ACTION_BUTTON_ITEM, ACTION_BUTTON_MACRO, ACTION_BUTTON_SPELL,
    };
    use crate::world::game::player::skills::state::{SkillData, SkillSaveState, SkillState};

    // ========== Helpers ==========

    fn test_guid(low: u32) -> ObjectGuid {
        ObjectGuid::new_without_entry(HighGuid::Player, low)
    }

    fn make_manager_with_player(guid: ObjectGuid) -> PlayerManager {
        let mgr = PlayerManager::new();
        let player = Player::new(
            guid,
            format!("Player{}", guid.counter()),
            0,
            0,
            1,
            1,
            1,
            1,
            0,
        );
        mgr.add_player(player, guid.counter());
        mgr
    }

    // ========== Spells ==========

    #[test]
    fn collect_spells_empty_when_no_spells_learned() {
        let guid = test_guid(1);
        let mgr = make_manager_with_player(guid);

        let spells = mgr.collect_spells_for_save(guid);
        assert!(spells.is_empty());
    }

    #[test]
    fn collect_spells_returns_all_learned_spells() {
        let guid = test_guid(2);
        let mgr = make_manager_with_player(guid);

        mgr.with_player_mut(guid, |p| {
            p.spells.learn_spell(133); // Fireball
            p.spells.learn_spell(2136); // Fire Blast
            p.spells.learn_spell(6603); // Attack
        });

        let mut spells = mgr.collect_spells_for_save(guid);
        spells.sort();
        assert_eq!(spells, vec![133, 2136, 6603]);
    }

    #[test]
    fn collect_spells_returns_empty_for_unknown_player() {
        let mgr = PlayerManager::new();
        let spells = mgr.collect_spells_for_save(test_guid(99));
        assert!(spells.is_empty());
    }

    // ========== Action buttons ==========

    #[test]
    fn collect_action_buttons_empty_when_no_buttons_set() {
        let guid = test_guid(10);
        let mgr = make_manager_with_player(guid);

        let buttons = mgr.collect_action_buttons_for_save(guid);
        assert!(buttons.is_empty());
    }

    #[test]
    fn collect_action_buttons_returns_set_slots() {
        let guid = test_guid(11);
        let mgr = make_manager_with_player(guid);

        mgr.with_player_mut(guid, |p| {
            p.settings.set_action_button(0, 133, ACTION_BUTTON_SPELL); // slot 0: Fireball
            p.settings.set_action_button(1, 2136, ACTION_BUTTON_SPELL); // slot 1: Fire Blast
            p.settings.set_action_button(11, 5, ACTION_BUTTON_ITEM); // slot 11: some item
        });

        let mut buttons = mgr.collect_action_buttons_for_save(guid);
        buttons.sort_by_key(|&(slot, _, _)| slot);

        assert_eq!(buttons.len(), 3);
        assert_eq!(buttons[0], (0, 133, ACTION_BUTTON_SPELL));
        assert_eq!(buttons[1], (1, 2136, ACTION_BUTTON_SPELL));
        assert_eq!(buttons[2], (11, 5, ACTION_BUTTON_ITEM));
    }

    #[test]
    fn collect_action_buttons_excludes_cleared_slots() {
        let guid = test_guid(12);
        let mgr = make_manager_with_player(guid);

        mgr.with_player_mut(guid, |p| {
            p.settings.set_action_button(0, 133, ACTION_BUTTON_SPELL);
            p.settings.set_action_button(1, 200, ACTION_BUTTON_MACRO);
            p.settings.clear_action_button(0); // clear slot 0
        });

        let buttons = mgr.collect_action_buttons_for_save(guid);
        assert_eq!(buttons.len(), 1);
        assert_eq!(buttons[0], (1, 200, ACTION_BUTTON_MACRO));
    }

    #[test]
    fn collect_action_buttons_returns_empty_for_unknown_player() {
        let mgr = PlayerManager::new();
        let buttons = mgr.collect_action_buttons_for_save(test_guid(99));
        assert!(buttons.is_empty());
    }

    // ========== Reputation ==========

    #[test]
    fn collect_reputation_empty_when_no_factions() {
        let guid = test_guid(20);
        let mgr = make_manager_with_player(guid);

        let factions = mgr.collect_reputation_for_save(guid);
        assert!(factions.is_empty());
    }

    #[test]
    fn collect_reputation_returns_all_faction_standings() {
        let guid = test_guid(21);
        let mgr = make_manager_with_player(guid);

        mgr.with_player_mut(guid, |p| {
            // Stormwind (faction 72, rep_list_id 0): friendly standing
            p.reputation
                .insert_standing(FactionStanding::new(72, 0, 3000, 1));
            // Orgrimmar (faction 76, rep_list_id 1): hostile standing
            p.reputation
                .insert_standing(FactionStanding::new(76, 1, -6000, 0));
        });

        let factions = mgr.collect_reputation_for_save(guid);
        assert_eq!(factions.len(), 2);

        let stormwind = factions.iter().find(|&&(fid, _, _)| fid == 72).unwrap();
        assert_eq!(stormwind.1, 3000);
        assert_eq!(stormwind.2, 1);

        let orgrimmar = factions.iter().find(|&&(fid, _, _)| fid == 76).unwrap();
        assert_eq!(orgrimmar.1, -6000);
        assert_eq!(orgrimmar.2, 0);
    }

    #[test]
    fn collect_reputation_returns_empty_for_unknown_player() {
        let mgr = PlayerManager::new();
        let factions = mgr.collect_reputation_for_save(test_guid(99));
        assert!(factions.is_empty());
    }

    // ========== Skills ==========

    #[test]
    fn collect_skills_empty_when_no_skills() {
        let guid = test_guid(30);
        let mgr = make_manager_with_player(guid);

        let skills = mgr.collect_skills_for_save(guid);
        assert!(skills.is_empty());
    }

    #[test]
    fn collect_skills_returns_active_skills() {
        let guid = test_guid(31);
        let mgr = make_manager_with_player(guid);

        mgr.with_player_mut(guid, |p| {
            p.skills.skills.insert(
                43,
                SkillData {
                    skill_id: 43, // Swords
                    current_value: 150,
                    max_value: 300,
                    step: 0,
                    position: 0,
                    state: SkillSaveState::New,
                },
            );
            p.skills.skills.insert(
                95,
                SkillData {
                    skill_id: 95, // Defense
                    current_value: 200,
                    max_value: 300,
                    step: 0,
                    position: 1,
                    state: SkillSaveState::Changed,
                },
            );
        });

        let mut skills = mgr.collect_skills_for_save(guid);
        skills.sort_by_key(|&(id, _, _)| id);

        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0], (43, 150, 300));
        assert_eq!(skills[1], (95, 200, 300));
    }

    #[test]
    fn collect_skills_excludes_deleted_skills() {
        let guid = test_guid(32);
        let mgr = make_manager_with_player(guid);

        mgr.with_player_mut(guid, |p| {
            p.skills.skills.insert(
                43,
                SkillData {
                    skill_id: 43,
                    current_value: 150,
                    max_value: 300,
                    step: 0,
                    position: 0,
                    state: SkillSaveState::Unchanged,
                },
            );
            p.skills.skills.insert(
                95,
                SkillData {
                    skill_id: 95,
                    current_value: 200,
                    max_value: 300,
                    step: 0,
                    position: 1,
                    state: SkillSaveState::Deleted, // should be excluded
                },
            );
        });

        let skills = mgr.collect_skills_for_save(guid);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0], (43, 150, 300));
    }

    #[test]
    fn collect_skills_returns_empty_for_unknown_player() {
        let mgr = PlayerManager::new();
        let skills = mgr.collect_skills_for_save(test_guid(99));
        assert!(skills.is_empty());
    }

    // ========== State-change → collection round-trip ==========

    #[test]
    fn spells_learned_during_session_appear_in_collection() {
        let guid = test_guid(40);
        let mgr = make_manager_with_player(guid);

        // Simulate trainer teaching a new spell mid-session
        mgr.with_player_mut(guid, |p| {
            p.spells.learn_spell(9875); // Some trainer spell
        });

        let spells = mgr.collect_spells_for_save(guid);
        assert!(
            spells.contains(&9875),
            "Newly learned spell must appear in save data"
        );
    }

    #[test]
    fn action_bar_changes_during_session_appear_in_collection() {
        let guid = test_guid(41);
        let mgr = make_manager_with_player(guid);

        // Simulate player dragging a spell onto slot 5
        mgr.with_player_mut(guid, |p| {
            p.settings.set_action_button(5, 9875, ACTION_BUTTON_SPELL);
        });

        let buttons = mgr.collect_action_buttons_for_save(guid);
        let slot5 = buttons.iter().find(|&&(slot, _, _)| slot == 5);
        assert!(
            slot5.is_some(),
            "Changed action bar slot must appear in save data"
        );
        assert_eq!(slot5.unwrap().1, 9875);
    }

    #[test]
    fn reputation_gained_during_session_appears_in_collection() {
        let guid = test_guid(42);
        let mgr = make_manager_with_player(guid);

        // Simulate gaining rep mid-session
        mgr.with_player_mut(guid, |p| {
            p.reputation
                .insert_standing(FactionStanding::new(72, 0, 1500, 1));
        });

        let factions = mgr.collect_reputation_for_save(guid);
        let sw = factions.iter().find(|&&(fid, _, _)| fid == 72).unwrap();
        assert_eq!(sw.1, 1500, "Gained reputation must appear in save data");
    }

    #[test]
    fn skill_gain_during_session_appears_in_collection() {
        let guid = test_guid(43);
        let mgr = make_manager_with_player(guid);

        mgr.with_player_mut(guid, |p| {
            p.skills.skills.insert(
                43,
                SkillData {
                    skill_id: 43,
                    current_value: 151, // skilled up from 150
                    max_value: 300,
                    step: 0,
                    position: 0,
                    state: SkillSaveState::Changed,
                },
            );
        });

        let skills = mgr.collect_skills_for_save(guid);
        let swords = skills.iter().find(|&&(id, _, _)| id == 43).unwrap();
        assert_eq!(swords.1, 151, "Skill-up value must appear in save data");
    }
}
