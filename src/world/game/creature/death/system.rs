use crate::shared::messages::update::{SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock, ObjectType};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};
use crate::world::game::common::update_fields::*;
use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
use crate::world::World;
use crate::world::game::broadcast_mgr::broadcast_around_creature;

/// Default corpse decay times by creature type
pub const CORPSE_DECAY_NORMAL: u32 = 60_000;    // 60 seconds
pub const CORPSE_DECAY_RARE: u32 = 300_000;     // 5 minutes
pub const CORPSE_DECAY_ELITE: u32 = 300_000;    // 5 minutes
pub const CORPSE_DECAY_BOSS: u32 = 3600_000;    // 1 hour

// Unit flags (UNIT_FIELD_FLAGS)
pub const UNIT_FLAG_NOT_SELECTABLE: u32 = 0x02000000;
pub const UNIT_FLAG_NOT_ATTACKABLE_1: u32 = 0x00000080;  // Matches MaNGOS - prevents attack selection
pub const UNIT_FLAG_IMMUNE_TO_PLAYER: u32 = 0x00000200;  // Actually 0x100, but we use 0x80 instead

// Dynamic flags (UNIT_DYNAMIC_FLAGS)
pub const UNIT_DYNFLAG_NONE: u32 = 0x0000;
pub const UNIT_DYNFLAG_LOOTABLE: u32 = 0x0001;         // Has loot, show sparkle
pub const UNIT_DYNFLAG_TRACK_UNIT: u32 = 0x0002;       // Show on minimap
pub const UNIT_DYNFLAG_TAPPED: u32 = 0x0004;           // Tapped by someone else (gray nameplate)
pub const UNIT_DYNFLAG_TAPPED_BY_PLAYER: u32 = 0x0008; // Tapped by you (normal nameplate)
pub const UNIT_DYNFLAG_SPECIALINFO: u32 = 0x0010;      // Shows ?
pub const UNIT_DYNFLAG_DEAD: u32 = 0x0020;             // Plays death animation
pub const UNIT_DYNFLAG_TAPPED_BY_ALL_THREAT_LIST: u32 = 0x0080; // Open tap (Phase 7 raid)

/// Process death updates for all creatures
pub async fn process_deaths(world: &World) -> anyhow::Result<()> {
    // 1. Process creatures that just died
    let just_died = world.managers.creature_mgr.get_just_died_creatures();

    // Only log if there are actually creatures to process
    if !just_died.is_empty() {
        tracing::info!("[DEATH] process_deaths: found {} creatures in JustDied state", just_died.len());
    }

    for guid in just_died {
        tracing::info!("[DEATH] process_deaths: processing death for {:?}", guid);
        process_creature_death(world, guid).await?;
    }

    Ok(())
}

