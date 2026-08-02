//! Per-target spell effect application.
//!
//! Each unit hit by a spell gets exactly one `TargetInfo`: the hit/miss/resist outcome
//! is resolved once for the whole target (not once per effect), the requested effect
//! mask is carried across every effect applied to that target, and per-target proc
//! flags/accumulated damage & healing are tracked across the effect pipeline.
//!
//! Effect handlers in `effects/*` still apply their own damage/heal immediately
//! (via `caster::deal_damage` / `caster::deal_heal`) rather than being deferred to a
//! single post-loop apply — see the module doc on
//! [`apply_target_effects`] for why that divergence is intentional for now.

use super::diminishing::{self, DiminishSnapshot};
use super::effects::{dispatch_effect, EffectInput, EffectResult, SpellEffectType};
use super::hit::{self, SpellHitOutcome};
use crate::dbc::structures::SpellEntry;
use crate::game::player::auras::proc;
use crate::World;
use anyhow::Result;
use oxcore_shared::protocol::ObjectGuid;

// ─── Spell attribute constants ─────────────────────────────────────────

/// Cannot be used in combat.
const SPELL_ATTR_NOT_IN_COMBAT_ONLY_PEACEFUL: u32 = 0x1000_0000; // bit 28

/// Does not break stealth.
const SPELL_ATTR_EX_ALLOW_WHILE_STEALTHED: u32 = 0x0000_0020; // bit 5

/// No threat generated.
const SPELL_ATTR_EX_NO_THREAT: u32 = 0x0000_0400; // bit 10

/// Target must not be in combat.
const SPELL_ATTR_EX_ONLY_PEACEFUL_TARGETS: u32 = 0x0000_0100; // bit 8

/// Threat only when the spell misses.
const SPELL_ATTR_EX_THREAT_ONLY_ON_MISS: u32 = 0x0020_0000; // bit 21

/// Does not break invisibility.
const SPELL_ATTR_EX2_ALLOW_WHILE_INVISIBLE: u32 = 0x0000_4000; // bit 14

/// Not considered an action.
const SPELL_ATTR_EX2_NOT_AN_ACTION: u32 = 0x1000_0000; // bit 28

/// No initial threat on cast.
const SPELL_ATTR_EX2_NO_INITIAL_THREAT: u32 = 0x0040_0000; // bit 22

/// Counts as hostile for PvP even without combat.
const SPELL_ATTR_EX3_PVP_ENABLING: u32 = 0x0000_0001; // bit 0

/// Bypasses damage and school immunities.
const SPELL_ATTR_NO_IMMUNITIES: u32 = 0x2000_0000;

/// Aura drops when its bearer is hit by a hostile spell.
const AURA_INTERRUPT_HOSTILE_ACTION_RECEIVED_CANCELS: u32 = 0x0000_0001;

// ─── Aura type constants ──────────────────────────────────────────────────────

/// Possess aura type (2).
const SPELL_AURA_MOD_POSSESS: u32 = 2;
/// Stealth aura type (16).
const SPELL_AURA_MOD_STEALTH: u32 = 16;
/// Invisibility aura type (18).
const SPELL_AURA_MOD_INVISIBILITY: u32 = 18;
/// Dispel effect (38).
const SPELL_EFFECT_DISPEL: u32 = 38;
/// School-damage effect (2).
const SPELL_EFFECT_SCHOOL_DAMAGE: u32 = 2;
/// Possess-pet aura type (128).
const SPELL_AURA_MOD_POSSESS_PET: u32 = 128;

/// Persistent-area-aura effect: applied once per aura holder, not per unit target
/// here, so its bit is always stripped from the effect mask.
const SPELL_EFFECT_PERSISTENT_AREA_AURA: u32 = 27;

/// Per-target proc flags accumulated while applying a target's effects.
/// Reuses the same bit space as [`crate::game::player::auras::proc::proc_flags`].
pub use crate::game::player::auras::proc::proc_flags;

/// Per-target bookkeeping for one spell cast against one unit target.
#[derive(Debug, Clone)]
pub struct TargetInfo {
    pub target_guid: ObjectGuid,
    /// Bitmask of effect indices (bit i = effect i) requested for this target.
    pub effect_mask: u8,
    /// Hit/miss/resist/immune outcome, resolved once for the whole target.
    pub miss_condition: SpellHitOutcome,
    /// Outcome of the *reflected* cast back onto the caster.
    /// Only meaningful when `miss_condition` is [`SpellHitOutcome::Reflect`]; `Hit` means
    /// the reflected spell lands on the caster.
    pub reflect_result: SpellHitOutcome,
    /// Whether `miss_condition` has already been rolled for this cast. Delayed
    /// projectiles resolve at launch and carry this snapshot to impact.
    pub outcome_resolved: bool,
    /// Set once this target's effects have been applied; a second call is a no-op.
    pub processed: bool,
    /// Attacker-side proc flags for this target.
    pub proc_attacker: u32,
    /// Victim-side proc flags for this target.
    pub proc_victim: u32,
    /// Direct damage accumulated across this target's effects.
    pub damage: u32,
    /// Healing accumulated across this target's effects.
    pub healing: u32,
    /// Damage absorbed across this target's effects.
    pub absorbed: u32,
    /// Diminishing-returns decision sampled when this target was hit, shared by every
    /// aura the hit applies.
    pub diminishing: DiminishSnapshot,
}

impl TargetInfo {
    pub fn new(target_guid: ObjectGuid, effect_mask: u8) -> Self {
        Self {
            target_guid,
            effect_mask,
            miss_condition: SpellHitOutcome::Hit,
            reflect_result: SpellHitOutcome::Hit,
            outcome_resolved: false,
            processed: false,
            proc_attacker: proc_flags::NONE,
            proc_victim: proc_flags::NONE,
            damage: 0,
            healing: 0,
            absorbed: 0,
            diminishing: DiminishSnapshot::default(),
        }
    }

    /// Clear the per-target effect result accumulators before processing this target.
    pub fn reset_effect_damage_and_heal(&mut self) {
        self.damage = 0;
        self.healing = 0;
        self.absorbed = 0;
    }
}

