//! Proc system handlers for aura-triggered effects

use crate::shared::protocol::ObjectGuid;
use crate::world::game::broadcast_mgr::BroadcastManager;
use crate::world::game::player::auras::effects::*;
use crate::world::game::player::auras::system::ProcCandidate;
use crate::world::World;
use std::sync::Arc;

use anyhow::Result;

/// Proc flags - bit flags indicating what combat event occurred.
/// These mirror the MaNGOS ProcFlags enum.
pub mod proc_flags {
    pub const NONE: u32 = 0x00000000;
    pub const KILLED: u32 = 0x00000001; // Target was killed
    pub const KILL: u32 = 0x00000002; // Attacker killed target
    pub const MELEE_HIT: u32 = 0x00000004; // Successful melee attack
    pub const MELEE_HIT_TAKEN: u32 = 0x00000008; // Was hit by melee
    pub const SPELL_HIT: u32 = 0x00000010; // Spell dealt damage
    pub const SPELL_HIT_TAKEN: u32 = 0x00000020; // Was hit by spell
    pub const SPELL_CAST: u32 = 0x00000040; // Completed a spell cast
    pub const DAMAGE_DEALT: u32 = 0x00000080; // Dealt any damage
    pub const DAMAGE_TAKEN: u32 = 0x00000100; // Took any damage
    pub const HEAL: u32 = 0x00000200; // Healed a target
    pub const HEAL_TAKEN: u32 = 0x00000400; // Received healing
    pub const ON_DO_PERIODIC: u32 = 0x00000800; // Dealt periodic damage
    pub const ON_TAKE_PERIODIC: u32 = 0x00001000; // Took periodic damage
    pub const ON_BLOCK: u32 = 0x00002000; // Blocked an attack
    pub const ON_DODGE: u32 = 0x00004000; // Dodged an attack
    pub const ON_PARRY: u32 = 0x00008000; // Parried an attack
}

/// Extended proc flags for specific hit results.
pub mod proc_flags_ex {
    pub const NONE: u32 = 0x00000000;
    pub const NORMAL_HIT: u32 = 0x00000001;
    pub const CRITICAL_HIT: u32 = 0x00000002;
    pub const MISS: u32 = 0x00000004;
    pub const RESIST: u32 = 0x00000008;
    pub const DODGE: u32 = 0x00000010;
    pub const PARRY: u32 = 0x00000020;
    pub const BLOCK: u32 = 0x00000040;
    pub const EVADE: u32 = 0x00000080;
    pub const IMMUNE: u32 = 0x00000100;
    pub const DEFLECT: u32 = 0x00000200;
    pub const ABSORB: u32 = 0x00000400;
    pub const REFLECT: u32 = 0x00000800;
    pub const INTERRUPT: u32 = 0x00001000;
}

/// Result of dispatching a proc — may request a triggered spell cast.
pub struct ProcResult {
    /// If set, this spell should be cast as triggered on the player's current target
    pub trigger_spell_id: Option<u32>,
}

/// Dispatch a proc event for a single aura candidate.
/// Returns a ProcResult indicating if a triggered spell cast is needed.
pub fn dispatch_proc(
    player_guid: ObjectGuid,
    candidate: &ProcCandidate,
    _proc_flags: u32,
    proc_spell_id: Option<u32>,
    damage: u32,
    world: &World,
    broadcast_mgr: &Arc<BroadcastManager>,
) -> Result<ProcResult> {
    match candidate.aura_type {
        AURA_PROC_TRIGGER_SPELL => {
            handle_proc_trigger_spell(player_guid, candidate, world, broadcast_mgr)
        }
        AURA_PROC_TRIGGER_DAMAGE => {
            handle_proc_trigger_damage(player_guid, candidate, damage, world, broadcast_mgr)?;
            Ok(ProcResult { trigger_spell_id: None })
        }
        AURA_DUMMY => {
            handle_dummy_proc(
                player_guid,
                candidate,
                proc_spell_id,
                damage,
                world,
                broadcast_mgr,
            )?;
            Ok(ProcResult { trigger_spell_id: None })
        }
        _ => {
            tracing::debug!(
                "Unhandled proc aura type {} for spell {}",
                candidate.aura_type,
                candidate.spell_id
            );
            Ok(ProcResult { trigger_spell_id: None })
        }
    }
}

