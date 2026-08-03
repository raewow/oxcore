//! Bonus-flat threat application per target.
//!
//! Reads the `spell_threat` flat bonus for the cast spell and, for every hit target,
//! either distributes that bonus to mobs hostile toward the
//! target (beneficial spells) or adds it directly to the target's threat list
//! (harmful spells). The bonus is written straight into the affected creatures'
//! [`ThreatManager`]s; the returned list records what was applied for inspection.

use crate::game::player::auras::proc::negative_effect_mask as build_negative_effect_mask;
use crate::game::player::spells::hit::SpellHitOutcome;
use crate::game::player::spells::target_info::TargetInfo;
use crate::game::spell::manager::SpellThreatEntry;
use crate::World;
use oxcore_shared::protocol::ObjectGuid;

/// School-damage effect targeting the caster counts as
/// negative even though the per-effect polarity check may read it positive.
const SPELL_EFFECT_SCHOOL_DAMAGE: u32 = 2;
/// Implicit-target A value for targeting the caster.
const TARGET_UNIT_CASTER: u32 = 1;

#[cfg(doc)]
use crate::game::creature::combat::threat::ThreatManager;

/// Spell polarity derived from intersecting the negative-effect mask with the
/// active-effect mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// No overlap: beneficial spell — bonus threat is distributed to mobs hostile
    /// toward the target (healing-style assist threat).
    Positive,
    /// Negative mask covers every active effect: harmful spell — bonus threat goes
    /// directly onto the target.
    Negative,
    /// Partial overlap only: mixed positive/negative spell — no bonus threat is
    /// applied to any target.
    Mixed,
}

/// Active-effect bitmask: bit `i` set iff
/// `effects[i] != 0`, for `i` in `0..MAX_EFFECT_INDEX` (3 effects).
pub fn build_effect_mask(effects: &[u32; 3]) -> u8 {
    let mut mask = 0u8;
    for i in 0..3 {
        if effects[i] != 0 {
            mask |= 1 << i;
        }
    }
    mask
}

/// `((~inverseEffectMask) & mask) != 0`. A target's own effect mask qualifies when
/// at least one of its effect bits is outside the threat entry's inverse mask.
pub fn can_cause_threat_on_mask(inverse_effect_mask: u32, mask: u8) -> bool {
    ((!inverse_effect_mask) & (mask as u32)) != 0
}

/// Decide a spell's threat polarity from the negative-effect mask and the active
/// effect mask.
///
/// - `Negative` when `negative & effect == effect` (the negative mask covers every
///   active effect);
/// - `Mixed` when `negative & effect != 0` but does not cover all active effects;
/// - `Positive` when there is no overlap.
pub fn spell_polarity(negative_effect_mask: u8, effect_mask: u8) -> Polarity {
    let overlap = negative_effect_mask & effect_mask;
    if overlap == 0 {
        Polarity::Positive
    } else if overlap == effect_mask {
        Polarity::Negative
    } else {
        Polarity::Mixed
    }
}

/// One flat-threat application that was applied to a single target. Records the
/// per-target write [`handle_threat_spells`] made into the threat system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThreatApplication {
    pub target_guid: ObjectGuid,
    pub amount: f32,
    pub positive: bool,
}

/// Resolve a spell's `effect` array from the world's `SpellManager`, defaulting to
/// an all-zero array when the spell is unknown.
fn spell_effects(spell_id: u32, world: &World) -> [u32; 3] {
    world
        .managers
        .spell_mgr
        .get(spell_id)
        .map(|entry| entry.effect)
        .unwrap_or([0; 3])
}

/// The caster resolves to a loaded player or creature.
fn caster_exists(caster_guid: ObjectGuid, world: &World) -> bool {
    if caster_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |_| ())
            .is_some()
    } else if caster_guid.is_creature() || caster_guid.is_pet() {
        world
            .managers
            .creature_mgr
            .with_creature(caster_guid, |_| ())
            .is_some()
    } else {
        false
    }
}