/// Per-unit pre-effect processing.
///
/// Called on a landed hit, **before** dispatching individual effects.  Handles:
///
/// 1. **Zero-effect-mask** — when `effect_mask` is zero but the spell is not
///    positive toward the target, still flags combat (here: `apply_damage(0)`
///    for creatures, `in_combat = true` for players).
///
/// 2. **Per-effect mechanic resistance** — each effect bit whose mechanic the
///    target resists is cleared.  When the mask becomes zero the hit is aborted.
///    *Approximated:* the per-effect mechanic-resist check is not yet ported;
///    this step is omitted until that helper exists — see TODO.
///
/// 3. **Delayed-spell immunity/evasion** — if the caster is not the target and
///    the spell has `speed > 0`, and the target is immune to the spell or its
///    damage school, the hit is aborted as immune.  Also, delayed negative spells
///    against friendly targets (post-duel) are aborted as evaded.  *Approximated:*
///    the immunity checks are not ported; the immunity branches are structured
///    but inert until those dependency APIs land.
///
/// 4. **Stealth/invisibility removal** — on hostile hits, removes stealth and
///    non-passive invisibility auras from the target, unless the spell's
///    `attributes_ex`/`attributes_ex2` carry the allow-while-stealthed /
///    allow-while-invisible flags.  Game-object casters (traps, etc.) skip stealth
///    removal.  The same removal runs against the *caster* once the hit passes the
///    threat gate.  Direct-damage hits also drop the target's
///    hostile-action-received-cancels auras (player targets only, since
///    interrupt-flag removal is not generalised to creatures yet).
///
/// 5. **Visibility check for delayed spells** — a delayed spell targeting the
///    explicit unit target that has become non-visible to the caster is evaded.
///    *Approximated:* visibility detection not ported; skip until the visibility
///    API lands.
///
/// 6. **Combat/threat entry** — complex attribute-gated combat start, stealth
///    removal from the *caster*, and related PvP-enabling fallthrough.
///    *Approximated:* uses `enter_combat_on_miss`-style `apply_damage(0)`
///    and `add_threat` / `set_in_combat` for creatures when the gate passes.
///    The whole hostile/friendly block is skipped when caster and target are the
///    same unit — a reflected cast lands back on its caster and must not put it
///    in combat with itself.
///
/// 7. **Friendly-target assist/PvP** — when a friendly target is in combat and
///    the spell would generate threat, the caster enters assisted combat and
///    distributes assist-threat.  PvP flagging is not ported.
///
/// 8. **Diminishing-returns snapshot + aura-holder creation** — stashes the DR
///    group/level for the hit and creates an empty aura holder when the spell
///    applies auras.  *Aura-holder creation not yet wired;* the effect mask starts
///    as the set of bits that survived resist checks, and later effect handling
///    runs only for those bits.  This function modifies `effect_mask` in-place.
///
/// # Returns
/// `None` — hit should be aborted (all effects resisted, delayed-spell immune
/// or evaded).  `Some(snapshot)` — continue with effect dispatch, carrying the
/// diminishing-returns decision every aura this hit applies must share.
async fn do_spell_hit_on_unit(
    spell_entry: &SpellEntry,
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    effect_mask: &mut u8,
    is_delayed: bool,
    is_triggered: bool,
    is_reflected: bool,
    world: &World,
) -> Option<DiminishSnapshot> {
    // ── Step 3.  Zero effect mask: just flag combat if non-positive ────────────
    if *effect_mask == 0 {
        if !spell_entry.is_positive_spell() {
            enter_combat_on_miss(caster_guid, target_guid, world);
        }
        return None;
    }

    // ── Step 4.  Per-effect mechanic resistance ───────────────────────────────
    // An effect-specific mechanic may be resisted independently of its sibling
    // effects when it differs from the spell-level mechanic.
    for effect_index in 0..3 {
        if *effect_mask & (1 << effect_index) != 0
            && target_resists_effect_mechanic(target_guid, spell_entry, effect_index, world)
        {
            *effect_mask &= !(1 << effect_index);
        }
    }
    if *effect_mask == 0 {
        return None;
    }

    if target_is_immune_to_school(target_guid, spell_entry, *effect_mask, world) {
        *effect_mask = 0;
        return None;
    }

    // ── Step 5.  Per-effect immunity re-check ─────────────────────────────────
    // Apply the common Unit aura immunity rules before dispatch, including for
    // delayed casts whose target state changed after target registration.
    for effect_index in 0..3 {
        if *effect_mask & (1 << effect_index) != 0
            && target_is_immune_to_spell_effect(target_guid, spell_entry, effect_index, world)
        {
            *effect_mask &= !(1 << effect_index);
        }
    }
    if *effect_mask == 0 {
        return None;
    }

    // ── Step 6a.  Hostile-side side effects ────────────────────────────────────
    // TODO: friendly-to-target check not ported — use `is_positive_spell()` as proxy.
    let caster_is_player = caster_guid.is_player();

    // A unit never breaks its own stealth or enters combat with itself. This matters for
    // reflected casts, which land back on the caster.
    if caster_guid == target_guid {
        return Some(snapshot_diminishing_for_hit(
            spell_entry,
            caster_guid,
            target_guid,
            *effect_mask,
            is_triggered,
            is_reflected,
            world,
        ));
    }

    if !spell_entry.is_positive_spell() {
        // Auras cancelled by taking a hostile action. The original gates this on
        // accumulated damage from the delayed launch; here the closest stand-in before
        // effects run is "this spell deals direct damage".
        if spell_entry.is_direct_damage_spell() {
            remove_auras_with_interrupt_flag_from_target(
                target_guid,
                AURA_INTERRUPT_HOSTILE_ACTION_RECEIVED_CANCELS,
                world,
            )
            .await;
        }

        // Stealth/invisibility removal on target.
        // Game-object caster check: if caster_guid is not a player, we approximate
        // as "caster is a game object" — traps etc. do not break stealth.
        let caster_is_object =
            !caster_is_player && !target_guid.is_creature() && !target_guid.is_player();

        if !caster_is_object && !spell_entry.has_attribute_ex2(SPELL_ATTR_EX2_NOT_AN_ACTION) {
            remove_stealth_and_invisibility(target_guid, spell_entry, world);
        }

        // Delayed-spell visibility check
        // TODO: visibility detection not ported — skip until it lands.

        // Combat/threat entry main gate
        let is_sap =
            spell_entry.spell_family_name == 8 && (spell_entry.spell_family_flags & 0x80) != 0; // Rogue-sap spell approximation
        let is_trap = !caster_is_player && target_guid.is_player();

        let can_enter_combat = (!spell_entry.is_positive_spell()
            || spell_entry.has_effect(SPELL_EFFECT_DISPEL))
            && !is_trap
            && !is_sap;

        // Visibility to target — approximated as always true until visibility detection lands.
        let caster_visible_to_target = true;

        if can_enter_combat && caster_visible_to_target {
            // Gate: not (triggered-by-aura without speed/threat)
            let can_threat =
                (!is_triggered || spell_entry.speed > 0.0 || has_direct_threat_effect(spell_entry))
                    && !spell_entry.has_attribute_ex(SPELL_ATTR_EX_NO_THREAT)
                    && !spell_entry.has_attribute_ex(SPELL_ATTR_EX_THREAT_ONLY_ON_MISS)
                    && !spell_entry.has_attribute_ex2(SPELL_ATTR_EX2_NO_INITIAL_THREAT);

            if can_threat {
                // The caster can be detected but still be carrying a stealth aura, so a
                // hostile action drops its own stealth/invisibility too.
                remove_stealth_and_invisibility(caster_guid, spell_entry, world);

                // Enter combat
                if !spell_has_aura_type(spell_entry, SPELL_AURA_MOD_POSSESS)
                    && !spell_has_aura_type(spell_entry, SPELL_AURA_MOD_POSSESS_PET)
                {
                    enter_combat_on_hit(caster_guid, target_guid, world);
                }
                set_caster_in_combat_with_victim(caster_guid, target_guid, world);
            } else if spell_entry.has_attribute_ex3(SPELL_ATTR_EX3_PVP_ENABLING) {
                // PvP-enabling only
                set_out_of_combat(caster_guid, target_guid, world);
            }
        } else {
            // Fallthrough: spell did not pass the combat-entry gate
            let not_only_peaceful = !spell_entry
                .has_attribute(SPELL_ATTR_NOT_IN_COMBAT_ONLY_PEACEFUL)
                || !spell_entry.has_attribute_ex(SPELL_ATTR_EX_ONLY_PEACEFUL_TARGETS);
            if not_only_peaceful {
                set_out_of_combat(caster_guid, target_guid, world);
            }
        }
    } else {
        // ── Step 6b.  Friendly-target side effects ────────────────────────────

        // Delayed negative spells on friendly targets (post-duel) → evade
        if is_delayed && !spell_entry.is_positive_spell() {
            return None;
        }

        // Combat assist
        // TODO: the in-combat check is not directly exposed — approximated via
        // checking `in_combat` on the target.
        let target_in_combat = world
            .managers
            .creature_mgr
            .with_creature(target_guid, |c| c.combat.in_combat)
            .unwrap_or(false)
            || world
                .systems
                .player
                .manager()
                .with_player(target_guid, |p| p.combat.in_combat)
                .unwrap_or(false);

        if target_in_combat
            && !spell_entry.has_attribute_ex(SPELL_ATTR_EX_NO_THREAT)
            && !is_triggered
            && !spell_entry.has_attribute_ex2(SPELL_ATTR_EX2_NO_INITIAL_THREAT)
        {
            assisted_combat(caster_guid, target_guid, world);
        }
        // TODO: PvP flagging not ported.
    }

    // ── Step 7.  Diminishing-returns snapshot ──────────────────────────────────
    // Sampled once per unit hit rather than per aura: a spell whose effect mask applies
    // several auras must give all of them one level and charge the target only once.
    // Aura-holder creation is still done by the aura system as each effect is applied,
    // so only the DR half of this block is ported.
    Some(snapshot_diminishing_for_hit(
        spell_entry,
        caster_guid,
        target_guid,
        *effect_mask,
        is_triggered,
        is_reflected,
        world,
    ))
}

/// Whether any effect in `effect_mask` applies an aura.
fn spell_applies_aura(spell_entry: &SpellEntry, effect_mask: u8) -> bool {
    const SPELL_EFFECT_APPLY_AURA: u32 = 6;
    const SPELL_EFFECT_PERSISTENT_AREA_AURA_EFFECT: u32 = 27;
    const SPELL_EFFECT_APPLY_AREA_AURA_PARTY: u32 = 65;

    (0..3).any(|i| {
        effect_mask & (1 << i) != 0
            && matches!(
                spell_entry.effect[i],
                SPELL_EFFECT_APPLY_AURA
                    | SPELL_EFFECT_PERSISTENT_AREA_AURA_EFFECT
                    | SPELL_EFFECT_APPLY_AREA_AURA_PARTY
            )
            && spell_entry.effect_apply_aura_name[i] != 0
    })
}

/// Take the per-hit diminishing-returns snapshot and charge the target's counter.
///
/// Divergences: aura-triggered status is approximated by `is_triggered`, and
/// the friendly-to-target check by the spell's polarity, which is the same proxy the
/// rest of this module uses. Only players and creatures carry DR state; any other caster
/// or target kind is treated as having none.
fn snapshot_diminishing_for_hit(
    spell_entry: &SpellEntry,
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    effect_mask: u8,
    is_triggered: bool,
    is_reflected: bool,
    world: &World,
) -> DiminishSnapshot {
    let group = diminishing::get_dr_group_for_spell(spell_entry, is_triggered);
    if diminishing::dr_type(group) == diminishing::DRType::None {
        return DiminishSnapshot::default();
    }

    let applies_aura = spell_applies_aura(spell_entry, effect_mask);
    let caster_is_friendly = spell_entry.is_positive_spell();
    let now = now_ms();

    let take = |state: &mut diminishing::DiminishingState, target_is_player_like: bool| {
        state.snapshot_for_hit(
            group,
            target_is_player_like,
            caster_guid.is_player(),
            applies_aura,
            caster_is_friendly,
            is_reflected,
            now,
        )
    };

    if target_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                take(&mut player.combat.diminishing, true)
            })
            .unwrap_or_default()
    } else if target_guid.is_creature() {
        world
            .managers
            .creature_mgr
            .with_creature_mut(target_guid, |creature| {
                take(&mut creature.combat.diminishing, false)
            })
            .unwrap_or_default()
    } else {
        DiminishSnapshot::default()
    }
}

