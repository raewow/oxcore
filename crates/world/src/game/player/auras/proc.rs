//! Proc system handlers for aura-triggered effects

use crate::game::broadcast_mgr::BroadcastManager;
use crate::game::player::auras::effects::*;
use crate::game::player::auras::system::ProcCandidate;
use crate::World;
use oxcore_shared::protocol::ObjectGuid;
use std::sync::Arc;

use anyhow::Result;

/// Proc flags — bit flags indicating what combat event occurred.
///
/// These MUST match the classic `ProcFlags` enum (SpellDefines.h) because they are matched
/// against each spell's `proc_flags` field loaded from the DBC. A mismatched bit layout makes
/// `spell.proc_flags & event_flags` silently fail, so procs never fire.
pub mod proc_flags {
    pub const NONE: u32 = 0x00000000;
    pub const HEARTBEAT: u32 = 0x00000001; // 00 On tick
    pub const KILL: u32 = 0x00000002; // 01 Killed a target
    pub const DEAL_MELEE_SWING: u32 = 0x00000004; // 02 Successful melee auto attack
    pub const TAKE_MELEE_SWING: u32 = 0x00000008; // 03 Took melee auto-attack damage
    pub const DEAL_MELEE_ABILITY: u32 = 0x00000010; // 04 Landed a melee-weapon spell
    pub const TAKE_MELEE_ABILITY: u32 = 0x00000020; // 05 Took melee-weapon spell damage
    pub const DEAL_RANGED_ATTACK: u32 = 0x00000040; // 06 Successful ranged auto attack
    pub const TAKE_RANGED_ATTACK: u32 = 0x00000080; // 07 Took ranged auto-attack damage
    pub const DEAL_RANGED_ABILITY: u32 = 0x00000100; // 08 Landed a ranged-weapon spell
    pub const TAKE_RANGED_ABILITY: u32 = 0x00000200; // 09 Took ranged-weapon spell damage
    pub const DEAL_HELPFUL_ABILITY: u32 = 0x00000400; // 10 Cast a positive no-damage-class spell
    pub const TAKE_HELPFUL_ABILITY: u32 = 0x00000800; // 11 Took a positive no-damage-class spell
    pub const DEAL_HARMFUL_ABILITY: u32 = 0x00001000; // 12 Cast a negative no-damage-class spell
    pub const TAKE_HARMFUL_ABILITY: u32 = 0x00002000; // 13 Took a negative no-damage-class spell
    pub const DEAL_HELPFUL_SPELL: u32 = 0x00004000; // 14 Cast a positive spell (default: healing)
    pub const TAKE_HELPFUL_SPELL: u32 = 0x00008000; // 15 Took a positive spell (default: healing)
    pub const DEAL_HARMFUL_SPELL: u32 = 0x00010000; // 16 Cast a negative spell (default: on damage)
    pub const TAKE_HARMFUL_SPELL: u32 = 0x00020000; // 17 Took a negative spell (default: on damage)
    pub const DEAL_HARMFUL_PERIODIC: u32 = 0x00040000; // 18 Dealt a periodic tick
    pub const TAKE_HARMFUL_PERIODIC: u32 = 0x00080000; // 19 Took a periodic tick
    pub const TAKEN_ANY_DAMAGE: u32 = 0x00100000; // 20 Took any damage
    pub const ON_TRAP_ACTIVATION: u32 = 0x00200000; // 21 Trap activated
    pub const MAIN_HAND_WEAPON_SWING: u32 = 0x00400000; // 22 Main-hand swing
    pub const OFF_HAND_WEAPON_SWING: u32 = 0x00800000; // 23 Off-hand swing
}

/// Extended proc flags describing the hit result of the event (classic `ProcFlagsEx`).
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
    pub const EX_TRIGGER_ALWAYS: u32 = 0x00010000; // Fire regardless of other flags (drop charges)
    pub const NO_PERIODIC: u32 = 0x00020000; // Never proc on a periodic event
    pub const PERIODIC_POSITIVE: u32 = 0x00040000; // Periodic heal
    pub const CAST_END: u32 = 0x00080000; // Procs at end of cast only

    /// Outcomes that did not connect (miss/dodge/parry/etc.) — used to skip damage procs.
    pub const NO_DAMAGE_MASK: u32 =
        MISS | RESIST | DODGE | PARRY | BLOCK | EVADE | IMMUNE | DEFLECT | ABSORB | REFLECT;
}

use crate::game::spell::manager::SpellProcEventEntry;