/// Handle PROC_TRIGGER_SPELL: cast a spell when the proc fires.
///
/// The triggered spell ID comes from the aura's spell entry (effect_trigger_spell).
/// Common examples:
/// - Fiery Weapon enchant: proc triggers Fire damage spell
/// - Seal of the Crusader: proc triggers bonus Holy damage
/// - Windfury Weapon: proc triggers extra attack spell
fn handle_proc_trigger_spell(
    player_guid: ObjectGuid,
    candidate: &ProcCandidate,
    world: &World,
    _broadcast_mgr: &Arc<BroadcastManager>,
) -> Result<ProcResult> {
    let trigger_spell_id = candidate.trigger_spell_id;
    if trigger_spell_id == 0 {
        tracing::debug!(
            "Proc trigger spell has no trigger_spell_id: spell_id={}",
            candidate.spell_id,
        );
        return Ok(ProcResult { trigger_spell_id: None });
    }

    // Verify the triggered spell exists
    if world.managers.spell_mgr.get(trigger_spell_id).is_none() {
        tracing::warn!(
            "Proc trigger spell {} not found (from aura spell {})",
            trigger_spell_id,
            candidate.spell_id,
        );
        return Ok(ProcResult { trigger_spell_id: None });
    }

    tracing::debug!(
        "Proc trigger spell: aura={} triggers spell={} on player={:?}",
        candidate.spell_id,
        trigger_spell_id,
        player_guid,
    );

    // Return the triggered spell ID so check_procs can cast it asynchronously
    Ok(ProcResult { trigger_spell_id: Some(trigger_spell_id) })
}

/// Handle PROC_TRIGGER_DAMAGE: deal damage when the proc fires.
///
/// The damage amount is the aura's current_value.
/// Common examples:
/// - Thorns: deal X nature damage when struck
/// - Retribution Aura: deal X holy damage when party member is struck
fn handle_proc_trigger_damage(
    _player_guid: ObjectGuid,
    candidate: &ProcCandidate,
    _event_damage: u32,
    _world: &World,
    _broadcast_mgr: &Arc<BroadcastManager>,
) -> Result<()> {
    let proc_damage = candidate.current_value.max(0) as u32;
    if proc_damage == 0 {
        return Ok(());
    }

    // TODO: Deal damage via CombatSystem
    // The target is whoever triggered the proc (attacker for defensive procs, victim for offensive)
    tracing::debug!(
        "Proc trigger damage: spell_id={}, damage={}, player={:?}",
        candidate.spell_id,
        proc_damage,
        _player_guid
    );
    Ok(())
}

/// Handle DUMMY aura proc: custom per-spell-ID logic.
///
/// Dummy auras use spell_family_name and spell_id to determine behavior.
/// Examples:
/// - Sweeping Strikes (12292): on melee, hit another nearby enemy
/// - Eye for an Eye (9799): on spell crit taken, reflect 30% damage
/// - Vengeance (20049): on crit, gain 15% physical damage for 8s
fn handle_dummy_proc(
    _player_guid: ObjectGuid,
    candidate: &ProcCandidate,
    _proc_spell_id: Option<u32>,
    damage: u32,
    _world: &World,
    _broadcast_mgr: &Arc<BroadcastManager>,
) -> Result<()> {
    // Dispatch by spell ID or spell family
    match candidate.spell_id {
        // Sweeping Strikes
        12292 | 18765 => {
            if damage > 1 {
                // TODO: Find another nearby enemy and deal damage
                tracing::debug!(
                    "Sweeping Strikes proc: damage={}, spell_id={}",
                    damage,
                    candidate.spell_id
                );
            }
        }
        // Retaliation
        20230 => {
            // TODO: Strike back at attacker
            tracing::debug!("Retaliation proc: spell_id={}", candidate.spell_id);
        }
        _ => {
            tracing::debug!("Unhandled dummy proc: spell_id={}", candidate.spell_id);
        }
    }

    Ok(())
}

/// Roll for proc chance.
///
/// `proc_chance` is from spell DBC (proc_chance field), range 0-100.
/// Some spells use PPM (procs per minute) instead, which depends on weapon speed.
pub fn roll_proc_chance(proc_chance: f32) -> bool {
    if proc_chance >= 100.0 {
        return true;
    }
    if proc_chance <= 0.0 {
        return false;
    }
    let roll = rand::random::<f32>() * 100.0;
    roll < proc_chance
}

/// Calculate PPM (procs per minute) chance for a given weapon speed.
///
/// Formula: chance = ppm_rate * weapon_speed_seconds / 60
/// Example: 1 PPM with 3.0s weapon = 3.0/60 = 5% per swing
pub fn ppm_proc_chance(ppm_rate: f32, weapon_speed_ms: u32) -> f32 {
    let weapon_speed_sec = weapon_speed_ms as f32 / 1000.0;
    ppm_rate * weapon_speed_sec / 60.0 * 100.0
}
