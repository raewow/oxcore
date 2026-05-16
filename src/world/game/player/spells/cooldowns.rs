//! Spell Cooldown Management
//!
//! Handles per-spell cooldowns, category cooldowns, GCD, and persistence.

use crate::shared::messages::spells::{SmsgClearCooldown, SmsgSpellCooldown};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::ObjectGuid;
use crate::world::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::world::World;
use anyhow::Result;
use std::sync::Arc;

/// Get current game time in milliseconds
fn get_game_time_ms(world: &World) -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Apply a cooldown for a spell after casting.
///
/// Cooldown sources:
/// 1. Per-spell cooldown (from Spell.dbc RecoveryTime field)
/// 2. Category cooldown (from Spell.dbc CategoryRecoveryTime field)
///    - Spells in the same category share a cooldown
///    - Example: Health Potion and Mana Potion share the "Potion" category
/// 3. GCD is handled separately in SpellSystem
#[allow(dead_code)]
pub fn apply_cooldown(caster_guid: ObjectGuid, spell_id: u32, world: &World) -> Result<()> {
    let now = get_game_time_ms(world);

    // Read cooldown values from spell entry
    let (spell_cooldown_ms, category_cooldown_ms, category_id) =
        match world.managers.spell_mgr.get(spell_id) {
            Some(entry) => (
                entry.recovery_time,
                entry.category_recovery_time,
                entry.category,
            ),
            None => (0, 0, 0),
        };

    world
        .systems
        .player
        .manager()
        .with_player_mut(caster_guid, |player| {
            // Set per-spell cooldown
            if spell_cooldown_ms > 0 {
                player.spells.add_cooldown(spell_id, spell_cooldown_ms, now);
            }

            // Set category cooldown
            if category_cooldown_ms > 0 && category_id > 0 {
                player
                    .spells
                    .category_cooldowns
                    .insert(category_id, now + category_cooldown_ms as u64);
            }
        });

    Ok(())
}

/// Apply a cooldown with specific duration (for spell modifiers that change cooldown)
pub fn apply_cooldown_with_duration(
    caster_guid: ObjectGuid,
    spell_id: u32,
    duration_ms: u32,
    world: &World,
) -> Result<()> {
    if duration_ms == 0 {
        return Ok(());
    }

    let now = get_game_time_ms(world);

    world
        .systems
        .player
        .manager()
        .with_player_mut(caster_guid, |player| {
            player.spells.add_cooldown(spell_id, duration_ms, now);
        });

    Ok(())
}

/// Check if a spell is on cooldown.
pub fn is_on_cooldown(caster_guid: ObjectGuid, spell_id: u32, world: &World) -> Result<bool> {
    let now = get_game_time_ms(world);
    let mut on_cd = false;

    world
        .systems
        .player
        .manager()
        .with_player_mut(caster_guid, |player| {
            // Check per-spell cooldown
            on_cd = player.spells.is_on_cooldown(spell_id, now);

            // Check category cooldown if not already on cooldown
            if !on_cd {
                if let Some(entry) = world.managers.spell_mgr.get(spell_id) {
                    let category_id = entry.category;
                    if category_id > 0 {
                        if let Some(&cd_end) = player.spells.category_cooldowns.get(&category_id) {
                            if cd_end > now {
                                on_cd = true;
                            }
                        }
                    }
                }
            }
        });

    Ok(on_cd)
}

/// Get remaining cooldown for a spell in milliseconds.
pub fn get_remaining_cooldown(
    caster_guid: ObjectGuid,
    spell_id: u32,
    world: &World,
) -> Result<u32> {
    let now = get_game_time_ms(world);
    let mut remaining = 0u32;

    world
        .systems
        .player
        .manager()
        .with_player_mut(caster_guid, |player| {
            remaining = player.spells.get_cooldown_remaining(spell_id, now);
        });

    Ok(remaining)
}

/// Clear expired cooldowns (housekeeping).
/// Called periodically to clean up the cooldown maps.
pub fn clear_expired_cooldowns(caster_guid: ObjectGuid, world: &World) -> Result<()> {
    let now = get_game_time_ms(world);

    world
        .systems
        .player
        .manager()
        .with_player_mut(caster_guid, |player| {
            player.spells.clear_expired_cooldowns(now);
        });

    Ok(())
}

/// Reset a specific spell's cooldown (from abilities like Cold Snap, Preparation).
pub async fn reset_cooldown(
    caster_guid: ObjectGuid,
    spell_id: u32,
    world: &World,
    broadcast_mgr: &Arc<dyn BroadcastManagerTrait>,
) -> Result<()> {
    world
        .systems
        .player
        .manager()
        .with_player_mut(caster_guid, |player| {
            player.spells.reset_cooldown(spell_id);
        });

    // Send SMSG_CLEAR_COOLDOWN to client
    let msg = SmsgClearCooldown {
        spell_id,
        caster_guid,
    };
    broadcast_mgr.send_msg_to_player(caster_guid, msg.to_world_packet());

    Ok(())
}

/// Reset all cooldowns (e.g., arena start, GM command).
pub async fn reset_all_cooldowns(
    caster_guid: ObjectGuid,
    world: &World,
    broadcast_mgr: &Arc<dyn BroadcastManagerTrait>,
) -> Result<()> {
    let spell_ids: Vec<u32> = world
        .systems
        .player
        .manager()
        .with_player_mut(caster_guid, |player| {
            let ids: Vec<u32> = player.spells.cooldowns.keys().copied().collect();
            player.spells.reset_all_cooldowns();
            ids
        })
        .unwrap_or_default();

    // Send SMSG_CLEAR_COOLDOWN for each spell
    for spell_id in spell_ids {
        let msg = SmsgClearCooldown {
            spell_id,
            caster_guid,
        };
        broadcast_mgr.send_msg_to_player(caster_guid, msg.to_world_packet());
    }

    Ok(())
}

/// Send all active cooldowns to client on login.
pub fn send_cooldowns_on_login(
    player_guid: ObjectGuid,
    world: &World,
    broadcast_mgr: &Arc<dyn BroadcastManagerTrait>,
) -> Result<()> {
    let now = get_game_time_ms(world);
    let mut active_cooldowns: Vec<(u32, u32)> = Vec::new();

    world
        .systems
        .player
        .manager()
        .with_player_mut(player_guid, |player| {
            for (&spell_id, &cd_end) in &player.spells.cooldowns {
                if cd_end > now {
                    let remaining = (cd_end - now) as u32;
                    active_cooldowns.push((spell_id, remaining));
                }
            }
        });

    if !active_cooldowns.is_empty() {
        let msg = SmsgSpellCooldown {
            caster_guid: player_guid,
            cooldowns: active_cooldowns,
        };
        broadcast_mgr.send_msg_to_player(player_guid, msg.to_world_packet());
    }

    Ok(())
}

/// Save cooldowns to database for persistence across logout/login.
#[allow(dead_code)]
pub fn save_cooldowns(_player_guid: ObjectGuid, _world: &World) -> Result<()> {
    // TODO: Implement database persistence
    // Only save cooldowns longer than 30 seconds
    // DELETE FROM character_spell_cooldowns WHERE guid = ?
    // INSERT INTO character_spell_cooldowns (guid, spell_id, remaining_ms) VALUES (?, ?, ?)
    Ok(())
}

/// Load cooldowns from database on login.
#[allow(dead_code)]
pub fn load_cooldowns(_player_guid: ObjectGuid, _world: &World) -> Result<()> {
    // TODO: Implement database loading
    Ok(())
}
