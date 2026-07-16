//! Per-target spell effect application.
//!
//! Faithful (simplified) port of MaNGOS `TargetInfo` / `Spell::DoAllEffectOnTarget`.
//! Each unit hit by a spell gets exactly one `TargetInfo`: the hit/miss/resist outcome
//! is resolved once for the whole target (not once per effect), the requested effect
//! mask is carried across every effect applied to that target, and per-target proc
//! flags/accumulated damage & healing are tracked the same way `m_procAttacker` /
//! `m_procVictim` / `m_damage` / `m_healing` are threaded through the C++ function.
//!
//! Effect handlers in `effects/*` still apply their own damage/heal immediately
//! (via `caster::deal_damage` / `caster::deal_heal`) rather than being deferred to a
//! single post-loop apply like the C++ does — see the module doc on
//! [`apply_target_effects`] for why that divergence is intentional for now.

use super::effects::{dispatch_effect, EffectInput, EffectResult, SpellEffectType};
use super::hit::{self, SpellHitOutcome};
use crate::World;
use anyhow::Result;
use oxcore_shared::protocol::ObjectGuid;

/// SPELL_EFFECT_PERSISTENT_AREA_AURA — applied once per aura holder, not per unit target
/// here, so its bit is always stripped from the effect mask (matches MaNGOS chunk_0).
const SPELL_EFFECT_PERSISTENT_AREA_AURA: u32 = 27;

/// Per-target proc flags accumulated while applying a target's effects.
/// Reuses the same bit space as [`crate::game::player::auras::proc::proc_flags`].
pub use crate::game::player::auras::proc::proc_flags;

/// Per-target bookkeeping for one spell cast against one unit target.
///
/// Equivalent of MaNGOS `TargetInfo` plus the `procAttacker`/`procVictim`/`procEx`/
/// `m_damage`/`m_healing`/`m_absorbed` locals that `Spell::DoAllEffectOnTarget` threads
/// through a single target's effect processing.
#[derive(Debug, Clone)]
pub struct TargetInfo {
    pub target_guid: ObjectGuid,
    /// Bitmask of effect indices (bit i = effect i) requested for this target.
    pub effect_mask: u8,
    /// Hit/miss/resist/immune outcome, resolved once for the whole target.
    pub miss_condition: SpellHitOutcome,
    /// Set once this target's effects have been applied; a second call is a no-op
    /// (mirrors `TargetInfo::processed`).
    pub processed: bool,
    /// Attacker-side proc flags for this target (`Spell::m_procAttacker` per-target view).
    pub proc_attacker: u32,
    /// Victim-side proc flags for this target (`Spell::m_procVictim` per-target view).
    pub proc_victim: u32,
    /// Direct damage accumulated across this target's effects (`Spell::m_damage` equivalent).
    pub damage: u32,
    /// Healing accumulated across this target's effects (`Spell::m_healing` equivalent).
    pub healing: u32,
    /// Damage absorbed across this target's effects (`Spell::m_absorbed` equivalent).
    pub absorbed: u32,
}

impl TargetInfo {
    pub fn new(target_guid: ObjectGuid, effect_mask: u8) -> Self {
        Self {
            target_guid,
            effect_mask,
            miss_condition: SpellHitOutcome::Hit,
            processed: false,
            proc_attacker: proc_flags::NONE,
            proc_victim: proc_flags::NONE,
            damage: 0,
            healing: 0,
            absorbed: 0,
        }
    }

    /// Clear the per-target effect result accumulators before processing this target.
    pub fn reset_effect_damage_and_heal(&mut self) {
        self.damage = 0;
        self.healing = 0;
        self.absorbed = 0;
    }
}

