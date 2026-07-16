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
use crate::dbc::structures::SpellEntry;
use crate::game::player::auras::proc;
use crate::World;
use anyhow::Result;
use oxcore_shared::protocol::ObjectGuid;

// ─── MaNGOS spell attribute constants ─────────────────────────────────────────
// Reference: SpellDefines.h

/// SPELL_ATTR_NOT_IN_COMBAT_ONLY_PEACEFUL — spell cannot be used in combat.
const SPELL_ATTR_NOT_IN_COMBAT_ONLY_PEACEFUL: u32 = 0x1000_0000; // bit 28

/// SPELL_ATTR_EX_ALLOW_WHILE_STEALTHED — does not break stealth.
const SPELL_ATTR_EX_ALLOW_WHILE_STEALTHED: u32 = 0x0000_0020; // bit 5

/// SPELL_ATTR_EX_NO_THREAT — no threat generated.
const SPELL_ATTR_EX_NO_THREAT: u32 = 0x0000_0400; // bit 10

/// SPELL_ATTR_EX_ONLY_PEACEFUL_TARGETS — target must not be in combat.
const SPELL_ATTR_EX_ONLY_PEACEFUL_TARGETS: u32 = 0x0000_0100; // bit 8

/// SPELL_ATTR_EX_THREAT_ONLY_ON_MISS — threat only when spell misses.
const SPELL_ATTR_EX_THREAT_ONLY_ON_MISS: u32 = 0x0020_0000; // bit 21

/// SPELL_ATTR_EX2_ALLOW_WHILE_INVISIBLE — does not break invisibility.
const SPELL_ATTR_EX2_ALLOW_WHILE_INVISIBLE: u32 = 0x0000_4000; // bit 14

/// SPELL_ATTR_EX2_NOT_AN_ACTION — not considered an action.
const SPELL_ATTR_EX2_NOT_AN_ACTION: u32 = 0x1000_0000; // bit 28

/// SPELL_ATTR_EX2_NO_INITIAL_THREAT — no initial threat on cast.
const SPELL_ATTR_EX2_NO_INITIAL_THREAT: u32 = 0x0040_0000; // bit 22

/// SPELL_ATTR_EX3_PVP_ENABLING — counts as hostile for PvP even without combat.
const SPELL_ATTR_EX3_PVP_ENABLING: u32 = 0x0000_0001; // bit 0

// ─── Aura type constants ──────────────────────────────────────────────────────
// Reference: SpellAuraDefines.h

/// SPELL_AURA_MOD_POSSESS = 2
const SPELL_AURA_MOD_POSSESS: u32 = 2;
/// SPELL_AURA_MOD_STEALTH = 16
const SPELL_AURA_MOD_STEALTH: u32 = 16;
/// SPELL_AURA_MOD_INVISIBILITY = 18
const SPELL_AURA_MOD_INVISIBILITY: u32 = 18;
/// SPELL_EFFECT_DISPEL = 38
const SPELL_EFFECT_DISPEL: u32 = 38;
/// SPELL_EFFECT_SCHOOL_DAMAGE = 2
const SPELL_EFFECT_SCHOOL_DAMAGE: u32 = 2;
/// SPELL_AURA_MOD_POSSESS_PET = 128
const SPELL_AURA_MOD_POSSESS_PET: u32 = 128;

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

