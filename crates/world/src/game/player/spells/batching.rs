//! Spell effect batching delay.
//!
//! Vanilla spell batching groups spell effects so they execute on the same
//! world tick. This module computes the per-effect delay used when scheduling
//! a spell effect for batched application.

use crate::World;
use oxcore_shared::protocol::ObjectGuid;

/// `SPELL_ATTR_EX_ONLY_PEACEFUL_TARGETS` — peaceful-target spells skip the
/// batching delay against creatures so they can apply immediately (e.g. Sap
/// before the creature reacts).
pub const SPELL_ATTR_EX_ONLY_PEACEFUL_TARGETS: u32 = 0x0000_0100;

/// Pure helper mirroring `World::GetDelayUntilNextSpellBatchingInterval`.
///
/// Returns `interval_ms - (now_ms % interval_ms)`, or `0` when batching is
/// disabled (`interval_ms == 0`). `now_ms` is the wrapping 32-bit world timer
/// used by the reference implementation (`WorldTimer::getMSTime()`), so the
/// result is the milliseconds remaining until the next batching boundary.
pub fn delay_until_next_batching_interval_ms(interval_ms: u32, now_ms: u32) -> u32 {
    if interval_ms == 0 {
        return 0;
    }
    interval_ms - (now_ms % interval_ms)
}

/// Current world timer as a wrapping 32-bit millisecond counter.
///
/// Mirrors `WorldTimer::getMSTime()`. The absolute epoch is irrelevant because
/// spell batching only consumes the value modulo the configured interval.
fn world_timer_ms() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u32
}

/// Pure decision gate from `Spell::GetSpellBatchingEffectDelay`.
///
/// Returns the configured batching interval when the effect should be delayed,
/// or `0` when it should apply immediately:
///
/// 1. batching is disabled (`interval_ms == 0`);
/// 2. the target is the caster unit and the effect has no chain target; or
/// 3. the spell has `SPELL_ATTR_EX_ONLY_PEACEFUL_TARGETS` and the target is a creature.
///
/// The returned `interval_ms` is later fed into
/// [`delay_until_next_batching_interval_ms`] to get the actual remaining delay.
pub fn spell_batching_effect_delay_ms(
    interval_ms: u32,
    target_guid: ObjectGuid,
    caster_unit_guid: ObjectGuid,
    effect_chain_target: u32,
    target_is_creature: bool,
    only_peaceful_targets: bool,
) -> u32 {
    if interval_ms == 0 {
        return 0;
    }
    if target_guid == caster_unit_guid && effect_chain_target == 0 {
        return 0;
    }
    if target_is_creature && only_peaceful_targets {
        return 0;
    }
    interval_ms
}