/// Wall-clock milliseconds, the time base the diminishing counters are kept in.
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// True when any effect type carries immediate threat (attack-me, threat-all, etc.).
fn has_direct_threat_effect(spell_entry: &SpellEntry) -> bool {
    const SPELL_EFFECT_ATTACK_ME: u32 = 74;
    const SPELL_EFFECT_THREAT_ALL: u32 = 116;
    spell_entry
        .effect
        .iter()
        .any(|&e| e == SPELL_EFFECT_ATTACK_ME || e == SPELL_EFFECT_THREAT_ALL)
}

/// Check whether the spell applies a specific aura type in any of its effect slots.
fn spell_has_aura_type(spell_entry: &SpellEntry, aura_type: u32) -> bool {
    spell_entry
        .effect_apply_aura_name
        .iter()
        .any(|&a| a == aura_type)
}

/// Return the implicit target A for the first non-zero effect in `effectMask`.
fn first_effect_target_a(spell_entry: &SpellEntry, effect_mask: u8) -> Option<u32> {
    for i in 0..3 {
        if effect_mask & (1 << i) != 0 && spell_entry.effect[i] != 0 {
            return Some(spell_entry.effect_implicit_target_a[i]);
        }
    }
    None
}

/// Check whether the implicit target A value denotes a friendly target.
fn is_friendly_target(target_a: u32) -> bool {
    matches!(
        target_a,
        1   // unit-friend
            | 6   // friendly-area (deprecated)
            | 11  // friendly-area
            | 24  // friendly-area
            | 44  // friendly-area
            | 45  // friendly-area
            | 52  // friendly-area
            | 53  // friendly-area
            | 54 // friendly-area
    )
}

/// Remove a specific aura type from a unit target if possible.
/// Drop the unit's stealth and invisibility for a hostile action, honouring the spell's
/// opt-outs (remove-stealth / remove-non-passive-invisibility).
///
/// Note the asymmetry: stealth is removed outright, but only *non-passive* invisibility
/// is, so permanent/racial invisibility survives.
fn remove_stealth_and_invisibility(unit_guid: ObjectGuid, spell: &SpellEntry, world: &World) {
    if !spell.has_attribute_ex(SPELL_ATTR_EX_ALLOW_WHILE_STEALTHED) {
        remove_aura_type_from_target(unit_guid, SPELL_AURA_MOD_STEALTH, world);
    }
    if !spell.has_attribute_ex2(SPELL_ATTR_EX2_ALLOW_WHILE_INVISIBLE) {
        remove_non_passive_aura_type_from_target(unit_guid, SPELL_AURA_MOD_INVISIBILITY, world);
    }
}

/// Remove every aura on a unit whose spell carries any of `interrupt_flags`
/// (interrupt-flag aura removal).
///
/// Creature targets are skipped: the aura system's interrupt-flag removal is player-only,
/// so creature auras are left alone until that path is generalised.
async fn remove_auras_with_interrupt_flag_from_target(
    target_guid: ObjectGuid,
    interrupt_flags: u32,
    world: &World,
) {
    if !target_guid.is_player() {
        return;
    }
    if let Err(error) = world
        .systems
        .auras
        .remove_auras_with_interrupt_flag(target_guid, interrupt_flags, world)
        .await
    {
        tracing::warn!(
            "failed to remove interrupt-flag auras from {:?}: {}",
            target_guid,
            error
        );
    }
}

fn remove_non_passive_aura_type_from_target(
    target_guid: ObjectGuid,
    aura_type: u32,
    world: &World,
) {
    if target_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |p| {
                p.auras
                    .container
                    .remove_non_passive_auras_by_type(aura_type);
            });
    } else if target_guid.is_creature() {
        world
            .managers
            .creature_mgr
            .with_creature_mut(target_guid, |creature| {
                creature.auras.remove_non_passive_auras_by_type(aura_type);
            });
    }
}

fn remove_aura_type_from_target(target_guid: ObjectGuid, aura_type: u32, world: &World) {
    if target_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |p| {
                p.auras.container.remove_auras_by_type(aura_type);
            });
    } else if target_guid.is_creature() {
        world
            .managers
            .creature_mgr
            .with_creature_mut(target_guid, |creature| {
                creature.auras.remove_auras_by_type(aura_type);
            });
    }
}

fn target_is_immune_to_spell_effect(
    target_guid: ObjectGuid,
    spell: &SpellEntry,
    effect_index: usize,
    world: &World,
) -> bool {
    const SPELL_ATTR_EX_IGNORE_CASTER_AND_TARGET_RESTRICTIONS: u32 = 0x0080_0000;
    const SPELL_ATTR_EX3_IGNORE_CASTER_AND_TARGET_RESTRICTIONS: u32 = 0x1000_0000;
    const SPELL_ATTR_EX_IMMUNITY_TO_HOSTILE_AND_FRIENDLY_EFFECTS: u32 = 0x0001_0000;

    if spell.attributes_ex & SPELL_ATTR_EX_IGNORE_CASTER_AND_TARGET_RESTRICTIONS != 0
        || spell.attributes_ex3 & SPELL_ATTR_EX3_IGNORE_CASTER_AND_TARGET_RESTRICTIONS != 0
    {
        return false;
    }

    let is_immune = |auras: &crate::game::player::auras::AuraContainer| {
        auras.is_immune_to_spell_effect(
            spell.effect[effect_index],
            spell.effect_mechanic[effect_index],
            spell.effect_apply_aura_name[effect_index],
            spell.is_positive_effect(effect_index),
            |immunity_spell_id| {
                world
                    .managers
                    .spell_mgr
                    .get(immunity_spell_id)
                    .is_some_and(|immunity_spell| {
                        immunity_spell.attributes_ex
                            & SPELL_ATTR_EX_IMMUNITY_TO_HOSTILE_AND_FRIENDLY_EFFECTS
                            != 0
                    })
            },
        )
    };

    if target_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player(target_guid, |player| is_immune(&player.auras.container))
            .unwrap_or(false)
    } else if target_guid.is_creature() {
        world
            .managers
            .creature_mgr
            .with_creature(target_guid, |creature| is_immune(&creature.auras))
            .unwrap_or(false)
    } else {
        false
    }
}

fn target_resists_effect_mechanic(
    target_guid: ObjectGuid,
    spell: &SpellEntry,
    effect_index: usize,
    world: &World,
) -> bool {
    use crate::game::player::auras::effects::AURA_MOD_MECHANIC_RESISTANCE;

    let mechanic = spell.effect_mechanic[effect_index];
    if mechanic == 0 || mechanic == spell.mechanic {
        return false;
    }

    let resistance = if target_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player(target_guid, |player| {
                player
                    .auras
                    .container
                    .get_total_aura_modifier_by_misc(AURA_MOD_MECHANIC_RESISTANCE, mechanic as i32)
            })
            .unwrap_or(0)
    } else if target_guid.is_creature() {
        world
            .managers
            .creature_mgr
            .with_creature(target_guid, |creature| {
                creature
                    .auras
                    .get_total_aura_modifier_by_misc(AURA_MOD_MECHANIC_RESISTANCE, mechanic as i32)
            })
            .unwrap_or(0)
    } else {
        0
    };

    if resistance <= 0 {
        return false;
    }
    if resistance >= 100 {
        return true;
    }

    use rand::Rng;
    rand::thread_rng().gen_range(0..100) < resistance
}

fn target_is_immune_to_school(
    target_guid: ObjectGuid,
    spell: &SpellEntry,
    effect_mask: u8,
    world: &World,
) -> bool {
    const SPELL_ATTR_EX_IMMUNITY_PURGES_EFFECT: u32 = 0x0000_8000;
    const SPELL_ATTR_EX2_NO_SCHOOL_IMMUNITIES: u32 = 0x0400_0000;
    const SPELL_ATTR_EX_IGNORE_CASTER_AND_TARGET_RESTRICTIONS: u32 = 0x0080_0000;
    const SPELL_ATTR_EX3_IGNORE_CASTER_AND_TARGET_RESTRICTIONS: u32 = 0x1000_0000;
    const SPELL_ATTR_EX_IMMUNITY_TO_HOSTILE_AND_FRIENDLY_EFFECTS: u32 = 0x0001_0000;

    if spell.attributes_ex
        & (SPELL_ATTR_EX_IMMUNITY_PURGES_EFFECT
            | SPELL_ATTR_EX_IGNORE_CASTER_AND_TARGET_RESTRICTIONS)
        != 0
        || spell.attributes_ex2 & SPELL_ATTR_EX2_NO_SCHOOL_IMMUNITIES != 0
        || spell.attributes_ex3 & SPELL_ATTR_EX3_IGNORE_CASTER_AND_TARGET_RESTRICTIONS != 0
    {
        return false;
    }

    let effects_are_positive = (0..3).all(|effect_index| {
        spell.effect[effect_index] == 0
            || effect_mask & (1 << effect_index) == 0
            || spell.is_positive_effect(effect_index)
    });
    let school_mask = 1u32.checked_shl(spell.school).unwrap_or(0);
    let is_immune = |auras: &crate::game::player::auras::AuraContainer| {
        auras.is_immune_to_school(
            school_mask,
            spell.id,
            effects_are_positive,
            |immunity_spell_id| {
                world
                    .managers
                    .spell_mgr
                    .get(immunity_spell_id)
                    .is_some_and(|immunity_spell| {
                        immunity_spell.attributes_ex
                            & SPELL_ATTR_EX_IMMUNITY_TO_HOSTILE_AND_FRIENDLY_EFFECTS
                            != 0
                    })
            },
        )
    };

    if target_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player(target_guid, |player| is_immune(&player.auras.container))
            .unwrap_or(false)
    } else if target_guid.is_creature() {
        world
            .managers
            .creature_mgr
            .with_creature(target_guid, |creature| is_immune(&creature.auras))
            .unwrap_or(false)
    } else {
        false
    }
}

