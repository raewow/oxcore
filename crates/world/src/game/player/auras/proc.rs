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

/// The caster-side proc flag for completing a spell cast (MaNGOS `m_procAttacker`,
/// `Spell::prepareDataForTriggerSystem`). Picks by damage class and polarity; the swing/trap/
/// periodic refinements are omitted (handled where those flags matter).
pub fn spell_cast_attacker_proc_flag(
    dmg_class: u32,
    is_positive: bool,
    is_heal: bool,
    is_auto_repeat: bool,
) -> u32 {
    use proc_flags as pf;
    const SPELL_DAMAGE_CLASS_MAGIC: u32 = 1;
    const SPELL_DAMAGE_CLASS_MELEE: u32 = 2;
    const SPELL_DAMAGE_CLASS_RANGED: u32 = 3;

    match dmg_class {
        SPELL_DAMAGE_CLASS_MELEE => pf::DEAL_MELEE_ABILITY,
        SPELL_DAMAGE_CLASS_RANGED => {
            if is_auto_repeat {
                pf::DEAL_RANGED_ATTACK
            } else {
                pf::DEAL_RANGED_ABILITY
            }
        }
        _ => {
            if is_positive {
                if is_heal {
                    pf::DEAL_HELPFUL_SPELL
                } else {
                    pf::DEAL_HELPFUL_ABILITY
                }
            } else if is_auto_repeat {
                pf::DEAL_RANGED_ATTACK
            } else if dmg_class == SPELL_DAMAGE_CLASS_MAGIC {
                pf::DEAL_HARMFUL_SPELL
            } else {
                pf::DEAL_HARMFUL_ABILITY
            }
        }
    }
}

/// Weapon attack type used to pick the melee swing sub-flag (MaNGOS `WeaponAttackType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponAttackType {
    Base,
    Offhand,
    Ranged,
}

/// Whether a completed cast may trigger procs at all (MaNGOS `Spell::prepareDataForTriggerSystem`,
/// the `m_canTrigger` decision chain).
///
/// `has_trigger_source` covers both "triggered by an aura" (`m_triggeredByAuraSpell`) and the
/// `SPELL_EFFECT_TRIGGER_SPELL` case that MaNGOS treats identically for this check.
///
/// Approximation: the spell-family/spell-ID exceptions that re-enable proccing for specific
/// triggered spells (mage/warlock/hunter/paladin/priest class-mask carve-outs, e.g. Holy Shock,
/// Rain of Fire, hunter traps) are not modeled — `SpellClassMask` bit layouts for those
/// families aren't available in this codebase, so a wrong guess would silently misfire procs.
/// Any triggered cast that depends on one of those carve-outs will not proc here.
pub fn can_trigger_procs(
    suppresses_both_procs: bool,
    cast_by_item: bool,
    is_positive_spell: bool,
    caster_is_game_object: bool,
    is_triggered_spell: bool,
    has_trigger_source: bool,
    is_not_a_proc: bool,
) -> bool {
    if suppresses_both_procs {
        return false;
    }
    if cast_by_item {
        return !is_positive_spell;
    }
    if caster_is_game_object {
        return false;
    }
    if !is_triggered_spell {
        return true;
    }
    if !has_trigger_source {
        return true;
    }
    is_not_a_proc
}

/// The main-hand/off-hand swing sub-flag ORed into the attacker's melee proc flags
/// (MaNGOS `PROC_FLAG_MAIN_HAND_WEAPON_SWING` / `PROC_FLAG_OFF_HAND_WEAPON_SWING`).
fn melee_swing_sub_flag(attack_type: WeaponAttackType) -> u32 {
    use proc_flags as pf;
    match attack_type {
        WeaponAttackType::Base => pf::MAIN_HAND_WEAPON_SWING,
        WeaponAttackType::Offhand => pf::OFF_HAND_WEAPON_SWING,
        WeaponAttackType::Ranged => 0,
    }
}