/// World-coupled entry for `Spell::GetSpellBatchingEffectDelay`.
///
/// Reads the configured `Spell.EffectDelay` interval and the current world timer,
/// applies the three early-exit branches, and returns the delay in milliseconds
/// until the next batching boundary.
pub fn get_spell_batching_effect_delay(
    world: &World,
    target_guid: ObjectGuid,
    caster_unit_guid: ObjectGuid,
    effect_chain_target: u32,
    target_is_creature: bool,
    only_peaceful_targets: bool,
) -> u32 {
    let interval_ms = world.config.spell_effect_delay_ms;
    let delay = spell_batching_effect_delay_ms(
        interval_ms,
        target_guid,
        caster_unit_guid,
        effect_chain_target,
        target_is_creature,
        only_peaceful_targets,
    );
    if delay == 0 {
        return 0;
    }
    delay_until_next_batching_interval_ms(interval_ms, world_timer_ms())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_until_next_boundary_basic() {
        // interval 400, now 150 -> 250ms until next boundary.
        assert_eq!(delay_until_next_batching_interval_ms(400, 150), 250);
        // On a boundary: result is the full interval (matches C++ formula).
        assert_eq!(delay_until_next_batching_interval_ms(400, 400), 400);
        // At time 0: full interval.
        assert_eq!(delay_until_next_batching_interval_ms(400, 0), 400);
    }

    #[test]
    fn delay_until_next_boundary_disabled() {
        assert_eq!(delay_until_next_batching_interval_ms(0, 12345), 0);
    }

    #[test]
    fn delay_until_next_boundary_wraps_modulo() {
        // u32 wrapping: 400 * 10_000_000 = 4_000_000_000, remainder 150.
        let now = 4_000_000_150u32;
        assert_eq!(delay_until_next_batching_interval_ms(400, now), 250);
    }

    #[test]
    fn batching_disabled_returns_zero() {
        let caster = ObjectGuid::new_player(1);
        let target = ObjectGuid::new_player(2);
        assert_eq!(
            spell_batching_effect_delay_ms(0, target, caster, 0, false, false),
            0
        );
    }

    #[test]
    fn self_target_without_chain_returns_zero() {
        let caster = ObjectGuid::new_player(1);
        assert_eq!(
            spell_batching_effect_delay_ms(400, caster, caster, 0, false, false),
            0
        );
    }

    #[test]
    fn self_target_with_chain_is_delayed() {
        let caster = ObjectGuid::new_player(1);
        assert_eq!(
            spell_batching_effect_delay_ms(400, caster, caster, 1, false, false),
            400
        );
    }

    #[test]
    fn different_target_is_delayed() {
        let caster = ObjectGuid::new_player(1);
        let target = ObjectGuid::new_player(2);
        assert_eq!(
            spell_batching_effect_delay_ms(400, target, caster, 0, false, false),
            400
        );
    }

    #[test]
    fn peaceful_target_spell_on_creature_returns_zero() {
        let caster = ObjectGuid::new_player(1);
        let creature = ObjectGuid::new_creature(1, 5);
        assert_eq!(
            spell_batching_effect_delay_ms(400, creature, caster, 0, true, true),
            0
        );
    }

    #[test]
    fn peaceful_target_spell_on_player_is_delayed() {
        let caster = ObjectGuid::new_player(1);
        let target = ObjectGuid::new_player(2);
        assert_eq!(
            spell_batching_effect_delay_ms(400, target, caster, 0, false, true),
            400
        );
    }

    #[test]
    fn non_peaceful_spell_on_creature_is_delayed() {
        let caster = ObjectGuid::new_player(1);
        let creature = ObjectGuid::new_creature(1, 5);
        assert_eq!(
            spell_batching_effect_delay_ms(400, creature, caster, 0, true, false),
            400
        );
    }

    #[test]
    fn gate_combinations_preserve_priority() {
        let caster = ObjectGuid::new_player(1);
        let creature = ObjectGuid::new_creature(1, 5);
        // Both self-target and peaceful-on-creature: disabled interval wins.
        assert_eq!(
            spell_batching_effect_delay_ms(0, creature, caster, 0, true, true),
            0
        );
        // Self-target with chain and peaceful-on-creature: self-target (no chain) not applicable,
        // peaceful branch wins -> 0.
        assert_eq!(
            spell_batching_effect_delay_ms(400, creature, caster, 1, true, true),
            0
        );
    }

    #[tokio::test]
    async fn world_coupled_entry_respects_interval_and_branches() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        let target = ObjectGuid::new_player(2);

        // Batching enabled: should return a positive delay <= interval.
        let delay = get_spell_batching_effect_delay(&world, target, caster, 0, false, false);
        assert!(delay > 0 && delay <= 400);

        // Disabled branch: returns 0.
        let zero = get_spell_batching_effect_delay(&world, target, caster, 0, true, true);
        assert_eq!(zero, 0);
    }

    fn test_world() -> World {
        use oxcore_shared::database::Databases;
        use sqlx::mysql::MySqlPoolOptions;
        use std::path::PathBuf;
        use std::sync::Arc;

        let pool = MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool should be constructible");
        let databases = Arc::new(Databases {
            world: pool.clone(),
            character: pool.clone(),
            auth: pool.clone(),
            logs: pool,
        });
        World::new(
            databases,
            Arc::new(crate::config::Config::default()),
            50,
            PathBuf::from("."),
        )
    }
}