pub(super) fn target_is_immune_to_damage(
    target_guid: ObjectGuid,
    spell: &SpellEntry,
    world: &World,
) -> bool {
    const SPELL_ATTR_EX_IGNORE_CASTER_AND_TARGET_RESTRICTIONS: u32 = 0x0080_0000;
    const SPELL_ATTR_EX3_IGNORE_CASTER_AND_TARGET_RESTRICTIONS: u32 = 0x1000_0000;

    if spell.attributes & SPELL_ATTR_NO_IMMUNITIES != 0
        || spell.attributes_ex & SPELL_ATTR_EX_IGNORE_CASTER_AND_TARGET_RESTRICTIONS != 0
        || spell.attributes_ex3 & SPELL_ATTR_EX3_IGNORE_CASTER_AND_TARGET_RESTRICTIONS != 0
    {
        return false;
    }

    let school_mask = 1u32.checked_shl(spell.school).unwrap_or(0);
    let is_immune =
        |auras: &crate::game::player::auras::AuraContainer| auras.is_immune_to_damage(school_mask);

    if target_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player(target_guid, |player| is_immune(&player.auras.container))
            .unwrap_or(false)
    } else if target_guid.is_creature() {
        world
            .managers
            .creature_mgr
            .with_creature(target_guid, |creature| is_immune(&creature.auras))
            .unwrap_or(false)
    } else {
        false
    }
}

/// Enter combat for a landed hit.
fn enter_combat_on_hit(caster_guid: ObjectGuid, target_guid: ObjectGuid, world: &World) {
    if target_guid.is_creature() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // Apply zero damage to flag combat + add initial threat
        world
            .managers
            .creature_mgr
            .apply_damage(target_guid, 0, caster_guid, timestamp);
        world
            .managers
            .creature_mgr
            .with_creature_mut(target_guid, |c| {
                c.combat.add_threat(caster_guid, 1.0, timestamp);
            });
    } else if target_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |p| {
                p.combat.enter_combat(caster_guid);
            });
    }
}

/// Flag the caster as being in combat with the victim.
pub(crate) fn set_caster_in_combat_with_victim(
    caster_guid: ObjectGuid,
    _target_guid: ObjectGuid,
    world: &World,
) {
    if caster_guid.is_creature() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        world
            .managers
            .creature_mgr
            .with_creature_mut(caster_guid, |c| {
                c.combat.enter_combat(_target_guid, timestamp);
            });
    } else if caster_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player_mut(caster_guid, |p| {
                p.combat.enter_combat(_target_guid);
            });
    }
}

/// Clear the combat/aggressor relationship.
fn set_out_of_combat(caster_guid: ObjectGuid, target_guid: ObjectGuid, world: &World) {
    // Not all combat systems expose this directly; no-op is safe for PvP-enabling.
    let _ = (caster_guid, target_guid, world);
}

/// Place the caster into assisted combat alongside the friendly target, distributing
/// assist threat.
fn assisted_combat(caster_guid: ObjectGuid, _target_guid: ObjectGuid, world: &World) {
    if caster_guid.is_creature() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        world
            .managers
            .creature_mgr
            .with_creature_mut(caster_guid, |c| {
                c.combat.enter_combat(_target_guid, timestamp);
            });
    } else if caster_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player_mut(caster_guid, |p| {
                p.combat.enter_combat(_target_guid);
            });
    }
    // threatAssist not yet wired for non-creature targets.
}

/// Apply a spell's effects to a single target.
///
/// Resolves the hit outcome once, strips persistent-area-aura bits from the effect
/// mask (those are applied once per aura holder elsewhere), and on a hit dispatches
/// every requested effect for this target, accumulating damage/healing into `target`.
///
/// Divergence: effect handlers (`effects::damage`, `effects::healing`, ...) already
/// call the real `caster::deal_damage` / `caster::deal_heal` themselves as they run,
/// instead of this function collecting damage/healing and applying them once at the
/// end. `target.damage` / `target.healing` are still accumulated here for target-level
/// visibility (and so `proc_attacker`/`proc_victim` can be set once per target), but
/// they are not re-applied — doing so would double-apply damage that the effect handler
/// already dealt. Migrating every damage/heal effect handler to a "compute only" model
/// so this function can be the single apply point is future work.
pub async fn apply_target_effects(
    target: &mut TargetInfo,
    caster_guid: ObjectGuid,
    cast_item_guid: Option<ObjectGuid>,
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

    // Game objects are spell targets but not units: they do not receive hit rolls,
    // combat processing, diminishing returns, or proc flags. Dispatch their effects
    // directly, as the open-lock effect does for a GO target.
    if target.target_guid.is_game_object() {
        for effect_index in 0..3usize {
            if target.effect_mask & (1 << effect_index) == 0 {
                continue;
            }
            let effect_type = spell_entry.effect[effect_index];
            let Some(effect_type_enum) = SpellEffectType::from_u32(effect_type) else {
                tracing::warn!("Unknown effect type {} for spell {}", effect_type, spell_id);
                continue;
            };
            let base_value = custom_base_points
                .and_then(|bp| bp[effect_index])
                .unwrap_or(spell_entry.effect_base_points[effect_index]);
            let input = EffectInput {
                caster_guid,
                cast_item_guid,
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
                diminishing: target.diminishing,
            };
            let mut result = dispatch_effect(effect_type_enum, &input, world).await?;
            result.target_guid = Some(target.target_guid);
            result.effect_index = effect_index as u8;
            results.push(result);
        }
        return Ok(results);
    }

    // Resolve the hit outcome once for the whole target (not once per effect — the old
    // per-effect dispatch loop rolled a fresh hit per effect index, which could both
    // double-roll and desync the miss packet/AI event from the effects actually applied).
    if !target.outcome_resolved {
        target.miss_condition = if is_triggered || target.target_guid == caster_guid {
            SpellHitOutcome::Hit
        } else {
            hit::roll_spell_hit(caster_guid, target.target_guid, spell_id, world)
        };
        target.reflect_result = if target.miss_condition == SpellHitOutcome::Reflect {
            resolve_reflect_result(caster_guid, &spell_entry, world)
        } else {
            SpellHitOutcome::Hit
        };
        target.outcome_resolved = true;
    }

    if target.miss_condition.is_hit()
        && target.target_guid != caster_guid
        && !spell_entry.is_positive_spell()
        && target_is_immune_to_damage(target.target_guid, &spell_entry, world)
    {
        target.miss_condition = SpellHitOutcome::Immune;
    }

    // The unit that actually receives the effects. A reflected spell bounces back onto
    // its caster; every other outcome applies to the registered target.
    let mut hit_target_guid = target.target_guid;
    let is_reflected = target.miss_condition == SpellHitOutcome::Reflect;

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
        // Reflected, but the caster cannot take the spell back (immune, or the cast has
        // no unit caster at all). The spell fizzles; SMSG_SPELL_GO still reports the
        // reflect together with this result byte.
        SpellHitOutcome::Reflect if !target.reflect_result.is_hit() => {
            tracing::debug!(
                "[SPELL-HIT] spell {} reflected by {:?} but caster {:?} did not take it ({:?})",
                spell_id,
                target.target_guid,
                caster_guid,
                target.reflect_result
            );
            return Ok(results);
        }
        SpellHitOutcome::Reflect | SpellHitOutcome::Hit | SpellHitOutcome::PartialResist(_) => {
            if target.miss_condition == SpellHitOutcome::Reflect {
                hit_target_guid = caster_guid;
            }

            fire_spell_hit_ai_event(caster_guid, hit_target_guid, spell_id, &spell_entry, world);

            // Per-unit pre-effect processing:
            // combat entry, stealth removal, resist checks, DR snapshot, etc.
            let is_delayed = spell_entry.speed > 0.0;
            match do_spell_hit_on_unit(
                &spell_entry,
                caster_guid,
                hit_target_guid,
                &mut target.effect_mask,
                is_delayed,
                is_triggered,
                is_reflected,
                world,
            )
            .await
            {
                // All effects resisted/aborted — nothing to dispatch.
                None => return Ok(results),
                Some(snapshot) => target.diminishing = snapshot,
            }
        }
    }

    // A fully diminished hit lands no auras at all: the aura holder is dropped before it
    // is added. Nothing else in the effect mask can run either, because the same mask is
    // what marked this hit as aura-applying.
    if target.diminishing.is_fully_diminished()
        && spell_applies_aura(&spell_entry, target.effect_mask)
    {
        tracing::debug!(
            "[DR] spell {} fully diminished on {:?} (group {:?})",
            spell_id,
            hit_target_guid,
            target.diminishing.group
        );
        return Ok(results);
    }

    // Per-target proc flags: helpful spells proc DEAL/TAKE_HELPFUL, harmful spells proc
    // DEAL/TAKE_HARMFUL (+ TAKEN_ANY_DAMAGE on the victim side). Simplified stand-in for
    // the original attack-type-derived flags (which get adjusted by negative-trigger and
    // secondary-target rules).
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
            cast_item_guid,
            target_guid: Some(hit_target_guid),
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
            diminishing: target.diminishing,
        };

        match dispatch_effect(effect_type_enum, &input, world).await {
            Ok(result) => {
                let mut result = result;
                result.target_guid = Some(hit_target_guid);
                result.effect_index = effect_index as u8;
                target.damage = target.damage.saturating_add(result.damage);
                target.healing = target.healing.saturating_add(result.healing);
                results.push(result);
            }
            Err(e) => {
                tracing::error!(
                    "Effect {} failed for spell {} target {:?}: {}",
                    effect_index,
                    spell_id,
                    hit_target_guid,
                    e
                );
                results.push(EffectResult::empty());
            }
        }
    }

    Ok(results)
}