/// Per-unit pre-effect processing (MaNGOS `Spell::DoSpellHitOnUnit` chunk_0,
/// Spell.cpp lines 1531-1680).
///
/// Called on a landed hit, **before** dispatching individual effects.  Handles:
///
/// 1. **Zero-effect-mask** — when `effect_mask` is zero but the spell is not
///    positive toward the target, still flags combat via `AttackedBy` (here:
///    `apply_damage(0)` for creatures, `in_combat = true` for players).
///
/// 2. **Per-effect mechanic resistance** — each effect bit whose mechanic the
///    target resists is cleared.  When the mask becomes zero the hit is aborted.
///    *Approximated:* the MaNGOS `Unit::IsEffectResist()` call is not yet ported;
///    this step is omitted until that helper exists — see TODO.
///
/// 3. **Delayed-spell immunity/evasion** — if the caster is not the target and
///    the spell has `speed > 0`, and the target is immune to the spell or its
///    damage school, the hit is aborted (`SPELL_MISS_IMMUNE`).  Also, delayed
///    negative spells against friendly targets (post-duel) are aborted
///    (`SPELL_MISS_EVADE`).  *Approximated:* `IsImmuneToDamage`/`IsImmuneToSpell`
///    are not ported; the immunity branches are structured but inert until those
///    dependency APIs land.
///
/// 4. **Stealth/invisibility removal** — on hostile hits, removes
///    `SPELL_AURA_MOD_STEALTH` and `SPELL_AURA_MOD_INVISIBILITY` from the
///    target, unless the spell's `AttributesEx`/`AttributesEx2` carry the
///    `ALLOW_WHILE_STEALTHED` / `ALLOW_WHILE_INVISIBLE` flags.  Game-object
///    casters (traps, etc.) skip stealth removal.
///    *Approximated:* `RemoveSpellsCausingAura` → `remove_auras_by_type` for
///    players; creature stealth removal is stub-only (no creature aura system).
///
/// 5. **Visibility check for delayed spells** — a delayed spell targeting the
///    explicit unit target that has become non-visible to the caster is evaded.
///    *Approximated:* `IsVisibleForOrDetect` not ported; skip until visibility
///    API lands.
///
/// 6. **Combat/threat entry** — complex attribute-gated combat start with
///    `AttackedBy`, `AddThreat`, `SetInCombatWithAggressor`/`SetInCombatWithVictim`,
///    stealth removal from the *caster*, and related PvP-enabling fallthrough.
///    *Approximated:* uses `enter_combat_on_miss`-style `apply_damage(0)`
///    and `add_threat` / `set_in_combat` for creatures when the gate passes.
///    Caster-stealth removal is a TODO.
///
/// 7. **Friendly-target assist/PvP** — when a friendly target is in combat and
///    the spell would generate threat, the caster enters assisted combat and
///    distributes assist-threat.  PvP flagging (`UpdatePvP`) is not ported.
///
/// 8. **Diminishing-returns snapshot + aura-holder creation** — stashes the DR
///    group/level for the hit and creates an empty `SpellAuraHolder` when the
///    spell applies auras.  *Aura-holder creation not yet wired;* the `effectMask`
///    argument to `DoSpellHitOnUnit` starts as the set of bits that survived resist
///    checks, and later `HandleEffects` runs only for those bits.  This function
///    modifies `effect_mask` in-place.
///
/// # Returns
/// `false` — hit should be aborted (all effects resisted, delayed-spell immune
/// or evaded).  `true` — continue with effect dispatch.
async fn do_spell_hit_on_unit(
    spell_entry: &SpellEntry,
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    effect_mask: &mut u8,
    is_delayed: bool,
    is_triggered: bool,
    world: &World,
) -> bool {
    // ── Step 3.  Zero effect mask: just flag combat if non-positive ────────────
    // (C++ lines 1540-1545)
    if *effect_mask == 0 {
        if !spell_entry.is_positive_spell() {
            enter_combat_on_miss(caster_guid, target_guid, world);
        }
        return false;
    }

    // ── Step 4.  Per-effect mechanic resistance ───────────────────────────────
    // (C++ lines 1547-1554)
    // TODO: MaNGOS `Unit::IsEffectResist(m_spellInfo, eff)` not yet ported.
    // When it lands, iterate 0..3, clear bit `eff` from `*effect_mask` when the
    // target resists that effect's mechanic, and return false if mask becomes 0.

    // ── Step 5.  Delayed-spell immunity re-check ──────────────────────────────
    // (C++ lines 1556-1566)
    // TODO: `IsImmuneToDamage` / `IsImmuneToSpell` not ported.  When they land,
    // check: if caster != target && spell.speed > 0 && (immunity), send miss and
    // return false.

    // ── Step 6a.  Hostile-side side effects ────────────────────────────────────
    // (C++ lines 1568-1641)
    // TODO: `IsFriendlyTo(unit)` not ported — use `is_positive_spell()` as proxy.
    let caster_is_player = caster_guid.is_player();

    if !spell_entry.is_positive_spell() {
        // AURA_INTERRUPT_HOSTILE_ACTION_RECEIVED_CANCELS not yet wired
        // (C++ line 1572-1573) — skip until interrupt flags are modelled.

        // Stealth/invisibility removal on target (C++ lines 1575-1583)
        // Game-object caster check: if caster_guid is not a player, we approximate
        // as "caster is a game object" — traps etc. do not break stealth.
        let caster_is_object =
            !caster_is_player && !target_guid.is_creature() && !target_guid.is_player();

        if !caster_is_object && !spell_entry.has_attribute(SPELL_ATTR_EX2_NOT_AN_ACTION) {
            if !spell_entry.has_attribute(SPELL_ATTR_EX_ALLOW_WHILE_STEALTHED) {
                remove_aura_type_from_target(target_guid, SPELL_AURA_MOD_STEALTH, world);
            }
            if !spell_entry.has_attribute(SPELL_ATTR_EX2_ALLOW_WHILE_INVISIBLE) {
                remove_aura_type_from_target(target_guid, SPELL_AURA_MOD_INVISIBILITY, world);
            }
        }

        // Delayed-spell visibility check (C++ lines 1585-1594)
        // TODO: `IsVisibleForOrDetect` not ported — skip until it lands.

        // Combat/threat entry main gate (C++ lines 1596-1641)
        let is_sap =
            spell_entry.spell_family_name == 8 && (spell_entry.spell_family_flags & 0x80) != 0; // CF_ROGUE_SAP approximation
        let is_trap = !caster_is_player && target_guid.is_player();

        let can_enter_combat = (!spell_entry.is_positive_spell()
            || spell_entry.has_effect(SPELL_EFFECT_DISPEL))
            && !is_trap
            && !is_sap;

        // Visibility to target (C++ lines 1600-1601) — approximated as always true
        // until `IsVisibleForOrDetect` lands: `m_caster->IsVisibleForOrDetect(unit, unit, false)`.
        let caster_visible_to_target = true;

        if can_enter_combat && caster_visible_to_target {
            // Gate: not (triggered-by-aura without speed/threat)  (C++ lines 1603-1606)
            let can_threat =
                (!is_triggered || spell_entry.speed > 0.0 || has_direct_threat_effect(spell_entry))
                    && !spell_entry.has_attribute(SPELL_ATTR_EX_NO_THREAT)
                    && !spell_entry.has_attribute(SPELL_ATTR_EX_THREAT_ONLY_ON_MISS)
                    && !spell_entry.has_attribute(SPELL_ATTR_EX2_NO_INITIAL_THREAT);

            if can_threat {
                // TODO: caster stealth removal (C++ lines 1609-1615) not ported —
                // would remove stealth/invisibility from the caster unit.

                // Enter combat (C++ lines 1617-1627)
                if !spell_has_aura_type(spell_entry, SPELL_AURA_MOD_POSSESS)
                    && !spell_has_aura_type(spell_entry, SPELL_AURA_MOD_POSSESS_PET)
                {
                    enter_combat_on_hit(caster_guid, target_guid, world);
                }
                set_caster_in_combat_with_victim(caster_guid, target_guid, world);
            } else if spell_entry.has_attribute(SPELL_ATTR_EX3_PVP_ENABLING) {
                // PvP-enabling only (C++ lines 1629-1634)
                set_out_of_combat(caster_guid, target_guid, world);
            }
        } else {
            // Fallthrough: spell did not pass the combat-entry gate
            // (C++ lines 1635-1641)
            let not_only_peaceful = !spell_entry
                .has_attribute(SPELL_ATTR_NOT_IN_COMBAT_ONLY_PEACEFUL)
                || !spell_entry.has_attribute(SPELL_ATTR_EX_ONLY_PEACEFUL_TARGETS);
            if not_only_peaceful {
                set_out_of_combat(caster_guid, target_guid, world);
            }
        }
    } else {
        // ── Step 6b.  Friendly-target side effects ────────────────────────────
        // (C++ lines 1643-1678)

        // Delayed negative spells on friendly targets (post-duel) → evade
        // (C++ lines 1645-1651)
        if is_delayed && !spell_entry.is_positive_spell() {
            return false;
        }

        // Combat assist (C++ lines 1656-1667)
        // TODO: `IsInCombat` check not directly exposed — approximated via
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
            && !spell_entry.has_attribute(SPELL_ATTR_EX_NO_THREAT)
            && !is_triggered
            && !spell_entry.has_attribute(SPELL_ATTR_EX2_NO_INITIAL_THREAT)
        {
            assisted_combat(caster_guid, target_guid, world);
        }
        // TODO: PvP flagging (C++ lines 1669-1675) — `UpdatePvP` not ported.
    }

    // ── Step 7.  Diminishing-returns snapshot + aura-holder setup ──────────────
    // (C++ lines 1681-1700)
    // TODO: DR group/level and aura-holder creation are not wired here yet.
    // The effect loop below dispatches effects directly; the aura system
    // creates holders as effects are applied.  When the DR/aura-holder
    // pipeline is refactored, the DR snapshot (`GetDiminishingReturnsGroup`,
    // `GetDiminishing`) belongs here.

    true
}