/// Process a single creature death
async fn process_creature_death(world: &World, guid: ObjectGuid) -> anyhow::Result<()> {
    tracing::info!("[DEATH] process_creature_death ENTRY: guid={:?}", guid);

    // Get creature info
    let info = world.managers.creature_mgr.with_creature_mut(guid, |creature| {
        tracing::info!("[DEATH] process_creature_death: death_state={:?}, loot_recipient={:?}, has_loot={}",
            creature.death_state, creature.loot_recipient, creature.has_loot);

        // If loot already generated (by killing blow handler), skip this
        if creature.has_loot {
            return None;
        }

        Some((creature.position, creature.entry, creature.loot_recipient, creature.map_id, creature.instance_id))
    });

    let Some(Some((position, entry, loot_recipient, map_id, instance_id))) = info else {
        if info.is_none() {
            tracing::warn!("[DEATH] process_creature_death: creature {:?} not found!", guid);
        } else {
            tracing::info!("[DEATH] process_creature_death: {:?} already has loot, skipping", guid);
        }
        return Ok(());
    };

    tracing::info!("[DEATH] Creature {:?} (entry {}) died", guid, entry);

    // NOTE: The killing blow handler (creature_combat.rs) already sent the death fields
    // (health=0, DYNFLAG_DEAD, stand state Dead, NPC flags cleared) in a single VALUES update.
    // We only need to send the loot/tapped flags below.

    // Determine decay time based on creature rank (from template)
    // Phase 3: Normal/Rare/Elite/Boss, Phase 5 TODO: Get from CreatureTemplate
    let decay_time = CORPSE_DECAY_NORMAL;

    // Get spawn data including flags
    let spawn_data = world.managers.creature_mgr
        .get_spawn_data(guid)
        .ok_or_else(|| anyhow::anyhow!("Spawn data not found"))?;

    // Count nearby players for dynamic respawn (within 100 yards)
    let map = world.managers.map_mgr.get_or_create_map(map_id, instance_id);
    let nearby_players = map
        .get_objects_in_range(spawn_data.position, 100.0)
        .into_iter()
        .filter(|g| g.is_player())
        .count() as u32;

    // Calculate respawn time with flags and population
    let respawn_time_secs = world.managers.creature_mgr
        .with_creature_mut(guid, |creature| {
            creature.calculate_respawn_time_with_flags(
                spawn_data.spawntimesecs,
                spawn_data.spawn_flags,
                nearby_players,
            )
        })
        .map(|time_ms| (time_ms / 1000) as u32)
        .unwrap_or(spawn_data.spawntimesecs); // Fallback to base time

    tracing::debug!(
        "[RESPAWN] Creature {:?} will respawn in {} seconds (base: {}, nearby players: {})",
        guid,
        respawn_time_secs,
        spawn_data.spawntimesecs,
        nearby_players
    );

    // Set respawn timer
    world.managers.creature_mgr.with_creature_mut(guid, |creature| {
        creature.set_respawn_timer(respawn_time_secs);
    });

    // Transition to corpse state
    world.managers.creature_mgr.set_corpse_state(guid, decay_time);

    // Give quest kill credit to the killer
    if let Some(recipient_guid) = loot_recipient {
        world.systems.quest.handle_kill_credit(recipient_guid, entry, guid);
    }

    // Grant XP to the loot recipient
    if let Some(recipient_guid) = loot_recipient {
        // Get creature level from template (use max_level as representative level)
        if let Some(creature_level) = world.managers.creature_mgr.get_template(entry)
            .map(|t| t.max_level)
        {
            let player_level = world.managers.player_mgr
                .get_player(recipient_guid)
                .map(|p| p.level)
                .unwrap_or(1);

            // Elite status not stored on creature instance; use false for now
            let is_elite = false;

            let xp = crate::world::game::player::experience::calculate_creature_xp(
                creature_level,
                player_level,
                is_elite,
            );

            if xp > 0 {
                use crate::shared::game::experience::XpSource;
                let _ = world.systems.experience.add_xp(
                    recipient_guid,
                    xp,
                    XpSource::Kill,
                    Some(guid),
                    0.0,
                ).await;
            }
        }
    }

    // Generate loot and mark as lootable if has loot recipient
    if let Some(recipient_guid) = loot_recipient {
        tracing::info!("[DEATH] process_creature_death {:?}: has loot recipient {:?}, generating loot", guid, recipient_guid);

        // Generate loot for the creature
        world.systems.loot.generate_creature_loot_on_death(guid, world).await?;
        tracing::info!("[DEATH] process_creature_death {:?}: loot generated", guid);

        // Mark creature as having loot so CREATE_OBJECT2 will include LOOTABLE flag
        world.managers.creature_mgr.with_creature_mut(guid, |creature| {
            creature.set_has_loot(true);
        });

        // Send LOOTABLE dynamic flag update (vmangos: only DYNFLAG_LOOTABLE, no DYNFLAG_DEAD)
        // DYNFLAG_DEAD is feign-death only. Client recognizes real death via health=0 + BYTES_1=7.
        let flags = UNIT_DYNFLAG_LOOTABLE;
        tracing::info!("[DEATH] process_creature_death {:?}: sending lootable update, dynflags=0x{:04X} (LOOTABLE)", guid, flags);
        send_complete_loot_update(world, guid, flags);

        // Phase 7 TODO: Check if recipient is in group
    } else {
        // No loot recipient — just clear dynamic flags (no DYNFLAG_DEAD, no LOOTABLE)
        tracing::info!("[DEATH] process_creature_death {:?}: no loot recipient, clearing dynflags", guid);
        send_dynamic_flags_update(world, guid, UNIT_DYNFLAG_NONE);
    }

    Ok(())
}

