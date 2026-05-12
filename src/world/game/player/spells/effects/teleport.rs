//! Teleportation Spell Effects
//!
//! Handles all teleportation, binding, and transportation effects.

use super::{EffectInput, EffectResult};
use crate::shared::protocol::{ObjectGuid, Opcode, Position, WorldPacket};
use crate::world::World;
use anyhow::Result;

/// TARGET_LOCATION_CASTER_HOME_BIND from MaNGOS (Hearthstone / recall)
const TARGET_LOCATION_CASTER_HOME_BIND: u32 = 9;
/// TARGET_LOCATION_DATABASE from MaNGOS (coordinates in spell_target_position table)
const TARGET_LOCATION_DATABASE: u32 = 17;
/// TARGET_ENUM_UNITS_SCRIPT_AOE_AT_SRC_LOC — also uses spell_target_position in this context
const TARGET_ENUM_UNITS_SCRIPT_AOE_AT_SRC_LOC: u32 = 7;

/// SPELL_EFFECT_TELEPORT_UNITS (5)
///
/// Teleport the target to a specific location.
/// Used for Hearthstone, portals, and recall spells.
pub async fn effect_teleport_units(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);

    // Look up spell entry to check implicit target B
    let spell_entry = world.managers.spell_mgr.get(input.spell_id);
    let implicit_target_b = spell_entry
        .map(|e| e.effect_implicit_target_b[input.effect_index as usize])
        .unwrap_or(0);

    // Determine destination
    let (dest_map, dest_pos) = if implicit_target_b == TARGET_LOCATION_CASTER_HOME_BIND {
        // Hearthstone / recall: teleport to homebind
        let homebind = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |p| {
                (
                    p.homebind_map,
                    p.homebind_x,
                    p.homebind_y,
                    p.homebind_z,
                )
            });

        match homebind {
            Some((map, x, y, z)) => {
                tracing::info!(
                    "Teleport to homebind: target={:?} map={} pos=({:.1}, {:.1}, {:.1})",
                    target_guid, map, x, y, z
                );
                (map, Position::new(x, y, z, 0.0))
            }
            None => {
                tracing::warn!("Player {:?} not found for teleport", target_guid);
                return Ok(EffectResult::empty());
            }
        }
    } else if implicit_target_b == TARGET_LOCATION_DATABASE || implicit_target_b == TARGET_ENUM_UNITS_SCRIPT_AOE_AT_SRC_LOC {
        // Portal / teleport spell: coordinates from spell_target_position table
        match world.managers.spell_mgr.get_spell_target_position(input.spell_id) {
            Some(pos) => {
                tracing::info!(
                    "Teleport to DB position: spell={} target={:?} map={} pos=({:.1}, {:.1}, {:.1})",
                    input.spell_id, target_guid, pos.map_id, pos.x, pos.y, pos.z
                );
                (pos.map_id, Position::new(pos.x, pos.y, pos.z, pos.orientation))
            }
            None => {
                tracing::warn!(
                    "No spell_target_position entry for spell {} (TARGET_LOCATION_DATABASE)",
                    input.spell_id
                );
                return Ok(EffectResult::empty());
            }
        }
    } else {
        tracing::debug!(
            "Teleport units: target={:?} spell={} implicit_target_b={} (unhandled)",
            target_guid, input.spell_id, implicit_target_b
        );
        return Ok(EffectResult::empty());
    };

    // Get session for sending packets
    let session = match world.session_mgr.get_session_by_player(target_guid) {
        Some(s) => s,
        None => {
            tracing::warn!("No session found for player {:?}", target_guid);
            return Ok(EffectResult::empty());
        }
    };

    // Always send SMSG_TRANSFER_PENDING — this triggers the client loading screen.
    // MaNGOS sends it unconditionally for all TeleportTo() calls, including same-map teleports.
    let mut transfer_packet = WorldPacket::new(Opcode::SMSG_TRANSFER_PENDING);
    transfer_packet.write_u32(dest_map);
    session.send_packet(transfer_packet)?;

    // Send SMSG_NEW_WORLD with destination
    let mut new_world_packet = WorldPacket::new(Opcode::SMSG_NEW_WORLD);
    new_world_packet.write_u32(dest_map);
    new_world_packet.write_f32(dest_pos.x);
    new_world_packet.write_f32(dest_pos.y);
    new_world_packet.write_f32(dest_pos.z);
    new_world_packet.write_f32(dest_pos.o);
    session.send_packet(new_world_packet)?;

    // Store pending teleport for worldport ACK handler (instance_id 0 for all portal teleports)
    session.set_pending_teleport(Some((dest_map, 0, dest_pos)));

    tracing::info!(
        "Teleport initiated: target={:?} to map={} pos=({:.1}, {:.1}, {:.1})",
        target_guid, dest_map, dest_pos.x, dest_pos.y, dest_pos.z
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_BIND (11)
///
/// Bind the target to a location (Hearthstone).
/// Sets the player's home location.
pub async fn effect_bind(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);

    // Get current position as bind location
    let current_pos = world.systems.player.manager().with_player(target_guid, |player| {
        player.movement.position.clone()
    }).ok_or_else(|| anyhow::anyhow!("Player not found"))?;

    let zone_id = world.systems.player.manager().with_player(target_guid, |player| {
        player.zone_id
    }).unwrap_or(0);

    // TODO: Set home bind in player data

    tracing::debug!(
        "Bind: target={:?} to zone={} pos=({:.1}, {:.1}, {:.1})",
        target_guid, zone_id, current_pos.x, current_pos.y, current_pos.z
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_TELEPORT_UNITS_FACE_CASTER (43)
///
/// Teleport target to caster and make them face the caster.
/// Used for some special teleport effects.
pub async fn effect_teleport_units_face_caster(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let caster_pos = world.systems.player.manager().with_player(input.caster_guid, |player| {
        player.movement.position.clone()
    }).ok_or_else(|| anyhow::anyhow!("Caster not found"))?;

    // TODO: Teleport target to caster's front and face caster

    tracing::debug!(
        "Teleport face caster: target={:?} to caster={:?}",
        target_guid, input.caster_guid
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_STUCK (84)
///
/// Teleport player to their Hearthstone location (unstuck).
/// Emergency teleport for stuck players.
pub async fn effect_stuck(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);

    // TODO: Get home bind location and teleport there

    tracing::debug!(
        "Stuck teleport: target={:?} to home bind",
        target_guid
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_SUMMON_PLAYER (85)
///
/// Summon a player to the caster's location.
/// Used by meeting stones and summon spells.
pub async fn effect_summon_player(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let caster_pos = world.systems.player.manager().with_player(input.caster_guid, |player| {
        player.movement.position.clone()
    }).ok_or_else(|| anyhow::anyhow!("Caster not found"))?;

    // TODO: Teleport target to caster

    tracing::debug!(
        "Summon player: target={:?} to caster={:?}",
        target_guid, input.caster_guid
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_SEND_TAXI (123)
///
/// Send the player on a taxi/flight path.
/// Used for flight masters.
pub async fn effect_send_taxi(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);

    // Get taxi path ID from misc_value
    let taxi_path_id = input.misc_value as u32;

    // TODO: Start taxi flight

    tracing::debug!(
        "Send taxi: target={:?} path_id={}",
        target_guid, taxi_path_id
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_PLAYER_PULL (124)
///
/// Pull the player toward the caster.
/// Opposite of knockback.
pub async fn effect_player_pull(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let pull_distance = input.base_value.max(0) as f32;

    // TODO: Pull target toward caster

    tracing::debug!(
        "Player pull: target={:?} distance={:.1}",
        target_guid, pull_distance
    );

    Ok(EffectResult::empty())
}
