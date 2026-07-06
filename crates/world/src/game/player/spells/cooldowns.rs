//! Spell Cooldown Management
//!
//! Handles per-spell cooldowns, category cooldowns, GCD, and persistence.

use crate::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::World;
use anyhow::Result;
use oxcore_dbc::structures::spell::SpellEntry;
use oxcore_shared::messages::spells::{SmsgClearCooldown, SmsgSpellCooldown};
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::ObjectGuid;
use std::sync::Arc;

/// SPELL_ATTR_COOLDOWN_ON_EVENT — cooldown only starts once the triggering
/// event fires, so the client is told the spell is on cooldown "forever"
/// until then.
const SPELL_ATTR_COOLDOWN_ON_EVENT: u32 = 0x0200_0000;

/// `Spell::SendSpellCooldown` — decide whether a cast should record/announce
/// a cooldown at all, and whether it should be flagged as event-triggered
/// (permanent until the triggering event fires).
///
/// Passive spells never go on cooldown. The C++ also skips this for casters
/// with the "no cooldown" cheat option enabled; that cheat flag has no
/// equivalent in this codebase yet, so it is not modelled here — see the
/// `blocked` note on the port-harness task.
pub fn should_apply_spell_cooldown(spell: &SpellEntry) -> bool {
    !spell.is_passive_spell()
}

/// Whether the cooldown just applied should be reported as event-triggered
/// (`SPELL_ATTR_COOLDOWN_ON_EVENT`), i.e. "permanent" until the event fires.
pub fn is_event_triggered_cooldown(spell: &SpellEntry) -> bool {
    spell.has_attribute(SPELL_ATTR_COOLDOWN_ON_EVENT)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_spell_entry(id: u32) -> SpellEntry {
        SpellEntry {
            id,
            name: format!("Spell{}", id),
            rank_text: String::new(),
            school: 0,
            category: 0,
            dispel: 0,
            mechanic: 0,
            attributes: 0,
            attributes_ex: 0,
            attributes_ex2: 0,
            attributes_ex3: 0,
            attributes_ex4: 0,
            stances: 0,
            stances_not: 0,
            targets: 0,
            target_creature_type: 0,
            requires_spell_focus: 0,
            caster_aura_state: 0,
            target_aura_state: 0,
            casting_time_index: 0,
            recovery_time: 0,
            category_recovery_time: 0,
            interrupt_flags: 0,
            aura_interrupt_flags: 0,
            channel_interrupt_flags: 0,
            proc_flags: 0,
            proc_chance: 0,
            proc_charges: 0,
            max_level: 0,
            base_level: 0,
            spell_level: 0,
            duration_index: 0,
            power_type: 0,
            mana_cost: 0,
            mana_cost_per_level: 0,
            mana_per_second: 0,
            mana_per_second_per_level: 0,
            range_index: 0,
            speed: 0.0,
            stack_amount: 0,
            totem: [0; 2],
            reagent: [0; 8],
            reagent_count: [0; 8],
            equipped_item_class: 0,
            equipped_item_sub_class_mask: 0,
            equipped_item_inventory_type_mask: 0,
            effect: [0; 3],
            effect_die_sides: [0; 3],
            effect_base_dice: [0; 3],
            effect_dice_per_level: [0.0; 3],
            effect_real_points_per_level: [0.0; 3],
            effect_base_points: [0; 3],
            effect_bonus_coefficient: [0.0; 3],
            effect_mechanic: [0; 3],
            effect_implicit_target_a: [0; 3],
            effect_implicit_target_b: [0; 3],
            effect_radius_index: [0; 3],
            effect_apply_aura_name: [0; 3],
            effect_amplitude: [0; 3],
            effect_multiple_value: [0.0; 3],
            effect_chain_target: [0; 3],
            effect_item_type: [0; 3],
            effect_misc_value: [0; 3],
            effect_trigger_spell: [0; 3],
            effect_points_per_combo_point: [0.0; 3],
            spell_visual: 0,
            spell_icon_id: 0,
            active_icon_id: 0,
            spell_priority: 0,
            min_target_level: 0,
            mana_cost_percentage: 0,
            start_recovery_category: 0,
            start_recovery_time: 0,
            max_target_level: 0,
            spell_family_name: 0,
            spell_family_flags: 0,
            max_affected_targets: 0,
            dmg_class: 0,
            prevention_type: 0,
            custom: 0,
            internal: 0,
            allowed_target_mask: 0,
            script_id: 0,
            dmg_multiplier: [1.0; 3],
        }
    }

    #[test]
    fn passive_spells_never_get_a_cooldown() {
        let mut passive = make_spell_entry(1);
        passive.attributes = 0x40; // SPELL_ATTR_PASSIVE
        assert!(!should_apply_spell_cooldown(&passive));
    }

    #[test]
    fn non_passive_spells_get_a_cooldown() {
        let normal = make_spell_entry(2);
        assert!(should_apply_spell_cooldown(&normal));
    }

    #[test]
    fn event_cooldown_attribute_is_detected() {
        let mut event_cd = make_spell_entry(3);
        event_cd.attributes = SPELL_ATTR_COOLDOWN_ON_EVENT;
        assert!(is_event_triggered_cooldown(&event_cd));

        let normal = make_spell_entry(4);
        assert!(!is_event_triggered_cooldown(&normal));
    }
}
