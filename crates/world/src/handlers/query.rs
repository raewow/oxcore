//! Query handlers - creature, item, name queries

use anyhow::Result;
use bytes::Buf;
use std::sync::Arc;

use crate::core::session::WorldSession;
use crate::game::items::manager::ItemTemplate;
use crate::World;
use oxcore_shared::database::{CharacterRepository, Databases};
use oxcore_shared::protocol::WorldPacket;

fn item_spell_query_values(
    template: &ItemTemplate,
    index: usize,
    world: &World,
) -> (u32, u32, i32, u32, u32, u32) {
    let spell_id = template.spell_id[index];
    if spell_id == 0 {
        return (0, 0, 0, u32::MAX, 0, u32::MAX);
    }

    let (spell_cooldown, spell_category, spell_category_cooldown) =
        if template.spell_cooldown[index] >= 0 || template.spell_category_cooldown[index] >= 0 {
            (
                template.spell_cooldown[index] as u32,
                template.spell_category[index],
                template.spell_category_cooldown[index] as u32,
            )
        } else if let Some(spell) = world.managers.spell_mgr.get(spell_id) {
            (
                spell.recovery_time,
                spell.category,
                spell.category_recovery_time,
            )
        } else {
            (0, 0, 0)
        };

    (
        spell_id,
        template.spell_trigger[index],
        template.spell_charges[index].max(0),
        spell_cooldown,
        spell_category,
        spell_category_cooldown,
    )
}

/// Handle CMSG_CREATURE_QUERY
///
/// Client sends this when it needs creature template info (name, type, etc.)
/// to display a creature's nameplate and determine interaction type.
pub async fn handle_creature_query(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    use oxcore_shared::messages::query::SmsgCreatureQueryResponse;

    // Read entry and GUID from packet (Vanilla 1.12.1 format)
    let entry = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read creature entry from CMSG_CREATURE_QUERY"))?;
    let _guid = packet
        .read_u64()
        .ok_or_else(|| anyhow::anyhow!("Failed to read creature GUID from CMSG_CREATURE_QUERY"))?;

    tracing::debug!("CMSG_CREATURE_QUERY: entry={}", entry);

    // Look up template in CreatureManager
    if let Some(template) = world.managers.creature_mgr.get_template(entry) {
        let response = SmsgCreatureQueryResponse::new(
            entry,
            &template.name,
            template.subname.as_deref().unwrap_or(""),
            0, // type_flags not stored in oxcore DB schema
            template.creature_type,
            template.get_display_id(), // Use first non-zero display_id
        );

        tracing::debug!(
            "SMSG_CREATURE_QUERY_RESPONSE: entry={}, name='{}'",
            entry,
            template.name
        );

        session.send_msg(response)?;
    } else {
        // Send not-found response
        tracing::warn!(
            "CMSG_CREATURE_QUERY: template not found for entry={}",
            entry
        );
        let response = SmsgCreatureQueryResponse::not_found(entry);
        session.send_msg(response)?;
    }

    Ok(())
}