/// `SpellEntry::HasDirectThreatIncreaseEffect()` — true when any effect type
/// carries immediate threat (SPELL_EFFECT_ATTACK_ME, etc.).
fn has_direct_threat_effect(spell_entry: &SpellEntry) -> bool {
    const SPELL_EFFECT_ATTACK_ME: u32 = 74;
    const SPELL_EFFECT_THREAT_ALL: u32 = 116;
    spell_entry
        .effect
        .iter()
        .any(|&e| e == SPELL_EFFECT_ATTACK_ME || e == SPELL_EFFECT_THREAT_ALL)
}

/// Check whether the spell applies a specific aura type in any of its effect
/// slots (MaNGOS: `m_spellInfo->HasAura(SPELL_AURA_MOD_POSSESS)` etc.).
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
        1   // TARGET_UNIT_FRIEND
            | 6   // TARGET_UNIT_FRIEND_AREA (deprecated)
            | 11  // TARGET_UNIT_FRIEND_AREA
            | 24  // TARGET_UNIT_FRIEND_AREA
            | 44  // TARGET_UNIT_FRIEND_AREA
            | 45  // TARGET_UNIT_FRIEND_AREA
            | 52  // TARGET_UNIT_FRIEND_AREA
            | 53  // TARGET_UNIT_FRIEND_AREA
            | 54 // TARGET_UNIT_FRIEND_AREA
    )
}

