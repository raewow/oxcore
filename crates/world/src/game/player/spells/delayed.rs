//! Spell cast pushback from damage taken (cast-bar partial interruption on a
//! currently preparing, non-channeled spell when the caster takes damage).

use crate::game::player::spells::modifiers::apply_spell_modifiers_to_value;
use crate::game::player::spells::state::{SpellMod, SpellModOp, SpellState};
use crate::World;
use oxcore_shared::protocol::ObjectGuid;

/// `SPELL_INTERRUPT_FLAG_DAMAGE_PUSHBACK` — spell loses casting time on damage.
/// Spelled out as a bit so callers can also test it directly on `SpellEntry::interrupt_flags`.
const SPELL_INTERRUPT_FLAG_DAMAGE_PUSHBACK: u32 = 0x02;

/// `SPELL_AURA_RESIST_PUSHBACK` — Concentration Aura-style reduction to cast pushback.
const SPELL_AURA_RESIST_PUSHBACK: u32 = 68;

/// Vanilla opcode the original code sends to the caster after applying a delay.
#[allow(dead_code)]
const SMSG_SPELL_DELAYED: u32 = 482;

/// Snapshot of the per-cast fields the decision reads.
///
/// The active-cast struct does not yet carry the running damage-count used to
/// derive the per-hit delay, so the caller owns [`DelayCastInput::delay_at_damage_count`]
/// and threads it through each call.
#[derive(Debug, Clone, Copy)]
pub struct DelayCastInput {
    /// `m_spellInfo->Id`.
    pub spell_id: u32,
    /// `m_spellState`.
    pub state: SpellState,
    /// `m_timer` (current remaining cast time, ms).
    pub timer: u32,
    /// `m_casttime` (original cast time of this cast, ms).
    pub casttime: u32,
    /// `m_spellInfo->InterruptFlags`.
    pub interrupt_flags: u32,
    /// `m_spellInfo->SpellFamilyName` — gates the `SPELLMOD_NOT_LOSE_CASTING_TIME` modifiers.
    pub spell_family_name: u32,
    /// `m_spellInfo->SpellFamilyFlags`.
    pub spell_family_flags: u64,
    /// Running count of damage hits that have already triggered pushback for this cast.
    /// Starts at 0; each call computes its delay from this count and then increments it.
    pub delay_at_damage_count: u32,
    /// `resistChance` after `SPELLMOD_NOT_LOSE_CASTING_TIME` has been applied to a base of 100.
    pub resist_chance_after_spell_mods: i32,
    /// `m_casterUnit->GetTotalAuraModifier(SPELL_AURA_RESIST_PUSHBACK)`.
    pub resist_pushback_aura_mod: i32,
}

/// Outcome of evaluating the pushback decision for a single damage event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PushbackDecision {
    /// Whether the delay was actually applied (the caller should persist `new_timer`).
    pub applied: bool,
    /// Whether the resist roll successfully resisted the pushback.
    pub resisted: bool,
    /// Per-hit `delaytime` (0 when an early guard fired before a delay was computed).
    pub delaytime: u32,
    /// New `m_timer` after pushback (clamped to `casttime`).
    pub new_timer: u32,
    /// `m_delayAtDamageCount` after this call (incremented only when a delay is computed).
    pub new_count: u32,
}

impl PushbackDecision {
    /// No-op outcome for a cast that did not reach the delay phase.
    fn passthrough(timer: u32, count: u32) -> Self {
        Self {
            applied: false,
            resisted: false,
            delaytime: 0,
            new_timer: timer,
            new_count: count,
        }
    }

    /// Resisted-the-roll outcome: nothing applied, count unchanged.
    fn resisted(timer: u32, count: u32) -> Self {
        Self {
            applied: false,
            resisted: true,
            delaytime: 0,
            new_timer: timer,
            new_count: count,
        }
    }
}