/// Resolve a non-caster target — present in the world
/// as a loaded unit. The caster target is always considered resolved (it is
/// the caster itself).
fn target_resolved(target_guid: ObjectGuid, is_caster_self: bool, world: &World) -> bool {
    if is_caster_self {
        return true;
    }
    if target_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player(target_guid, |_| ())
            .is_some()
    } else if target_guid.is_creature() || target_guid.is_pet() {
        world
            .managers
            .creature_mgr
            .with_creature(target_guid, |_| ())
            .is_some()
    } else {
        false
    }
}

/// Only creatures carry a threat list; players never do.
fn can_have_threat_list(target_guid: ObjectGuid, world: &World) -> bool {
    target_guid.is_creature()
        && world
            .managers
            .creature_mgr
            .with_creature(target_guid, |_| ())
            .is_some()
}

/// Negative path: add the flat bonus onto
/// the hostile creature's threat list toward the caster.
fn add_direct_threat(caster_guid: ObjectGuid, target_guid: ObjectGuid, threat: f32, world: &World) {
    world
        .managers
        .creature_mgr
        .with_creature_mut(target_guid, |c| {
            c.threat_manager.add_threat(caster_guid, threat);
        });
}

/// Positive path: split the flat bonus evenly across every creature that is
/// threatening the assisted unit.
///
/// The assisted unit is threatened by a set of creatures; each share is added to the
/// corresponding creature's threat list, attributed to the caster (the healer/buffer).
/// We model that by scanning for the creatures that currently have `assisted` on
/// their threat list.
///
/// Approximation: the per-creature school/aura scaling of the share is not applied —
/// only the flat `threat / count` share is distributed.
fn apply_assist_threat(
    caster_guid: ObjectGuid,
    assisted_guid: ObjectGuid,
    threat: f32,
    world: &World,
) {
    let attackers: Vec<ObjectGuid> = world
        .managers
        .creature_mgr
        .iter_creatures()
        .filter(|c| c.threat_manager.has_target(assisted_guid))
        .map(|c| *c.key())
        .collect();

    let count = attackers.len();
    if count == 0 {
        return;
    }
    let share = threat / count as f32;
    for attacker in attackers {
        world
            .managers
            .creature_mgr
            .with_creature_mut(attacker, |c| {
                c.threat_manager.add_threat(caster_guid, share);
            });
    }
}

/// Applies the `spell_threat` flat bonus for this cast to every eligible hit target:
/// the negative path adds threat directly onto a hostile creature's threat list,
/// while the positive path distributes the bonus to the mobs fighting the assisted
/// ally. The returned [`ThreatApplication`] list records what was applied.
///
/// `negative_effect_mask` is not yet a field on `ActiveCast`/`TargetInfo`, so it is
/// threaded in as a parameter.
/// `threat_entry`/`inverse_effect_mask` stand in for the loaded spell-threat entry
/// and its `inverse_effect_mask` member — the `SpellManager` getter is not exposed
/// yet.
// TODO: fold `inverse_effect_mask` onto `SpellThreatEntry` once it is loaded from data,
// then drop the parameter here and in `apply_cast_threat`.
pub fn handle_threat_spells(
    caster_guid: ObjectGuid,
    spell_id: u32,
    negative_effect_mask: u8,
    targets: &[TargetInfo],
    threat_entry: Option<&SpellThreatEntry>,
    inverse_effect_mask: u32,
    world: &World,
) -> Vec<ThreatApplication> {
    if !caster_exists(caster_guid, world) {
        return Vec::new();
    }
    if targets.is_empty() {
        return Vec::new();
    }

    let Some(threat_entry) = threat_entry else {
        return Vec::new();
    };
    if threat_entry.flat == 0 {
        return Vec::new();
    }

    let threat = threat_entry.flat as f32;

    let effect_mask = build_effect_mask(&spell_effects(spell_id, world));
    let polarity = spell_polarity(negative_effect_mask, effect_mask);
    if polarity == Polarity::Mixed {
        tracing::debug!(
            target: "spell_cast",
            spell_id,
            effect_mask,
            negative_effect_mask,
            "HandleThreatSpells: mixed positive/negative spell — skipping bonus threat",
        );
        return Vec::new();
    }
    let positive = polarity == Polarity::Positive;

    let mut applications = Vec::new();
    for ihit in targets {
        if !ihit.miss_condition.is_hit() {
            continue;
        }
        if !can_cause_threat_on_mask(inverse_effect_mask, ihit.effect_mask) {
            continue;
        }
        let is_caster_self = ihit.target_guid == caster_guid;
        if !target_resolved(ihit.target_guid, is_caster_self, world) {
            continue;
        }

        if positive {
            // Positive path: distribute the flat bonus to mobs hostile toward the target.
            apply_assist_threat(caster_guid, ihit.target_guid, threat, world);
            applications.push(ThreatApplication {
                target_guid: ihit.target_guid,
                amount: threat,
                positive: true,
            });
        } else {
            // Negative path: skip targets that cannot carry a threat list, then
            // add the threat directly.
            if !can_have_threat_list(ihit.target_guid, world) {
                continue;
            }
            add_direct_threat(caster_guid, ihit.target_guid, threat, world);
            applications.push(ThreatApplication {
                target_guid: ihit.target_guid,
                amount: threat,
                positive: false,
            });
        }
    }

    tracing::debug!(
        target: "spell_cast",
        spell_id,
        threat,
        kind = if positive { "assisting" } else { "harming" },
        count = applications.len(),
        "HandleThreatSpells applied flat bonus threat",
    );
    applications
}