/// Apply a spell's effects to a single target (faithful `Spell::DoAllEffectOnTarget`).
///
/// Resolves the hit outcome once, strips persistent-area-aura bits from the effect
/// mask (those are applied once per aura holder elsewhere), and on a hit dispatches
/// every requested effect for this target, accumulating damage/healing into `target`.
///
/// Divergence from the C++: effect handlers (`effects::damage`, `effects::healing`, ...)
/// already call the real `caster::deal_damage` / `caster::deal_heal` themselves as they
/// run, instead of this function collecting `m_damage`/`m_healing` and applying them once
/// at the end. `target.damage` / `target.healing` are still accumulated here for
/// target-level visibility (and so `proc_attacker`/`proc_victim` can be set the way
/// MaNGOS sets them once per target), but they are not re-applied — doing so would
/// double-apply damage that the effect handler already dealt. Migrating every damage/heal
/// effect handler to a "compute only" model so this function can be the single apply
/// point is future work.
pub async fn apply_target_effects(
    target: &mut TargetInfo,
    caster_guid: ObjectGuid,
    spell_id: u32,
    is_triggered: bool,
    custom_base_points: Option<[Option<i32>; 3]>,
    world: &World,
) -> Result<Vec<EffectResult>> {
    let mut results = Vec::new();

    if target.processed {
        return Ok(results);
    }
    target.processed = true;
    target.reset_effect_damage_and_heal();

    let spell_entry = match world.managers.spell_mgr.get(spell_id) {
        Some(entry) => entry,
        None => {
            tracing::warn!("Spell {} not found", spell_id);
            return Ok(results);
        }
    };

    // Strip persistent-area-aura bits (chunk_0: cleared before resolving the unit/hit).
    for effect_index in 0..3 {
        if spell_entry.effect[effect_index] == SPELL_EFFECT_PERSISTENT_AREA_AURA {
            target.effect_mask &= !(1 << effect_index);
        }
    }
    if target.effect_mask == 0 {
        return Ok(results);
    }

    // Resolve the hit outcome once for the whole target (not once per effect — the old
    // per-effect dispatch loop rolled a fresh hit per effect index, which could both
    // double-roll and desync the miss packet/AI event from the effects actually applied).
    target.miss_condition = if is_triggered || target.target_guid == caster_guid {
        SpellHitOutcome::Hit
    } else {
        hit::roll_spell_hit(caster_guid, target.target_guid, spell_id, world)
    };

    match target.miss_condition {
        SpellHitOutcome::Miss | SpellHitOutcome::Resist | SpellHitOutcome::Immune => {
            let miss_info = match target.miss_condition {
                SpellHitOutcome::Miss => hit::SpellMissInfo::Miss,
                SpellHitOutcome::Resist => hit::SpellMissInfo::Resist,
                SpellHitOutcome::Immune => hit::SpellMissInfo::Immune,
                _ => unreachable!(),
            };
            world.systems.spells.send_spell_miss(
                caster_guid,
                target.target_guid,
                spell_id,
                miss_info,
            );

            if !spell_entry.is_positive_spell() {
                enter_combat_on_miss(caster_guid, target.target_guid, world);
            }
            return Ok(results);
        }
        SpellHitOutcome::Reflect => {
            // Reflecting the effect back onto the caster is not yet supported; the
            // original dispatch loop had the same limitation (see effects/mod.rs history).
            tracing::debug!(
                "[SPELL-HIT] spell {} REFLECTED by {:?} (reflect-to-caster not yet supported)",
                spell_id,
                target.target_guid
            );
            return Ok(results);
        }
        SpellHitOutcome::Hit | SpellHitOutcome::PartialResist(_) => {
            fire_spell_hit_ai_event(
                caster_guid,
                target.target_guid,
                spell_id,
                &spell_entry,
                world,
            );
        }
    }

    // Per-target proc flags: helpful spells proc DEAL/TAKE_HELPFUL, harmful spells proc
    // DEAL/TAKE_HARMFUL (+ TAKEN_ANY_DAMAGE on the victim side). Simplified stand-in for
    // MaNGOS's `m_procAttacker`/`m_procVictim` (which start from attack-type-derived flags
    // and get adjusted by NEGATIVE_TRIGGER_MASK / secondary-target rules).
    if spell_entry.is_positive_spell() {
        target.proc_attacker = proc_flags::DEAL_HELPFUL_SPELL;
        target.proc_victim = proc_flags::TAKE_HELPFUL_SPELL;
    } else {
        target.proc_attacker = proc_flags::DEAL_HARMFUL_SPELL;
        target.proc_victim = proc_flags::TAKE_HARMFUL_SPELL | proc_flags::TAKEN_ANY_DAMAGE;
    }

    for effect_index in 0..3usize {
        if target.effect_mask & (1 << effect_index) == 0 {
            continue;
        }
        let effect_type = spell_entry.effect[effect_index];
        if effect_type == 0 {
            continue;
        }
        let Some(effect_type_enum) = SpellEffectType::from_u32(effect_type) else {
            tracing::warn!("Unknown effect type {} for spell {}", effect_type, spell_id);
            continue;
        };

        let base_value = custom_base_points
            .and_then(|bp| bp[effect_index])
            .unwrap_or(spell_entry.effect_base_points[effect_index]);
        let input = EffectInput {
            caster_guid,
            target_guid: Some(target.target_guid),
            spell_id,
            effect_index: effect_index as u8,
            base_value,
            misc_value: spell_entry.effect_misc_value[effect_index],
            misc_value_b: 0,
            is_triggered,
            die_sides: spell_entry.effect_die_sides[effect_index],
            points_per_level: spell_entry.effect_real_points_per_level[effect_index],
            spell_coefficient: spell_entry.effect_bonus_coefficient[effect_index],
            spell_school: spell_entry.school as u8,
            casting_time_ms: world
                .dbc
                .read()
                .get_spell_cast_time(spell_entry.casting_time_index)
                .map(|entry| entry.cast_time.max(0) as u32)
                .unwrap_or(0),
        };

        match dispatch_effect(effect_type_enum, &input, world).await {
            Ok(result) => {
                target.damage = target.damage.saturating_add(result.damage);
                target.healing = target.healing.saturating_add(result.healing);
                results.push(result);
            }
            Err(e) => {
                tracing::error!(
                    "Effect {} failed for spell {} target {:?}: {}",
                    effect_index,
                    spell_id,
                    target.target_guid,
                    e
                );
                results.push(EffectResult::empty());
            }
        }
    }

    Ok(results)
}