/// Pure `GetNextDelayAtDamageMsTime`: `(1000 - count*200).max(200)`.
///
/// For `count` starting at 0 this yields 1000, 800, 600, 400, 200, 200, ...
/// Rather than mutating a counter, the pure helper takes the current count and
/// returns the per-hit delay; the increment is the caller's responsibility
/// (see [`push_back_decision`]).
pub fn get_next_delay_at_damage_ms_time(count: u32) -> u32 {
    1000u32.saturating_sub(count.saturating_mul(200)).max(200)
}

/// Pure clamp for `m_timer += delaytime`: returns the new timer, never exceeding `casttime`.
///
/// If `timer + delaytime` would overshoot `casttime`, the timer is pinned at `casttime`.
pub fn apply_pushback_to_timer(timer: u32, casttime: u32, delaytime: u32) -> u32 {
    timer.saturating_add(delaytime).min(casttime)
}

/// Pure `roll_chance_i` predicate with the supplied `irand(0,99)` roll.
///
/// Returns true when `chance > roll` (i.e. the pushback is resisted).
pub fn roll_chance_i_with(chance: i32, roll: i32) -> bool {
    chance > roll
}

/// Pure final resist chance after the aura modifier.
///
/// Mirrors `resistChance += GetTotalAuraModifier(SPELL_AURA_RESIST_PUSHBACK) - 100`:
/// the caster's pushback-resist aura total above 100 increases the resist chance.
pub fn compute_resist_chance(
    resist_chance_after_spell_mods: i32,
    resist_pushback_aura_mod: i32,
) -> i32 {
    resist_chance_after_spell_mods + (resist_pushback_aura_mod - 100)
}

/// `m_spellInfo->InterruptFlags & SPELL_INTERRUPT_FLAG_DAMAGE_PUSHBACK`.
pub fn has_damage_pushback_flag(interrupt_flags: u32) -> bool {
    interrupt_flags & SPELL_INTERRUPT_FLAG_DAMAGE_PUSHBACK != 0
}

/// Pure decision mirroring the body of the original method.
///
/// Order of guards, faithful to the spec:
/// 1. If the cast is already delayed, no pushback (can't time-back a delayed cast).
/// 2. If the spell is not flagged for damage pushback, no pushback.
/// 3. Roll the resist chance; on success the pushback is resisted (no delay).
/// 4. Compute the per-hit delay from the running count and apply it to the timer.
///
/// `roll` is `irand(0,99)` — supply a fixed value for deterministic tests.
pub fn push_back_decision(input: &DelayCastInput, roll: i32) -> PushbackDecision {
    if input.state == SpellState::Delayed {
        return PushbackDecision::passthrough(input.timer, input.delay_at_damage_count);
    }

    if !has_damage_pushback_flag(input.interrupt_flags) {
        return PushbackDecision::passthrough(input.timer, input.delay_at_damage_count);
    }

    let resist_chance = compute_resist_chance(
        input.resist_chance_after_spell_mods,
        input.resist_pushback_aura_mod,
    );

    if roll_chance_i_with(resist_chance, roll) {
        return PushbackDecision::resisted(input.timer, input.delay_at_damage_count);
    }

    let new_count = input.delay_at_damage_count + 1;
    let delaytime = get_next_delay_at_damage_ms_time(input.delay_at_damage_count);
    let new_timer = apply_pushback_to_timer(input.timer, input.casttime, delaytime);

    PushbackDecision {
        applied: true,
        resisted: false,
        delaytime,
        new_timer,
        new_count,
    }
}

/// `Player::ApplySpellMod(m_spellInfo->Id, SPELLMOD_NOT_LOSE_CASTING_TIME, &resistChance)` —
/// applied to an initial base of 100, returning the post-modifier value.
pub fn apply_not_lose_casting_time_mod(
    modifiers: &[SpellMod],
    spell_family_name: u32,
    spell_family_flags: u64,
) -> i32 {
    apply_spell_modifiers_to_value(
        modifiers,
        SpellModOp::NotLoseCastTime,
        100,
        spell_family_name,
        spell_family_flags,
    )
}

