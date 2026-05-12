use super::super::models::character::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{MySqlPool, Row};
use std::sync::Arc;

pub struct CharacterRepository {
    pool: Arc<MySqlPool>,
}

impl CharacterRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ========== CHARACTER QUERY METHODS ==========

    /// Find a character by GUID.
    pub async fn find_by_guid(&self, guid: u32) -> Result<Option<CharacterRow>> {
        sqlx::query_as::<_, CharacterRow>(
            r#"SELECT guid, account, name, race, class, gender, skin, face,
                      hair_style, hair_color, facial_hair, level, xp, money,
                      character_flags, zone, map, instance, position_x, position_y, position_z,
                      orientation, transport_guid, transport_x, transport_y, transport_z, transport_o,
                      known_taxi_mask, current_taxi_path, online, played_time_total, played_time_level,
                      create_time, logout_time, rest_bonus, reset_talents_multiplier, reset_talents_time,
                      death_expire_time, stable_slots, bank_bag_slots, extra_flags,
                      honor_rank_points, honor_highest_rank, honor_standing,
                      honor_last_week_hk, honor_last_week_cp, honor_stored_hk, honor_stored_dk,
                      watched_faction, drunk, health, power1, power2, power3, power4, power5,
                      explored_zones, equipment_cache, ammo_id, action_bars,
                      deleted_account, deleted_name, deleted_time, world_phase_mask
               FROM characters
               WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch character by GUID")
    }

    /// Find all characters for an account.
    pub async fn find_by_account(&self, account_id: u32) -> Result<Vec<CharacterRow>> {
        sqlx::query_as::<_, CharacterRow>(
            r#"SELECT guid, account, name, race, class, gender, skin, face,
                      hair_style, hair_color, facial_hair, level, xp, money,
                      character_flags, zone, map, instance, position_x, position_y, position_z,
                      orientation, transport_guid, transport_x, transport_y, transport_z, transport_o,
                      known_taxi_mask, current_taxi_path, online, played_time_total, played_time_level,
                      create_time, logout_time, rest_bonus, reset_talents_multiplier, reset_talents_time,
                      death_expire_time, stable_slots, bank_bag_slots, extra_flags,
                      honor_rank_points, honor_highest_rank, honor_standing,
                      honor_last_week_hk, honor_last_week_cp, honor_stored_hk, honor_stored_dk,
                      watched_faction, drunk, health, power1, power2, power3, power4, power5,
                      explored_zones, equipment_cache, ammo_id, action_bars,
                      deleted_account, deleted_name, deleted_time, world_phase_mask
               FROM characters
               WHERE account = ?
               ORDER BY guid"#,
        )
        .bind(account_id)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch characters by account")
    }

    /// Find a character by name.
    pub async fn find_by_name(&self, name: &str) -> Result<Option<CharacterRow>> {
        sqlx::query_as::<_, CharacterRow>(
            r#"SELECT guid, account, name, race, class, gender, skin, face,
                      hair_style, hair_color, facial_hair, level, xp, money,
                      character_flags, zone, map, instance, position_x, position_y, position_z,
                      orientation, transport_guid, transport_x, transport_y, transport_z, transport_o,
                      known_taxi_mask, current_taxi_path, online, played_time_total, played_time_level,
                      create_time, logout_time, rest_bonus, reset_talents_multiplier, reset_talents_time,
                      death_expire_time, stable_slots, bank_bag_slots, extra_flags,
                      honor_rank_points, honor_highest_rank, honor_standing,
                      honor_last_week_hk, honor_last_week_cp, honor_stored_hk, honor_stored_dk,
                      watched_faction, drunk, health, power1, power2, power3, power4, power5,
                      explored_zones, equipment_cache, ammo_id, action_bars,
                      deleted_account, deleted_name, deleted_time, world_phase_mask
               FROM characters
               WHERE name = ?"#,
        )
        .bind(name)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch character by name")
    }

    /// Check if a character name exists.
    pub async fn exists_by_name(&self, name: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM characters WHERE name = ?")
            .bind(name)
            .fetch_one(&*self.pool)
            .await
            .context("Failed to check character name existence")?;

        Ok(count > 0)
    }

    // ========== SPELLS ==========

    /// Find all spells for a character.
    pub async fn find_spells(&self, guid: u32) -> Result<Vec<CharacterSpellRow>> {
        sqlx::query_as::<_, CharacterSpellRow>(
            r#"SELECT guid, spell, active, disabled
               FROM character_spell
               WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch character spells")
    }

    /// Add a spell to a character.
    pub async fn add_spell(&self, guid: u32, spell: u32, active: u8, disabled: u8) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO character_spell (guid, spell, active, disabled)
               VALUES (?, ?, ?, ?)
               ON DUPLICATE KEY UPDATE active = VALUES(active), disabled = VALUES(disabled)"#,
        )
        .bind(guid)
        .bind(spell)
        .bind(active)
        .bind(disabled)
        .execute(&*self.pool)
        .await
        .context("Failed to add character spell")?;

        Ok(())
    }

    /// Delete a spell from a character.
    pub async fn delete_spell(&self, guid: u32, spell: u32) -> Result<()> {
        sqlx::query("DELETE FROM character_spell WHERE guid = ? AND spell = ?")
            .bind(guid)
            .bind(spell)
            .execute(&*self.pool)
            .await
            .context("Failed to delete character spell")?;

        Ok(())
    }

    // ========== AURAS ==========

    /// Find all auras for a character.
    pub async fn find_auras(&self, guid: u32) -> Result<Vec<CharacterAuraRow>> {
        sqlx::query_as::<_, CharacterAuraRow>(
            r#"SELECT guid, caster_guid, item_guid, spell, stacks, charges,
                      base_points0, base_points1, base_points2,
                      periodic_time0, periodic_time1, periodic_time2,
                      max_duration, duration, effect_index_mask
               FROM character_aura
               WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch character auras")
    }

    /// Delete all auras for a character.
    pub async fn delete_auras(&self, guid: u32) -> Result<()> {
        sqlx::query("DELETE FROM character_aura WHERE guid = ?")
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete character auras")?;

        Ok(())
    }

    // ========== INVENTORY ==========

    /// Find equipped items for a character (bag=0/255, slots 0-18).
    /// Returns Vec of (slot, item_id) pairs for character enumeration.
    /// Note: item_id is stored directly in character_inventory, no JOIN needed.
    pub async fn find_equipped_items(&self, guid: u32) -> Result<Vec<(u8, u32)>> {
        let rows = sqlx::query(
            r#"SELECT slot, item_id
               FROM character_inventory
               WHERE guid = ? AND bag IN (0, 255) AND slot < 19
               ORDER BY slot"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch equipped items")?;

        rows.iter()
            .map(|row| {
                Ok((
                    row.try_get::<u8, _>("slot")?,
                    row.try_get::<u32, _>("item_id")?,
                ))
            })
            .collect()
    }

    /// Find equipped items for multiple characters in a single query (for character enumeration).
    /// Returns HashMap<guid, Vec<(slot, item_id)>>.
    /// This eliminates the N+1 query problem when loading equipment for character select screen.
    pub async fn find_equipped_items_batch(
        &self,
        character_guids: &[u32],
    ) -> Result<std::collections::HashMap<u32, Vec<(u8, u32)>>> {
        use std::collections::HashMap;

        if character_guids.is_empty() {
            return Ok(HashMap::new());
        }

        // Build placeholders for IN clause
        let placeholders = character_guids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let query = format!(
            r#"SELECT guid, slot, item_id
               FROM character_inventory
               WHERE guid IN ({}) AND bag IN (0, 255) AND slot < 19
               ORDER BY guid, slot"#,
            placeholders
        );

        let mut query_builder = sqlx::query(&query);
        for &guid in character_guids {
            query_builder = query_builder.bind(guid);
        }

        let rows = query_builder
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch equipped items batch")?;

        let mut result: HashMap<u32, Vec<(u8, u32)>> = HashMap::new();
        for row in rows {
            let guid: u32 = row
                .try_get("guid")
                .context("Failed to read guid from equipped items batch")?;
            let slot: u8 = row
                .try_get("slot")
                .context("Failed to read slot from equipped items batch")?;
            let item_id: u32 = row
                .try_get("item_id")
                .context("Failed to read item_id from equipped items batch")?;

            result
                .entry(guid)
                .or_insert_with(Vec::new)
                .push((slot, item_id));
        }

        Ok(result)
    }

    /// Find all inventory items for a character.
    pub async fn find_inventory(&self, guid: u32) -> Result<Vec<CharacterInventoryRow>> {
        sqlx::query_as::<_, CharacterInventoryRow>(
            r#"SELECT guid, bag, slot, item_guid, item_id
               FROM character_inventory
               WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch character inventory")
    }

    /// Find an empty inventory slot for a character.
    /// Returns the first available slot between INVENTORY_SLOT_ITEM_START (23) and INVENTORY_SLOT_ITEM_END (39).
    pub async fn find_empty_inventory_slot(&self, guid: u32) -> Result<Option<u8>> {
        const INVENTORY_SLOT_ITEM_START: u8 = 23;
        const INVENTORY_SLOT_ITEM_END: u8 = 39;

        for slot in INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_END {
            let existing: Option<u32> = sqlx::query_scalar(
                "SELECT item_guid FROM character_inventory WHERE guid = ? AND bag = 0 AND slot = ?",
            )
            .bind(guid)
            .bind(slot)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to check inventory slot")?;

            if existing.is_none() {
                return Ok(Some(slot));
            }
        }

        Ok(None)
    }

    /// Add an item to a character's inventory.
    pub async fn add_item_to_inventory(
        &self,
        guid: u32,
        bag: u32,
        slot: u8,
        item_guid: u32,
        item_id: u32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO character_inventory (guid, bag, slot, item_guid, item_id) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(guid)
        .bind(bag)
        .bind(slot)
        .bind(item_guid)
        .bind(item_id)
        .execute(&*self.pool)
        .await
        .context("Failed to add item to inventory")?;

        Ok(())
    }

    /// Create an item instance.
    pub async fn create_item_instance(
        &self,
        guid: u32,
        item_id: u32,
        owner_guid: u32,
        creator_guid: u32,
        count: u32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO item_instance (guid, item_id, owner_guid, creator_guid, count, enchantments) VALUES (?, ?, ?, ?, ?, '')",
        )
        .bind(guid)
        .bind(item_id)
        .bind(owner_guid)
        .bind(creator_guid)
        .bind(count)
        .execute(&*self.pool)
        .await
        .context("Failed to create item instance")?;

        Ok(())
    }

    // ========== SKILLS ==========

    /// Find all skills for a character.
    pub async fn find_skills(&self, guid: u32) -> Result<Vec<CharacterSkillRow>> {
        sqlx::query_as::<_, CharacterSkillRow>(
            r#"SELECT guid, skill, value, max
               FROM character_skills
               WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch character skills")
    }

    // ========== REPUTATION ==========

    /// Find all reputation standings for a character.
    pub async fn find_reputation(&self, guid: u32) -> Result<Vec<CharacterReputationRow>> {
        sqlx::query_as::<_, CharacterReputationRow>(
            r#"SELECT guid, faction, standing, flags
               FROM character_reputation
               WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch character reputation")
    }

    // ========== ACTIONS ==========

    /// Find all action bar bindings for a character.
    pub async fn find_actions(&self, guid: u32) -> Result<Vec<CharacterActionRow>> {
        sqlx::query_as::<_, CharacterActionRow>(
            r#"SELECT guid, button, action, type
               FROM character_action
               WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch character actions")
    }

    /// Save all action bar bindings for a character (delete-all then re-insert).
    pub async fn save_actions(&self, guid: u32, buttons: &[(u8, u32, u8)]) -> Result<()> {
        sqlx::query("DELETE FROM character_action WHERE guid = ?")
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete character actions")?;
        for &(button, action, r#type) in buttons {
            sqlx::query(
                "INSERT INTO character_action (guid, button, action, type) VALUES (?, ?, ?, ?)",
            )
            .bind(guid)
            .bind(button)
            .bind(action)
            .bind(r#type)
            .execute(&*self.pool)
            .await
            .context("Failed to insert character action")?;
        }
        Ok(())
    }

    /// Save skill values for a character (REPLACE INTO for each skill).
    pub async fn save_skills(&self, guid: u32, skills: &[(u16, u16, u16)]) -> Result<()> {
        for &(skill_id, value, max) in skills {
            sqlx::query(
                "REPLACE INTO character_skills (guid, skill, value, max) VALUES (?, ?, ?, ?)",
            )
            .bind(guid)
            .bind(skill_id)
            .bind(value)
            .bind(max)
            .execute(&*self.pool)
            .await
            .context("Failed to save character skill")?;
        }
        Ok(())
    }

    // ========== QUESTS ==========

    /// Find all quest status entries for a character.
    pub async fn find_quest_status(&self, guid: u32) -> Result<Vec<CharacterQuestStatusRow>> {
        sqlx::query_as::<_, CharacterQuestStatusRow>(
            r#"SELECT guid, quest, status, rewarded, explored, timer,
                      mob_count1, mob_count2, mob_count3, mob_count4,
                      item_count1, item_count2, item_count3, item_count4,
                      reward_choice
               FROM character_queststatus
               WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch character quest status")
    }

    // ========== HOMEBIND ==========

    /// Find homebind (hearthstone location) for a character.
    pub async fn find_homebind(&self, guid: u32) -> Result<Option<CharacterHomebindRow>> {
        sqlx::query_as::<_, CharacterHomebindRow>(
            r#"SELECT guid, map, zone, position_x, position_y, position_z
               FROM character_homebind
               WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch character homebind")
    }

    /// Upsert homebind (hearthstone location) for a character.
    /// Called from the bindpoint handler when the player binds at an innkeeper.
    pub async fn save_homebind(
        &self,
        guid: u32,
        map: u32,
        zone: u32,
        x: f32,
        y: f32,
        z: f32,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO character_homebind
                   (guid, map, zone, position_x, position_y, position_z)
               VALUES (?, ?, ?, ?, ?, ?)
               ON DUPLICATE KEY UPDATE
                   map = VALUES(map),
                   zone = VALUES(zone),
                   position_x = VALUES(position_x),
                   position_y = VALUES(position_y),
                   position_z = VALUES(position_z)"#,
        )
        .bind(guid)
        .bind(map)
        .bind(zone)
        .bind(x)
        .bind(y)
        .bind(z)
        .execute(&*self.pool)
        .await
        .context("Failed to save character homebind")?;

        Ok(())
    }

    // ========== SPELL COOLDOWNS ==========

    /// Find all spell cooldowns for a character.
    pub async fn find_spell_cooldowns(&self, guid: u32) -> Result<Vec<CharacterSpellCooldownRow>> {
        sqlx::query_as::<_, CharacterSpellCooldownRow>(
            r#"SELECT guid, spell, spell_expire_time, category, category_expire_time, item_id
               FROM character_spell_cooldown
               WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch character spell cooldowns")
    }

    /// Delete all spell cooldowns for a character.
    pub async fn delete_spell_cooldowns(&self, guid: u32) -> Result<()> {
        sqlx::query("DELETE FROM character_spell_cooldown WHERE guid = ?")
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete character spell cooldowns")?;

        Ok(())
    }

    // ========== CHARACTER WRITE OPERATIONS ==========

    /// Create a new character (simplified - only sets required fields, lets database use defaults)
    pub async fn create_simple(
        &self,
        guid: u32,
        account: u32,
        name: &str,
        race: u8,
        class: u8,
        gender: u8,
        skin: u8,
        face: u8,
        hair_style: u8,
        hair_color: u8,
        facial_hair: u8,
        map: u32,
        zone: u32,
        position_x: f32,
        position_y: f32,
        position_z: f32,
        orientation: f32,
        health: u32,
        power1: u32,
        money: u32,
    ) -> Result<()> {
        let create_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        sqlx::query(
            r#"INSERT INTO characters
               (guid, account, name, race, class, gender, skin, face,
                hair_style, hair_color, facial_hair, level, xp, money,
                character_flags, zone, map, position_x, position_y, position_z,
                orientation, health, power1, create_time)
               VALUES
               (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 0, ?, 0, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(guid)
        .bind(account)
        .bind(name)
        .bind(race)
        .bind(class)
        .bind(gender)
        .bind(skin)
        .bind(face)
        .bind(hair_style)
        .bind(hair_color)
        .bind(facial_hair)
        .bind(money)
        .bind(zone)
        .bind(map)
        .bind(position_x)
        .bind(position_y)
        .bind(position_z)
        .bind(orientation)
        .bind(health)
        .bind(power1)
        .bind(create_time)
        .execute(&*self.pool)
        .await
        .context("Failed to create character")?;

        Ok(())
    }

    /// Create a new character.
    pub async fn create(&self, character: &CharacterRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO characters
               (guid, account, name, race, class, gender, skin, face,
                hair_style, hair_color, facial_hair, level, xp, money,
                character_flags, zone, map, instance, position_x, position_y, position_z,
                orientation, transport_guid, transport_x, transport_y, transport_z, transport_o,
                known_taxi_mask, current_taxi_path, online, played_time_total, played_time_level,
                create_time, logout_time, rest_bonus, reset_talents_multiplier, reset_talents_time,
                death_expire_time, stable_slots, bank_bag_slots, extra_flags,
                honor_rank_points, honor_highest_rank, honor_standing,
                honor_last_week_hk, honor_last_week_cp, honor_stored_hk, honor_stored_dk,
                watched_faction, drunk, health, power1, power2, power3, power4, power5,
                explored_zones, equipment_cache, ammo_id, action_bars,
                deleted_account, deleted_name, deleted_time, world_phase_mask)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                       ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                       ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                       ?, ?, ?, ?)"#,
        )
        // Character identity and appearance (1-11)
        .bind(character.guid)
        .bind(character.account)
        .bind(&character.name)
        .bind(character.race)
        .bind(character.class)
        .bind(character.gender)
        .bind(character.skin)
        .bind(character.face)
        .bind(character.hair_style)
        .bind(character.hair_color)
        .bind(character.facial_hair)
        // Level and progression (12-14)
        .bind(character.level)
        .bind(character.xp)
        .bind(character.money)
        // Flags and location (15-21)
        .bind(character.character_flags)
        .bind(character.zone)
        .bind(character.map)
        .bind(character.instance)
        .bind(character.position_x)
        .bind(character.position_y)
        .bind(character.position_z)
        // Orientation and transport (22-27)
        .bind(character.orientation)
        .bind(character.transport_guid)
        .bind(character.transport_x)
        .bind(character.transport_y)
        .bind(character.transport_z)
        .bind(character.transport_o)
        // Taxi and online status (28-30)
        .bind(&character.known_taxi_mask)
        .bind(&character.current_taxi_path)
        .bind(character.online)
        // Play time and timestamps (31-34)
        .bind(character.played_time_total)
        .bind(character.played_time_level)
        .bind(character.create_time)
        .bind(character.logout_time)
        // Rest and talents (35-37)
        .bind(character.rest_bonus)
        .bind(character.reset_talents_multiplier)
        .bind(character.reset_talents_time)
        // Death and bags (38-40)
        .bind(character.death_expire_time)
        .bind(character.stable_slots)
        .bind(character.bank_bag_slots)
        // Extra flags (41)
        .bind(character.extra_flags)
        // Honor system (42-48)
        .bind(character.honor_rank_points)
        .bind(character.honor_highest_rank)
        .bind(character.honor_standing)
        .bind(character.honor_last_week_hk)
        .bind(character.honor_last_week_cp)
        .bind(character.honor_stored_hk)
        .bind(character.honor_stored_dk)
        // Misc (49-50)
        .bind(character.watched_faction)
        .bind(character.drunk)
        // Resources (51-56)
        .bind(character.health)
        .bind(character.power1)
        .bind(character.power2)
        .bind(character.power3)
        .bind(character.power4)
        .bind(character.power5)
        // Blobs and UI (57-59)
        .bind(&character.explored_zones)
        .bind(&character.equipment_cache)
        .bind(character.ammo_id)
        .bind(character.action_bars)
        // Deletion info (60-63)
        .bind(character.deleted_account)
        .bind(&character.deleted_name)
        .bind(character.deleted_time)
        .bind(character.world_phase_mask)
        .execute(&*self.pool)
        .await
        .context("Failed to create character")?;

        Ok(())
    }

    /// Update character position, zone, and map.
    pub async fn update_position(
        &self,
        guid: u32,
        map: u32,
        instance: u32,
        zone: u32,
        position_x: f32,
        position_y: f32,
        position_z: f32,
        orientation: f32,
    ) -> Result<()> {
        sqlx::query(
            r#"UPDATE characters
               SET map = ?, instance = ?, zone = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ?
               WHERE guid = ?"#,
        )
        .bind(map)
        .bind(instance)
        .bind(zone)
        .bind(position_x)
        .bind(position_y)
        .bind(position_z)
        .bind(orientation)
        .bind(guid)
        .execute(&*self.pool)
        .await
        .context("Failed to update character position")?;

        Ok(())
    }

    /// Update character online status.
    pub async fn update_online(&self, guid: u32, online: bool) -> Result<()> {
        sqlx::query("UPDATE characters SET online = ? WHERE guid = ?")
            .bind(if online { 1u8 } else { 0u8 })
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update character online status")?;

        Ok(())
    }

    /// Update character name and flags (for rename operation)
    pub async fn update_name_and_flags(&self, guid: u32, name: &str, flags: u32) -> Result<()> {
        sqlx::query("UPDATE characters SET name = ?, character_flags = ? WHERE guid = ?")
            .bind(name)
            .bind(flags)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update character name and flags")?;

        Ok(())
    }

    /// Update character experience and level
    pub async fn update_experience(&self, guid: u32, xp: u32, level: u8) -> Result<()> {
        sqlx::query("UPDATE characters SET xp = ?, level = ? WHERE guid = ?")
            .bind(xp)
            .bind(level)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update character experience")?;

        Ok(())
    }

    /// Update character health and power values
    pub async fn update_health_and_power(
        &self,
        guid: u32,
        health: u32,
        power1: u32,
        power2: u32,
        power3: u32,
        power4: u32,
        power5: u32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE characters SET health = ?, power1 = ?, power2 = ?, power3 = ?, power4 = ?, power5 = ? WHERE guid = ?",
        )
        .bind(health)
        .bind(power1)
        .bind(power2)
        .bind(power3)
        .bind(power4)
        .bind(power5)
        .bind(guid)
        .execute(&*self.pool)
        .await
        .context("Failed to update character health and power")?;

        Ok(())
    }

    /// Update character rest data (rest bonus, logout time, character flags)
    pub async fn update_rest_data(
        &self,
        guid: u32,
        rest_bonus: f32,
        logout_time: u64,
        character_flags: u32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE characters SET rest_bonus = ?, logout_time = ?, character_flags = ? WHERE guid = ?",
        )
        .bind(rest_bonus)
        .bind(logout_time)
        .bind(character_flags)
        .bind(guid)
        .execute(&*self.pool)
        .await
        .context("Failed to update character rest data")?;

        Ok(())
    }

    /// Save or update quest status for a character
    pub async fn save_quest_status(
        &self,
        guid: u32,
        quest_id: u32,
        status: u8,
        rewarded: bool,
        explored: bool,
        timer: u32,
        mob_count1: u32,
        mob_count2: u32,
        mob_count3: u32,
        mob_count4: u32,
        item_count1: u32,
        item_count2: u32,
        item_count3: u32,
        item_count4: u32,
        reward_choice: u32,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO character_queststatus
               (guid, quest, status, rewarded, explored, timer,
                mob_count1, mob_count2, mob_count3, mob_count4,
                item_count1, item_count2, item_count3, item_count4,
                reward_choice)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON DUPLICATE KEY UPDATE
                   status = VALUES(status),
                   rewarded = VALUES(rewarded),
                   explored = VALUES(explored),
                   timer = VALUES(timer),
                   mob_count1 = VALUES(mob_count1),
                   mob_count2 = VALUES(mob_count2),
                   mob_count3 = VALUES(mob_count3),
                   mob_count4 = VALUES(mob_count4),
                   item_count1 = VALUES(item_count1),
                   item_count2 = VALUES(item_count2),
                   item_count3 = VALUES(item_count3),
                   item_count4 = VALUES(item_count4),
                   reward_choice = VALUES(reward_choice)"#,
        )
        .bind(guid)
        .bind(quest_id)
        .bind(status)
        .bind(if rewarded { 1u8 } else { 0u8 })
        .bind(if explored { 1u8 } else { 0u8 })
        .bind(timer)
        .bind(mob_count1)
        .bind(mob_count2)
        .bind(mob_count3)
        .bind(mob_count4)
        .bind(item_count1)
        .bind(item_count2)
        .bind(item_count3)
        .bind(item_count4)
        .bind(reward_choice)
        .execute(&*self.pool)
        .await
        .context("Failed to save quest status")?;

        Ok(())
    }

    /// Delete a character (and all related data).
    pub async fn delete(&self, guid: u32) -> Result<()> {
        // Note: This should delete from all character-related tables.
        // In production, this would be part of a larger transaction that
        // includes character_spell, character_aura, character_inventory, etc.
        let mut tx = self.pool.begin().await?;

        // Delete from all related tables
        let tables = vec![
            "character_spell",
            "character_aura",
            "character_inventory",
            "character_skills",
            "character_reputation",
            "character_action",
            "character_queststatus",
            "character_spell_cooldown",
            "character_homebind",
            "character_social",
        ];

        for table in tables {
            sqlx::query(&format!("DELETE FROM {} WHERE guid = ?", table))
                .bind(guid)
                .execute(&mut *tx)
                .await
                .context(format!("Failed to delete from {}", table))?;
        }

        // Finally delete the character itself
        sqlx::query("DELETE FROM characters WHERE guid = ?")
            .bind(guid)
            .execute(&mut *tx)
            .await
            .context("Failed to delete character")?;

        tx.commit()
            .await
            .context("Failed to commit character deletion")?;
        Ok(())
    }
}

/// Repository trait for character data access
/// Enables mocking and testing for systems
#[async_trait]
pub trait CharacterRepositoryTrait: Send + Sync {
    async fn find_by_guid(&self, guid: u32) -> Result<Option<CharacterRow>>;
    async fn find_by_account(&self, account_id: u32) -> Result<Vec<CharacterRow>>;
    async fn find_by_name(&self, name: &str) -> Result<Option<CharacterRow>>;
    async fn exists_by_name(&self, name: &str) -> Result<bool>;
    async fn find_empty_inventory_slot(&self, guid: u32) -> Result<Option<u8>>;
    async fn add_item_to_inventory(
        &self,
        guid: u32,
        bag: u32,
        slot: u8,
        item_guid: u32,
        item_id: u32,
    ) -> Result<()>;
    async fn create_item_instance(
        &self,
        guid: u32,
        item_id: u32,
        owner_guid: u32,
        creator_guid: u32,
        count: u32,
    ) -> Result<()>;
}

/// Trait implementation for existing CharacterRepository
#[async_trait]
impl CharacterRepositoryTrait for CharacterRepository {
    async fn find_by_guid(&self, guid: u32) -> Result<Option<CharacterRow>> {
        self.find_by_guid(guid).await
    }

    async fn find_by_account(&self, account_id: u32) -> Result<Vec<CharacterRow>> {
        self.find_by_account(account_id).await
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<CharacterRow>> {
        self.find_by_name(name).await
    }

    async fn exists_by_name(&self, name: &str) -> Result<bool> {
        self.exists_by_name(name).await
    }

    async fn find_empty_inventory_slot(&self, guid: u32) -> Result<Option<u8>> {
        self.find_empty_inventory_slot(guid).await
    }

    async fn add_item_to_inventory(
        &self,
        guid: u32,
        bag: u32,
        slot: u8,
        item_guid: u32,
        item_id: u32,
    ) -> Result<()> {
        self.add_item_to_inventory(guid, bag, slot, item_guid, item_id)
            .await
    }

    async fn create_item_instance(
        &self,
        guid: u32,
        item_id: u32,
        owner_guid: u32,
        creator_guid: u32,
        count: u32,
    ) -> Result<()> {
        self.create_item_instance(guid, item_id, owner_guid, creator_guid, count)
            .await
    }
}