/// Whether a proc aura may trigger for a given combat event
/// (MaNGOS `SpellMgr::IsSpellProcEventCanTriggeredBy`).
///
/// - `proc_event`: the aura spell's custom `spell_proc_event` config (None if unconfigured).
/// - `aura_proc_flags`: the proc aura's own `proc_flags` (the "EventProcFlag").
/// - `event_proc_flags` / `event_proc_ex`: the actual event's flags and hit-outcome flags.
/// - `is_melee`: the event came from a melee swing (no triggering spell).
/// - `proc_spell_school_mask` / `proc_spell_family`: the triggering spell's school mask and family.
/// - `proc_spell_is_periodic`: the triggering spell applies a periodic aura.
pub fn is_spell_proc_event_can_triggered_by(
    proc_event: Option<&SpellProcEventEntry>,
    aura_proc_flags: u32,
    event_proc_flags: u32,
    event_proc_ex: u32,
    is_melee: bool,
    proc_spell_school_mask: u32,
    proc_spell_family: u32,
    proc_spell_is_periodic: bool,
) -> bool {
    use proc_flags as pf;
    use proc_flags_ex as pex;

    let event_procex = proc_event.map(|e| e.proc_ex).unwrap_or(pex::NONE);

    // The event flags must intersect the aura's proc flags.
    if event_proc_flags & aura_proc_flags == 0 {
        return false;
    }

    // Either both require cast-end, or neither — keeps cast-end procs and hit procs separate.
    if (event_proc_ex & pex::CAST_END) != (event_procex & pex::CAST_END) {
        return false;
    }

    // Kill / heartbeat / trap activation always trigger.
    if event_proc_flags & (pf::HEARTBEAT | pf::KILL | pf::ON_TRAP_ACTIVATION) != 0 {
        return true;
    }

    // School / family gates, only when the aura has custom proc-event data.
    if let Some(ev) = proc_event {
        const SCHOOL_MASK_NORMAL: u32 = 0x01;
        if is_melee {
            if ev.school_mask != 0 && ev.school_mask & SCHOOL_MASK_NORMAL == 0 {
                return false;
            }
        } else {
            if ev.school_mask != 0 && ev.school_mask & proc_spell_school_mask == 0 {
                return false;
            }
            if ev.spell_family != 0 && ev.spell_family != proc_spell_family {
                return false;
            }
        }
    }

    if event_procex == pex::NONE {
        // No custom requirement: never proc from a periodic heal; otherwise proc on hit/crit.
        if event_proc_flags & (pf::DEAL_HARMFUL_PERIODIC | pf::TAKE_HARMFUL_PERIODIC) != 0
            && event_proc_ex & pex::PERIODIC_POSITIVE != 0
        {
            return false;
        }
        if event_proc_ex & (pex::NORMAL_HIT | pex::CRITICAL_HIT) != 0 {
            return true;
        }
    } else {
        // Custom requirement present (resist/reflect/immune/periodic/specific outcome).
        if event_procex & pex::EX_TRIGGER_ALWAYS != 0 {
            return true;
        }
        if event_procex & pex::NO_PERIODIC != 0
            && (event_proc_flags & (pf::DEAL_HARMFUL_PERIODIC | pf::TAKE_HARMFUL_PERIODIC) != 0
                || proc_spell_is_periodic)
        {
            return false;
        }
        if event_procex & event_proc_ex != 0 {
            return true;
        }
    }

    false
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
    _proc_ex: u32,
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
            Ok(ProcResult {
                trigger_spell_id: None,
            })
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
            Ok(ProcResult {
                trigger_spell_id: None,
            })
        }
        _ => {
            tracing::debug!(
                "Unhandled proc aura type {} for spell {}",
                candidate.aura_type,
                candidate.spell_id
            );
            Ok(ProcResult {
                trigger_spell_id: None,
            })
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
        return Ok(ProcResult {
            trigger_spell_id: None,
        });
    }

    // Verify the triggered spell exists
    if world.managers.spell_mgr.get(trigger_spell_id).is_none() {
        tracing::warn!(
            "Proc trigger spell {} not found (from aura spell {})",
            trigger_spell_id,
            candidate.spell_id,
        );
        return Ok(ProcResult {
            trigger_spell_id: None,
        });
    }

    tracing::debug!(
        "Proc trigger spell: aura={} triggers spell={} on player={:?}",
        candidate.spell_id,
        trigger_spell_id,
        player_guid,
    );

    // Return the triggered spell ID so check_procs can cast it asynchronously
    Ok(ProcResult {
        trigger_spell_id: Some(trigger_spell_id),
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_flags_match_classic_dbc_layout() {
        // These bit positions are what the spell DBC's proc_flags field uses; they must match
        // SpellDefines.h ProcFlags exactly or DBC matching silently fails.
        assert_eq!(proc_flags::DEAL_MELEE_SWING, 0x00000004);
        assert_eq!(proc_flags::DEAL_MELEE_ABILITY, 0x00000010);
        assert_eq!(proc_flags::DEAL_HELPFUL_SPELL, 0x00004000);
        assert_eq!(proc_flags::TAKE_HELPFUL_SPELL, 0x00008000);
        assert_eq!(proc_flags::DEAL_HARMFUL_SPELL, 0x00010000);
        assert_eq!(proc_flags::TAKE_HARMFUL_SPELL, 0x00020000);
        assert_eq!(proc_flags::TAKEN_ANY_DAMAGE, 0x00100000);
    }

    #[test]
    fn proc_flags_ex_match_classic_dbc_layout() {
        assert_eq!(proc_flags_ex::NORMAL_HIT, 0x00000001);
        assert_eq!(proc_flags_ex::CRITICAL_HIT, 0x00000002);
        assert_eq!(proc_flags_ex::CAST_END, 0x00080000);
        assert_eq!(proc_flags_ex::EX_TRIGGER_ALWAYS, 0x00010000);
    }

    #[test]
    fn no_damage_mask_covers_avoidance_outcomes() {
        use proc_flags_ex::*;
        assert_ne!(NO_DAMAGE_MASK & MISS, 0);
        assert_ne!(NO_DAMAGE_MASK & DODGE, 0);
        assert_ne!(NO_DAMAGE_MASK & PARRY, 0);
        assert_ne!(NO_DAMAGE_MASK & RESIST, 0);
        // Connecting outcomes are NOT in the no-damage mask.
        assert_eq!(NO_DAMAGE_MASK & NORMAL_HIT, 0);
        assert_eq!(NO_DAMAGE_MASK & CRITICAL_HIT, 0);
    }

    #[test]
    fn ppm_chance_scales_with_weapon_speed() {
        // 1 PPM with a 3.0s weapon → 5% per swing.
        assert!((ppm_proc_chance(1.0, 3000) - 5.0).abs() < 0.001);
        // Faster weapon → lower per-swing chance.
        assert!(ppm_proc_chance(1.0, 1500) < ppm_proc_chance(1.0, 3000));
    }

    fn event(proc_ex: u32) -> SpellProcEventEntry {
        SpellProcEventEntry {
            school_mask: 0,
            spell_family: 0,
            spell_family_mask: 0,
            proc_flags: 0,
            proc_ex,
            ppm_rate: 0.0,
            custom_chance: 0.0,
            cooldown: 0,
        }
    }

    // Convenience: harmful-spell event with the given hit outcome.
    fn can_trigger(proc_event: Option<&SpellProcEventEntry>, aura_flags: u32, ev_ex: u32) -> bool {
        is_spell_proc_event_can_triggered_by(
            proc_event,
            aura_flags,
            proc_flags::DEAL_HARMFUL_SPELL,
            ev_ex,
            false,
            0x04, // some school mask
            5,    // some family
            false,
        )
    }

    #[test]
    fn rejects_when_proc_flags_do_not_intersect() {
        // Aura procs on melee swing, event is a harmful spell → no intersection.
        assert!(!can_trigger(None, proc_flags::DEAL_MELEE_SWING, proc_flags_ex::NORMAL_HIT));
    }

    #[test]
    fn no_event_triggers_on_hit_and_crit_only() {
        let aura = proc_flags::DEAL_HARMFUL_SPELL;
        assert!(can_trigger(None, aura, proc_flags_ex::NORMAL_HIT));
        assert!(can_trigger(None, aura, proc_flags_ex::CRITICAL_HIT));
        // A pure miss/resist does not trigger a default proc.
        assert!(!can_trigger(None, aura, proc_flags_ex::MISS));
        assert!(!can_trigger(None, aura, proc_flags_ex::RESIST));
    }

    #[test]
    fn cast_end_must_be_paired() {
        let aura = proc_flags::DEAL_HARMFUL_SPELL;
        // Cast-end event but a default (non-cast-end) proc → rejected.
        assert!(!can_trigger(None, aura, proc_flags_ex::CAST_END));
        // Cast-end event and a cast-end-configured proc → eligible.
        let ev = event(proc_flags_ex::CAST_END);
        assert!(can_trigger(Some(&ev), aura, proc_flags_ex::CAST_END));
        // Cast-end proc on a normal hit event → rejected (CAST_END mismatch).
        assert!(!can_trigger(Some(&ev), aura, proc_flags_ex::NORMAL_HIT));
    }

    #[test]
    fn custom_proc_ex_requires_matching_outcome() {
        let aura = proc_flags::DEAL_HARMFUL_SPELL;
        let crit_only = event(proc_flags_ex::CRITICAL_HIT);
        assert!(can_trigger(Some(&crit_only), aura, proc_flags_ex::CRITICAL_HIT));
        assert!(!can_trigger(Some(&crit_only), aura, proc_flags_ex::NORMAL_HIT));
    }

    #[test]
    fn ex_trigger_always_bypasses_outcome() {
        let aura = proc_flags::DEAL_HARMFUL_SPELL;
        let always = event(proc_flags_ex::EX_TRIGGER_ALWAYS);
        assert!(can_trigger(Some(&always), aura, proc_flags_ex::MISS));
    }

    #[test]
    fn school_mask_gate_rejects_wrong_school() {
        let aura = proc_flags::DEAL_HARMFUL_SPELL;
        let mut ev = event(proc_flags_ex::NORMAL_HIT);
        ev.school_mask = 0x02; // requires a different school than the event's 0x04
        assert!(!can_trigger(Some(&ev), aura, proc_flags_ex::NORMAL_HIT));
        ev.school_mask = 0x04; // now matches
        assert!(can_trigger(Some(&ev), aura, proc_flags_ex::NORMAL_HIT));
    }
}