/// World-coupled entry.
///
/// Applies cast-bar pushback to a currently preparing (non-channeled) spell in
/// the caster's Generic slot when the caster (a player) takes damage. The
/// caller owns `delay_at_damage_count` and must persist the returned
/// `decision.new_count` between calls for the same cast.
///
/// Mirrors every branch of the original method's guards and arithmetic; the
/// `SMSG_SPELL_DELAYED` packet send is not yet wired.
pub fn delayed(
    caster_guid: ObjectGuid,
    delay_at_damage_count: &mut u32,
    world: &World,
) -> PushbackDecision {
    // Guard: caster must be a player.
    if !caster_guid.is_player() {
        return PushbackDecision::passthrough(0, *delay_at_damage_count);
    }

    // Acquire the player and process the cast in one critical section so a
    // concurrent cancel cannot observe a half-applied pushback.
    let decision = world
        .systems
        .player
        .manager()
        .with_player_mut(caster_guid, |player| {
            // Only the Generic (non-channeled) slot is subject to damage pushback.
            let active = match player
                .spells
                .current_spells
                [crate::game::player::spells::state::CurrentSpellType::Generic as usize]
                .as_mut()
            {
                Some(cast) => cast,
                None => return PushbackDecision::passthrough(0, *delay_at_damage_count),
            };

            // Look up the spell entry for its InterruptFlags / family fields.
            let spell_entry = match world.managers.spell_mgr.get(active.spell_id) {
                Some(entry) => entry,
                None => return PushbackDecision::passthrough(active.cast_time_remaining_ms, *delay_at_damage_count),
            };

            // Step 1: resistChance = 100; Step 2: ApplySpellMod with SPELLMOD_NOT_LOSE_CASTING_TIME.
            let resist_chance_after_spell_mods = apply_not_lose_casting_time_mod(
                &player.spells.spell_modifiers,
                spell_entry.spell_family_name,
                spell_entry.spell_family_flags,
            );

            // Step 3: += GetTotalAuraModifier(SPELL_AURA_RESIST_PUSHBACK) - 100.
            let resist_pushback_aura_mod = player
                .auras
                .container
                .get_total_aura_modifier(SPELL_AURA_RESIST_PUSHBACK);

            let input = DelayCastInput {
                spell_id: active.spell_id,
                state: active.state,
                timer: active.cast_time_remaining_ms,
                casttime: active.original_cast_time_ms,
                interrupt_flags: spell_entry.interrupt_flags,
                spell_family_name: spell_entry.spell_family_name,
                spell_family_flags: spell_entry.spell_family_flags,
                delay_at_damage_count: *delay_at_damage_count,
                resist_chance_after_spell_mods,
                resist_pushback_aura_mod,
            };

            // `irand(0,99)`.
            let roll = (rand::random::<u32>() % 100) as i32;
            let decision = push_back_decision(&input, roll);

            if decision.applied {
                active.cast_time_remaining_ms = decision.new_timer;
                active.total_pushback_ms = active
                    .total_pushback_ms
                    .saturating_add(decision.delaytime);

                tracing::debug!(
                    "Spell pushback (delayed): spell {} pushed back {}ms (timer {} -> {})",
                    active.spell_id,
                    decision.delaytime,
                    input.timer,
                    decision.new_timer
                );

                // packet send not yet wired — the original builds
                // WorldPacket(SMSG_SPELL_DELAYED) writing caster ObjectGuid (8 bytes)
                // + uint32 delaytime and sends it to the caster.
            }

            decision
        });

    match decision {
        Some(d) => {
            *delay_at_damage_count = d.new_count;
            d
        }
        None => PushbackDecision::passthrough(0, *delay_at_damage_count),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input(timer: u32, casttime: u32, count: u32) -> DelayCastInput {
        DelayCastInput {
            spell_id: 1234,
            state: SpellState::Preparing,
            timer,
            casttime,
            interrupt_flags: SPELL_INTERRUPT_FLAG_DAMAGE_PUSHBACK,
            spell_family_name: 0,
            spell_family_flags: 0,
            delay_at_damage_count: count,
            resist_chance_after_spell_mods: 100,
            resist_pushback_aura_mod: 100,
        }
    }

    #[test]
    fn next_delay_sequence_first_six_then_floor() {
        assert_eq!(get_next_delay_at_damage_ms_time(0), 1000);
        assert_eq!(get_next_delay_at_damage_ms_time(1), 800);
        assert_eq!(get_next_delay_at_damage_ms_time(2), 600);
        assert_eq!(get_next_delay_at_damage_ms_time(3), 400);
        assert_eq!(get_next_delay_at_damage_ms_time(4), 200);
        assert_eq!(get_next_delay_at_damage_ms_time(5), 200);
        assert_eq!(get_next_delay_at_damage_ms_time(6), 200);
    }

    #[test]
    fn timer_clamp_at_casttime() {
        assert_eq!(apply_pushback_to_timer(800, 1000, 300), 1000);
        assert_eq!(apply_pushback_to_timer(900, 1000, 200), 1000);
        assert_eq!(apply_pushback_to_timer(1000, 1000, 100), 1000);
    }

    #[test]
    fn timer_non_clamp_adds_delaytime() {
        assert_eq!(apply_pushback_to_timer(300, 1000, 400), 700);
        assert_eq!(apply_pushback_to_timer(0, 5000, 1000), 1000);
    }

    #[test]
    fn saturating_timer_overflow_pins_to_casttime() {
        assert_eq!(apply_pushback_to_timer(u32::MAX, u32::MAX, 1), u32::MAX);
    }

    #[test]
    fn flag_gate_no_delay_when_pushback_flag_absent() {
        let mut input = base_input(300, 1000, 0);
        input.interrupt_flags = 0;
        let decision = push_back_decision(&input, 99);
        assert!(!decision.applied);
        assert!(!decision.resisted);
        assert_eq!(decision.delaytime, 0);
        assert_eq!(decision.new_timer, 300);
        assert_eq!(decision.new_count, 0);
    }

    #[test]
    fn already_delayed_state_guard_no_delay() {
        let mut input = base_input(300, 1000, 0);
        input.state = SpellState::Delayed;
        let decision = push_back_decision(&input, 99);
        assert!(!decision.applied);
        assert!(!decision.resisted);
        assert_eq!(decision.delaytime, 0);
        assert_eq!(decision.new_timer, 300);
        assert_eq!(decision.new_count, 0);
    }

    #[test]
    fn finished_state_is_not_delayed_and_still_pushes_back() {
        let mut input = base_input(300, 1000, 0);
        input.state = SpellState::Finished;
        let decision = push_back_decision(&input, 100);
        assert!(decision.applied);
        assert_eq!(decision.delaytime, 1000);
        assert_eq!(decision.new_timer, 1000);
        assert_eq!(decision.new_count, 1);
    }

    #[tokio::test]
    async fn non_player_guard_passthrough() {
        let creature = ObjectGuid::new_creature(1, 1);
        let mut count = 5;
        let decision = delayed(creature, &mut count, dummy_world());
        // Scorekeeper-style: any non-player GUID is short-circuited before
        // touching the manager, so the count is unchanged.
        assert!(!decision.applied);
        assert_eq!(count, 5);
    }

    #[test]
    fn resist_chance_arithmetic_aura_above_100_increases_chance() {
        // Base after spell mods of 100, aura total 170 → final 170.
        assert_eq!(compute_resist_chance(100, 170), 170);
        // Aura total of exactly 100 → net zero change.
        assert_eq!(compute_resist_chance(100, 100), 100);
        // Aura below 100 reduces the chance.
        assert_eq!(compute_resist_chance(100, 50), 50);
    }

    #[test]
    fn resist_roll_resists_pushback() {
        let input = base_input(300, 1000, 0);
        // final resist chance = 100 + (100 - 100) = 100; roll 99 < 100 → resisted.
        let decision = push_back_decision(&input, 99);
        assert!(decision.resisted);
        assert!(!decision.applied);
        assert_eq!(decision.delaytime, 0);
        assert_eq!(decision.new_timer, 300);
        assert_eq!(decision.new_count, 0);
    }

    #[test]
    fn high_roll_does_not_resist_and_pushback_applies() {
        let input = base_input(300, 1000, 0);
        // final chance 100; roll 100 NOT > roll 100 (strictly greater), so pushback applies.
        let decision = push_back_decision(&input, 100);
        assert!(decision.applied);
        assert!(!decision.resisted);
        assert_eq!(decision.delaytime, 1000);
        assert_eq!(decision.new_timer, 1000); // 300 + 1000 > 1000 → clamp.
        assert_eq!(decision.new_count, 1);
    }

    #[test]
    fn decision_applies_incrementing_count_across_calls() {
        let mut input = base_input(0, 5000, 0);
        // Hit 1: 1000ms delay.
        let d1 = push_back_decision(&input, 200);
        assert!(d1.applied);
        assert_eq!(d1.delaytime, 1000);
        assert_eq!(d1.new_timer, 1000);
        assert_eq!(d1.new_count, 1);

        // Hit 2: 800ms delay, timer carries the prior pushback.
        input.delay_at_damage_count = d1.new_count;
        input.timer = d1.new_timer;
        let d2 = push_back_decision(&input, 200);
        assert!(d2.applied);
        assert_eq!(d2.delaytime, 800);
        assert_eq!(d2.new_timer, 1800);
        assert_eq!(d2.new_count, 2);

        // Hit 5: still 200ms (floor), count continues to climb.
        input.delay_at_damage_count = 4;
        input.timer = 2000;
        let d5 = push_back_decision(&input, 200);
        assert_eq!(d5.delaytime, 200);
        assert_eq!(d5.new_count, 5);
        assert_eq!(d5.new_timer, 2200);
    }

    #[test]
    fn roll_chance_i_strict_inequality() {
        // chance > roll, strictly.
        assert!(roll_chance_i_with(100, 99));
        assert!(!roll_chance_i_with(100, 100));
        // Negative chance never beats any roll >= 0.
        assert!(!roll_chance_i_with(-1, 0));
    }

    #[test]
    fn flag_bit_detection() {
        assert!(has_damage_pushback_flag(0x02));
        assert!(has_damage_pushback_flag(0x02 | 0x10));
        assert!(!has_damage_pushback_flag(0x00));
        assert!(!has_damage_pushback_flag(0x10));
    }

    /// A throwaway World is only used to satisfy the non-player guard, which
    /// short-circuits before ever touching the player manager, so it never
    /// needs a real player in this test.
    fn dummy_world() -> &'static World {
        // The non-player guard returns before dereferencing `world`, so we can
        // hand it a leaked zero-world to satisfy the type. Constructing one for
        // real requires a lazy DB pool mirroring the targets.rs test helper;
        // we only reach the player-manager path when the GUID is a player, which
        // this test never exercises.
        static WORLD: std::sync::OnceLock<World> = std::sync::OnceLock::new();
        WORLD.get_or_init(|| build_dummy_world())
    }

    fn build_dummy_world() -> World {
        use std::sync::Arc;
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool should be constructible");
        let databases = Arc::new(oxcore_shared::database::Databases {
            world: pool.clone(),
            character: pool.clone(),
            auth: pool.clone(),
            logs: pool,
        });
        World::new(
            databases,
            Arc::new(crate::config::Config::default()),
            50,
            std::path::PathBuf::from("."),
        )
    }
}
