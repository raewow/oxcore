//! Spell Cast Validation
//!
//! All pre-cast validation checks are pure functions that read player state.

use crate::game::player::spells::state::{SpellCastError, NUM_SPELL_SCHOOLS};
use crate::World;
use anyhow::Result;
use oxcore_shared::protocol::ObjectGuid;

/// Get current game time in milliseconds
fn get_game_time_ms(world: &World) -> u64 {
    // Use system time for now; in a full implementation this would use world game time
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Validate a spell cast. Returns SpellCastError::None on success.
///
/// Checks performed in order:
/// 1. Player knows the spell
/// 2. Caster is alive
/// 3. Not stunned, silenced, pacified, confused, fleeing
/// 4. School is not locked out
/// 5. Spell is not on cooldown
/// 6. GCD is not active (unless triggered)
/// 7. Has enough resources (mana/rage/energy)
/// 8. Not already casting (unless triggered)
/// 9. Target is valid
/// 10. Target is in range
/// 11. Not moving (for most cast-time spells)
pub fn validate_cast(
    caster_guid: ObjectGuid,
    spell_id: u32,
    _target_guid: Option<ObjectGuid>,
    is_triggered: bool,
    world: &World,
) -> Result<SpellCastError> {
    let now = get_game_time_ms(world);
    let mut error = SpellCastError::None;

    // Snapshot of spell data needed for validation (read before player lock)
    struct SpellData {
        school: u8,
        power_type: u8,
        mana_cost: u32,
        range_index: u32,
        max_range: f32,
        cast_time: u32,
        attributes_ex2: u32,
        stances: u32,
        stances_not: u32,
        caster_aura_state: u32,
        target_aura_state: u32,
    }

    let spell_data = match world.managers.spell_mgr.get(spell_id) {
        Some(s) => {
            let max_range = if s.range_index > 0 {
                let dbc = world.dbc.read();
                dbc.get_spell_range(s.range_index)
                    .map(|r| r.range_max)
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            let cast_time = if s.casting_time_index > 0 {
                let dbc = world.dbc.read();
                dbc.get_spell_cast_time(s.casting_time_index)
                    .map(|ct| ct.cast_time.max(0) as u32)
                    .unwrap_or(0)
            } else {
                0
            };
            SpellData {
                school: s.school as u8,
                power_type: s.power_type as u8,
                mana_cost: s.mana_cost,
                range_index: s.range_index,
                max_range,
                cast_time,
                attributes_ex2: s.attributes_ex2,
                stances: s.stances,
                stances_not: s.stances_not,
                caster_aura_state: s.caster_aura_state,
                target_aura_state: s.target_aura_state,
            }
        }
        None => SpellData {
            school: 0,
            power_type: 0,
            mana_cost: 0,
            range_index: 0,
            max_range: 0.0,
            cast_time: 0,
            attributes_ex2: 0,
            stances: 0,
            stances_not: 0,
            caster_aura_state: 0,
            target_aura_state: 0,
        },
    };

    let spell_school = spell_data.school;
    let spell_power_type = spell_data.power_type;
    let spell_mana_cost = spell_data.mana_cost;
    let spell_range_index = spell_data.range_index;
    let spell_max_range = spell_data.max_range;
    let spell_cast_time = spell_data.cast_time;

    world.systems.player.manager().with_player_mut(caster_guid, |player| {
        // 1. Check spell is known (skip for triggered spells - they might be procs)
        if !is_triggered && !player.spells.knows_spell(spell_id) {
            error = SpellCastError::SpellNotKnown;
            return;
        }

        // 2. Check caster is alive
        if player.stats.health == 0 {
            error = SpellCastError::CasterDead;
            return;
        }

        // 3. Check unit state flags
        {
            use crate::game::player::auras::effects::{
                UNIT_FLAG_STUNNED, UNIT_FLAG_CONFUSED, UNIT_FLAG_FLEEING,
                UNIT_FLAG_SILENCED, UNIT_FLAG_PACIFIED,
            };

            let flags = player.unit_flags;
            if flags & UNIT_FLAG_STUNNED != 0 {
                error = SpellCastError::Stunned;
                return;
            }
            if flags & UNIT_FLAG_SILENCED != 0 && spell_school != 0 {
                // Silence only blocks non-physical spells
                error = SpellCastError::Silenced;
                return;
            }
            if flags & UNIT_FLAG_PACIFIED != 0 {
                error = SpellCastError::Pacified;
                return;
            }
            if flags & UNIT_FLAG_CONFUSED != 0 {
                error = SpellCastError::Confused;
                return;
            }
            if flags & UNIT_FLAG_FLEEING != 0 {
                error = SpellCastError::Fleeing;
                return;
            }
        }

        // 3b. Check shapeshift form requirements (stances)
        // stances is a bitmask: if nonzero, player must be in one of the listed forms
        // stances_not: if nonzero, player must NOT be in one of the listed forms
        if !is_triggered && (spell_data.stances != 0 || spell_data.stances_not != 0) {
            let form = player.shapeshift_form;
            let form_bit = if form > 0 { 1u32 << (form - 1) } else { 0 };

            // stances: must be in one of these forms (0 = any form allowed)
            if spell_data.stances != 0 && (form_bit == 0 || (spell_data.stances & form_bit) == 0) {
                error = SpellCastError::WrongShapeshift;
                return;
            }

            // stances_not: must NOT be in any of these forms
            if spell_data.stances_not != 0 && form_bit != 0 && (spell_data.stances_not & form_bit) != 0 {
                error = SpellCastError::WrongShapeshift;
                return;
            }
        }

        // 3c. Check caster aura state requirements (e.g., Execute requires target < 20% HP)
        // Aura states are conditions like AURASTATE_HEALTHLESS_20_PERCENT
        if !is_triggered && spell_data.caster_aura_state != 0 {
            // Check caster aura state flags
            // AURASTATE_FLAG_DODGE_BLOCK (1), AURASTATE_FLAG_HEALTH_20_PCT (2), etc.
            let aura_state_bit = 1u32 << (spell_data.caster_aura_state - 1);
            let caster_aura_state_flags = player.aura_state_flags;
            if (caster_aura_state_flags & aura_state_bit) == 0 {
                error = SpellCastError::CasterAuraState;
                return;
            }
        }

        // 4. Check school lockout
        if (spell_school as usize) < NUM_SPELL_SCHOOLS {
            if player.spells.is_school_locked(spell_school, now) {
                error = SpellCastError::SchoolLockout;
                return;
            }
        }

        // 5. Check spell cooldown (skip for triggered)
        if !is_triggered {
            if player.spells.is_on_cooldown(spell_id, now) {
                error = SpellCastError::SpellOnCooldown;
                return;
            }
            // Also check category cooldown
            if let Some(entry) = world.managers.spell_mgr.get(spell_id) {
                if entry.category > 0 {
                    if let Some(&cd_end) = player.spells.category_cooldowns.get(&entry.category) {
                        if cd_end > now {
                            error = SpellCastError::SpellOnCooldown;
                            return;
                        }
                    }
                }
            }
        }

        // 6. Check GCD (skip for triggered)
        if !is_triggered && player.spells.is_on_gcd(now) {
            error = SpellCastError::NotReady;
            return;
        }

        // 7. Check resources (skip for triggered spells)
        if !is_triggered && spell_mana_cost > 0 {
            let current_power = player.power.current[spell_power_type as usize];
            if current_power < spell_mana_cost {
                error = match spell_power_type {
                    0 => SpellCastError::NotEnoughMana,
                    1 => SpellCastError::NotEnoughRage,
                    3 => SpellCastError::NotEnoughEnergy,
                    _ => SpellCastError::NotEnoughMana,
                };
                return;
            }
        }

        // 8. Check not already casting in Generic/Channeled slot (skip for triggered)
        if !is_triggered && player.spells.is_casting() {
            error = SpellCastError::AlreadyCasting;
            return;
        }

        // 9. Validate target (exists, alive, valid type)
        // Read target_guid from the function parameter by rebinding
        let target_guid_val = _target_guid;
        if let Some(target) = target_guid_val {
            if target != caster_guid {
                let target_exists = if target.is_player() {
                    world.systems.player.manager().with_player(target, |_| true).unwrap_or(false)
                } else if target.is_creature() {
                    world.managers.creature_mgr.with_creature(target, |_| true).unwrap_or(false)
                } else {
                    true // GameObjects etc. - skip validation for now
                };
                if !target_exists {
                    error = SpellCastError::InvalidTarget;
                    return;
                }

                // Check if target is dead (unless spell targets dead)
                // SPELL_ATTR_EX2_CAN_TARGET_DEAD = 0x00000800
                let can_target_dead = (spell_data.attributes_ex2 & 0x00000800) != 0;
                if !can_target_dead {
                    let target_dead = if target.is_player() {
                        world.systems.player.manager().with_player(target, |p| p.stats.health == 0).unwrap_or(false)
                    } else if target.is_creature() {
                        world.managers.creature_mgr.with_creature(target, |c| c.current_health == 0).unwrap_or(false)
                    } else {
                        false
                    };
                    if target_dead {
                        error = SpellCastError::InvalidTarget;
                        return;
                    }
                }

                // 9b. Check target aura state requirements (e.g., Execute needs target < 20% HP)
                if !is_triggered && spell_data.target_aura_state != 0 {
                    let target_state_met = if target.is_player() {
                        world.systems.player.manager().with_player(target, |p| {
                            let bit = 1u32 << (spell_data.target_aura_state - 1);
                            (p.aura_state_flags & bit) != 0
                        }).unwrap_or(false)
                    } else {
                        // Creatures: check health percentage for common aura states
                        // AURASTATE_HEALTHLESS_20_PERCENT = 2
                        if spell_data.target_aura_state == 2 {
                            world.managers.creature_mgr.with_creature(target, |c| {
                                c.max_health > 0 && (c.current_health * 100 / c.max_health) < 20
                            }).unwrap_or(false)
                        } else {
                            true // Unknown state — allow
                        }
                    };
                    if !target_state_met {
                        error = SpellCastError::TargetAuraState;
                        return;
                    }
                }
            }
        }

        // 10. Range check (from SpellRange.dbc)
        if let Some(target) = target_guid_val {
            if target != caster_guid && spell_range_index > 0 {
                let target_pos = if target.is_player() {
                    world.systems.player.manager().with_player(target, |p| p.movement.position)
                } else if target.is_creature() {
                    world.managers.creature_mgr.with_creature(target, |c| {
                        oxcore_shared::protocol::Position {
                            x: c.position.x,
                            y: c.position.y,
                            z: c.position.z,
                            o: c.position.o,
                        }
                    })
                } else {
                    None
                };

                if let Some(target_pos) = target_pos {
                    let dx = player.movement.position.x - target_pos.x;
                    let dy = player.movement.position.y - target_pos.y;
                    let dz = player.movement.position.z - target_pos.z;
                    let distance = (dx * dx + dy * dy + dz * dz).sqrt();

                    if distance > spell_max_range + 2.0 {
                        // +2.0 yards tolerance for hitbox
                        tracing::warn!(
                            "[SPELL_RANGE] spell={spell_id} range_index={} max_range={:.1} distance={:.1} caster=({:.1},{:.1},{:.1}) target=({:.1},{:.1},{:.1}) target_guid={:?}",
                            spell_range_index, spell_max_range, distance,
                            player.movement.position.x, player.movement.position.y, player.movement.position.z,
                            target_pos.x, target_pos.y, target_pos.z, target
                        );
                        error = SpellCastError::TargetOutOfRange;
                        return;
                    }
                }
            }
        }

        // 11. Movement check (most cast-time spells can't be cast while moving)
        // Check if player has movement flags and spell has a cast time
        if !is_triggered && spell_cast_time > 0 {
            // MOVEFLAG_FORWARD | MOVEFLAG_BACKWARD | MOVEFLAG_STRAFE_LEFT | MOVEFLAG_STRAFE_RIGHT
            const MOVING_FLAGS: u32 = 0x01 | 0x02 | 0x04 | 0x08;
            if player.movement.movement_flags & MOVING_FLAGS != 0 {
                error = SpellCastError::NotWhileMoving;
                return;
            }
        }
    });

    if error != SpellCastError::None {
        tracing::warn!("[SPELL_VALIDATE] spell={spell_id} caster={caster_guid:?} failed={error:?} (client_code=0x{:02X})", spell_cast_error_to_u8(error));
    }

    Ok(error)
}

/// Check if a spell can be cast while moving.
///
/// In Vanilla WoW, most spells with a cast time cannot be cast while moving.
/// Returns true only for instant-cast spells and special flagged spells.
pub fn can_cast_while_moving(spell_id: u32, world: &World) -> bool {
    let spell_entry = match world.managers.spell_mgr.get(spell_id) {
        Some(entry) => entry,
        None => return true, // Unknown spell = allow
    };

    // Check cast time
    if spell_entry.casting_time_index == 0 {
        return true; // Instant cast = can move
    }

    // Check SPELL_ATTR_EX5_USABLE_WHILE_MOVING or similar flags
    // For vanilla 1.12.1, there's no such flag - cast-time spells always cancel on move
    false
}

/// Convert SpellCastError to the vanilla 1.12.1 client SpellCastResult enum value.
/// Values from mangos/src/game/Spells/SpellDefines.h (vanilla 1.12.x build).
pub fn spell_cast_error_to_u8(error: SpellCastError) -> u8 {
    use SpellCastError::*;
    match error {
        None => 0,
        CasterDead => 0x13,             // SPELL_FAILED_CASTER_DEAD
        SpellNotKnown => 0x38,          // SPELL_FAILED_NOT_KNOWN
        NotEnoughMana => 0x4D,          // SPELL_FAILED_NO_POWER (mana/rage/energy all use this)
        NotEnoughRage => 0x4D,          // SPELL_FAILED_NO_POWER
        NotEnoughEnergy => 0x4D,        // SPELL_FAILED_NO_POWER
        SpellOnCooldown => 0x3C,        // SPELL_FAILED_NOT_READY
        NotReady => 0x3C,               // SPELL_FAILED_NOT_READY (GCD)
        InvalidTarget => 0x0A,          // SPELL_FAILED_BAD_TARGETS
        TargetOutOfRange => 0x59,       // SPELL_FAILED_OUT_OF_RANGE
        TargetNotInLineOfSight => 0x2A, // SPELL_FAILED_LINE_OF_SIGHT
        NotWhileMoving => 0x2E,         // SPELL_FAILED_MOVING
        Stunned => 0x64,                // SPELL_FAILED_STUNNED
        Silenced => 0x60,               // SPELL_FAILED_SILENCED
        Pacified => 0x5A,               // SPELL_FAILED_PACIFIED
        Confused => 0x16,               // SPELL_FAILED_CONFUSED
        Fleeing => 0x1E,                // SPELL_FAILED_FLEEING
        SchoolLockout => 0x60,          // SPELL_FAILED_SILENCED (school lockout shows as silenced)
        AlreadyCasting => 0x61,         // SPELL_FAILED_SPELL_IN_PROGRESS
        Interrupted => 0x23,            // SPELL_FAILED_INTERRUPTED
        WrongShapeshift => 0x56,        // SPELL_FAILED_ONLY_SHAPESHIFT
        CasterAuraState => 0x12,        // SPELL_FAILED_CASTER_AURASTATE
        TargetAuraState => 0x67,        // SPELL_FAILED_TARGET_AURASTATE
        NoComboPoints => 0x46,          // SPELL_FAILED_NO_COMBO_POINTS
        DontReport => 0x18,             // SPELL_FAILED_DONT_REPORT (silent)
        OutOfRange => 0x59,             // SPELL_FAILED_OUT_OF_RANGE
        TooClose => 0x69,               // SPELL_FAILED_TOO_CLOSE
        NotInFront => 0x41,             // SPELL_FAILED_UNIT_NOT_INFRONT
        LowCastLevel => 0x28,           // SPELL_FAILED_LOW_CASTLEVEL
        // Item check errors
        AlreadyAtFullHealth => 0x01, // SPELL_FAILED_ALREADY_AT_FULL_HEALTH
        AlreadyAtFullPower => 0x02,  // SPELL_FAILED_ALREADY_AT_FULL_POWER
        CantBeDisenchanted => 0x0C,  // SPELL_FAILED_CANT_BE_DISENCHANTED
        EquippedItem => 0x18,        // SPELL_FAILED_EQUIPPED_ITEM
        EquippedItemClass => 0x19,   // SPELL_FAILED_EQUIPPED_ITEM_CLASS
        EquippedItemClassMainhand => 0x1A, // SPELL_FAILED_EQUIPPED_ITEM_CLASS_MAINHAND
        EquippedItemClassOffhand => 0x1B, // SPELL_FAILED_EQUIPPED_ITEM_CLASS_OFFHAND
        Error => 0x1C,               // SPELL_FAILED_ERROR
        ItemGone => 0x26,            // SPELL_FAILED_ITEM_GONE
        ItemNotReady => 0x27,        // SPELL_FAILED_ITEM_NOT_READY
        LowLevel => 0x2B,            // SPELL_FAILED_LOWLEVEL
        MainhandEmpty => 0x2D,       // SPELL_FAILED_MAINHAND_EMPTY
        NeedExoticAmmo => 0x31,      // SPELL_FAILED_NEED_EXOTIC_AMMO
        NoAmmo => 0x43,              // SPELL_FAILED_NO_AMMO
        NoChargesRemain => 0x44,     // SPELL_FAILED_NO_CHARGES_REMAIN
        NotTradeable => 0x3F,        // SPELL_FAILED_NOT_TRADEABLE
        RequiresSpellFocus => 0x5E,  // SPELL_FAILED_REQUIRES_SPELL_FOCUS
        TargetNotPlayer => 0x71,     // SPELL_FAILED_TARGET_NOT_PLAYER
        // CheckCast / CheckPetCast errors
        NotStanding => 0x3D,           // SPELL_FAILED_NOT_STANDING
        AffectingCombat => 0x00,       // SPELL_FAILED_AFFECTING_COMBAT
        OnlyStealthed => 0x57,         // SPELL_FAILED_ONLY_STEALTHED
        OnlyOutdoors => 0x55,          // SPELL_FAILED_ONLY_OUTDOORS
        OnlyIndoors => 0x52,           // SPELL_FAILED_ONLY_INDOORS
        AuraBounced => 0x07,           // SPELL_FAILED_AURA_BOUNCED
        TargetNotDead => 0x6E,         // SPELL_FAILED_TARGET_NOT_DEAD
        HighLevel => 0x20,             // SPELL_FAILED_HIGHLEVEL
        TargetAffectingCombat => 0x66, // SPELL_FAILED_TARGET_AFFECTING_COMBAT
        CantCastOnTapped => 0x0E,      // SPELL_FAILED_CANT_CAST_ON_TAPPED
        TargetIsPlayer => 0x6D,        // SPELL_FAILED_TARGET_IS_PLAYER
        NotBehind => 0x32,             // SPELL_FAILED_NOT_BEHIND
        NotOnTaxi => 0x3A,             // SPELL_FAILED_NOT_ON_TAXI
        NoPet => 0x4C,                 // SPELL_FAILED_NO_PET
        TargetsDead => 0x65,           // SPELL_FAILED_TARGETS_DEAD
        DamageImmune => 0x22,          // SPELL_FAILED_IMMUNE (closest mapping)
        AlreadyBeingTamed => 0x03,     // SPELL_FAILED_ALREADY_BEING_TAMED
        RequiresArea => 0x54,          // SPELL_FAILED_REQUIRES_AREA
        OnlyBattlegrounds => 0x53,     // SPELL_FAILED_ONLY_BATTLEGROUNDS
    }
}

// ─── Spell::IgnoreItemRequirements ───────────────────────────────────────────
//
// C++ lines 7062–7077. Pure boolean; determines whether reagent/item checks
// should be skipped for a triggered spell.
//
// Rules (in order):
//   1. Non-triggered spells always check item requirements.
//   2. Triggered with a foreign-owned item target: still check (trade-slot case).
//   3. Triggered with no triggering spell proto: skip checks.
//   4. Triggered, triggering spell has Reagent[0] != 0: skip (master already consumed them).
//   5. Triggered, triggering spell has Reagent[0] == 0: still check.
pub fn ignore_item_requirements(
    is_triggered: bool,
    // true when the item target's owner GUID != the caster's GUID (trade slot case)
    item_target_has_foreign_owner: bool,
    // None = no triggering spell info; Some(n) = triggering spell's Reagent[0] value
    triggering_spell_reagent_0: Option<i32>,
) -> bool {
    if !is_triggered {
        return false;
    }
    if item_target_has_foreign_owner {
        return false;
    }
    match triggering_spell_reagent_0 {
        None => true,     // no triggering spell proto → ignore requirements
        Some(0) => false, // triggering spell has no reagents → don't ignore
        Some(_) => true,  // triggering spell has reagents → master already used them
    }
}

// ─── Spell::CheckPower ───────────────────────────────────────────────────────
//
// C++ lines 7022–7060. Validates that the caster has enough of the required
// power resource to cast the spell.
//
// Bypasses (all return Ok immediately):
//   - cast-item spells (item pays the cost)
//   - triggered spells
//   - no caster unit
//
// Special cases:
//   - NeedsComboPoints + explicit unit target: target must be the combo target;
//     on mismatch warrior returns BadTargets, everyone else NoComboPoints.
//   - powerType == POWER_HEALTH (-2): health must exceed cost or → CasterAuraState.
//   - unknown powerType (>= MAX_POWERS or negative but not -2) → DontReport.
//   - non-pet creature without the relevant power type: always Ok.
pub fn check_power(
    has_cast_item: bool,
    is_triggered: bool,
    has_caster_unit: bool,
    // spell requires combo points (NeedsComboPoints())
    needs_combo_points: bool,
    // effect 0 uses an explicitly selected unit target (IsExplicitlySelectedUnitTarget)
    effect_0_explicit_unit_target: bool,
    unit_target_guid: Option<ObjectGuid>,
    combo_target_guid: Option<ObjectGuid>,
    // CLASS_WARRIOR = 1 in vanilla
    caster_class: u8,
    // powerType from SpellEntry: 0=mana,1=rage,2=focus,3=energy,4=happiness,-2=health
    power_type: i32,
    power_cost: i32,
    caster_health: u32,
    caster_current_power: u32,
    // true for non-pet creatures that either lack base mana (for mana spells) or use
    // a non-mana power type that normal creatures cannot have
    bypass_creature_power_check: bool,
) -> SpellCastError {
    const POWER_HEALTH: i32 = -2;
    const MAX_POWERS: i32 = 5;
    const CLASS_WARRIOR: u8 = 1;

    if has_cast_item || is_triggered || !has_caster_unit {
        return SpellCastError::None;
    }

    if needs_combo_points && effect_0_explicit_unit_target {
        let on_combo_target = match (unit_target_guid, combo_target_guid) {
            (Some(t), Some(c)) => t == c,
            _ => false,
        };
        if !on_combo_target {
            return if caster_class == CLASS_WARRIOR {
                SpellCastError::InvalidTarget
            } else {
                SpellCastError::NoComboPoints
            };
        }
    }

    if power_type == POWER_HEALTH {
        return if (caster_health as i32) <= power_cost {
            SpellCastError::CasterAuraState
        } else {
            SpellCastError::None
        };
    }

    if power_type < 0 || power_type >= MAX_POWERS {
        return SpellCastError::DontReport;
    }

    if bypass_creature_power_check {
        return SpellCastError::None;
    }

    if power_cost > 0 && caster_current_power < power_cost as u32 {
        return SpellCastError::NotEnoughMana;
    }

    SpellCastError::None
}

// ─── Spell::CheckTamingSpell ──────────────────────────────────────────────────
//
// C++ lines 6829–6852. Validates all preconditions for taming a creature.
// Returns a PetTameFailure reason; None means success (PETTAME_NONE).
//
// Checks (in order):
//   1. Non-hunter + not GM → UnitsCantTame
//   2. Caster already has a pet, charm, or DB-stored pet → AnotherSummonActive
//   3. Target is not a creature → InvalidCreature
//   4. Target is already a pet or charmed → CreatureAlreadyOwned
//   5. Target level > caster level (non-GM) → TooHighLevel
//   6. Creature info says not tameable → NotTameable
//   None: all checks pass

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetTameFailure {
    UnitsCantTame,
    AnotherSummonActive,
    InvalidCreature,
    CreatureAlreadyOwned,
    TooHighLevel,
    NotTameable,
}

pub fn check_taming_spell(
    caster_class: u8,
    is_gm: bool,
    // true if caster already has a summoned/stored pet
    caster_has_active_or_stored_pet: bool,
    // true if caster currently has a charmed unit
    caster_has_charm: bool,
    unit_target_is_creature: bool,
    target_is_pet: bool,
    target_is_charmed: bool,
    target_level: u32,
    caster_level: u32,
    target_is_tameable: bool,
) -> Option<PetTameFailure> {
    const CLASS_HUNTER: u8 = 3;

    if caster_class != CLASS_HUNTER && !is_gm {
        return Some(PetTameFailure::UnitsCantTame);
    }

    if caster_has_active_or_stored_pet || caster_has_charm {
        return Some(PetTameFailure::AnotherSummonActive);
    }

    if !unit_target_is_creature {
        return Some(PetTameFailure::InvalidCreature);
    }

    if target_is_pet || target_is_charmed {
        return Some(PetTameFailure::CreatureAlreadyOwned);
    }

    if target_level > caster_level && !is_gm {
        return Some(PetTameFailure::TooHighLevel);
    }

    if !target_is_tameable {
        return Some(PetTameFailure::NotTameable);
    }

    None
}

// ─── Spell::CheckRange ───────────────────────────────────────────────────────
//
// C++ lines 6854–6946. Validates spell range against target.
//
// Special range indices:
//   SELF_ONLY (1): always OK
//   COMBAT (2): melee range — facing check + CanReachWithMeleeSpellAttack
//
// Generic range:
//   range_mod = strict ? (player 1.25 / non-player 0.0) : (player 6.25 / non-player 2.25)
//              + leeway_bonus (both units moving)
//   max_range from DBC + SPELLMOD_RANGE + range_mod
//   GO target: IsAtInteractDistance(max_range)
//   Unit target: dist > max → OutOfRange; dist < min → TooClose; face check if CUSTOM flag
//   Dest location: 3D dist check vs max/min
//
// Caller is responsible for pet implicit-target replacement (if any effect uses
// TARGET_UNIT_CASTER_PET, substitute pet for unit_target before calling).

pub const RANGE_INDEX_SELF: u32 = 1;
pub const RANGE_INDEX_COMBAT: u32 = 2;

#[derive(Debug)]
pub struct CheckRangeInput {
    pub range_index: u32,
    pub strict: bool,
    pub caster_is_player: bool,
    pub leeway_bonus: f32,
    // Combat-range fields (only read when range_index == RANGE_INDEX_COMBAT)
    pub target_is_self_or_next_melee: bool,
    pub caster_facing_target: bool,
    pub combat_range_mod: f32,
    pub can_reach_melee: bool,
    // Generic-range fields
    // max/min from DBC, SPELLMOD_RANGE already applied by caller
    pub max_range: f32,
    pub min_range: f32,
    // GO target: Some(true) = in range, Some(false) = out of range, None = no GO target
    pub go_in_range: Option<bool>,
    // Unit target distance (None = no distinct unit target)
    pub unit_target_distance: Option<f32>,
    pub spell_custom_face_target: bool,
    // Dest location 3D distance (None = no dest target or all-zero dest)
    pub dest_distance: Option<f32>,
}

pub fn check_range(input: &CheckRangeInput) -> SpellCastError {
    if input.range_index == RANGE_INDEX_SELF {
        return SpellCastError::None;
    }

    if input.range_index == RANGE_INDEX_COMBAT {
        if let Some(dist) = input.unit_target_distance {
            if input.target_is_self_or_next_melee {
                return SpellCastError::None;
            }
            if !input.caster_facing_target {
                return SpellCastError::NotInFront;
            }
            let _ = dist;
            return if input.can_reach_melee {
                SpellCastError::None
            } else {
                SpellCastError::OutOfRange
            };
        }
        // No distinct unit target: fall through to generic range check
    }

    let range_mod = (if input.strict {
        if input.caster_is_player {
            1.25_f32
        } else {
            0.0_f32
        }
    } else {
        if input.caster_is_player {
            6.25_f32
        } else {
            2.25_f32
        }
    }) + input.leeway_bonus;

    let max_range = input.max_range + range_mod;
    let min_range = input.min_range;

    if let Some(in_range) = input.go_in_range {
        if !in_range {
            return SpellCastError::OutOfRange;
        }
    }

    if let Some(dist) = input.unit_target_distance {
        if dist > max_range {
            return SpellCastError::OutOfRange;
        }
        if min_range > 0.0 && dist < min_range {
            return SpellCastError::TooClose;
        }
        if input.spell_custom_face_target && !input.caster_facing_target {
            return SpellCastError::NotInFront;
        }
    }

    if let Some(dist) = input.dest_distance {
        if dist > max_range {
            return SpellCastError::OutOfRange;
        }
        if min_range > 0.0 && dist < min_range {
            return SpellCastError::TooClose;
        }
    }

    SpellCastError::None
}

// ─── Spell::CheckCasterAuras ─────────────────────────────────────────────────
//
// C++ lines 6559–6676. Checks whether any unit-state aura (stun, fear, silence,
// pacify, confuse) prevents the cast, accounting for any immunity the spell
// being cast might grant.
//
// Bypasses:
//   - no caster unit
//   - spell ignores caster/target restrictions (IsIgnoringCasterAndTargetRestrictions)
//
// Immunity mask building (SPELL_ATTR_EX_IMMUNITY_PURGES_EFFECT):
//   Reads the spell's own effect aura names to build school/mechanic/dispel immunity
//   masks. These are passed in pre-built by the caller.
//
// Prevention logic (in priority order):
//   1. STUNNED + (instant OR has STUN interrupt flag) → Stunned
//   2. CONFUSED → Confused
//   3. FLEEING → Fleeing (build ≤1.6.1: also set if has SPELL_AURA_MOD_FEAR aura)
//   4. PreventionType==SILENCE + (SILENCED flag OR school lockout) → Silenced
//   5. PreventionType==PACIFY + PACIFIED flag → Pacified
//
// If prevented AND any immunity mask is non-zero: per-aura recheck to find the
// specific failing aura (allows immunity to one mechanic while failing another).
// NOTE: the per-aura recheck loop requires the active aura holder list. Pass
// `immunity_aura_recheck` = None to skip the recheck (conservative: always return
// prevented_reason, which is correct for the common no-immunity case).
//
// spell_prevention_type: 0=none, 1=silence, 2=pacify

pub struct CasterAuraCheckInput {
    pub has_caster_unit: bool,
    pub ignores_restrictions: bool,
    pub unit_flags: u32,
    pub spell_is_instant: bool,
    pub spell_has_stun_interrupt_flag: bool,
    pub spell_prevention_type: u8,
    pub caster_school_locked_out: bool,
    // Immunity masks built from the spell's own effects (0 = no immunity granted)
    pub school_immune: u8,
    pub mechanic_immune: u32,
    pub dispel_immune: u32,
}

// UNIT_FLAG constants (from MaNGOS UnitDefines.h)
const UNIT_FLAG_STUNNED: u32 = 0x00040000;
const UNIT_FLAG_CONFUSED: u32 = 0x00000004;
const UNIT_FLAG_FLEEING: u32 = 0x00000800;
const UNIT_FLAG_SILENCED: u32 = 0x00000200;
const UNIT_FLAG_PACIFIED: u32 = 0x00020000;

// MECHANIC bit positions (1-indexed in C++: 1 << (MECHANIC_X - 1))
const MECHANIC_STUN_BIT: u32 = 1 << 11; // MECHANIC_STUN = 12
const MECHANIC_FEAR_BIT: u32 = 1 << 7; // MECHANIC_FEAR = 8
                                       // CONFUSED_MECHANIC_MASK covers disoriented + confused mechanics
const CONFUSED_MECHANIC_MASK: u32 = (1 << 4) | (1 << 22); // MECHANIC_DISORIENTED=5, MECHANIC_CONFUSED=23

const SPELL_PREVENTION_TYPE_SILENCE: u8 = 1;
const SPELL_PREVENTION_TYPE_PACIFY: u8 = 2;

pub fn check_caster_auras(input: &CasterAuraCheckInput) -> SpellCastError {
    if !input.has_caster_unit || input.ignores_restrictions {
        return SpellCastError::None;
    }

    let flags = input.unit_flags;

    let prevented = if (flags & UNIT_FLAG_STUNNED != 0)
        && (mechanic_immune_bit(input.mechanic_immune, MECHANIC_STUN_BIT) == 0)
        && (input.spell_is_instant || input.spell_has_stun_interrupt_flag)
    {
        Some(SpellCastError::Stunned)
    } else if (flags & UNIT_FLAG_CONFUSED != 0)
        && (input.mechanic_immune & CONFUSED_MECHANIC_MASK == 0)
    {
        Some(SpellCastError::Confused)
    } else if (flags & UNIT_FLAG_FLEEING != 0)
        && (mechanic_immune_bit(input.mechanic_immune, MECHANIC_FEAR_BIT) == 0)
    {
        Some(SpellCastError::Fleeing)
    } else if input.spell_prevention_type == SPELL_PREVENTION_TYPE_SILENCE
        && ((flags & UNIT_FLAG_SILENCED != 0) || input.caster_school_locked_out)
    {
        Some(SpellCastError::Silenced)
    } else if input.spell_prevention_type == SPELL_PREVENTION_TYPE_PACIFY
        && (flags & UNIT_FLAG_PACIFIED != 0)
    {
        Some(SpellCastError::Pacified)
    } else {
        None
    };

    match prevented {
        None => SpellCastError::None,
        Some(reason) => {
            // If the spell grants immunity masks, a per-aura recheck could let the cast
            // through (e.g. casting a stun-immune spell while stunned by a different
            // mechanic). Callers that can supply the active aura list should perform that
            // recheck themselves before calling this function. Without it we return the
            // conservative result: cast is prevented.
            reason
        }
    }
}

fn mechanic_immune_bit(mechanic_immune: u32, bit: u32) -> u32 {
    mechanic_immune & bit
}

// ─── Spell::CheckItems ───────────────────────────────────────────────────────
//
// C++ lines 7079–7459. Validates item and equipment requirements before a spell fires.
// All deep inventory lookups are pre-resolved by the caller into this struct.
//
// Sections (in C++ order):
//   1. Creature weapon-disarm check for WEAPON_DAMAGE effects
//   2. Non-player → Ok
//   3. Cast-item checks (trade, inventory, charges, consumable full-resource)
//   4. Item-target checks OR equipped-item fit check
//   5. Spell-focus GO search
//   6. Reagent loop (calls ignore_item_requirements)
//   7. Totem loop
//   8. Effects switch: CREATE_ITEM, ENCHANT_ITEM, ENCHANT_ITEM_TEMPORARY,
//      DISENCHANT, WEAPON_DAMAGE ranged ammo

// Per-effect consumable check data (only populated for build > 1.10.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumableEffectKind {
    Heal,
    Energize,
    Other,
}

#[derive(Debug, Clone)]
pub struct ConsumableEffect {
    pub kind: ConsumableEffectKind,
    pub implicit_target_is_pet: bool,
    pub target_at_full: bool, // full health (Heal) or full power (Energize)
}

#[derive(Debug, Clone)]
pub struct CastItemData {
    pub is_in_trade: bool,
    pub is_in_inventory: bool,
    pub proto_exists: bool,
    /// GetSpellCharges for spell slots 0..4; 0 means that slot is not a charge spell
    pub charges: [i32; 5],
    /// proto->Spells[s].SpellCharges for spell slots 0..4
    pub proto_spell_charges: [i32; 5],
    /// True if the item class is ITEM_CLASS_CONSUMABLE
    pub is_consumable: bool,
    /// Per-effect consumable data for the full-resource check (build > 1.10.2)
    pub consumable_effects: [Option<ConsumableEffect>; 3],
    /// The item's entry ID, for the reagent-overlap adjustment
    pub entry: u32,
}

#[derive(Debug, Clone)]
pub struct ItemTargetData {
    pub exists: bool,
    pub fits_spell_requirements: bool,
    /// True if the item is in a trade slot (TARGET_FLAG_TRADE_ITEM in target mask)
    pub is_trade: bool,
    /// EQUIPMENT_SLOT_MAINHAND = 15
    pub slot: u8,
}

#[derive(Debug, Clone)]
pub struct EnchantItemData {
    pub target_exists: bool,
    pub item_level_ok: bool, // item.GetProto()->ItemLevel >= spell.baseLevel
    pub owner_is_caster: bool,
    pub own_item_only_attr: bool, // SPELL_ATTR_EX2_ENCHANT_OWN_ITEM_ONLY
    pub enchant_can_soulbound: bool, // pEnchant->slot & ENCHANTMENT_CAN_SOULBOUND
    pub enchant_entry_exists: bool,
}

#[derive(Debug, Clone)]
pub struct DisenchantData {
    pub target_exists: bool,
    pub has_generated_loot: bool,
    pub owner_is_caster: bool,
    pub proto_exists: bool,
    pub has_disenchant_id: bool,
    pub flagged_no_disenchant: bool, // ITEM_FLAG_NO_DISENCHANT
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangedWeaponSubclass {
    Thrown,
    Gun,
    Bow,
    Crossbow,
    Wand,
    Other,
}

#[derive(Debug, Clone)]
pub struct WeaponDamageRangedData {
    pub weapon_exists: bool,
    pub weapon_subclass: RangedWeaponSubclass,
    /// For thrown: player has at least 1 of the thrown item
    pub thrown_has_ammo: bool,
    /// For gun/bow/xbow: PLAYER_AMMO_ID != 0
    pub ammo_id_set: bool,
    pub ammo_proto_exists: bool,
    pub ammo_needs_exotic: bool, // SPELL_ATTR_NEED_EXOTIC_AMMO on the spell
    pub ammo_is_exotic: bool,    // ITEM_FLAG_EXOTIC on ammo proto
    pub ammo_is_projectile_class: bool,
    pub ammo_matches_weapon: bool, // arrow for bow/xbow, bullet for gun
    pub ammo_has_count: bool,      // player has at least 1 ammo
}

#[derive(Debug, Clone)]
pub struct CheckItemsInput {
    pub is_triggered: bool,
    pub attack_type: u8, // 0=BASE_ATTACK, 1=OFF_ATTACK, 2=RANGED_ATTACK

    // Effects (3 effect slots)
    pub effects: [u32; 3],

    // Reagents from spell entry
    pub reagent: [i32; 8],
    pub reagent_count: [u32; 8],

    // Totems from spell entry
    pub totem: [u32; 2],

    pub requires_spell_focus: bool,
    pub held_item_only_attr: bool,     // SPELL_ATTR_HELD_ITEM_ONLY
    pub requires_main_hand_attr: bool, // SPELL_ATTR_EX3_REQUIRES_MAIN_HAND_WEAPON (build > 1.5.1)
    pub requires_offhand_attr: bool,   // SPELL_ATTR_EX3_REQUIRES_OFFHAND_WEAPON (build > 1.9.4)

    // === Section 1: Creature weapon ===
    pub creature_has_weapon_but_disarmed: bool,
    pub has_weapon_damage_effect: bool,

    // === Section 2: Player check ===
    pub caster_is_player: bool,

    // === Section 3: Cast item ===
    pub cast_item: Option<CastItemData>,

    // === Section 4: Item target / equipment fit ===
    pub item_target_guid_set: bool,
    pub item_target: Option<ItemTargetData>,
    pub item_target_is_trade_mask: bool, // (target_mask & TARGET_FLAG_TRADE_ITEM) != 0
    // When no item target: Some(bool) = HasItemFitToSpellRequirements, None = non-player
    pub player_has_fit_equipment: Option<bool>,

    // === Section 5: Spell focus ===
    pub spell_focus_found: bool,

    // === Section 6: Reagents ===
    pub triggering_spell_reagent_0: Option<i32>,
    pub item_target_has_foreign_owner: bool,
    pub reagent_player_has: [bool; 8],
    // For the cast-item-as-reagent charge adjustment
    pub cast_item_charges_for_reagent: [Option<(i32, i32)>; 5], // (charges, proto_spell_charges)

    // === Section 7: Totems ===
    pub totem_player_has: [bool; 2],

    // === Section 8: Effects ===
    // CREATE_ITEM (effect index 0 only)
    pub create_item_target_is_player: bool,
    pub create_item_has_space: bool,
    // ENCHANT_ITEM
    pub enchant_item: Option<EnchantItemData>,
    // ENCHANT_ITEM_TEMPORARY
    pub enchant_item_temp: Option<EnchantItemData>,
    // DISENCHANT
    pub disenchant: Option<DisenchantData>,
    // WEAPON_DAMAGE ranged
    pub weapon_damage_ranged: Option<WeaponDamageRangedData>,
}

// Spell effect type constants relevant to CheckItems
const SPELL_EFFECT_CREATE_ITEM: u32 = 24;
const SPELL_EFFECT_ENCHANT_ITEM: u32 = 53;
const SPELL_EFFECT_ENCHANT_ITEM_TEMPORARY: u32 = 92;
const SPELL_EFFECT_ENCHANT_HELD_ITEM: u32 = 106;
const SPELL_EFFECT_DISENCHANT: u32 = 77;
const SPELL_EFFECT_WEAPON_DAMAGE: u32 = 36;
const SPELL_EFFECT_WEAPON_DAMAGE_NOSCHOOL: u32 = 69;

const EQUIPMENT_SLOT_MAINHAND: u8 = 15;

pub fn check_items(input: &CheckItemsInput) -> SpellCastError {
    // 1. Creature weapon-disarm check
    if !input.caster_is_player
        && input.creature_has_weapon_but_disarmed
        && input.has_weapon_damage_effect
    {
        return SpellCastError::EquippedItem;
    }

    // 2. Non-player casters pass all remaining item checks
    if !input.caster_is_player {
        return SpellCastError::None;
    }

    // 3. Cast-item checks
    if let Some(ref ci) = input.cast_item {
        if ci.is_in_trade {
            return SpellCastError::ItemGone;
        }
        if !ci.is_in_inventory || !ci.proto_exists {
            return SpellCastError::ItemNotReady;
        }
        for s in 0..5 {
            if ci.proto_spell_charges[s] != 0 && ci.charges[s] == 0 {
                return SpellCastError::NoChargesRemain;
            }
        }

        // Build > 1.10.2: consumable full-resource check
        if ci.is_consumable {
            let mut fail_reason = SpellCastError::None;
            let mut found_ok = false;
            for eff in &ci.consumable_effects {
                if let Some(ce) = eff {
                    if ce.implicit_target_is_pet {
                        continue;
                    }
                    match ce.kind {
                        ConsumableEffectKind::Heal => {
                            if ce.target_at_full {
                                fail_reason = SpellCastError::AlreadyAtFullHealth;
                                continue;
                            }
                            found_ok = true;
                            break;
                        }
                        ConsumableEffectKind::Energize => {
                            if ce.target_at_full {
                                fail_reason = SpellCastError::AlreadyAtFullPower;
                                continue;
                            }
                            found_ok = true;
                            break;
                        }
                        ConsumableEffectKind::Other => {}
                    }
                }
            }
            if !found_ok && fail_reason != SpellCastError::None {
                return fail_reason;
            }
        }
    }

    // 4a. Item target checks
    if input.item_target_guid_set {
        let is_trade = input.item_target_is_trade_mask;

        // Triggered non-trade targets on non-player casters → DontReport
        if !input.caster_is_player {
            return if input.is_triggered && !is_trade {
                SpellCastError::DontReport
            } else {
                SpellCastError::InvalidTarget
            };
        }

        let Some(ref target) = input.item_target else {
            return if input.is_triggered && !is_trade {
                SpellCastError::DontReport
            } else {
                SpellCastError::ItemGone
            };
        };

        if !target.exists {
            return if input.is_triggered && !is_trade {
                SpellCastError::DontReport
            } else {
                SpellCastError::ItemGone
            };
        }

        if !target.fits_spell_requirements {
            return if input.is_triggered && !is_trade {
                SpellCastError::DontReport
            } else {
                SpellCastError::EquippedItemClass
            };
        }

        if input.held_item_only_attr && target.slot != EQUIPMENT_SLOT_MAINHAND {
            return if input.is_triggered && !is_trade {
                SpellCastError::DontReport
            } else {
                SpellCastError::MainhandEmpty
            };
        }
    } else {
        // 4b. No item target: check equipped items fit spell requirements
        if let Some(has_fit) = input.player_has_fit_equipment {
            if !has_fit {
                if input.is_triggered {
                    return SpellCastError::DontReport;
                }
                if input.requires_main_hand_attr {
                    return SpellCastError::EquippedItemClassMainhand;
                }
                if input.requires_offhand_attr {
                    return SpellCastError::EquippedItemClassOffhand;
                }
                return SpellCastError::EquippedItemClass;
            }
        }
    }

    // 5. Spell focus
    if input.requires_spell_focus && !input.spell_focus_found {
        return SpellCastError::RequiresSpellFocus;
    }

    // 6. Reagents
    let ignore = ignore_item_requirements(
        input.is_triggered,
        input.item_target_has_foreign_owner,
        input.triggering_spell_reagent_0,
    );
    if !ignore {
        for i in 0..8 {
            if input.reagent[i] <= 0 {
                continue;
            }
            let mut needed = input.reagent_count[i];

            // Cast-item-as-reagent charge adjustment
            if let Some(ci) = &input.cast_item {
                if ci.entry == input.reagent[i] as u32 {
                    for s in 0..5 {
                        if let Some((charges, proto_charges)) =
                            input.cast_item_charges_for_reagent[s]
                        {
                            if proto_charges < 0 && charges.abs() < 2 && needed > 1 {
                                needed += 1;
                                break;
                            }
                        }
                    }
                }
            }

            if !input.reagent_player_has[i] {
                return SpellCastError::ItemNotReady;
            }
            let _ = needed;
        }

        // Totems
        let mut totems_remaining: u32 = 2;
        for i in 0..2 {
            if input.totem[i] != 0 {
                if input.totem_player_has[i] {
                    totems_remaining -= 1;
                }
            } else {
                totems_remaining -= 1;
            }
        }
        if totems_remaining != 0 {
            return SpellCastError::ItemGone;
        }
    }

    // 8. Effect-specific checks
    for i in 0..3 {
        match input.effects[i] {
            SPELL_EFFECT_CREATE_ITEM => {
                if !input.is_triggered && i == 0 {
                    if !input.create_item_target_is_player {
                        return SpellCastError::InvalidTarget;
                    }
                    if !input.create_item_has_space {
                        return SpellCastError::DontReport;
                    }
                }
            }
            SPELL_EFFECT_ENCHANT_ITEM => {
                if let Some(ref d) = input.enchant_item {
                    if !d.target_exists {
                        return SpellCastError::ItemGone;
                    }
                    if !d.item_level_ok {
                        return SpellCastError::LowLevel;
                    }
                    if !d.owner_is_caster {
                        if d.own_item_only_attr {
                            return SpellCastError::NotTradeable;
                        }
                        if !d.enchant_entry_exists {
                            return SpellCastError::Error;
                        }
                        if d.enchant_can_soulbound {
                            return SpellCastError::NotTradeable;
                        }
                    }
                }
            }
            SPELL_EFFECT_ENCHANT_ITEM_TEMPORARY => {
                if let Some(ref d) = input.enchant_item_temp {
                    if !d.target_exists {
                        return SpellCastError::ItemGone;
                    }
                    if !d.owner_is_caster {
                        if d.own_item_only_attr {
                            return SpellCastError::NotTradeable;
                        }
                        if !d.enchant_entry_exists {
                            return SpellCastError::Error;
                        }
                        if d.enchant_can_soulbound {
                            return SpellCastError::NotTradeable;
                        }
                    }
                }
            }
            SPELL_EFFECT_ENCHANT_HELD_ITEM => {
                // Item existence checked during effect application, not here
            }
            SPELL_EFFECT_DISENCHANT => {
                if let Some(ref d) = input.disenchant {
                    if !d.target_exists || d.has_generated_loot {
                        return SpellCastError::CantBeDisenchanted;
                    }
                    if !d.owner_is_caster {
                        return SpellCastError::CantBeDisenchanted;
                    }
                    if !d.proto_exists {
                        return SpellCastError::CantBeDisenchanted;
                    }
                    if !d.has_disenchant_id || d.flagged_no_disenchant {
                        return SpellCastError::CantBeDisenchanted;
                    }
                }
            }
            SPELL_EFFECT_WEAPON_DAMAGE | SPELL_EFFECT_WEAPON_DAMAGE_NOSCHOOL => {
                if !input.caster_is_player {
                    return SpellCastError::TargetNotPlayer;
                }
                if input.attack_type != 2 {
                    // Not ranged → no ammo check needed
                    break;
                }
                if let Some(ref r) = input.weapon_damage_ranged {
                    if !r.weapon_exists {
                        return SpellCastError::EquippedItem;
                    }
                    match r.weapon_subclass {
                        RangedWeaponSubclass::Thrown => {
                            if !r.thrown_has_ammo {
                                return SpellCastError::NoAmmo;
                            }
                        }
                        RangedWeaponSubclass::Gun
                        | RangedWeaponSubclass::Bow
                        | RangedWeaponSubclass::Crossbow => {
                            if !r.ammo_id_set || !r.ammo_proto_exists {
                                return SpellCastError::NoAmmo;
                            }
                            if r.ammo_needs_exotic && !r.ammo_is_exotic {
                                return SpellCastError::NeedExoticAmmo;
                            }
                            if !r.ammo_is_projectile_class {
                                return SpellCastError::NoAmmo;
                            }
                            if !r.ammo_matches_weapon {
                                return SpellCastError::NoAmmo;
                            }
                            if !r.ammo_has_count {
                                return SpellCastError::NoAmmo;
                            }
                        }
                        RangedWeaponSubclass::Wand | RangedWeaponSubclass::Other => {}
                    }
                }
            }
            _ => {}
        }
    }

    SpellCastError::None
}

// ─── Spell::CheckCast ────────────────────────────────────────────────────────
//
// C++ lines 5347–6479. The master pre-cast validation gate. Delegates to all
// sub-validation functions. All world-state lookups are pre-resolved by the
// caller into CheckCastInput.
//
// Section order (mirrors C++ exactly):
//   0. GM cheat bypass
//   1. Standing check (non-triggered)
//   2. Player cooldown (non-triggered, player only)
//   3. Death state check
//   4. GCD / BG-ended / combat+non-combat / explicit-mask / shapeshift /
//      only-stealthed (non-triggered, strict)
//   5. Outdoor/indoor (player, !GM)
//   6. CasterAuraState gate
//   7. Moving check (auto-repeat or standing-cancels)
//   8. NO_ACTIVE_PETS player gate
//   9. Unit target block: aura-applies checks, death-only, level range,
//      LoS, combat, rank, creature type, targeting mode, immune, facing
//  10. Pet-target effect block (TARGET_UNIT_CASTER_PET)
//  11. GO target immunity check
//  12. Dest LoS (empty targets)
//  13. Zone/location gate
//  14. Mounted / taxi check
//  15. CheckItems (non-passive)
//  16. Non-triggered: CheckRange → BG spell check → CheckPower → CheckCasterAuras
//  17. Possessed caster Enrage gate
//  18. TargetAuraState 20% health gate
//  19. SPECIAL_TAMING_FLAG check (→ check_taming_spell)
//  20. Per-effect switch (pre-resolved by caller into `per_effect_results`)
//
// Gaps (not yet wired to world state):
//  - Loatheb SPELL_AURA_OVERRIDE_CLASS_SCRIPTS (class-specific heal/dispel block)
//  - OPEN_LOCK BattleGround flag/banner checks
//  - CHARGE path-finding (PATHFIND_INCOMPLETE → NOPATH)
//  - LEAP/BLINK battleground state checks
//  - SUMMON_PLAYER dungeon/BG map entry checks

#[derive(Debug, Clone)]
pub struct CheckCastInput {
    pub strict: bool,
    pub is_triggered: bool,
    pub is_triggered_by_aura: bool,

    // === Section 0: GM bypass ===
    pub caster_has_no_check_cast_cheat: bool,

    // === Section 1: Standing ===
    pub caster_is_standing: bool,
    pub spell_allow_while_sitting: bool,

    // === Section 2: Cooldown (non-triggered, player) ===
    pub caster_is_player: bool,
    pub spell_is_passive: bool,
    pub spell_is_auto_repeat: bool,
    pub spell_on_cooldown: bool,
    pub spell_cooldown_on_event: bool,

    // === Section 3: Death check ===
    pub caster_unit_exists: bool,
    pub caster_is_alive: bool,
    pub spell_allow_while_dead: bool,

    // === Section 4: GCD / BG / combat / mask / shapeshift (strict, non-triggered) ===
    pub gcd_active: bool,         // HasGCD(spell), strict only
    pub battleground_ended: bool, // bg status == STATUS_WAIT_LEAVE
    pub caster_is_in_combat: bool,
    pub spell_is_non_combat: bool,
    pub explicit_target_mask_valid: bool, // ValidateExplicitTargetMask result
    pub is_client_started: bool,          // m_isClientStarted flag
    pub shapeshift_error: SpellCastError, // GetErrorAtShapeshiftedCast (SpellCastError::None = ok)
    pub spell_only_stealthed: bool,
    pub caster_has_stealth_aura: bool,

    // === Section 5: Outdoor/indoor (player, !GM, vmap enabled) ===
    pub vmap_indoor_check_enabled: bool,
    pub caster_is_gm: bool,
    pub spell_only_outdoors: bool,
    pub spell_only_indoors: bool,
    pub caster_is_outdoors: bool,

    // === Section 6: CasterAuraState ===
    pub spell_caster_aura_state: u32,
    pub caster_meets_aura_state: bool,

    // === Section 7: Moving ===
    pub caster_is_moving: bool,
    pub caster_is_falling_far: bool,
    pub spell_effect_0_is_stuck: bool,
    pub spell_has_standing_cancels_flag: bool,

    // === Section 8: NO_ACTIVE_PETS ===
    pub spell_no_active_pets: bool,
    pub player_has_pet_or_charm: bool,

    // === Sections 9–11: Target ===
    pub has_unit_target: bool,
    pub has_go_target: bool,
    pub has_dest_location: bool,
    pub targets_empty: bool,

    // Unit target: aura-applies block (positive spell, applies-aura, non-AoE)
    pub spell_applies_aura_non_aoe: bool, // IsSpellAppliesAura && !IsAoE
    pub target_aura_bounced: bool,
    pub target_is_shapeshifted: bool,
    pub target_is_mounted: bool,
    pub spell_interrupt_shapeshifting: bool, // HasAuraInterruptFlag(SHAPESHIFTING_CANCELS)
    pub spell_interrupt_mount: bool,         // HasAuraInterruptFlag(MOUNT_CANCELS)

    // Unit target: general state
    pub spell_death_only: bool,
    pub target_is_alive: bool,
    pub spell_min_target_level: u32,
    pub spell_max_target_level: u32,
    pub target_level: u32,

    // Unit target: non-caster, non-triggered checks
    pub non_caster_target: bool, // target != caster && !caster_source_targets_only
    pub target_is_taxiing: bool,
    pub spell_has_summon_player_effect: bool,
    pub target_in_los: bool,
    pub spell_ignore_los: bool,
    pub target_in_combat: bool,
    pub spell_only_peaceful_targets: bool,
    pub spell_allow_low_level_buff: bool,
    pub caster_is_player_not_cast_item: bool,
    pub target_rank_correct: bool, // SelectAuraRankForLevel matches (or not a player)
    pub spell_cannot_cast_on_tapped: bool,
    pub target_tapped_by_others: bool,
    pub spell_only_on_player: bool,
    pub target_is_player: bool,
    pub spell_is_aoe: bool, // IsAreaOfEffectSpell() — exempts ONLY_ON_PLAYER gate

    // Unit target: creature type and targeting mode
    pub target_creature_type_ok: bool, // CheckTargetCreatureType
    pub target_is_valid_help: bool,    // IsValidHelpfulTarget (explicit positive effects)
    pub target_is_valid_attack: bool,  // IsValidAttackTarget (explicit negative effects)
    pub has_explicit_positive_effect: bool, // any IsExplicitPositiveTarget in effects
    pub has_explicit_negative_effect: bool,
    pub caster_is_creature_with_owner: bool, // for pet/charmed extra check
    pub spell_is_positive_simple: bool,      // IsPositiveSpell() no target context
    pub spell_has_dispel: bool,

    // Unit target: immune, facing, positive context
    pub target_immune_to_spell: bool,
    pub spell_is_positive_for_target: bool, // IsPositiveSpell(caster, target)
    pub spell_is_from_behind: bool,
    pub caster_is_behind_target: bool,
    pub spell_target_faces_caster: bool, // Attributes == 0x150010 special check
    pub target_faces_caster: bool,

    // Unit target: pet-target effect block
    pub has_pet_target_effect: bool, // any EffectImplicitTargetA == TARGET_UNIT_CASTER_PET
    pub pet_exists: bool,
    pub pet_is_alive: bool,
    pub pet_in_los: bool,
    pub is_triggered_by_aura_spell: bool, // m_triggeredByAuraSpell != nullptr
    pub target_alive_state_ok: bool,      // CanTargetAliveState for explicit non-pet effects

    // GO target
    pub go_immune_under_immunity: bool,

    // Dest LoS
    pub dest_in_los: bool,

    // === Section 13: Zone ===
    pub location_error: SpellCastError, // GetSpellAllowedInLocationError

    // === Section 14: Mounted ===
    pub caster_is_mounted: bool,
    pub caster_is_taxiing: bool,
    pub spell_allow_while_mounted: bool,
    pub spell_is_not_passive: bool,

    // === Sections 15–16: Sub-function pre-computed results ===
    // Caller must invoke each sub-function and pass the result.
    pub items_result: SpellCastError, // check_items result (non-passive)
    pub range_result: SpellCastError, // check_range result (non-triggered)
    pub bg_spell_check: SpellCastError, // bg->CheckSpellCast (None if not in BG)
    pub power_result: SpellCastError, // check_power result (non-triggered)
    pub aura_result: SpellCastError,  // check_caster_auras result (non-triggered)

    // === Section 17: Possessed caster ===
    pub caster_is_possessed: bool,
    pub spell_category_is_enrage: bool, // category == 21

    // === Section 18: TargetAuraState 20% ===
    pub spell_target_aura_state_20pct: bool,
    pub target_has_20pct_aura_state: bool,
    pub unit_target_exists_for_20pct: bool,

    // === Section 19: Taming ===
    pub spell_has_taming_flag: bool,
    pub taming_failure: Option<PetTameFailure>, // from check_taming_spell; None = ok
    pub already_being_tamed: bool,

    // === Section 20: Per-effect pre-resolved results ===
    // Caller computes per-effect requirements (RESURRECT, TAMECREATURE, OPEN_LOCK, etc.)
    // and puts SpellCastError::None if the effect passes, or an error if it fails.
    pub per_effect_results: [SpellCastError; 3],
}

pub fn check_cast(input: &CheckCastInput) -> SpellCastError {
    // 0. GM cheat bypass
    if input.caster_has_no_check_cast_cheat {
        return SpellCastError::None;
    }

    // 1. Standing (non-triggered, caster unit exists)
    if !input.is_triggered
        && input.caster_unit_exists
        && !input.caster_is_standing
        && !input.spell_allow_while_sitting
    {
        return SpellCastError::NotStanding;
    }

    // 2. Player cooldown (non-triggered, player, not passive/auto-repeat)
    if !input.is_triggered
        && input.caster_is_player
        && !input.spell_is_passive
        && !input.spell_is_auto_repeat
        && input.spell_on_cooldown
    {
        return if input.is_triggered_by_aura || input.spell_cooldown_on_event {
            SpellCastError::DontReport
        } else {
            SpellCastError::SpellOnCooldown
        };
    }

    // 3. Death check
    if input.caster_unit_exists
        && !input.caster_is_alive
        && !input.spell_is_passive
        && !input.spell_allow_while_dead
        && !(input.is_triggered && !input.is_triggered_by_aura)
    {
        return if input.is_triggered_by_aura {
            SpellCastError::DontReport
        } else {
            SpellCastError::CasterDead
        };
    }

    // 4. GCD / BG-ended / combat+non-combat / mask / shapeshift / only-stealthed
    //    (non-triggered)
    if !input.is_triggered {
        // GCD (strict, player)
        if input.strict && input.gcd_active {
            return if input.spell_cooldown_on_event {
                SpellCastError::DontReport
            } else {
                SpellCastError::NotReady
            };
        }

        // Battleground ended
        if input.caster_is_player && input.battleground_ended {
            return SpellCastError::DontReport;
        }

        // Strict-mode additional checks
        if input.strict && input.caster_unit_exists {
            if input.caster_is_in_combat && input.spell_is_non_combat {
                return SpellCastError::AffectingCombat;
            }
            if input.is_client_started && !input.explicit_target_mask_valid {
                return SpellCastError::InvalidTarget;
            }
            if input.shapeshift_error != SpellCastError::None {
                return input.shapeshift_error;
            }
            if input.spell_only_stealthed && !input.caster_has_stealth_aura {
                return SpellCastError::OnlyStealthed;
            }
        }
    }

    // 5. Outdoor/indoor (player, !GM, vmap check enabled)
    if input.caster_is_player && !input.caster_is_gm && input.vmap_indoor_check_enabled {
        if input.spell_only_outdoors && !input.caster_is_outdoors {
            return SpellCastError::OnlyOutdoors;
        }
        if input.spell_only_indoors && input.caster_is_outdoors {
            return SpellCastError::OnlyIndoors;
        }
    }

    // 6. CasterAuraState gate
    if input.spell_caster_aura_state != 0
        && input.caster_unit_exists
        && !input.caster_meets_aura_state
    {
        return SpellCastError::CasterAuraState;
    }

    // 7. Moving (player, non-triggered)
    if input.caster_is_player && input.caster_is_moving {
        if !input.caster_is_falling_far || !input.spell_effect_0_is_stuck {
            if input.spell_is_auto_repeat || input.spell_has_standing_cancels_flag {
                return SpellCastError::NotWhileMoving;
            }
        }
    }

    // 8. NO_ACTIVE_PETS (player)
    if input.caster_is_player && input.spell_no_active_pets && input.player_has_pet_or_charm {
        return SpellCastError::DontReport;
    }

    // 9. Unit target block
    if input.has_unit_target {
        // Aura-applies, non-AoE spell checks
        if input.spell_applies_aura_non_aoe {
            if input.spell_is_positive_for_target && input.target_aura_bounced {
                return SpellCastError::AuraBounced;
            }
            if input.spell_interrupt_shapeshifting && input.target_is_shapeshifted {
                return SpellCastError::InvalidTarget;
            }
            if input.spell_interrupt_mount && input.target_is_mounted {
                return SpellCastError::InvalidTarget;
            }
        }

        // Death-only spell on alive target (non-triggered)
        if !input.is_triggered && input.spell_death_only && input.target_is_alive {
            return SpellCastError::TargetNotDead;
        }

        // Level range
        if input.spell_min_target_level > 0
            && input.target_level < input.spell_min_target_level as u32
        {
            return SpellCastError::LowLevel;
        }
        if input.spell_max_target_level > 0
            && input.target_level > input.spell_max_target_level as u32
        {
            return SpellCastError::HighLevel;
        }

        // Non-caster target, non-triggered checks
        if input.non_caster_target {
            if input.target_is_taxiing && !input.spell_has_summon_player_effect {
                return SpellCastError::InvalidTarget;
            }
            if !input.is_triggered {
                if !input.spell_ignore_los && !input.target_in_los {
                    return SpellCastError::TargetNotInLineOfSight;
                }
                if input.strict && input.spell_only_peaceful_targets && input.target_in_combat {
                    return SpellCastError::TargetAffectingCombat;
                }
                if input.caster_is_player_not_cast_item
                    && !input.spell_allow_low_level_buff
                    && !input.target_rank_correct
                {
                    return SpellCastError::LowLevel;
                }
                if input.spell_cannot_cast_on_tapped && input.target_tapped_by_others {
                    return SpellCastError::CantCastOnTapped;
                }
                if input.strict
                    && input.spell_only_on_player
                    && !input.target_is_player
                    && !input.spell_is_aoe
                {
                    return SpellCastError::InvalidTarget;
                }
            }
        }

        // Pet-target effect check (TARGET_UNIT_CASTER_PET)
        if input.has_pet_target_effect {
            if !input.pet_exists {
                return if input.is_triggered_by_aura_spell {
                    SpellCastError::DontReport
                } else {
                    SpellCastError::NoPet
                };
            }
            if !input.pet_is_alive {
                return if input.is_triggered_by_aura_spell {
                    SpellCastError::DontReport
                } else {
                    SpellCastError::TargetsDead
                };
            }
            if !input.spell_ignore_los && !input.pet_in_los {
                return SpellCastError::TargetNotInLineOfSight;
            }
        } else if !input.target_alive_state_ok {
            return SpellCastError::InvalidTarget;
        }

        // Creature type check (non-caster target)
        if input.non_caster_target {
            if !input.target_creature_type_ok {
                return if input.target_is_player {
                    SpellCastError::TargetIsPlayer
                } else {
                    SpellCastError::InvalidTarget
                };
            }

            // Explicit positive/negative targeting mode
            if input.has_explicit_positive_effect && !input.target_is_valid_help {
                return SpellCastError::InvalidTarget;
            }
            if input.has_explicit_negative_effect && !input.target_is_valid_attack {
                return SpellCastError::InvalidTarget;
            }
            // Pet/charmed caster implicit positive/negative check
            if !input.has_explicit_positive_effect
                && !input.has_explicit_negative_effect
                && input.caster_is_creature_with_owner
                && !input.spell_has_dispel
            {
                if input.spell_is_positive_simple {
                    if !input.target_is_valid_help {
                        return SpellCastError::InvalidTarget;
                    }
                } else if !input.target_is_valid_attack {
                    return SpellCastError::InvalidTarget;
                }
            }

            // Immune to spell (positive)
            if input.spell_is_positive_for_target && input.target_immune_to_spell {
                return SpellCastError::TargetAuraState;
            }

            // Positional: must be behind / facing
            if input.spell_is_from_behind && !input.caster_is_behind_target {
                return SpellCastError::NotBehind;
            }
            if input.spell_target_faces_caster && !input.target_faces_caster {
                return SpellCastError::NotInFront;
            }
        }
    }

    // GO target immune check
    if input.has_go_target && input.go_immune_under_immunity {
        return SpellCastError::DamageImmune;
    }

    // Dest LoS (empty targets + player + dest location)
    if input.targets_empty
        && input.caster_is_player
        && input.has_dest_location
        && !input.dest_in_los
    {
        return SpellCastError::TargetNotInLineOfSight;
    }

    // 13. Zone check
    if input.location_error != SpellCastError::None {
        return input.location_error;
    }

    // 14. Mounted / taxi (non-triggered, player)
    if input.caster_is_player
        && !input.is_triggered
        && input.caster_is_mounted
        && !input.spell_is_not_passive
        && !input.spell_allow_while_mounted
    {
        if input.caster_is_taxiing {
            return SpellCastError::NotOnTaxi;
        }
        // Non-taxiing: the C++ unmounts the player (side effect, not modelled here)
    }

    // 15. CheckItems (non-passive) — pre-computed by caller
    if input.spell_is_not_passive && input.items_result != SpellCastError::None {
        return input.items_result;
    }

    // 16. Non-triggered: CheckRange → BG spell check → CheckPower → CheckCasterAuras
    if !input.is_triggered {
        if !input.is_triggered_by_aura {
            if input.range_result != SpellCastError::None {
                return input.range_result;
            }
            if input.bg_spell_check != SpellCastError::None {
                return input.bg_spell_check;
            }
        }

        if input.caster_unit_exists {
            if input.power_result != SpellCastError::None {
                return input.power_result;
            }
            if input.aura_result != SpellCastError::None {
                return input.aura_result;
            }
            // Possessed caster: block Enrage (category 21)
            if input.caster_is_possessed && input.spell_category_is_enrage {
                return SpellCastError::SpellOnCooldown;
            }
        }
    }

    // 18. TargetAuraState 20% health gate
    if input.spell_target_aura_state_20pct {
        if !input.unit_target_exists_for_20pct {
            return SpellCastError::InvalidTarget;
        }
        if !input.target_has_20pct_aura_state {
            return SpellCastError::InvalidTarget;
        }
    }

    // 19. SPECIAL_TAMING_FLAG check
    if input.spell_has_taming_flag {
        if let Some(_failure) = input.taming_failure {
            return SpellCastError::DontReport;
        }
        if input.already_being_tamed {
            return SpellCastError::AlreadyBeingTamed;
        }
    }

    // 20. Per-effect pre-resolved results
    for &r in &input.per_effect_results {
        if r != SpellCastError::None {
            return r;
        }
    }

    SpellCastError::None
}

// ─── PlayerAI::CanCastSpell ──────────────────────────────────────────────────
//
// C++ PlayerAI.cpp 39–81. Whether an AI-controlled player may cast `pSpell` at
// `pTarget`. Pure over pre-resolved caster/target/spell state.
//
// Order (mirrors C++):
//   1. No target → false.
//   2. Non-triggered gate: react-state, silence/pacify prevention, power.
//   3. Range: if a range entry exists, out-of-range/too-close (skipped for self)
//      → false, else true. No range entry → false.

#[derive(Debug, Clone)]
pub struct PlayerAiCanCastInput {
    pub target_exists: bool,
    pub is_triggered: bool,
    /// checkControlled arg — selects the stricter react-state mask.
    pub check_controlled: bool,
    /// HasUnitState(UNIT_STATE_CAN_NOT_REACT).
    pub caster_cannot_react: bool,
    /// HasUnitState(UNIT_STATE_CAN_NOT_REACT_OR_LOST_CONTROL).
    pub caster_cannot_react_or_lost_control: bool,
    pub spell_prevention_type: u8,
    pub caster_silenced: bool,
    pub caster_pacified: bool,
    pub caster_power: u32,
    pub spell_mana_cost: u32,
    pub target_is_self: bool,
    /// sSpellRangeStore.LookupEntry(rangeIndex) resolved to a range entry.
    pub has_range_entry: bool,
    /// GetCombatDistance(pTarget).
    pub distance: f32,
    pub max_range: f32,
    pub min_range: f32,
}

pub fn player_ai_can_cast_spell(input: &PlayerAiCanCastInput) -> bool {
    // 1. No target.
    if !input.target_exists {
        return false;
    }

    // 2. Non-triggered checks.
    if !input.is_triggered {
        let cannot_react = if input.check_controlled {
            input.caster_cannot_react_or_lost_control
        } else {
            input.caster_cannot_react
        };
        if cannot_react {
            return false;
        }
        if input.spell_prevention_type == SPELL_PREVENTION_TYPE_SILENCE && input.caster_silenced {
            return false;
        }
        if input.spell_prevention_type == SPELL_PREVENTION_TYPE_PACIFY && input.caster_pacified {
            return false;
        }
        if input.caster_power < input.spell_mana_cost {
            return false;
        }
    }

    // 3. Range gate — only when the spell has a range entry.
    if input.has_range_entry {
        if !input.target_is_self {
            if input.distance > input.max_range {
                return false;
            }
            if input.min_range > 0.0 && input.distance < input.min_range {
                return false;
            }
        }
        return true;
    }

    false
}

// ─── Spell::CheckPetCast ─────────────────────────────────────────────────────
//
// C++ lines 6481–6557. Pet-cast wrapper that validates pet-specific preconditions
// then delegates to check_cast(strict=true). All world-state lookups are
// pre-resolved by the caller.
//
// The `check_cast_input` must be constructed with strict=true by the caller.

#[derive(Debug, Clone)]
pub struct CheckPetCastInput {
    pub caster_is_alive: bool,
    pub non_melee_spell_in_progress: bool,
    pub caster_is_in_combat: bool,
    pub spell_is_non_combat: bool,

    // Pet / charmed caster block (only evaluated when caster is pet/charmed creature)
    pub caster_is_pet_or_charmed: bool,
    pub owner_is_alive: bool, // GetCharmerOrOwner()->IsAlive(); true if no owner
    pub target_resolved: bool, // after fallback: m_targets.getUnitTarget() exists
    pub effects_need_unit_target: bool, // any EffectImplicitTargetA in target-requiring set
    pub target_is_explicitly_selected: bool, // IsExplicitlySelectedUnitTarget(effect[0])
    pub target_is_targetable: bool, // IsTargetableBy(caster, false, true, positive)
    pub caster_is_hostile_to_target: bool,
    pub spell_is_positive_for_target: bool,
    pub check_valid_attack_target: bool, // true unless any effect = TARGET_UNIT/_CONE/_SRC
    pub target_is_valid_attack: bool,    // IsValidAttackTarget
    pub spell_is_ready: bool,            // IsSpellReady

    // Pre-computed full CheckCast(true) result — caller builds CheckCastInput and runs check_cast
    pub check_cast_result: SpellCastError,
}

pub fn check_pet_cast(input: &CheckPetCastInput) -> SpellCastError {
    // 1. Caster alive
    if !input.caster_is_alive {
        return SpellCastError::CasterDead;
    }

    // 2. Non-melee spell already in progress
    if input.non_melee_spell_in_progress {
        return SpellCastError::AlreadyCasting;
    }

    // 3. Combat + non-combat spell
    if input.caster_is_in_combat && input.spell_is_non_combat {
        return SpellCastError::AffectingCombat;
    }

    // 4. Pet / charmed caster specific checks
    if input.caster_is_pet_or_charmed {
        // 4a. Owner alive
        if !input.owner_is_alive {
            return SpellCastError::CasterDead;
        }

        // 4b. Requires unit target but none resolved
        if input.effects_need_unit_target && !input.target_resolved {
            return SpellCastError::InvalidTarget; // SPELL_FAILED_BAD_IMPLICIT_TARGETS
        }

        // 4c. Explicit target validity
        if input.target_resolved && input.target_is_explicitly_selected {
            if !input.target_is_targetable {
                return SpellCastError::InvalidTarget;
            }
            if input.spell_is_positive_for_target {
                if input.caster_is_hostile_to_target {
                    return SpellCastError::InvalidTarget;
                }
            } else {
                if input.check_valid_attack_target && !input.target_is_valid_attack {
                    return SpellCastError::InvalidTarget;
                }
            }
        }

        // 4d. Cooldown
        if !input.spell_is_ready {
            return SpellCastError::SpellOnCooldown;
        }
    }

    // 5. Delegate to check_cast(strict=true)
    input.check_cast_result
}

// Additional SpellCastError variants used by check_cast / check_pet_cast
// (wire codes added to spell_cast_error_to_u8)
// ─── Spell::ValidateExplicitTargetMask ───────────────────────────────────────
//
// C++ lines 6678–6757. Anticheat check: validates that the client's target mask
// is consistent with what the spell's spell info permits. Player casters only —
// non-player casters always pass.
//
// Two kinds of failures:
//   1. Disallowed flag: client sent a flag not in `allowed_target_mask`.
//   2. Missing expected flag: spell's `expected_target_mask` requires a flag the
//      client did not include.
//
// Both are anticheat events (logged). Returns false on any violation.
//
// SpellEntry::Targets unit/item/etc flag values match the SpellCastTargets
// packet flag values in vanilla (direct bitmask comparison in C++).
use crate::game::player::spells::state::{
    TARGET_FLAG_CORPSE_ALLY, TARGET_FLAG_CORPSE_ENEMY, TARGET_FLAG_DEST_LOCATION, TARGET_FLAG_ITEM,
    TARGET_FLAG_LOCKED, TARGET_FLAG_OBJECT, TARGET_FLAG_SOURCE_LOCATION, TARGET_FLAG_TRADE_ITEM,
    TARGET_FLAG_UNIT, TARGET_FLAG_UNIT_MINIPET,
};

// SpellEntry::Targets extended unit/corpse flags (same bit range as packet flags)
const SPELL_TARGET_UNIT_RAID: u32 = 0x00000004;
const SPELL_TARGET_UNIT_PARTY: u32 = 0x00000008;
const SPELL_TARGET_UNIT_ENEMY: u32 = 0x00000080;
const SPELL_TARGET_UNIT_ALLY: u32 = 0x00000100;
const SPELL_TARGET_UNIT_DEAD: u32 = 0x00000400;

pub fn validate_explicit_target_mask(
    caster_is_player: bool,
    spell_id: u32,
    client_target_mask: u32,
    allowed_target_mask: u32,
    expected_target_mask: u32,
) -> bool {
    if !caster_is_player {
        return true;
    }

    const VERIFIABLE: &[u32] = &[
        TARGET_FLAG_UNIT,
        TARGET_FLAG_ITEM,
        TARGET_FLAG_TRADE_ITEM,
        TARGET_FLAG_SOURCE_LOCATION,
        TARGET_FLAG_DEST_LOCATION,
        TARGET_FLAG_OBJECT,
        TARGET_FLAG_CORPSE_ENEMY,
        TARGET_FLAG_CORPSE_ALLY,
    ];

    if allowed_target_mask == 0 && client_target_mask != 0 {
        tracing::warn!(
            "[ANTICHEAT] spell={spell_id} unexpected target mask {client_target_mask:#010x} when none expected"
        );
        return false;
    }

    for &flag in VERIFIABLE {
        if (client_target_mask & flag) != 0 && (allowed_target_mask & flag) == 0 {
            tracing::warn!(
                "[ANTICHEAT] spell={spell_id} disallowed target flag {flag:#010x} in client mask {client_target_mask:#010x}"
            );
            return false;
        }
    }

    // Check that required flags from expectedTargetMask are present in client mask
    let all_unit_flags = TARGET_FLAG_UNIT
        | SPELL_TARGET_UNIT_RAID
        | SPELL_TARGET_UNIT_PARTY
        | SPELL_TARGET_UNIT_ENEMY
        | SPELL_TARGET_UNIT_ALLY
        | TARGET_FLAG_UNIT_MINIPET
        | SPELL_TARGET_UNIT_DEAD;

    if (client_target_mask & TARGET_FLAG_UNIT) == 0 && (expected_target_mask & all_unit_flags) != 0
    {
        tracing::warn!("[ANTICHEAT] spell={spell_id} expected UNIT flag missing from client mask {client_target_mask:#010x}");
        return false;
    }

    if (client_target_mask & (TARGET_FLAG_ITEM | TARGET_FLAG_TRADE_ITEM)) == 0
        && (expected_target_mask & TARGET_FLAG_ITEM) != 0
    {
        tracing::warn!("[ANTICHEAT] spell={spell_id} expected ITEM flag missing from client mask {client_target_mask:#010x}");
        return false;
    }

    if (client_target_mask & TARGET_FLAG_SOURCE_LOCATION) == 0
        && (expected_target_mask & TARGET_FLAG_SOURCE_LOCATION) != 0
    {
        tracing::warn!("[ANTICHEAT] spell={spell_id} expected SOURCE_LOCATION flag missing");
        return false;
    }

    if (client_target_mask & TARGET_FLAG_DEST_LOCATION) == 0
        && (expected_target_mask & TARGET_FLAG_DEST_LOCATION) != 0
    {
        tracing::warn!("[ANTICHEAT] spell={spell_id} expected DEST_LOCATION flag missing");
        return false;
    }

    if (client_target_mask & TARGET_FLAG_OBJECT) == 0
        && (expected_target_mask & TARGET_FLAG_OBJECT) != 0
    {
        tracing::warn!("[ANTICHEAT] spell={spell_id} expected GAMEOBJECT flag missing");
        return false;
    }

    if (client_target_mask & (TARGET_FLAG_ITEM | TARGET_FLAG_OBJECT)) == 0
        && (expected_target_mask & TARGET_FLAG_LOCKED) != 0
    {
        tracing::warn!("[ANTICHEAT] spell={spell_id} expected LOCKED flag missing");
        return false;
    }

    let corpse_flags = TARGET_FLAG_CORPSE_ENEMY | TARGET_FLAG_CORPSE_ALLY;
    if (client_target_mask & (TARGET_FLAG_UNIT | corpse_flags)) == 0
        && (expected_target_mask & corpse_flags) != 0
    {
        tracing::warn!("[ANTICHEAT] spell={spell_id} expected CORPSE flag missing");
        return false;
    }

    true
}

// ─── Spell::CanOpenLock ──────────────────────────────────────────────────────
//
// C++ lines 7864–7918. Validates whether the spell can open a given lock, and
// computes the effective skill values for the "orange" skill-chance check in
// CheckCast's OPEN_LOCK case.
//
// Returns Ok(OpenLockResult) on success, Err(SpellCastError) on failure.
//
// Checks (per 8 lock slots in order):
//   LOCK_KEY_ITEM: if cast-item entry matches slot's item index → succeed immediately.
//   LOCK_KEY_SKILL: if spell's EffectMiscValue[eff] matches slot index:
//       skillValue = (cast_item or non-player) ? 0 : player.GetSkillValue(skillId)
//       skillValue += spell_skill_bonus
//       if skillValue < reqSkillValue → LowCastLevel
//       else → succeed
//   No matching slot → BadTargets (locked, no valid key/skill)
//
// Caller must pre-resolve the LockEntry from DBC (pass None if lockId not found).
// Caller pre-computes skill_id and is_blasting_type from SkillByLockType(LockType(index)).
// LOCKTYPE_BLASTING (seaforium charges) is treated as having a skill even when skill_id==SKILL_NONE.

pub const LOCK_KEY_NONE: u8 = 0;
pub const LOCK_KEY_ITEM: u8 = 1;
pub const LOCK_KEY_SKILL: u8 = 2;

#[derive(Debug, Clone, Copy)]
pub struct LockEntrySlot {
    pub key_type: u8,
    pub index: u32, // item entry (LOCK_KEY_ITEM) or LockType index (LOCK_KEY_SKILL)
    pub required_skill: i32,
    pub skill_id: u32,          // result of SkillByLockType(index); 0 = SKILL_NONE
    pub is_blasting_type: bool, // LockType(index) == LOCKTYPE_BLASTING
}

#[derive(Debug, Clone, Copy)]
pub struct LockEntry {
    pub slots: [LockEntrySlot; 8],
}

#[derive(Debug, Clone, Copy)]
pub struct OpenLockResult {
    pub skill_id: u32,
    pub req_skill_value: i32,
    pub skill_value: i32,
}

pub fn can_open_lock(
    lock_id: u32,
    lock_entry: Option<&LockEntry>,
    // m_spellInfo->EffectMiscValue[effIndex] — must match slot index for LOCK_KEY_SKILL
    effect_misc_value: u32,
    // m_currentBasePoints[effIndex] — skill bonus from the cast spell
    spell_skill_bonus: i32,
    cast_item_entry: Option<u32>,
    // Player's GetSkillValue(skill_id). None for non-player casters or when using cast-item
    // (in those cases the C++ uses 0 as the base skill value).
    get_player_skill: Option<&dyn Fn(u32) -> i32>,
) -> Result<OpenLockResult, SpellCastError> {
    if lock_id == 0 {
        return Ok(OpenLockResult {
            skill_id: 0,
            req_skill_value: 0,
            skill_value: 0,
        });
    }

    let entry = match lock_entry {
        Some(e) => e,
        None => return Err(SpellCastError::InvalidTarget),
    };

    for slot in &entry.slots {
        match slot.key_type {
            LOCK_KEY_ITEM => {
                if slot.index != 0 {
                    if let Some(item_entry) = cast_item_entry {
                        if item_entry == slot.index {
                            return Ok(OpenLockResult {
                                skill_id: 0,
                                req_skill_value: 0,
                                skill_value: 0,
                            });
                        }
                    }
                }
            }
            LOCK_KEY_SKILL => {
                if effect_misc_value != slot.index {
                    continue;
                }

                if slot.skill_id != 0 || slot.is_blasting_type {
                    let base_skill = if cast_item_entry.is_some() || get_player_skill.is_none() {
                        0_i32
                    } else {
                        get_player_skill.unwrap()(slot.skill_id)
                    };
                    let skill_value = base_skill + spell_skill_bonus;
                    let req = slot.required_skill;

                    if skill_value < req {
                        return Err(SpellCastError::LowCastLevel);
                    }

                    return Ok(OpenLockResult {
                        skill_id: slot.skill_id,
                        req_skill_value: req,
                        skill_value,
                    });
                }

                return Ok(OpenLockResult {
                    skill_id: slot.skill_id,
                    req_skill_value: slot.required_skill,
                    skill_value: spell_skill_bonus,
                });
            }
            _ => {}
        }
    }

    Err(SpellCastError::InvalidTarget)
}

#[cfg(test)]
mod player_ai_tests {
    use super::*;

    fn base_input() -> PlayerAiCanCastInput {
        PlayerAiCanCastInput {
            target_exists: true,
            is_triggered: false,
            check_controlled: false,
            caster_cannot_react: false,
            caster_cannot_react_or_lost_control: false,
            spell_prevention_type: 0,
            caster_silenced: false,
            caster_pacified: false,
            caster_power: 100,
            spell_mana_cost: 50,
            target_is_self: false,
            has_range_entry: true,
            distance: 10.0,
            max_range: 30.0,
            min_range: 0.0,
        }
    }

    #[test]
    fn ok_when_in_range_and_ready() {
        assert!(player_ai_can_cast_spell(&base_input()));
    }

    #[test]
    fn no_target_fails() {
        let mut i = base_input();
        i.target_exists = false;
        assert!(!player_ai_can_cast_spell(&i));
    }

    #[test]
    fn no_range_entry_fails_even_when_otherwise_ok() {
        let mut i = base_input();
        i.has_range_entry = false;
        assert!(!player_ai_can_cast_spell(&i));
    }

    #[test]
    fn out_of_range_and_too_close_fail() {
        let mut far = base_input();
        far.distance = 40.0;
        assert!(!player_ai_can_cast_spell(&far));

        let mut close = base_input();
        close.min_range = 5.0;
        close.distance = 3.0;
        assert!(!player_ai_can_cast_spell(&close));
    }

    #[test]
    fn self_target_skips_range_checks() {
        let mut i = base_input();
        i.target_is_self = true;
        i.distance = 999.0; // ignored for self
        assert!(player_ai_can_cast_spell(&i));
    }

    #[test]
    fn silence_and_pacify_block_matching_prevention_type() {
        let mut sil = base_input();
        sil.spell_prevention_type = SPELL_PREVENTION_TYPE_SILENCE;
        sil.caster_silenced = true;
        assert!(!player_ai_can_cast_spell(&sil));

        let mut pac = base_input();
        pac.spell_prevention_type = SPELL_PREVENTION_TYPE_PACIFY;
        pac.caster_pacified = true;
        assert!(!player_ai_can_cast_spell(&pac));
    }

    #[test]
    fn insufficient_power_fails_unless_triggered() {
        let mut i = base_input();
        i.caster_power = 10;
        i.spell_mana_cost = 50;
        assert!(!player_ai_can_cast_spell(&i));

        // Triggered casts skip the whole non-triggered gate.
        i.is_triggered = true;
        assert!(player_ai_can_cast_spell(&i));
    }

    #[test]
    fn check_controlled_uses_stricter_react_state() {
        let mut i = base_input();
        i.check_controlled = true;
        i.caster_cannot_react_or_lost_control = true;
        assert!(!player_ai_can_cast_spell(&i));

        // Without check_controlled, the lost-control state is ignored.
        i.check_controlled = false;
        assert!(player_ai_can_cast_spell(&i));
    }
}