/// Full attacker/victim proc-flag pair for a completed cast (MaNGOS `m_procAttacker` /
/// `m_procVictim`, `Spell::prepareDataForTriggerSystem`). Covers melee/ranged/default
/// dmg-class branches, the next-melee-swing and auto-repeat special cases, the
/// periodic-treat-as override, and hunter trap activation ORing.
///
/// `is_auto_repeat_ranged` corresponds to `SPELL_ATTR_EX2_AUTO_REPEAT`; `treat_as_periodic`
/// to `SPELL_ATTR_EX3_TREAT_AS_PERIODIC`; `is_wand_id_2094_or_23577` special-cases the two
/// legacy wand spell IDs that MaNGOS zeroes out under `SPELL_DAMAGE_CLASS_RANGED`.
///
/// Approximation: the hunter-trap `PROC_FLAG_ON_TRAP_ACTIVATION` OR-in (`is_trap_spell`) is
/// exposed as a plain bool since the CF_HUNTER_* class-mask bit layout isn't available here;
/// callers that know a spell is a trap-family spell by other means (e.g. spell ID) can still
/// pass `true`.
#[allow(clippy::too_many_arguments)]
pub fn spell_attacker_victim_proc_flags(
    dmg_class: u32,
    attack_type: WeaponAttackType,
    is_next_melee_swing_spell: bool,
    is_auto_repeat_ranged: bool,
    is_wand_id_2094_or_23577: bool,
    is_area_of_effect: bool,
    is_positive_spell: bool,
    is_heal_spell: bool,
    treat_as_periodic: bool,
    is_trap_spell: bool,
) -> (u32, u32) {
    use proc_flags as pf;
    const SPELL_DAMAGE_CLASS_MAGIC: u32 = 1;
    const SPELL_DAMAGE_CLASS_MELEE: u32 = 2;
    const SPELL_DAMAGE_CLASS_RANGED: u32 = 3;
    let _ = is_area_of_effect; // reserved: no dmg-class branch below reads it directly

    match dmg_class {
        SPELL_DAMAGE_CLASS_MELEE => {
            let mut attacker = pf::DEAL_MELEE_ABILITY | melee_swing_sub_flag(attack_type);
            let mut victim = pf::TAKE_MELEE_ABILITY;
            if is_next_melee_swing_spell {
                attacker |= pf::DEAL_MELEE_SWING;
                victim |= pf::TAKE_MELEE_SWING;
            }
            (attacker, victim)
        }
        SPELL_DAMAGE_CLASS_RANGED => {
            if is_auto_repeat_ranged {
                (pf::DEAL_RANGED_ATTACK, pf::TAKE_RANGED_ATTACK)
            } else if is_wand_id_2094_or_23577 {
                (pf::NONE, pf::NONE)
            } else {
                (pf::DEAL_RANGED_ABILITY, pf::TAKE_RANGED_ABILITY)
            }
        }
        _ => {
            if is_positive_spell {
                if is_heal_spell {
                    (pf::DEAL_HELPFUL_SPELL, pf::TAKE_HELPFUL_SPELL)
                } else {
                    (pf::DEAL_HELPFUL_ABILITY, pf::TAKE_HELPFUL_ABILITY)
                }
            } else if is_auto_repeat_ranged {
                (pf::DEAL_RANGED_ATTACK, pf::TAKE_RANGED_ATTACK)
            } else {
                let (mut attacker, mut victim) = if treat_as_periodic {
                    (pf::DEAL_HARMFUL_PERIODIC, pf::TAKE_HARMFUL_PERIODIC)
                } else if dmg_class == SPELL_DAMAGE_CLASS_MAGIC {
                    (pf::DEAL_HARMFUL_SPELL, pf::TAKE_HARMFUL_SPELL)
                } else {
                    (pf::DEAL_HARMFUL_ABILITY, pf::TAKE_HARMFUL_ABILITY)
                };
                if is_trap_spell {
                    attacker |= pf::ON_TRAP_ACTIVATION;
                }
                (attacker, victim)
            }
        }
    }
}