/// Handle CMSG_ITEM_QUERY_SINGLE
pub async fn handle_item_query(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    use oxcore_shared::messages::inventory::SmsgItemQuerySingleResponse;
    use oxcore_shared::messages::ToWorldPacket;

    // Read item entry (u32) and skip GUID (8 bytes)
    let item_entry = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read item entry from CMSG_ITEM_QUERY_SINGLE"))?;

    // Skip GUID (8 bytes) - client may or may not send this field
    // Only skip if we have enough bytes remaining to prevent packet desync
    if packet.data().remaining() >= 8 {
        packet.read_skip(8)?;
    }

    tracing::debug!("CMSG_ITEM_QUERY_SINGLE: entry={}", item_entry);

    // Look up item template
    if let Some(template) = world.managers.item_mgr.get_template(item_entry) {
        // The client builds on-use tooltip text from the item spell records, so preserve
        // every loaded template spell field in this 1.12 response.

        let spell1 = item_spell_query_values(&template, 0, world);
        let spell2 = item_spell_query_values(&template, 1, world);
        let spell3 = item_spell_query_values(&template, 2, world);
        let spell4 = item_spell_query_values(&template, 3, world);
        let spell5 = item_spell_query_values(&template, 4, world);

        let response = SmsgItemQuerySingleResponse {
            entry: template.entry,
            class: template.item_class,
            subclass: template.item_subclass,
            name_0: template.name.clone(),
            name_1: String::new(),
            name_2: String::new(),
            name_3: String::new(),
            display_id: template.display_id,
            quality: template.quality as u32,
            flags: 0, // TODO: Load from expanded template
            buy_price: template.buy_price,
            sell_price: template.sell_price,
            inventory_type: template.inventory_type as u32,
            allowable_class: -1, // -1 = all classes
            allowable_race: -1,  // -1 = all races
            item_level: template.item_level,
            required_level: template.required_level,
            required_skill: 0,
            required_skill_rank: 0,
            required_spell: 0,
            required_honor_rank: 0,
            required_city_rank: 0,
            required_reputation_faction: 0,
            required_reputation_rank: 0,
            max_count: template.max_count,
            stackable: template.stackable,
            container_slots: template.container_slots as u32,

            stat_type1: template.stat_type[0] as i32,
            stat_value1: template.stat_value[0] as i32,
            stat_type2: template.stat_type[1] as i32,
            stat_value2: template.stat_value[1] as i32,
            stat_type3: template.stat_type[2] as i32,
            stat_value3: template.stat_value[2] as i32,
            stat_type4: template.stat_type[3] as i32,
            stat_value4: template.stat_value[3] as i32,
            stat_type5: template.stat_type[4] as i32,
            stat_value5: template.stat_value[4] as i32,
            stat_type6: template.stat_type[5] as i32,
            stat_value6: template.stat_value[5] as i32,
            stat_type7: template.stat_type[6] as i32,
            stat_value7: template.stat_value[6] as i32,
            stat_type8: template.stat_type[7] as i32,
            stat_value8: template.stat_value[7] as i32,
            stat_type9: template.stat_type[8] as i32,
            stat_value9: template.stat_value[8] as i32,
            stat_type10: template.stat_type[9] as i32,
            stat_value10: template.stat_value[9] as i32,

            damage_min1: template.dmg_min[0],
            damage_max1: template.dmg_max[0],
            damage_type1: template.dmg_type[0] as u32,
            damage_min2: template.dmg_min[1],
            damage_max2: template.dmg_max[1],
            damage_type2: template.dmg_type[1] as u32,
            damage_min3: template.dmg_min[2],
            damage_max3: template.dmg_max[2],
            damage_type3: template.dmg_type[2] as u32,
            damage_min4: template.dmg_min[3],
            damage_max4: template.dmg_max[3],
            damage_type4: template.dmg_type[3] as u32,
            damage_min5: template.dmg_min[4],
            damage_max5: template.dmg_max[4],
            damage_type5: template.dmg_type[4] as u32,

            armor: template.armor.max(0) as u32,
            holy_res: template.holy_res.max(0) as u32,
            fire_res: template.fire_res.max(0) as u32,
            nature_res: template.nature_res.max(0) as u32,
            frost_res: template.frost_res.max(0) as u32,
            shadow_res: template.shadow_res.max(0) as u32,
            arcane_res: template.arcane_res.max(0) as u32,
            delay: template.delay as u32,
            ammo_type: template.ammo_type as u32,
            ranged_mod_range: 0.0,

            spell_id1: spell1.0,
            spell_trigger1: spell1.1,
            spell_charges1: spell1.2,
            spell_cooldown1: spell1.3,
            spell_category1: spell1.4,
            spell_category_cooldown1: spell1.5,
            spell_id2: spell2.0,
            spell_trigger2: spell2.1,
            spell_charges2: spell2.2,
            spell_cooldown2: spell2.3,
            spell_category2: spell2.4,
            spell_category_cooldown2: spell2.5,
            spell_id3: spell3.0,
            spell_trigger3: spell3.1,
            spell_charges3: spell3.2,
            spell_cooldown3: spell3.3,
            spell_category3: spell3.4,
            spell_category_cooldown3: spell3.5,
            spell_id4: spell4.0,
            spell_trigger4: spell4.1,
            spell_charges4: spell4.2,
            spell_cooldown4: spell4.3,
            spell_category4: spell4.4,
            spell_category_cooldown4: spell4.5,
            spell_id5: spell5.0,
            spell_trigger5: spell5.1,
            spell_charges5: spell5.2,
            spell_cooldown5: spell5.3,
            spell_category5: spell5.4,
            spell_category_cooldown5: spell5.5,

            // Misc fields
            bonding: 0,
            description: String::new(),
            page_text_id: 0,
            language_id: 0,
            page_material: 0,
            start_quest: template.start_quest,
            lock_id: 0,
            material: 0,
            sheath: 0,
            random_property: 0,
            random_suffix: 0,
            block: 0,
            item_set: 0,
            max_durability: template.max_durability,
            area: 0,
            map: 0,
            bag_family: 0,

            // TBC+ fields (not needed for vanilla 1.12.1 but included in struct)
            totem_category: 0,
            socket_color1: 0,
            socket_color2: 0,
            socket_color3: 0,
            socket_bonus: 0,
            gem_properties: 0,
            required_disenchant_skill: 0,
            armor_damage_modifier: 0.0,
            duration: 0,
            item_limit_id: 0,
            item_limit_category: 0,
            quality2: 0,
        };

        tracing::info!(
            "SMSG_ITEM_QUERY_SINGLE_RESPONSE: entry={}, name='{}', spells={:?}",
            item_entry,
            template.name,
            template.spell_id
        );

        session.send_msg(response)?;
    } else {
        tracing::warn!(
            "Item template not found for entry={} - sending placeholder response",
            item_entry
        );

        // Send minimal placeholder response to prevent client crash
        // The client expects a response for every query, even if the item doesn't exist
        let placeholder_response = SmsgItemQuerySingleResponse {
            entry: item_entry,
            class: 15, // Miscellaneous
            subclass: 0,
            name_0: "Unknown Item".to_string(),
            name_1: String::new(),
            name_2: String::new(),
            name_3: String::new(),
            display_id: 0,
            quality: 0, // Poor (gray)
            flags: 0,
            buy_price: 0,
            sell_price: 0,
            inventory_type: 0,
            allowable_class: -1, // All classes
            allowable_race: -1,  // All races
            item_level: 1,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_spell: 0,
            required_honor_rank: 0,
            required_city_rank: 0,
            required_reputation_faction: 0,
            required_reputation_rank: 0,
            max_count: 0,
            stackable: 1,
            container_slots: 0,

            // Stats (10 slots) - all zero
            stat_type1: 0,
            stat_value1: 0,
            stat_type2: 0,
            stat_value2: 0,
            stat_type3: 0,
            stat_value3: 0,
            stat_type4: 0,
            stat_value4: 0,
            stat_type5: 0,
            stat_value5: 0,
            stat_type6: 0,
            stat_value6: 0,
            stat_type7: 0,
            stat_value7: 0,
            stat_type8: 0,
            stat_value8: 0,
            stat_type9: 0,
            stat_value9: 0,
            stat_type10: 0,
            stat_value10: 0,

            // Damage (5 slots) - all zero
            damage_min1: 0.0,
            damage_max1: 0.0,
            damage_type1: 0,
            damage_min2: 0.0,
            damage_max2: 0.0,
            damage_type2: 0,
            damage_min3: 0.0,
            damage_max3: 0.0,
            damage_type3: 0,
            damage_min4: 0.0,
            damage_max4: 0.0,
            damage_type4: 0,
            damage_min5: 0.0,
            damage_max5: 0.0,
            damage_type5: 0,

            // Resistances - all zero
            armor: 0,
            holy_res: 0,
            fire_res: 0,
            nature_res: 0,
            frost_res: 0,
            shadow_res: 0,
            arcane_res: 0,
            delay: 0,
            ammo_type: 0,
            ranged_mod_range: 0.0,

            // Spells (5 slots) - all zero
            spell_id1: 0,
            spell_trigger1: 0,
            spell_charges1: 0,
            spell_cooldown1: 0,
            spell_category1: 0,
            spell_category_cooldown1: 0,
            spell_id2: 0,
            spell_trigger2: 0,
            spell_charges2: 0,
            spell_cooldown2: 0,
            spell_category2: 0,
            spell_category_cooldown2: 0,
            spell_id3: 0,
            spell_trigger3: 0,
            spell_charges3: 0,
            spell_cooldown3: 0,
            spell_category3: 0,
            spell_category_cooldown3: 0,
            spell_id4: 0,
            spell_trigger4: 0,
            spell_charges4: 0,
            spell_cooldown4: 0,
            spell_category4: 0,
            spell_category_cooldown4: 0,
            spell_id5: 0,
            spell_trigger5: 0,
            spell_charges5: 0,
            spell_cooldown5: 0,
            spell_category5: 0,
            spell_category_cooldown5: 0,

            // Misc fields
            bonding: 0,
            description: String::new(),
            page_text_id: 0,
            language_id: 0,
            page_material: 0,
            start_quest: 0,
            lock_id: 0,
            material: 0,
            sheath: 0,
            random_property: 0,
            random_suffix: 0,
            block: 0,
            item_set: 0,
            max_durability: 0,
            area: 0,
            map: 0,
            bag_family: 0,

            // TBC+ fields (not needed for vanilla 1.12.1 but included in struct)
            totem_category: 0,
            socket_color1: 0,
            socket_color2: 0,
            socket_color3: 0,
            socket_bonus: 0,
            gem_properties: 0,
            required_disenchant_skill: 0,
            armor_damage_modifier: 0.0,
            duration: 0,
            item_limit_id: 0,
            item_limit_category: 0,
            quality2: 0,
        };

        session.send_msg(placeholder_response)?;
    }

    Ok(())
}