/// Remove a specific aura type from a unit target if possible.
fn remove_aura_type_from_target(target_guid: ObjectGuid, aura_type: u32, world: &World) {
    if target_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |p| {
                p.auras.container.remove_auras_by_type(aura_type);
            });
    }
    // Creature aura removal by type is not yet exposed.
}

/// Enter combat for a landed hit (MaNGOS: `unit->AttackedBy(pRealUnitCaster)`,
/// `unit->AddThreat(pRealUnitCaster)`, `SetInCombatWithAggressor`,
/// `SetInCombatWithVictim`).
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

/// Flag the caster as being in combat with the victim (MaNGOS:
/// `pRealUnitCaster->SetInCombatWithVictim(unit)`).
fn set_caster_in_combat_with_victim(
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

/// Clear the combat/aggressor relationship (MaNGOS:
/// `SetOutOfCombatWithAggressor` / `SetOutOfCombatWithVictim`).
fn set_out_of_combat(caster_guid: ObjectGuid, target_guid: ObjectGuid, world: &World) {
    // Not all combat systems expose this directly; no-op is safe for PvP-enabling.
    let _ = (caster_guid, target_guid, world);
}

/// Place the caster into assisted combat alongside the friendly target
/// (MaNGOS: `pRealUnitCaster->SetInCombatWithAssisted(unit)`,
/// `unit->GetHostileRefManager().threatAssist(...)`).
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

            // Per-unit pre-effect processing (DoSpellHitOnUnit chunk_0):
            // combat entry, stealth removal, resist checks, DR snapshot, etc.
            let is_delayed = spell_entry.speed > 0.0;
            if !do_spell_hit_on_unit(
                &spell_entry,
                caster_guid,
                target.target_guid,
                &mut target.effect_mask,
                is_delayed,
                is_triggered,
                world,
            )
            .await
            {
                // All effects resisted/aborted — nothing to dispatch.
                return Ok(results);
            }
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
    use crate::config::Config;
    use oxcore_shared::database::Databases;
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
            logs: lazy_pool(),
        });
        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
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
        assert!(is_friendly_target(1)); // TARGET_UNIT_FRIEND
        assert!(is_friendly_target(11)); // TARGET_UNIT_FRIEND_AREA
        assert!(!is_friendly_target(16)); // TARGET_ENUM_UNITS_ENEMY_AOE_AT_DEST_LOC
        assert!(!is_friendly_target(0));
        assert!(!is_friendly_target(99));
    }

    #[test]
    fn has_direct_threat_effect_detects_threat_effects() {
        let mut entry = harmful_spell(1);
        // No threat effects by default
        assert!(!has_direct_threat_effect(&entry));

        entry.effect[0] = 74; // SPELL_EFFECT_ATTACK_ME
        assert!(has_direct_threat_effect(&entry));

        entry.effect[0] = 116; // SPELL_EFFECT_THREAT_ALL
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
            &world,
        )
        .await;

        assert!(!result, "zero mask should abort hit");
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
            &world,
        )
        .await;

        assert!(
            !result,
            "zero mask should abort hit even for positive spells"
        );
    }

    #[tokio::test]
    async fn hostile_hit_removes_stealth_from_target_without_allow_flag() {
        let world = test_world();
        let spell = harmful_spell(102);
        assert!(!spell.is_positive_spell(), "test requires harmful spell");
        assert!(
            !spell.has_attribute(SPELL_ATTR_EX_ALLOW_WHILE_STEALTHED),
            "test requires no allow-stealth flag"
        );

        let mut mask = 0b001; // one effect bit

        let result = do_spell_hit_on_unit(
            &spell,
            ObjectGuid::new_player(3),
            ObjectGuid::new_player(4),
            &mut mask,
            false,
            false,
            &world,
        )
        .await;

        assert!(
            result,
            "hostile hit should continue with effect application"
        );
        // Stealth removal is called; the player target has no stealth to remove
        // so the call is a no-op but should not panic.
    }

    #[tokio::test]
    async fn allow_while_stealthed_suppresses_stealth_removal() {
        let world = test_world();
        let mut spell = harmful_spell(103);
        spell.attributes_ex |= SPELL_ATTR_EX_ALLOW_WHILE_STEALTHED;
        let mut mask = 0b001;

        let result = do_spell_hit_on_unit(
            &spell,
            ObjectGuid::new_player(5),
            ObjectGuid::new_player(6),
            &mut mask,
            false,
            false,
            &world,
        )
        .await;

        assert!(result, "hit should continue");
    }

    #[tokio::test]
    async fn allow_while_invisible_suppresses_invis_removal() {
        let world = test_world();
        let mut spell = harmful_spell(104);
        spell.attributes_ex2 |= SPELL_ATTR_EX2_ALLOW_WHILE_INVISIBLE;
        let mut mask = 0b001;

        let result = do_spell_hit_on_unit(
            &spell,
            ObjectGuid::new_player(7),
            ObjectGuid::new_player(8),
            &mut mask,
            false,
            false,
            &world,
        )
        .await;

        assert!(result, "hit should continue");
    }

    #[tokio::test]
    async fn not_an_action_suppresses_stealth_removal() {
        let world = test_world();
        let mut spell = harmful_spell(105);
        spell.attributes_ex2 |= SPELL_ATTR_EX2_NOT_AN_ACTION;
        let mut mask = 0b001;

        let result = do_spell_hit_on_unit(
            &spell,
            ObjectGuid::new_player(9),
            ObjectGuid::new_player(10),
            &mut mask,
            false,
            false,
            &world,
        )
        .await;

        assert!(result, "hit should continue");
    }

    #[tokio::test]
    async fn possess_spell_skips_combat_entry() {
        let world = test_world();
        let mut spell = harmful_spell(106);
        // Make the spell apply MOD_POSSESS aura
        spell.effect[0] = 6; // SPELL_EFFECT_APPLY_AURA
        spell.effect_apply_aura_name[0] = SPELL_AURA_MOD_POSSESS;
        let mut mask = 0b001;

        let result = do_spell_hit_on_unit(
            &spell,
            ObjectGuid::new_player(11),
            ObjectGuid::new_creature(3, 50),
            &mut mask,
            false,
            false,
            &world,
        )
        .await;

        assert!(result, "possess should continue with effect application");
        // Combat entry should be skipped (AttackedBy not called for possess).
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
            &world,
        )
        .await;

        assert!(result, "no-threat hit should still continue with effects");
    }
}