/// Resolve the outcome of a reflected spell landing back on its caster.
///
/// The outcome is immune when there is no unit caster to take the spell back (a
/// game-object caster, say); otherwise the hit is re-rolled with the caster as its own
/// victim. That self-target path short-circuits after the immunity checks, so it reduces
/// to "immune, or it lands". A second reflect is therefore impossible and a
/// reflect-to-parry downgrade is unreachable here.
///
/// Simplification: only damage immunity is consulted, matching the immunity surface
/// `roll_spell_hit` itself uses; the full spell-immunity check is not ported yet.
fn resolve_reflect_result(
    caster_guid: ObjectGuid,
    spell: &SpellEntry,
    world: &World,
) -> SpellHitOutcome {
    if !caster_guid.is_player() && !caster_guid.is_creature() {
        return SpellHitOutcome::Immune;
    }
    if target_is_immune_to_damage(caster_guid, spell, world) {
        return SpellHitOutcome::Immune;
    }
    SpellHitOutcome::Hit
}

/// Put the caster and a non-positive-spell miss target into combat on a miss/resist/immune.
///
/// Simplified: mirrors the attacked-by handling plus the initial threat so the creature
/// AI has an active victim to chase, without double-counting damage threat (which is
/// added when a hit lands). Full failure-breaks-stealth / stealth-removal handling is
/// not ported yet.
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
        world
            .managers
            .creature_mgr
            .with_creature_mut(target_guid, |creature| {
                // Being attacked makes the creature AI start attacking, which adds initial
                // threat. The AI snapshot selects targets from the threat manager, not the
                // legacy combat threat list populated by apply_damage(0).
                creature.threat_manager.add_threat(caster_guid, 1.0);
                creature.combat.add_threat(caster_guid, 1.0, timestamp);
            });
        // The attacked-by event is what makes the AI enter combat *and* set an attack
        // target; seeding threat alone leaves the creature chasing without a victim.
        crate::game::creature::ai::queue_event(
            world,
            target_guid,
            crate::game::creature::ai::AIEvent::AttackedBy {
                attacker_guid: caster_guid,
            },
        );
    } else if target_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |p| p.combat.enter_combat(caster_guid));
    }

    set_caster_in_combat_with_victim(caster_guid, target_guid, world);
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
    use crate::config::Config;
    use crate::game::player::auras::{Aura, AuraFlags};
    use crate::game::player::Player;
    use oxcore_db::database::Databases;
    use sqlx::mysql::MySqlPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn lazy_pool() -> sqlx::MySqlPool {
        MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    /// Minimal test world (no database back-end).
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

    fn add_test_player(world: &World, guid: ObjectGuid) {
        let player = Player::new(guid, format!("P{}", guid.counter()), 0, 0, 0, 60, 1, 1, 0);
        world.managers.player_mgr.add_player(player, guid.counter());
    }

    fn add_test_creature(world: &World, guid: ObjectGuid) {
        use crate::game::creature::{Creature, CreatureTemplate};
        use oxcore_shared::protocol::Position;

        let entry = guid.entry();
        let template = CreatureTemplate {
            entry,
            name: format!("Creature {entry}"),
            subname: None,
            min_level: 60,
            max_level: 60,
            faction: 14,
            model_id_1: 1,
            model_id_2: 0,
            model_id_3: 0,
            model_id_4: 0,
            scale: 1.0,
            npc_flags: 0,
            unit_flags: 0,
            static_flags1: 0,
            flags_extra: 0,
            creature_type: 0,
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
        };
        world.managers.creature_mgr.add_creature(Creature::new(
            guid,
            entry,
            60,
            Position::default(),
            0,
            0,
            &template,
            1,
            None,
        ));
    }

    /// Create a minimal harmful spell entry with one damage effect.
    fn harmful_spell(id: u32) -> SpellEntry {
        let mut effect = [0u32; 3];
        effect[0] = SPELL_EFFECT_SCHOOL_DAMAGE;
        SpellEntry {
            id,
            name: format!("Harmful{id}"),
            rank_text: String::new(),
            school: 1,
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
            effect,
            effect_die_sides: [0; 3],
            effect_base_dice: [0; 3],
            effect_dice_per_level: [0.0; 3],
            effect_real_points_per_level: [0.0; 3],
            effect_base_points: [10; 3],
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
            dmg_multiplier: [0.0; 3],
        }
    }

    // ─── Tests for pure helper functions ───────────────────────────────────────────

    #[test]
    fn is_friendly_target_accepts_friend_types() {
        assert!(is_friendly_target(1)); // friendly-unit target
        assert!(is_friendly_target(11)); // friendly-area target
        assert!(!is_friendly_target(16)); // enemy-aoe-at-dest target
        assert!(!is_friendly_target(0));
        assert!(!is_friendly_target(99));
    }

    #[test]
    fn has_direct_threat_effect_detects_threat_effects() {
        let mut entry = harmful_spell(1);
        // No threat effects by default
        assert!(!has_direct_threat_effect(&entry));

        entry.effect[0] = 74; // attack-me effect
        assert!(has_direct_threat_effect(&entry));

        entry.effect[0] = 116; // threat-all effect
        assert!(has_direct_threat_effect(&entry));
    }

    #[test]
    fn spell_has_aura_type_checks_apply_aura_name() {
        let mut entry = harmful_spell(2);
        entry.effect_apply_aura_name = [0; 3];
        assert!(!spell_has_aura_type(&entry, SPELL_AURA_MOD_STEALTH));

        entry.effect_apply_aura_name[0] = SPELL_AURA_MOD_STEALTH;
        assert!(spell_has_aura_type(&entry, SPELL_AURA_MOD_STEALTH));
        assert!(!spell_has_aura_type(&entry, SPELL_AURA_MOD_INVISIBILITY));
    }

    #[test]
    fn first_effect_target_a_returns_first_nonzero_effect() {
        let mut entry = harmful_spell(3);
        entry.effect_implicit_target_a = [0; 3];
        // No effect bits set — returns None
        assert_eq!(first_effect_target_a(&entry, 0), None);

        // Effect 1 has target_a = 16, effect 0 is zero
        entry.effect[0] = 0;
        entry.effect[1] = SPELL_EFFECT_SCHOOL_DAMAGE;
        entry.effect_implicit_target_a[1] = 16;
        assert_eq!(first_effect_target_a(&entry, 0b010), Some(16));

        // Effect 0 is non-zero in mask but zero in entry → should skip to effect 1
        assert_eq!(first_effect_target_a(&entry, 0b001), None);
    }

    // ─── Tests for the previous helpers ────────────────────────────────────────────

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

    // ─── Tests for do_spell_hit_on_unit ────────────────────────────────────────────

    #[tokio::test]
    async fn zero_effect_mask_non_positive_enters_combat() {
        let world = test_world();
        let spell = harmful_spell(100);
        let mut mask = 0u8;

        // Non-positive spell with zero mask -> enters combat, returns false
        let result = do_spell_hit_on_unit(
            &spell,
            ObjectGuid::new_player(1),
            ObjectGuid::new_creature(1, 50),
            &mut mask,
            false,
            false,
            false,
            &world,
        )
        .await;

        assert!(result.is_none(), "zero mask should abort hit");
        assert_eq!(mask, 0, "mask should still be zero");
    }

    #[tokio::test]
    async fn zero_effect_mask_positive_does_not_enter_combat() {
        let world = test_world();
        let mut spell = harmful_spell(101);
        // Mark as positive (clear the negative bit and set no harmful effects)
        spell.attributes = 0x0400_0000;
        let mut mask = 0u8;

        let result = do_spell_hit_on_unit(
            &spell,
            ObjectGuid::new_player(2),
            ObjectGuid::new_creature(2, 50),
            &mut mask,
            false,
            false,
            false,
            &world,
        )
        .await;

        assert!(
            result.is_none(),
            "zero mask should abort hit even for positive spells"
        );
    }

    #[tokio::test]
    async fn harmful_spell_miss_gives_creature_an_ai_threat_target() {
        let world = test_world();
        let caster = ObjectGuid::new_player(20);
        let target = ObjectGuid::new_creature(20, 50);
        add_test_player(&world, caster);
        add_test_creature(&world, target);

        let spell = harmful_spell(102);
        world.managers.spell_mgr.add_spell(spell.clone());
        let mut info = TargetInfo::new(target, 0b001);
        info.miss_condition = SpellHitOutcome::Miss;
        info.outcome_resolved = true;

        let results = apply_target_effects(&mut info, caster, None, spell.id, false, None, &world)
            .await
            .expect("a miss should be handled");

        assert!(results.is_empty());
        assert!(world.managers.creature_mgr.is_in_combat(target));
        assert_eq!(
            world
                .managers
                .creature_mgr
                .get_highest_threat_target(target),
            Some(caster),
            "the AI selects the missed spell's caster as its victim"
        );
        assert!(
            world
                .managers
                .player_mgr
                .with_player(caster, |player| player.combat.in_combat)
                .unwrap_or(false),
            "the caster also enters combat with the missed target"
        );
    }

    #[tokio::test]
    async fn effect_immunity_removes_only_the_matching_effect_from_live_target() {
        let world = test_world();
        let caster = ObjectGuid::new_player(30);
        let target = ObjectGuid::new_player(31);
        add_test_player(&world, caster);
        add_test_player(&world, target);
        world
            .systems
            .player
            .manager()
            .with_player_mut(target, |player| {
                let mut immunity = Aura::new(
                    9000,
                    caster,
                    0,
                    crate::game::player::auras::effects::AURA_EFFECT_IMMUNITY,
                    SPELL_EFFECT_SCHOOL_DAMAGE as i32,
                    0,
                    None,
                    0,
                    1,
                    0,
                    AuraFlags {
                        is_positive: true,
                        ..AuraFlags::default()
                    },
                );
                immunity.flags.is_positive = true;
                player.auras.container.add_aura(immunity);
            });

        let spell = harmful_spell(108);
        let mut mask = 0b001;
        assert!(do_spell_hit_on_unit(
            &spell, caster, target, &mut mask, false, false, false, &world
        )
        .await
        .is_none());
        assert_eq!(mask, 0);
    }

    #[tokio::test]
    async fn school_immunity_aborts_the_live_target_hit() {
        let world = test_world();
        let caster = ObjectGuid::new_player(32);
        let target = ObjectGuid::new_player(33);
        add_test_player(&world, caster);
        add_test_player(&world, target);
        world
            .systems
            .player
            .manager()
            .with_player_mut(target, |player| {
                player.auras.container.add_aura(Aura::new(
                    9001,
                    caster,
                    0,
                    crate::game::player::auras::effects::AURA_SCHOOL_IMMUNITY,
                    1 << 2,
                    0,
                    None,
                    0,
                    1,
                    0,
                    AuraFlags {
                        is_positive: true,
                        ..AuraFlags::default()
                    },
                ));
            });

        let mut spell = harmful_spell(109);
        spell.school = 2;
        let mut mask = 0b001;
        assert!(do_spell_hit_on_unit(
            &spell, caster, target, &mut mask, false, false, false, &world
        )
        .await
        .is_none());
        assert_eq!(mask, 0);
    }

    #[tokio::test]
    async fn damage_immunity_blocks_matching_school_unless_the_spell_bypasses_immunities() {
        let world = test_world();
        let caster = ObjectGuid::new_player(36);
        let target = ObjectGuid::new_player(37);
        add_test_player(&world, caster);
        add_test_player(&world, target);
        world
            .systems
            .player
            .manager()
            .with_player_mut(target, |player| {
                player.auras.container.add_aura(Aura::new(
                    9003,
                    caster,
                    0,
                    crate::game::player::auras::effects::AURA_DAMAGE_IMMUNITY,
                    1 << 2,
                    0,
                    None,
                    0,
                    1,
                    0,
                    AuraFlags {
                        is_positive: true,
                        ..AuraFlags::default()
                    },
                ));
            });

        let mut spell = harmful_spell(111);
        spell.school = 2;
        assert!(target_is_immune_to_damage(target, &spell, &world));

        spell.attributes |= SPELL_ATTR_NO_IMMUNITIES;
        assert!(!target_is_immune_to_damage(target, &spell, &world));
    }

    // ─── Hostile-action side effects ──────────────────────────────────────────────

    fn give_aura(world: &World, guid: ObjectGuid, spell_id: u32, aura_type: u32, passive: bool) {
        world
            .systems
            .player
            .manager()
            .with_player_mut(guid, |player| {
                player.auras.container.add_aura(Aura::new(
                    spell_id,
                    guid,
                    0,
                    aura_type,
                    0,
                    0,
                    None,
                    0,
                    1,
                    0,
                    AuraFlags {
                        is_passive: passive,
                        ..AuraFlags::default()
                    },
                ));
            });
    }

    fn has_aura_type(world: &World, guid: ObjectGuid, aura_type: u32) -> bool {
        world
            .systems
            .player
            .manager()
            .with_player(guid, |player| {
                player.auras.container.has_aura_type(aura_type)
            })
            .unwrap_or(false)
    }

    /// Invisibility removal is `RemoveNonPassiveSpellsCausingAura`: passive invisibility
    /// survives a hostile hit that strips the ordinary kind.
    #[tokio::test]
    async fn hostile_hit_strips_only_non_passive_invisibility() {
        let world = test_world();
        let caster = ObjectGuid::new_player(60);
        let target = ObjectGuid::new_player(61);
        add_test_player(&world, caster);
        add_test_player(&world, target);

        give_aura(&world, target, 8000, SPELL_AURA_MOD_INVISIBILITY, false);
        give_aura(&world, target, 8001, SPELL_AURA_MOD_INVISIBILITY, true);

        let spell = harmful_spell(9400);
        let mut mask = 0b001;
        do_spell_hit_on_unit(
            &spell, caster, target, &mut mask, false, false, false, &world,
        )
        .await
        .expect("hit should continue");

        let remaining: Vec<u32> = world
            .systems
            .player
            .manager()
            .with_player(target, |player| {
                player
                    .auras
                    .container
                    .get_auras_by_type(SPELL_AURA_MOD_INVISIBILITY)
                    .iter()
                    .map(|aura| aura.spell_id)
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(
            remaining,
            vec![8001],
            "only the passive invisibility should survive"
        );
    }

    /// A hostile action drops the caster's own stealth once the hit passes the threat gate.
    #[tokio::test]
    async fn hostile_hit_strips_the_casters_stealth() {
        let world = test_world();
        let caster = ObjectGuid::new_player(62);
        let target = ObjectGuid::new_player(63);
        add_test_player(&world, caster);
        add_test_player(&world, target);

        give_aura(&world, caster, 8002, SPELL_AURA_MOD_STEALTH, false);

        let spell = harmful_spell(9401);
        let mut mask = 0b001;
        do_spell_hit_on_unit(
            &spell, caster, target, &mut mask, false, false, false, &world,
        )
        .await
        .expect("hit should continue");

        assert!(
            !has_aura_type(&world, caster, SPELL_AURA_MOD_STEALTH),
            "the caster breaks its own stealth"
        );
    }

    /// `ALLOW_WHILE_STEALTHED` keeps the caster hidden too, not just the target.
    #[tokio::test]
    async fn allow_while_stealthed_keeps_the_casters_stealth() {
        let world = test_world();
        let caster = ObjectGuid::new_player(64);
        let target = ObjectGuid::new_player(65);
        add_test_player(&world, caster);
        add_test_player(&world, target);

        give_aura(&world, caster, 8003, SPELL_AURA_MOD_STEALTH, false);

        let mut spell = harmful_spell(9402);
        spell.attributes_ex |= SPELL_ATTR_EX_ALLOW_WHILE_STEALTHED;
        let mut mask = 0b001;
        do_spell_hit_on_unit(
            &spell, caster, target, &mut mask, false, false, false, &world,
        )
        .await
        .expect("hit should continue");

        assert!(has_aura_type(&world, caster, SPELL_AURA_MOD_STEALTH));
    }

    /// A unit is never hostile to itself: a reflected cast landing back on its caster must
    /// not strip the caster's stealth or drag it into combat with itself.
    #[tokio::test]
    async fn a_self_hit_runs_no_hostile_side_effects() {
        let world = test_world();
        let caster = ObjectGuid::new_player(66);
        add_test_player(&world, caster);

        give_aura(&world, caster, 8004, SPELL_AURA_MOD_STEALTH, false);

        let spell = harmful_spell(9403);
        let mut mask = 0b001;
        do_spell_hit_on_unit(
            &spell, caster, caster, &mut mask, false, false, true, &world,
        )
        .await
        .expect("self hit should continue");

        assert!(
            has_aura_type(&world, caster, SPELL_AURA_MOD_STEALTH),
            "a self-hit is not a hostile action against oneself"
        );
        assert!(
            !world
                .systems
                .player
                .manager()
                .with_player(caster, |p| p.combat.in_combat)
                .unwrap_or(false),
            "a unit does not enter combat with itself"
        );
    }

    // ─── Diminishing returns ──────────────────────────────────────────────────────

    /// A stun (mechanic 12, `DRTYPE_ALL`) applying one aura effect.
    fn stun_spell(id: u32) -> SpellEntry {
        const SPELL_EFFECT_APPLY_AURA: u32 = 6;
        const MECHANIC_STUNNED: u32 = 12;
        let mut spell = harmful_spell(id);
        spell.effect = [SPELL_EFFECT_APPLY_AURA, 0, 0];
        spell.effect_apply_aura_name = [crate::game::player::auras::effects::AURA_MOD_STUN, 0, 0];
        spell.mechanic = MECHANIC_STUNNED;
        // A bare APPLY_AURA fixture with zeroed implicit targets classifies as positive,
        // which would make the caster count as friendly and skip DR. Real CC spells carry
        // the negative override, so set it explicitly here.
        spell.custom |= SPELL_CUSTOM_NEGATIVE;
        spell
    }

    /// `SPELL_CUSTOM_NEGATIVE` — the explicit "this spell is hostile" override honoured
    /// ahead of the effect-based polarity heuristic.
    const SPELL_CUSTOM_NEGATIVE: u32 = 0x0000_0002;

    #[tokio::test]
    async fn stun_hits_diminish_and_then_fully_diminish_a_player_target() {
        let world = test_world();
        let caster = ObjectGuid::new_player(50);
        let target = ObjectGuid::new_player(51);
        add_test_player(&world, caster);
        add_test_player(&world, target);

        let spell = stun_spell(9300);
        let mut levels = Vec::new();
        for _ in 0..4 {
            let mut mask = 0b001;
            let snapshot = do_spell_hit_on_unit(
                &spell, caster, target, &mut mask, false, false, false, &world,
            )
            .await
            .expect("stun hit should not abort");
            levels.push(snapshot.level);
        }

        assert_eq!(
            levels,
            vec![0, 1, 2, 3],
            "each stun landing raises the target's level for the next one"
        );
        assert!(
            DiminishSnapshot {
                group: diminishing::DRGroup::Stun,
                level: 3,
                diminishes_duration: true,
            }
            .is_fully_diminished(),
            "the fourth stun lands with zero duration"
        );
    }

    /// One cast whose effect mask applies two auras must charge the counter once and hand
    /// both auras the same level — the reason DR is sampled on hit, not on aura add.
    #[tokio::test]
    async fn a_multi_aura_cast_charges_the_counter_once() {
        const SPELL_EFFECT_APPLY_AURA: u32 = 6;
        let world = test_world();
        let caster = ObjectGuid::new_player(52);
        let target = ObjectGuid::new_player(53);
        add_test_player(&world, caster);
        add_test_player(&world, target);

        let mut spell = stun_spell(9301);
        spell.effect[1] = SPELL_EFFECT_APPLY_AURA;
        spell.effect_apply_aura_name[1] = crate::game::player::auras::effects::AURA_MOD_ROOT;

        let mut mask = 0b011;
        let first = do_spell_hit_on_unit(
            &spell, caster, target, &mut mask, false, false, false, &world,
        )
        .await
        .expect("hit should not abort");
        assert_eq!(first.level, 0);

        let mut mask = 0b011;
        let second = do_spell_hit_on_unit(
            &spell, caster, target, &mut mask, false, false, false, &world,
        )
        .await
        .expect("hit should not abort");
        assert_eq!(
            second.level, 1,
            "two aura effects in one cast must advance the level by one, not two"
        );
    }

    /// `DRTYPE_ALL` groups diminish creatures too, so a creature must carry its own counter.
    #[tokio::test]
    async fn stun_diminishes_creature_targets() {
        let world = test_world();
        let caster = ObjectGuid::new_player(54);
        let target = ObjectGuid::new_creature(1, 55);
        add_test_player(&world, caster);
        add_test_creature(&world, target);

        let spell = stun_spell(9302);

        let mut mask = 0b001;
        let first = do_spell_hit_on_unit(
            &spell, caster, target, &mut mask, false, false, false, &world,
        )
        .await
        .expect("hit should not abort");
        assert!(first.diminishes_duration, "stuns diminish on creatures");
        assert_eq!(first.apply_to_duration(Some(8_000)), Some(8_000));

        let mut mask = 0b001;
        let second = do_spell_hit_on_unit(
            &spell, caster, target, &mut mask, false, false, false, &world,
        )
        .await
        .expect("hit should not abort");
        assert_eq!(second.apply_to_duration(Some(8_000)), Some(4_000));
    }

    /// A fear (`DRTYPE_PLAYER`) leaves creature durations alone.
    #[tokio::test]
    async fn player_only_groups_do_not_diminish_creatures() {
        const SPELL_EFFECT_APPLY_AURA: u32 = 6;
        const MECHANIC_FLEEING: u32 = 5;
        let world = test_world();
        let caster = ObjectGuid::new_player(56);
        let target = ObjectGuid::new_creature(1, 57);
        add_test_player(&world, caster);
        add_test_creature(&world, target);

        let mut spell = harmful_spell(9303);
        spell.effect = [SPELL_EFFECT_APPLY_AURA, 0, 0];
        spell.effect_apply_aura_name = [crate::game::player::auras::effects::AURA_MOD_FEAR, 0, 0];
        spell.mechanic = MECHANIC_FLEEING;
        spell.custom |= SPELL_CUSTOM_NEGATIVE;

        for _ in 0..3 {
            let mut mask = 0b001;
            let snapshot = do_spell_hit_on_unit(
                &spell, caster, target, &mut mask, false, false, false, &world,
            )
            .await
            .expect("hit should not abort");
            assert!(!snapshot.diminishes_duration);
            assert_eq!(snapshot.apply_to_duration(Some(8_000)), Some(8_000));
        }
    }

    /// A spell that applies no aura reads the level without charging the counter.
    #[tokio::test]
    async fn direct_damage_hits_never_charge_the_counter() {
        let world = test_world();
        let caster = ObjectGuid::new_player(58);
        let target = ObjectGuid::new_player(59);
        add_test_player(&world, caster);
        add_test_player(&world, target);

        // A pure damage spell that still classifies into a DR group by its mechanic.
        let mut spell = harmful_spell(9304);
        spell.mechanic = 12; // MECHANIC_STUNNED

        for _ in 0..3 {
            let mut mask = 0b001;
            let snapshot = do_spell_hit_on_unit(
                &spell, caster, target, &mut mask, false, false, false, &world,
            )
            .await
            .expect("hit should not abort");
            assert_eq!(
                snapshot.level, 0,
                "no aura applied, so no level is consumed"
            );
        }
    }

    // ─── Reflect ──────────────────────────────────────────────────────────────────

    /// A plain magic spell registered in the manager, reflectable and guaranteed to be
    /// reflected by a victim carrying a 100% reflect aura.
    fn reflectable_spell(id: u32) -> SpellEntry {
        let mut spell = harmful_spell(id);
        spell.dmg_class = 1; // magic damage class
        spell
    }

    fn give_reflect_aura(world: &World, guid: ObjectGuid, chance: i32, school_mask: i32) {
        world
            .systems
            .player
            .manager()
            .with_player_mut(guid, |player| {
                player.auras.container.add_aura(Aura::new(
                    9100,
                    guid,
                    0,
                    crate::game::player::auras::effects::AURA_REFLECT_SPELLS_SCHOOL,
                    school_mask,
                    chance,
                    None,
                    0,
                    1,
                    0,
                    AuraFlags {
                        is_positive: true,
                        ..AuraFlags::default()
                    },
                ));
            });
    }

    #[test]
    fn only_plain_magic_spells_are_reflectable() {
        let mut spell = reflectable_spell(9200);
        assert!(hit::is_reflectable_spell(&spell));

        // Non-magic damage classes are never reflectable.
        for class in [0u32, 2, 3] {
            spell.dmg_class = class;
            assert!(!hit::is_reflectable_spell(&spell), "dmg_class {class}");
        }
        spell.dmg_class = 1;

        // Ability, passive, and no-immunities attributes; no-reflection on AttributesEx.
        for attr in [0x0000_0010u32, 0x0000_0040, 0x2000_0000] {
            let mut other = spell.clone();
            other.attributes |= attr;
            assert!(!hit::is_reflectable_spell(&other), "attribute {attr:#x}");
        }
        let mut no_reflection = spell.clone();
        no_reflection.attributes_ex |= 0x0000_0080;
        assert!(!hit::is_reflectable_spell(&no_reflection));
    }

    #[tokio::test]
    async fn reflect_aura_bounces_the_spell_back_onto_the_caster() {
        let world = test_world();
        let caster = ObjectGuid::new_player(40);
        let target = ObjectGuid::new_player(41);
        add_test_player(&world, caster);
        add_test_player(&world, target);

        let spell = reflectable_spell(9201);
        world.managers.spell_mgr.add_spell(spell.clone());
        // 100% reflect against the spell's school (school 1 -> mask 0b10).
        give_reflect_aura(&world, target, 100, 1 << spell.school);

        let mut info = TargetInfo::new(target, 0b001);
        let results =
            apply_target_effects(&mut info, caster, None, spell.id, false, None, &world).await;

        assert_eq!(info.miss_condition, SpellHitOutcome::Reflect);
        assert_eq!(
            info.reflect_result,
            SpellHitOutcome::Hit,
            "a caster with no immunity takes the reflected spell"
        );

        let results = results.expect("reflected effects should dispatch");
        assert!(!results.is_empty(), "the reflected effect must still run");
        for result in &results {
            assert_eq!(
                result.target_guid,
                Some(caster),
                "reflected effects apply to the caster, not the reflecting victim"
            );
        }
    }

    #[tokio::test]
    async fn reflected_spell_fizzles_when_the_caster_is_immune() {
        let world = test_world();
        let caster = ObjectGuid::new_player(42);
        let target = ObjectGuid::new_player(43);
        add_test_player(&world, caster);
        add_test_player(&world, target);

        let mut spell = reflectable_spell(9202);
        spell.school = 2;
        world.managers.spell_mgr.add_spell(spell.clone());
        give_reflect_aura(&world, target, 100, 1 << spell.school);

        // The caster is immune to the school it is about to receive back.
        world
            .systems
            .player
            .manager()
            .with_player_mut(caster, |player| {
                player.auras.container.add_aura(Aura::new(
                    9103,
                    caster,
                    0,
                    crate::game::player::auras::effects::AURA_DAMAGE_IMMUNITY,
                    1 << spell.school,
                    0,
                    None,
                    0,
                    1,
                    0,
                    AuraFlags {
                        is_positive: true,
                        ..AuraFlags::default()
                    },
                ));
            });

        let mut info = TargetInfo::new(target, 0b001);
        let results = apply_target_effects(&mut info, caster, None, spell.id, false, None, &world)
            .await
            .expect("reflect handling should not error");

        assert_eq!(info.miss_condition, SpellHitOutcome::Reflect);
        assert_eq!(info.reflect_result, SpellHitOutcome::Immune);
        assert!(
            results.is_empty(),
            "nothing lands when the caster cannot take the spell back"
        );
    }

    #[tokio::test]
    async fn damage_immunity_is_checked_before_the_reflect_roll() {
        let world = test_world();
        let caster = ObjectGuid::new_player(44);
        let target = ObjectGuid::new_player(45);
        add_test_player(&world, caster);
        add_test_player(&world, target);

        let mut spell = reflectable_spell(9203);
        spell.school = 3;
        world.managers.spell_mgr.add_spell(spell.clone());

        // The victim would reflect every cast, but it is also immune to the damage:
        // an immune victim returns immune before ever rolling reflect.
        give_reflect_aura(&world, target, 100, 1 << spell.school);
        world
            .systems
            .player
            .manager()
            .with_player_mut(target, |player| {
                player.auras.container.add_aura(Aura::new(
                    9104,
                    target,
                    0,
                    crate::game::player::auras::effects::AURA_DAMAGE_IMMUNITY,
                    1 << spell.school,
                    0,
                    None,
                    0,
                    1,
                    0,
                    AuraFlags {
                        is_positive: true,
                        ..AuraFlags::default()
                    },
                ));
            });

        assert_eq!(
            hit::roll_spell_hit(caster, target, spell.id, &world),
            SpellHitOutcome::Immune
        );
    }

    #[tokio::test]
    async fn spell_without_reflect_aura_is_not_reflected() {
        let world = test_world();
        let caster = ObjectGuid::new_player(46);
        let target = ObjectGuid::new_player(47);
        add_test_player(&world, caster);
        add_test_player(&world, target);

        let spell = reflectable_spell(9204);
        world.managers.spell_mgr.add_spell(spell.clone());

        for _ in 0..50 {
            assert_ne!(
                hit::roll_spell_hit(caster, target, spell.id, &world),
                SpellHitOutcome::Reflect,
                "reflect requires a reflect aura on the victim"
            );
        }
    }

    #[tokio::test]
    async fn mechanic_resistance_removes_only_the_matching_live_effect() {
        let world = test_world();
        let caster = ObjectGuid::new_player(34);
        let target = ObjectGuid::new_player(35);
        add_test_player(&world, caster);
        add_test_player(&world, target);
        world
            .systems
            .player
            .manager()
            .with_player_mut(target, |player| {
                player.auras.container.add_aura(Aura::new(
                    9002,
                    caster,
                    0,
                    crate::game::player::auras::effects::AURA_MOD_MECHANIC_RESISTANCE,
                    5,
                    100,
                    None,
                    0,
                    1,
                    0,
                    AuraFlags {
                        is_positive: true,
                        ..AuraFlags::default()
                    },
                ));
            });

        let mut spell = harmful_spell(110);
        spell.effect_mechanic[0] = 5;
        let mut mask = 0b001;
        assert!(do_spell_hit_on_unit(
            &spell, caster, target, &mut mask, false, false, false, &world
        )
        .await
        .is_none());
        assert_eq!(mask, 0);
    }

    #[tokio::test]
    async fn hostile_hit_removes_stealth_from_target_without_allow_flag() {
        let world = test_world();
        let caster = ObjectGuid::new_player(3);
        let target = ObjectGuid::new_player(4);
        add_test_player(&world, caster);
        add_test_player(&world, target);

        let spell = harmful_spell(102);
        assert!(!spell.is_positive_spell(), "test requires harmful spell");
        assert!(
            !spell.has_attribute_ex(SPELL_ATTR_EX_ALLOW_WHILE_STEALTHED),
            "test requires no allow-stealth flag"
        );
        give_aura(&world, target, 8100, SPELL_AURA_MOD_STEALTH, false);

        let mut mask = 0b001; // one effect bit
        let result = do_spell_hit_on_unit(
            &spell, caster, target, &mut mask, false, false, false, &world,
        )
        .await;

        assert!(
            result.is_some(),
            "hostile hit should continue with effect application"
        );
        assert!(
            !has_aura_type(&world, target, SPELL_AURA_MOD_STEALTH),
            "a hostile hit breaks the target's stealth"
        );
    }

    #[tokio::test]
    async fn allow_while_stealthed_suppresses_stealth_removal() {
        let world = test_world();
        let caster = ObjectGuid::new_player(5);
        let target = ObjectGuid::new_player(6);
        add_test_player(&world, caster);
        add_test_player(&world, target);

        let mut spell = harmful_spell(103);
        spell.attributes_ex |= SPELL_ATTR_EX_ALLOW_WHILE_STEALTHED;
        give_aura(&world, target, 8101, SPELL_AURA_MOD_STEALTH, false);

        let mut mask = 0b001;
        let result = do_spell_hit_on_unit(
            &spell, caster, target, &mut mask, false, false, false, &world,
        )
        .await;

        assert!(result.is_some(), "hit should continue");
        assert!(
            has_aura_type(&world, target, SPELL_AURA_MOD_STEALTH),
            "ALLOW_WHILE_STEALTHED leaves the target's stealth intact"
        );
    }

    #[tokio::test]
    async fn allow_while_invisible_suppresses_invis_removal() {
        let world = test_world();
        let caster = ObjectGuid::new_player(7);
        let target = ObjectGuid::new_player(8);
        add_test_player(&world, caster);
        add_test_player(&world, target);

        let mut spell = harmful_spell(104);
        spell.attributes_ex2 |= SPELL_ATTR_EX2_ALLOW_WHILE_INVISIBLE;
        give_aura(&world, target, 8102, SPELL_AURA_MOD_INVISIBILITY, false);

        let mut mask = 0b001;
        let result = do_spell_hit_on_unit(
            &spell, caster, target, &mut mask, false, false, false, &world,
        )
        .await;

        assert!(result.is_some(), "hit should continue");
        assert!(
            has_aura_type(&world, target, SPELL_AURA_MOD_INVISIBILITY),
            "ALLOW_WHILE_INVISIBLE leaves the target's invisibility intact"
        );
    }

    #[tokio::test]
    async fn not_an_action_suppresses_stealth_removal() {
        let world = test_world();
        let caster = ObjectGuid::new_player(9);
        let target = ObjectGuid::new_player(10);
        add_test_player(&world, caster);
        add_test_player(&world, target);

        let mut spell = harmful_spell(105);
        spell.attributes_ex2 |= SPELL_ATTR_EX2_NOT_AN_ACTION;
        give_aura(&world, target, 8103, SPELL_AURA_MOD_STEALTH, false);

        let mut mask = 0b001;
        let result = do_spell_hit_on_unit(
            &spell, caster, target, &mut mask, false, false, false, &world,
        )
        .await;

        assert!(result.is_some(), "hit should continue");
        assert!(
            has_aura_type(&world, target, SPELL_AURA_MOD_STEALTH),
            "a spell that is not an action does not break stealth"
        );
    }

    #[tokio::test]
    async fn possess_spell_skips_combat_entry() {
        let world = test_world();
        let mut spell = harmful_spell(106);
        // Make the spell apply MOD_POSSESS aura
        spell.effect[0] = 6; // apply-aura effect
        spell.effect_apply_aura_name[0] = SPELL_AURA_MOD_POSSESS;
        let mut mask = 0b001;

        let result = do_spell_hit_on_unit(
            &spell,
            ObjectGuid::new_player(11),
            ObjectGuid::new_creature(3, 50),
            &mut mask,
            false,
            false,
            false,
            &world,
        )
        .await;

        assert!(
            result.is_some(),
            "possess should continue with effect application"
        );
        // Combat entry should be skipped for possess spells.
    }

    #[tokio::test]
    async fn no_threat_attr_skips_combat_entry() {
        let world = test_world();
        let mut spell = harmful_spell(107);
        spell.attributes_ex |= SPELL_ATTR_EX_NO_THREAT;
        let mut mask = 0b001;

        let result = do_spell_hit_on_unit(
            &spell,
            ObjectGuid::new_player(13),
            ObjectGuid::new_creature(4, 50),
            &mut mask,
            false,
            false,
            false,
            &world,
        )
        .await;

        assert!(
            result.is_some(),
            "no-threat hit should still continue with effects"
        );
    }
}