/// Handle CMSG_ITEM_NAME_QUERY
pub async fn handle_item_name_query(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    use oxcore_shared::messages::inventory::SmsgItemNameQueryResponse;

    let item_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read item ID from CMSG_ITEM_NAME_QUERY"))?;

    // MaNGOS reads an item GUID after the item ID. Some clients/proxies omit it,
    // so consume it only when present to keep the handler tolerant like item query.
    let _guid = if packet.data().remaining() >= 8 {
        packet.read_u64()
    } else {
        None
    };

    tracing::debug!("CMSG_ITEM_NAME_QUERY: item_id={}", item_id);

    if let Some(template) = world.managers.item_mgr.get_template(item_id) {
        session.send_msg(SmsgItemNameQueryResponse {
            item_id,
            name: &template.name,
        })?;
    } else {
        tracing::warn!(
            "CMSG_ITEM_NAME_QUERY: item template not found for item_id={}",
            item_id
        );
    }

    Ok(())
}

/// Handle CMSG_NAME_QUERY
///
/// Looks up player name, race, gender, class by GUID.
/// First checks online players, then falls back to database for offline players.
/// Also handles creature/unit GUIDs for NPC name lookups.
pub async fn handle_name_query(
    session: &WorldSession,
    packet: &mut WorldPacket,
    databases: &Databases,
    world: &World,
) -> Result<()> {
    use oxcore_shared::messages::query::SmsgNameQueryResponse;
    use oxcore_shared::protocol::ObjectGuid;

    // Read GUID from packet (u64 little-endian)
    let guid_raw = packet
        .read_u64()
        .ok_or_else(|| anyhow::anyhow!("Failed to read GUID from CMSG_NAME_QUERY"))?;
    let guid = ObjectGuid::from_raw(guid_raw);

    tracing::debug!(
        "CMSG_NAME_QUERY: Received query for GUID {:?} (raw=0x{:016X})",
        guid,
        guid_raw
    );

    // 1. Handle creature/unit GUIDs (NPCs, pets, etc.)
    if guid.is_creature() || guid.is_pet() {
        // Get creature entry from GUID and look up template
        let entry = guid.entry();
        if let Some(template) = world.managers.creature_mgr.get_template(entry) {
            // Creatures use race=0, gender=0, class=1 (warrior) as defaults
            let response = SmsgNameQueryResponse::new(
                guid,
                &template.name,
                0, // race (creatures don't have player races)
                0, // gender (0 = male)
                1, // class (1 = warrior, default for creatures)
            );

            tracing::debug!(
                "SMSG_NAME_QUERY_RESPONSE: Sending name '{}' for creature entry {} GUID {:?}",
                template.name,
                entry,
                guid
            );

            session.send_msg(response)?;
            return Ok(());
        }

        tracing::warn!(
            "CMSG_NAME_QUERY: Creature template not found for entry {} (GUID {:?}, raw=0x{:016X})",
            entry,
            guid,
            guid_raw
        );
        return Ok(());
    }

    // 2. Try online player first (fast path)
    if let Some(player) = world.managers.player_mgr.get_player(guid) {
        let response = SmsgNameQueryResponse::new(
            guid,
            &player.name,
            player.race,
            player.gender,
            player.class,
        );

        tracing::debug!(
            "SMSG_NAME_QUERY_RESPONSE: Sending name '{}' for online player GUID {:?}",
            player.name,
            guid
        );

        session.send_msg(response)?;
        return Ok(());
    }

    // 3. Fall back to database for offline players
    let char_repo = CharacterRepository::new(Arc::new(databases.character.clone()));
    if let Some(character) = char_repo.find_by_guid(guid.counter()).await? {
        let response = SmsgNameQueryResponse::new(
            guid,
            &character.name,
            character.race,
            character.gender,
            character.class,
        );

        tracing::debug!(
            "SMSG_NAME_QUERY_RESPONSE: Sending name '{}' for offline player GUID {:?}",
            character.name,
            guid
        );

        session.send_msg(response)?;
        return Ok(());
    }

    // 4. Player truly not found (deleted character or invalid GUID)
    tracing::warn!(
        "CMSG_NAME_QUERY: Player not found for GUID {:?} (raw=0x{:016X}) - not online and not in database",
        guid,
        guid_raw
    );

    Ok(())
}