/// Process corpse decay
pub async fn process_corpse_decay(world: &World, diff_ms: u32) -> anyhow::Result<()> {
    // Update all corpse timers
    world.managers.creature_mgr.update_corpse_timers(diff_ms);

    // Get expired corpses
    let expired = world.managers.creature_mgr.get_expired_corpses();

    for guid in expired {
        remove_corpse(world, guid).await?;
    }

    Ok(())
}

/// Remove corpse from world
async fn remove_corpse(world: &World, guid: ObjectGuid) -> anyhow::Result<()> {
    tracing::debug!("[DEATH] Removing corpse for {:?}", guid);

    // 1. Get position, map_id, and instance_id before state change
    let (position, map_id, instance_id) = world.managers.creature_mgr
        .with_creature_mut(guid, |c| (c.position, c.map_id, c.instance_id))
        .ok_or_else(|| anyhow::anyhow!("Creature not found"))?;

    // 2. Remove from visibility system - remove from all players' visible sets
    // Get all online players
    let all_players = world.session_mgr.get_all_sessions();
    for player_guid in all_players {
        world.systems.visibility.remove_visible(player_guid, guid);
    }

    // 3. Remove from map grid
    let map = world.managers.map_mgr.get_or_create_map(map_id, instance_id);
    map.remove_creature(guid, position);

    // 4. Send SMSG_DESTROYOBJECT to nearby players
    send_destroy_object(world, guid, position);

    // 5. Clean up loot data
    world.systems.loot.remove_loot(guid);

    // 6. Transition to Dead state + mark not in world
    world.managers.creature_mgr.with_creature_mut(guid, |creature| {
        creature.remove_corpse();
        creature.in_world = false; // Phase 1: Mark as not spawned
    });

    Ok(())
}

/// Send death animation packet (health=0 + UNIT_DYNFLAG_DEAD)
fn send_death_packet(world: &World, guid: ObjectGuid) {
    let world_guid = WorldObjectGuid::new_creature(guid.entry(), guid.counter());
    let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
        ValuesUpdateBlock::new(world_guid, ObjectType::Unit)
            .set_field(UNIT_FIELD_HEALTH, 0u32)
            .set_field(UNIT_DYNAMIC_FLAGS, UNIT_DYNFLAG_DEAD)
            .set_field(UNIT_FIELD_BYTES_1, 7u32) // Stand state Dead = 7 (UNIT_STAND_STATE_DEAD)
            .set_field(UNIT_NPC_FLAGS, 0u32) // Clear NPC interaction flags on death
    ));

    broadcast_around_creature(world, guid, &update.to_world_packet());
}

/// Send complete death+loot update with ALL fields (health, flags, dynamic flags, etc.)
/// This ensures the client sees the complete corpse state in one atomic update
/// Send UNIT_DYNFLAG_LOOTABLE update after loot generation.
/// vmangos: SetFlag(UNIT_DYNAMIC_FLAGS, UNIT_DYNFLAG_LOOTABLE) — just the flag, nothing else.
fn send_complete_loot_update(world: &World, guid: ObjectGuid, dynamic_flags: u32) {
    let world_guid = WorldObjectGuid::new_creature(guid.entry(), guid.counter());
    let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
        ValuesUpdateBlock::new(world_guid, ObjectType::Unit)
            .set_field(UNIT_DYNAMIC_FLAGS, dynamic_flags) // LOOTABLE only
    ));

    tracing::info!("[DEATH] Sending lootable update: dynflags=0x{:04X}", dynamic_flags);
    broadcast_around_creature(world, guid, &update.to_world_packet());
}

/// Update dynamic flags only (e.g. clear lootable after all loot taken)
pub fn send_dynamic_flags_update(world: &World, guid: ObjectGuid, flags: u32) {
    let world_guid = WorldObjectGuid::new_creature(guid.entry(), guid.counter());
    let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
        ValuesUpdateBlock::new(world_guid, ObjectType::Unit)
            .set_field(UNIT_DYNAMIC_FLAGS, flags)
    ));

    broadcast_around_creature(world, guid, &update.to_world_packet());
}

/// Send destroy object packet
fn send_destroy_object(world: &World, guid: ObjectGuid, _position: crate::shared::protocol::Position) {
    let world_guid = WorldObjectGuid::new_creature(guid.entry(), guid.counter());
    let mut packet = WorldPacket::new(Opcode::SMSG_DESTROY_OBJECT);
    packet.write_guid_raw(world_guid.raw());

    broadcast_around_creature(world, guid, &packet);
}