/// Once-per-cast entry point that wires [`handle_threat_spells`] into the live effect
/// pipeline. Called from `EffectsDispatcher::dispatch_with_targets` after every target's
/// effects have been applied — once per cast, not per target.
///
/// Resolves the `spell_threat` flat bonus from the [`crate::game::spell::manager::SpellManager`]
/// and derives the negative-effect mask from the spell proto's per-effect positivity
/// (school-damage-on-caster effects counting as negative). The threat entry's
/// `inverse_effect_mask` is not modelled on the Rust [`SpellThreatEntry`] yet, so `0`
/// is passed (any active effect bit set then qualifies). If no threat entry is
/// configured for the spell this returns without touching the threat system.
pub(crate) fn apply_cast_threat(
    caster_guid: ObjectGuid,
    spell_id: u32,
    targets: &[TargetInfo],
    world: &World,
) -> Vec<ThreatApplication> {
    let Some(threat_entry) = world.managers.spell_mgr.get_spell_threat_entry(spell_id) else {
        return Vec::new();
    };

    let negative_effect_mask = world
        .managers
        .spell_mgr
        .get(spell_id)
        .map(|entry| {
            build_negative_effect_mask::<3>(
                |i| entry.is_positive_effect(i),
                |i| {
                    entry.effect[i] == SPELL_EFFECT_SCHOOL_DAMAGE
                        && entry.effect_implicit_target_a[i] == TARGET_UNIT_CASTER
                },
            )
        })
        .unwrap_or(0);

    handle_threat_spells(
        caster_guid,
        spell_id,
        negative_effect_mask,
        targets,
        Some(&threat_entry),
        0,
        world,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::dbc::structures::SpellEntry;
    use crate::game::creature::creature::Creature;
    use crate::game::creature::manager::CreatureTemplate;
    use crate::game::player::player::Player;
    use oxcore_db::database::Databases;
    use oxcore_shared::protocol::Position;
    use sqlx::postgres::PgPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn lazy_pool() -> sqlx::PgPool {
        PgPoolOptions::new()
            .connect_lazy("postgres://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    fn test_world() -> World {
        let databases = Arc::new(Databases {
            world: lazy_pool(),
            character: lazy_pool(),
            auth: lazy_pool(),
            logs: oxcore_db::database::lazy_logs_pool(),
        });
        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    fn threat_entry(flat: i32) -> SpellThreatEntry {
        SpellThreatEntry {
            flat,
            pct: 0.0,
            ap_bonus: 0.0,
        }
    }

    fn spell_with_effects(id: u32, effects: [u32; 3]) -> SpellEntry {
        let mut entry = base_spell(id);
        entry.effect = effects;
        entry
    }

    fn base_spell(id: u32) -> SpellEntry {
        SpellEntry {
            id,
            name: format!("S{id}"),
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

    fn creature_template(entry: u32) -> CreatureTemplate {
        CreatureTemplate {
            entry,
            name: format!("C{entry}"),
            subname: None,
            min_level: 50,
            max_level: 50,
            faction: 1,
            model_id_1: 1,
            model_id_2: 0,
            model_id_3: 0,
            model_id_4: 0,
            scale: 1.0,
            npc_flags: 0,
            unit_flags: 0,
            static_flags1: 0,
            flags_extra: 0,
            creature_type: 1,
            unit_class: 1,
            health_multiplier: 1.0,
            power_multiplier: 1.0,
            armor_multiplier: 1.0,
            damage_multiplier: 1.0,
            damage_variance: 0.0,
            attack_time: 2000,
            rank: 0,
            gossip_menu_id: 0,
            vendor_id: 0,
            trainer_id: 0,
            trainer_type: 0,
            spells: [0; 4],
        }
    }

    fn creature(guid: ObjectGuid, entry: u32) -> Creature {
        Creature::new(
            guid,
            entry,
            1,
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                o: 0.0,
            },
            1,
            0,
            &creature_template(entry),
            1,
            None,
        )
    }

    fn add_player(world: &World, guid: ObjectGuid) {
        world.managers.player_mgr.add_player(
            Player::new(guid, format!("P{}", guid.counter()), 1, 0, 0, 60, 1, 1, 0),
            guid.counter(),
        );
    }

    fn add_creature(world: &World, guid: ObjectGuid, entry: u32) {
        world
            .managers
            .creature_mgr
            .add_template(creature_template(entry));
        world
            .managers
            .creature_mgr
            .add_creature_for_test(creature(guid, entry));
    }

    fn target_info(guid: ObjectGuid, effect_mask: u8) -> TargetInfo {
        let mut ti = TargetInfo::new(guid, effect_mask);
        ti.miss_condition = SpellHitOutcome::Hit;
        ti
    }

    // === pure-helper tests (no DB) ===

    #[test]
    fn build_effect_mask_sets_bit_per_nonzero_effect() {
        assert_eq!(build_effect_mask(&[0, 0, 0]), 0b000);
        assert_eq!(build_effect_mask(&[2, 0, 0]), 0b001);
        assert_eq!(build_effect_mask(&[0, 36, 0]), 0b010);
        assert_eq!(build_effect_mask(&[0, 0, 6]), 0b100);
        assert_eq!(build_effect_mask(&[2, 36, 6]), 0b111);
    }

    #[test]
    fn can_cause_threat_on_mask_matches_mangos_formula() {
        // inverse=0 → ~0 is all-ones, any nonzero mask qualifies.
        assert!(can_cause_threat_on_mask(0, 0b001));
        assert!(can_cause_threat_on_mask(0, 0b111));
        // A zero mask never qualifies.
        assert!(!can_cause_threat_on_mask(0, 0));
        // inverse covers the mask entirely → disqualified.
        assert!(!can_cause_threat_on_mask(0b111, 0b111));
        // Partial inverse: a bit outside the inverse mask still qualifies.
        assert!(can_cause_threat_on_mask(0b001, 0b010));
        assert!(can_cause_threat_on_mask(0b001, 0b011));
        // All of the mask's bits are inside the inverse mask → disqualified.
        assert!(!can_cause_threat_on_mask(0b011, 0b010));
    }

    #[test]
    fn spell_polarity_positive_when_no_overlap() {
        assert_eq!(spell_polarity(0b000, 0b111), Polarity::Positive);
        assert_eq!(spell_polarity(0b100, 0b011), Polarity::Positive);
    }

    #[test]
    fn spell_polarity_negative_when_full_cover() {
        assert_eq!(spell_polarity(0b111, 0b111), Polarity::Negative);
        assert_eq!(spell_polarity(0b011, 0b011), Polarity::Negative);
        // Extra negative bits beyond the active effects still count as full cover.
        assert_eq!(spell_polarity(0b111, 0b001), Polarity::Negative);
    }

    #[test]
    fn spell_polarity_mixed_when_partial_overlap() {
        assert_eq!(spell_polarity(0b001, 0b011), Polarity::Mixed);
        assert_eq!(spell_polarity(0b011, 0b101), Polarity::Mixed);
    }

    // === world-coupled plan tests (lazy pool, no DB connection) ===

    #[tokio::test]
    async fn empty_targets_returns_empty() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        add_player(&world, caster);
        let apps = handle_threat_spells(caster, 9001, 0, &[], Some(&threat_entry(50)), 0, &world);
        assert!(apps.is_empty());
    }

    #[tokio::test]
    async fn caster_null_returns_empty() {
        let world = test_world();
        // Caster guid never registered → resolves to no caster.
        let caster = ObjectGuid::new_player(99);
        let target = ObjectGuid::new_creature(1, 1);
        let apps = handle_threat_spells(
            caster,
            9002,
            0,
            &[target_info(target, 0b001)],
            Some(&threat_entry(50)),
            0,
            &world,
        );
        assert!(apps.is_empty());
    }

    #[tokio::test]
    async fn missing_threat_entry_returns_empty() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        add_player(&world, caster);
        let apps = handle_threat_spells(
            caster,
            9003,
            0,
            &[target_info(ObjectGuid::new_creature(1, 1), 0b001)],
            None,
            0,
            &world,
        );
        assert!(apps.is_empty());
    }

    #[tokio::test]
    async fn zero_flat_threat_returns_empty() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        add_player(&world, caster);
        let apps = handle_threat_spells(
            caster,
            9004,
            0,
            &[target_info(ObjectGuid::new_creature(1, 1), 0b001)],
            Some(&threat_entry(0)),
            0,
            &world,
        );
        assert!(apps.is_empty());
    }

    #[tokio::test]
    async fn mixed_spell_skips_bonus_threat_entirely() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        add_player(&world, caster);
        // Two active effects (bits 0 and 1); only bit 0 is negative → mixed.
        world
            .managers
            .spell_mgr
            .add_spell(spell_with_effects(9100, [2, 36, 0]));
        let t1 = ObjectGuid::new_creature(1, 1);
        let t2 = ObjectGuid::new_creature(1, 2);
        add_creature(&world, t1, 1);
        add_creature(&world, t2, 1);

        let apps = handle_threat_spells(
            caster,
            9100,
            0b001,
            &[target_info(t1, 0b011), target_info(t2, 0b011)],
            Some(&threat_entry(50)),
            0,
            &world,
        );
        assert!(
            apps.is_empty(),
            "mixed spell must not apply any bonus threat"
        );
    }

    #[tokio::test]
    async fn miss_condition_skip_excludes_target() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        add_player(&world, caster);
        world
            .managers
            .spell_mgr
            .add_spell(spell_with_effects(9200, [2, 0, 0]));
        let hit_target = ObjectGuid::new_creature(1, 1);
        let miss_target = ObjectGuid::new_creature(1, 2);
        add_creature(&world, hit_target, 1);
        add_creature(&world, miss_target, 1);

        let mut missed = target_info(miss_target, 0b001);
        missed.miss_condition = SpellHitOutcome::Miss;

        let apps = handle_threat_spells(
            caster,
            9200,
            0b001,
            &[target_info(hit_target, 0b001), missed],
            Some(&threat_entry(50)),
            0,
            &world,
        );
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].target_guid, hit_target);
        assert!(!apps[0].positive);
        assert_eq!(apps[0].amount, 50.0);
    }

    #[tokio::test]
    async fn negative_path_skips_player_targets_without_threat_list() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        add_player(&world, caster);
        world
            .managers
            .spell_mgr
            .add_spell(spell_with_effects(9300, [2, 0, 0]));
        let creature_target = ObjectGuid::new_creature(1, 1);
        let player_target = ObjectGuid::new_player(2);
        add_creature(&world, creature_target, 1);
        add_player(&world, player_target);

        let apps = handle_threat_spells(
            caster,
            9300,
            0b001,
            &[
                target_info(creature_target, 0b001),
                target_info(player_target, 0b001),
            ],
            Some(&threat_entry(50)),
            0,
            &world,
        );
        // Only the creature target can carry a threat list.
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].target_guid, creature_target);
        assert!(!apps[0].positive);
    }

    #[tokio::test]
    async fn positive_path_applies_to_player_and_creature_targets() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        add_player(&world, caster);
        // Healing-style effect (36 = APPLY_AURA) with no negative bits → positive.
        world
            .managers
            .spell_mgr
            .add_spell(spell_with_effects(9400, [36, 0, 0]));
        let player_target = ObjectGuid::new_player(2);
        let creature_target = ObjectGuid::new_creature(1, 1);
        add_player(&world, player_target);
        add_creature(&world, creature_target, 1);
        // Caster-self is also a valid positive target.
        let self_target = target_info(caster, 0b001);

        let apps = handle_threat_spells(
            caster,
            9400,
            0,
            &[
                target_info(player_target, 0b001),
                target_info(creature_target, 0b001),
                self_target,
            ],
            Some(&threat_entry(50)),
            0,
            &world,
        );
        assert_eq!(apps.len(), 3);
        assert!(apps.iter().all(|a| a.positive && a.amount == 50.0));
    }

    #[tokio::test]
    async fn can_cause_threat_on_mask_filter_excludes_disqualified_targets() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        add_player(&world, caster);
        world
            .managers
            .spell_mgr
            .add_spell(spell_with_effects(9500, [2, 0, 0]));
        let qualifies = ObjectGuid::new_creature(1, 1);
        let disqualified = ObjectGuid::new_creature(1, 2);
        add_creature(&world, qualifies, 1);
        add_creature(&world, disqualified, 1);

        // inverse_effect_mask = 0b001 → a target whose effect mask is only bit 0 is
        // fully inside the inverse mask (disqualified); bit 1 alone qualifies.
        let apps = handle_threat_spells(
            caster,
            9500,
            0b001,
            &[
                target_info(qualifies, 0b010),
                target_info(disqualified, 0b001),
            ],
            Some(&threat_entry(50)),
            0b001,
            &world,
        );
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].target_guid, qualifies);
    }

    #[tokio::test]
    async fn unresolved_non_caster_target_is_skipped() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        add_player(&world, caster);
        world
            .managers
            .spell_mgr
            .add_spell(spell_with_effects(9600, [2, 0, 0]));
        let present = ObjectGuid::new_creature(1, 1);
        let absent = ObjectGuid::new_creature(1, 2);
        add_creature(&world, present, 1);
        // `absent` is never registered → resolves to no unit.

        let apps = handle_threat_spells(
            caster,
            9600,
            0b001,
            &[target_info(present, 0b001), target_info(absent, 0b001)],
            Some(&threat_entry(50)),
            0,
            &world,
        );
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].target_guid, present);
    }

    // === threat-system write tests (wiring into ThreatManager) ===

    #[tokio::test]
    async fn negative_path_writes_flat_threat_onto_target() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        add_player(&world, caster);
        world
            .managers
            .spell_mgr
            .add_spell(spell_with_effects(9700, [2, 0, 0]));
        let target = ObjectGuid::new_creature(1, 1);
        add_creature(&world, target, 1);

        let apps = handle_threat_spells(
            caster,
            9700,
            0b001,
            &[target_info(target, 0b001)],
            Some(&threat_entry(50)),
            0,
            &world,
        );
        assert_eq!(apps.len(), 1);
        // The flat bonus is now live on the target creature's threat list.
        let threat = world
            .managers
            .creature_mgr
            .with_creature(target, |c| c.threat_manager.get_threat(caster))
            .unwrap();
        assert_eq!(threat, 50.0);
    }

    #[tokio::test]
    async fn positive_path_distributes_assist_threat_to_attackers() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        add_player(&world, caster);
        // Healing-style effect (36 = APPLY_AURA), no negative bits → positive.
        world
            .managers
            .spell_mgr
            .add_spell(spell_with_effects(9800, [36, 0, 0]));
        let assisted = ObjectGuid::new_player(2);
        add_player(&world, assisted);

        // Two creatures currently fighting the assisted ally.
        let attacker_a = ObjectGuid::new_creature(1, 1);
        let attacker_b = ObjectGuid::new_creature(1, 2);
        add_creature(&world, attacker_a, 1);
        add_creature(&world, attacker_b, 1);
        for atk in [attacker_a, attacker_b] {
            world
                .managers
                .creature_mgr
                .with_creature_mut(atk, |c| c.threat_manager.add_threat(assisted, 100.0));
        }

        let apps = handle_threat_spells(
            caster,
            9800,
            0,
            &[target_info(assisted, 0b001)],
            Some(&threat_entry(60)),
            0,
            &world,
        );
        assert_eq!(apps.len(), 1);
        assert!(apps[0].positive);

        // Flat bonus (60) split evenly across the two attackers → 30 each, attributed
        // to the caster.
        for atk in [attacker_a, attacker_b] {
            let threat = world
                .managers
                .creature_mgr
                .with_creature(atk, |c| c.threat_manager.get_threat(caster))
                .unwrap();
            assert_eq!(threat, 30.0);
        }
    }

    #[tokio::test]
    async fn positive_path_no_attackers_writes_nothing() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        add_player(&world, caster);
        world
            .managers
            .spell_mgr
            .add_spell(spell_with_effects(9900, [36, 0, 0]));
        let assisted = ObjectGuid::new_player(2);
        add_player(&world, assisted);
        // A creature exists but is not threatening the assisted ally.
        let bystander = ObjectGuid::new_creature(1, 1);
        add_creature(&world, bystander, 1);

        let apps = handle_threat_spells(
            caster,
            9900,
            0,
            &[target_info(assisted, 0b001)],
            Some(&threat_entry(60)),
            0,
            &world,
        );
        assert_eq!(apps.len(), 1);
        let threat = world
            .managers
            .creature_mgr
            .with_creature(bystander, |c| c.threat_manager.get_threat(caster))
            .unwrap();
        assert_eq!(threat, 0.0);
    }

    // === once-per-cast wiring (`apply_cast_threat`) ===

    #[tokio::test]
    async fn apply_cast_threat_writes_threat_after_negative_cast() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        add_player(&world, caster);
        // School-damage effect (2) with default caster target → negative-polarity spell.
        world
            .managers
            .spell_mgr
            .add_spell(spell_with_effects(9950, [2, 0, 0]));
        // A `spell_threat` flat bonus is now configured for this spell in the manager,
        // so the wired call resolves it without any parameters threaded in.
        world
            .managers
            .spell_mgr
            .add_spell_threat(9950, threat_entry(50));
        let target = ObjectGuid::new_creature(1, 1);
        add_creature(&world, target, 1);

        // Targets as `dispatch_with_targets` hands them over: resolved hit + effect mask.
        let apps = apply_cast_threat(caster, 9950, &[target_info(target, 0b001)], &world);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].target_guid, target);
        assert!(!apps[0].positive);

        // Threat is live on the creature's threat list toward the caster.
        let threat = world
            .managers
            .creature_mgr
            .with_creature(target, |c| c.threat_manager.get_threat(caster))
            .unwrap();
        assert_eq!(threat, 50.0);
    }

    #[tokio::test]
    async fn apply_cast_threat_without_entry_writes_nothing() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        add_player(&world, caster);
        world
            .managers
            .spell_mgr
            .add_spell(spell_with_effects(9960, [2, 0, 0]));
        // No `spell_threat` entry configured → nothing to distribute.
        let target = ObjectGuid::new_creature(1, 1);
        add_creature(&world, target, 1);

        let apps = apply_cast_threat(caster, 9960, &[target_info(target, 0b001)], &world);
        assert!(apps.is_empty());
        let threat = world
            .managers
            .creature_mgr
            .with_creature(target, |c| c.threat_manager.get_threat(caster))
            .unwrap();
        assert_eq!(threat, 0.0);
    }
}