/// Handle CMSG_GAMEOBJECT_QUERY
///
/// Client sends this when it needs gameobject template info (name, type, etc.)
/// to display a gameobject's nameplate and determine interaction type.
pub async fn handle_gameobject_query(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    // Read entry and GUID from packet (Vanilla 1.12.1 format)
    let entry = packet.read_u32().ok_or_else(|| {
        anyhow::anyhow!("Failed to read gameobject entry from CMSG_GAMEOBJECT_QUERY")
    })?;
    let _guid = packet.read_u64().ok_or_else(|| {
        anyhow::anyhow!("Failed to read gameobject GUID from CMSG_GAMEOBJECT_QUERY")
    })?;

    tracing::debug!("CMSG_GAMEOBJECT_QUERY: entry={}", entry);

    if let Some(query_packet) = world
        .managers
        .gameobject_mgr
        .build_gameobject_query_packet(entry)
    {
        session.send_packet(query_packet)?;
    } else {
        tracing::warn!(
            "CMSG_GAMEOBJECT_QUERY: template not found for entry={}",
            entry
        );
        // Send not-found response (entry | 0x80000000 signals "not found" to client)
        let mut response = oxcore_shared::protocol::WorldPacket::new(
            oxcore_shared::protocol::Opcode::SMSG_GAMEOBJECT_QUERY_RESPONSE,
        );
        response.write_u32(entry | 0x80000000);
        session.send_packet(response)?;
    }

    Ok(())
}

/// Handle CMSG_SET_SELECTION
pub async fn handle_set_selection(
    _session: &mut WorldSession,
    _packet: &mut WorldPacket,
    _world: &World,
) -> Result<()> {
    // TODO: Implement target selection
    // 1. Read target GUID
    // 2. Validate target exists
    // 3. Update player's selection
    Ok(())
}

/// Handle CMSG_QUERY_TIME - client queries server time
/// Packet is empty - responds with SMSG_QUERY_TIME_RESPONSE
/// Note: This doesn't actually set the client's time (SMSG_LOGIN_SETTIMESPEED does that),
/// but it should use the same time format for consistency.
pub async fn handle_query_time(session: &WorldSession) -> Result<()> {
    use oxcore_shared::protocol::Opcode;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Get current time in seconds since Unix epoch (same format as SMSG_LOGIN_SETTIMESPEED)
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("Failed to get system time: {}", e))?
        .as_secs() as u32;

    // Build response packet
    let mut response = WorldPacket::new(Opcode::SMSG_QUERY_TIME_RESPONSE);
    response.write_u32(current_time);

    // Send response
    session.send_packet(response)?;

    Ok(())
}