/// Put the caster and a non-positive-spell miss target into combat (MaNGOS: `unit->AttackedBy`,
/// `AddThreat`, `SetInCombatWithAggressor`/`SetInCombatWithVictim` on a miss/resist/immune).
///
/// Simplified: registers zero threat on creature victims (enough to aggro without
/// double-counting damage threat, which is added separately when a hit actually lands)
/// and flags player victims as in combat. Full `SPELL_ATTR_EX_FAILURE_BREAKS_STEALTH`/
/// stealth-removal handling is not ported yet.
fn enter_combat_on_miss(caster_guid: ObjectGuid, target_guid: ObjectGuid, world: &World) {
    if target_guid.is_creature() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        world
            .managers
            .creature_mgr
            .apply_damage(target_guid, 0, caster_guid, timestamp);
    } else if target_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |p| p.combat.in_combat = true);
    }
}

/// Fire the AI SpellHit event for a creature target on a landed hit (once per target,
/// not once per effect — matches the `effect_index == 0` guard the old per-effect
/// dispatch loop used to avoid duplicate callbacks).
fn fire_spell_hit_ai_event(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    spell_id: u32,
    spell_entry: &crate::dbc::structures::SpellEntry,
    world: &World,
) {
    let is_creature = world
        .managers
        .creature_mgr
        .with_creature(target_guid, |_| ())
        .is_some();
    if !is_creature {
        return;
    }
    crate::game::creature::ai::queue_event(
        world,
        target_guid,
        crate::game::creature::ai::AIEvent::SpellHit {
            caster_guid,
            spell_id,
            spell_is_positive: spell_entry.is_positive_spell(),
            spell_is_direct_damage: spell_entry.is_direct_damage_spell(),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_effect_damage_and_heal_clears_all_accumulators() {
        let mut target = TargetInfo::new(ObjectGuid::empty(), 0);
        target.damage = 150;
        target.healing = 75;
        target.absorbed = 25;

        target.reset_effect_damage_and_heal();

        assert_eq!(target.damage, 0);
        assert_eq!(target.healing, 0);
        assert_eq!(target.absorbed, 0);
    }

    #[test]
    fn reset_effect_damage_and_heal_is_unconditional() {
        let mut target = TargetInfo::new(ObjectGuid::empty(), 0);
        target.processed = true;
        target.damage = 1;
        target.healing = 1;
        target.absorbed = 1;

        target.reset_effect_damage_and_heal();

        assert_eq!((target.damage, target.healing, target.absorbed), (0, 0, 0));
    }
}