/// Per-effect negative-hit bitmask used to skip harmful procs on positive-only hits
/// (MaNGOS `m_negativeEffectMask`, built in `Spell::prepareDataForTriggerSystem`).
///
/// `is_positive_effect(i)` should be `SpellEntry::IsPositiveEffect`. `is_school_damage_on_caster(i)`
/// marks effect `i` as `SPELL_EFFECT_SCHOOL_DAMAGE` targeting `TARGET_UNIT_CASTER` (self-damage is
/// treated as negative for proc filtering even though the effect itself reads as positive).
pub fn negative_effect_mask<const N: usize>(
    is_positive_effect: impl Fn(usize) -> bool,
    is_school_damage_on_caster: impl Fn(usize) -> bool,
) -> u8 {
    let mut mask = 0u8;
    for i in 0..N {
        if !is_positive_effect(i) || is_school_damage_on_caster(i) {
            mask |= 1 << i;
        }
    }
    mask
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
        assert!(!can_trigger(
            None,
            proc_flags::DEAL_MELEE_SWING,
            proc_flags_ex::NORMAL_HIT
        ));
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
        assert!(can_trigger(
            Some(&crit_only),
            aura,
            proc_flags_ex::CRITICAL_HIT
        ));
        assert!(!can_trigger(
            Some(&crit_only),
            aura,
            proc_flags_ex::NORMAL_HIT
        ));
    }

    #[test]
    fn ex_trigger_always_bypasses_outcome() {
        let aura = proc_flags::DEAL_HARMFUL_SPELL;
        let always = event(proc_flags_ex::EX_TRIGGER_ALWAYS);
        assert!(can_trigger(Some(&always), aura, proc_flags_ex::MISS));
    }

    #[test]
    fn cast_attacker_flag_by_class_and_polarity() {
        // Harmful magic spell → deal-harmful-spell.
        assert_eq!(
            spell_cast_attacker_proc_flag(1, false, false, false),
            proc_flags::DEAL_HARMFUL_SPELL
        );
        // Positive heal → deal-helpful-spell; positive non-heal → deal-helpful-ability.
        assert_eq!(
            spell_cast_attacker_proc_flag(1, true, true, false),
            proc_flags::DEAL_HELPFUL_SPELL
        );
        assert_eq!(
            spell_cast_attacker_proc_flag(1, true, false, false),
            proc_flags::DEAL_HELPFUL_ABILITY
        );
        // Melee/ranged ability spells.
        assert_eq!(
            spell_cast_attacker_proc_flag(2, false, false, false),
            proc_flags::DEAL_MELEE_ABILITY
        );
        assert_eq!(
            spell_cast_attacker_proc_flag(3, false, false, false),
            proc_flags::DEAL_RANGED_ABILITY
        );
        // Auto-repeat (wand/auto-shot) → ranged auto attack.
        assert_eq!(
            spell_cast_attacker_proc_flag(0, false, false, true),
            proc_flags::DEAL_RANGED_ATTACK
        );
    }

    #[test]
    fn cast_end_proc_does_not_double_fire_with_damage() {
        // A Clearcasting-style aura: procs on DEAL_HARMFUL_SPELL, configured with CAST_END.
        let aura = proc_flags::DEAL_HARMFUL_SPELL;
        let ev = event(proc_flags_ex::CAST_END);
        // Fires on the cast-end event...
        assert!(can_trigger(
            Some(&ev),
            aura,
            proc_flags_ex::CAST_END | proc_flags_ex::NORMAL_HIT
        ));
        // ...but NOT on the later damage event (no CAST_END) → no double proc.
        assert!(!can_trigger(Some(&ev), aura, proc_flags_ex::NORMAL_HIT));
    }

    #[test]
    fn can_trigger_suppressed_both_procs_wins_first() {
        // Suppress-both takes priority even over cast-by-item / triggered checks.
        assert!(!can_trigger_procs(
            true, true, false, false, false, false, false
        ));
    }

    #[test]
    fn can_trigger_item_cast_gates_on_positivity() {
        // Negative item-cast spell can trigger; positive item-cast spell cannot.
        assert!(can_trigger_procs(
            false, true, false, false, false, false, false
        ));
        assert!(!can_trigger_procs(
            false, true, true, false, false, false, false
        ));
    }

    #[test]
    fn can_trigger_game_object_caster_never_procs() {
        assert!(!can_trigger_procs(
            false, false, false, true, false, false, false
        ));
    }

    #[test]
    fn can_trigger_normal_cast_always_procs() {
        assert!(can_trigger_procs(
            false, false, false, false, false, false, false
        ));
    }

    #[test]
    fn can_trigger_triggered_without_source_procs() {
        // Triggered but not via an aura/trigger-spell effect -> still allowed.
        assert!(can_trigger_procs(
            false, false, false, false, true, false, false
        ));
    }

    #[test]
    fn can_trigger_triggered_with_source_needs_not_a_proc_attr() {
        assert!(!can_trigger_procs(
            false, false, false, false, true, true, false
        ));
        assert!(can_trigger_procs(
            false, false, false, false, true, true, true
        ));
    }

    #[test]
    fn attacker_victim_flags_melee_base_and_offhand() {
        let (a, v) = spell_attacker_victim_proc_flags(
            2,
            WeaponAttackType::Base,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
        );
        assert_eq!(a, proc_flags::DEAL_MELEE_ABILITY | proc_flags::MAIN_HAND_WEAPON_SWING);
        assert_eq!(v, proc_flags::TAKE_MELEE_ABILITY);

        let (a, _) = spell_attacker_victim_proc_flags(
            2,
            WeaponAttackType::Offhand,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
        );
        assert_eq!(a, proc_flags::DEAL_MELEE_ABILITY | proc_flags::OFF_HAND_WEAPON_SWING);
    }

    #[test]
    fn attacker_victim_flags_melee_next_swing_adds_swing_flags() {
        let (a, v) = spell_attacker_victim_proc_flags(
            2,
            WeaponAttackType::Ranged,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
        );
        assert_eq!(a & proc_flags::DEAL_MELEE_SWING, proc_flags::DEAL_MELEE_SWING);
        assert_eq!(v & proc_flags::TAKE_MELEE_SWING, proc_flags::TAKE_MELEE_SWING);
    }

    #[test]
    fn attacker_victim_flags_ranged_auto_repeat_vs_ability() {
        let (a, v) = spell_attacker_victim_proc_flags(
            3, WeaponAttackType::Ranged, false, true, false, false, false, false, false, false,
        );
        assert_eq!(a, proc_flags::DEAL_RANGED_ATTACK);
        assert_eq!(v, proc_flags::TAKE_RANGED_ATTACK);

        let (a, v) = spell_attacker_victim_proc_flags(
            3, WeaponAttackType::Ranged, false, false, false, false, false, false, false, false,
        );
        assert_eq!(a, proc_flags::DEAL_RANGED_ABILITY);
        assert_eq!(v, proc_flags::TAKE_RANGED_ABILITY);
    }

    #[test]
    fn attacker_victim_flags_ranged_legacy_wand_ids_zeroed() {
        let (a, v) = spell_attacker_victim_proc_flags(
            3, WeaponAttackType::Ranged, false, false, true, false, false, false, false, false,
        );
        assert_eq!(a, proc_flags::NONE);
        assert_eq!(v, proc_flags::NONE);
    }

    #[test]
    fn attacker_victim_flags_default_positive_heal_vs_ability() {
        let (a, v) = spell_attacker_victim_proc_flags(
            0, WeaponAttackType::Ranged, false, false, false, false, true, true, false, false,
        );
        assert_eq!(a, proc_flags::DEAL_HELPFUL_SPELL);
        assert_eq!(v, proc_flags::TAKE_HELPFUL_SPELL);

        let (a, v) = spell_attacker_victim_proc_flags(
            0, WeaponAttackType::Ranged, false, false, false, false, true, false, false, false,
        );
        assert_eq!(a, proc_flags::DEAL_HELPFUL_ABILITY);
        assert_eq!(v, proc_flags::TAKE_HELPFUL_ABILITY);
    }

    #[test]
    fn attacker_victim_flags_default_negative_periodic_vs_magic_vs_ability() {
        // Periodic-treat override wins even for magic dmg-class.
        let (a, v) = spell_attacker_victim_proc_flags(
            1, WeaponAttackType::Ranged, false, false, false, false, false, false, true, false,
        );
        assert_eq!(a, proc_flags::DEAL_HARMFUL_PERIODIC);
        assert_eq!(v, proc_flags::TAKE_HARMFUL_PERIODIC);

        let (a, v) = spell_attacker_victim_proc_flags(
            1, WeaponAttackType::Ranged, false, false, false, false, false, false, false, false,
        );
        assert_eq!(a, proc_flags::DEAL_HARMFUL_SPELL);
        assert_eq!(v, proc_flags::TAKE_HARMFUL_SPELL);

        let (a, v) = spell_attacker_victim_proc_flags(
            0, WeaponAttackType::Ranged, false, false, false, false, false, false, false, false,
        );
        assert_eq!(a, proc_flags::DEAL_HARMFUL_ABILITY);
        assert_eq!(v, proc_flags::TAKE_HARMFUL_ABILITY);
    }

    #[test]
    fn attacker_victim_flags_trap_activation_ors_into_attacker_only() {
        let (a, v) = spell_attacker_victim_proc_flags(
            0, WeaponAttackType::Ranged, false, false, false, false, false, false, false, true,
        );
        assert_eq!(a & proc_flags::ON_TRAP_ACTIVATION, proc_flags::ON_TRAP_ACTIVATION);
        assert_eq!(v & proc_flags::ON_TRAP_ACTIVATION, 0);
    }

    #[test]
    fn negative_effect_mask_flags_non_positive_effects() {
        // Effect 0 positive, effect 1 negative, effect 2 positive -> only bit 1 set.
        let positive = [true, false, true];
        let mask = negative_effect_mask::<3>(|i| positive[i], |_| false);
        assert_eq!(mask, 0b010);
    }

    #[test]
    fn negative_effect_mask_self_school_damage_counts_as_negative() {
        // All effects read positive, but effect 2 is self-targeted school damage.
        let self_damage = [false, false, true];
        let mask = negative_effect_mask::<3>(|_| true, |i| self_damage[i]);
        assert_eq!(mask, 0b100);
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
