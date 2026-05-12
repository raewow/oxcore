//! Spell Cast Validation
//!
//! All pre-cast validation checks are pure functions that read player state.

use crate::shared::protocol::ObjectGuid;
use crate::world::game::player::spells::state::{SpellCastError, NUM_SPELL_SCHOOLS};
use crate::world::World;
use anyhow::Result;

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
            school: 0, power_type: 0, mana_cost: 0, range_index: 0,
            max_range: 0.0, cast_time: 0, attributes_ex2: 0,
            stances: 0, stances_not: 0, caster_aura_state: 0, target_aura_state: 0,
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
            use crate::world::game::player::auras::effects::{
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
                        crate::shared::protocol::Position {
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

/// Convert SpellCastError to the u8 code expected by the client
/// Convert SpellCastError to the vanilla 1.12.1 client SpellCastResult enum value.
/// Values from mangos/src/game/Spells/SpellDefines.h (vanilla 1.12.x build).
pub fn spell_cast_error_to_u8(error: SpellCastError) -> u8 {
    use SpellCastError::*;
    match error {
        None => 0,
        CasterDead => 0x13,           // SPELL_FAILED_CASTER_DEAD
        SpellNotKnown => 0x38,        // SPELL_FAILED_NOT_KNOWN
        NotEnoughMana => 0x4D,        // SPELL_FAILED_NO_POWER (mana/rage/energy all use this)
        NotEnoughRage => 0x4D,        // SPELL_FAILED_NO_POWER
        NotEnoughEnergy => 0x4D,      // SPELL_FAILED_NO_POWER
        SpellOnCooldown => 0x3C,      // SPELL_FAILED_NOT_READY
        NotReady => 0x3C,             // SPELL_FAILED_NOT_READY (GCD)
        InvalidTarget => 0x0A,        // SPELL_FAILED_BAD_TARGETS
        TargetOutOfRange => 0x59,     // SPELL_FAILED_OUT_OF_RANGE
        TargetNotInLineOfSight => 0x2A, // SPELL_FAILED_LINE_OF_SIGHT
        NotWhileMoving => 0x2E,       // SPELL_FAILED_MOVING
        Stunned => 0x64,              // SPELL_FAILED_STUNNED
        Silenced => 0x60,             // SPELL_FAILED_SILENCED
        Pacified => 0x5A,             // SPELL_FAILED_PACIFIED
        Confused => 0x16,             // SPELL_FAILED_CONFUSED
        Fleeing => 0x1E,              // SPELL_FAILED_FLEEING
        SchoolLockout => 0x60,        // SPELL_FAILED_SILENCED (school lockout shows as silenced)
        AlreadyCasting => 0x61,       // SPELL_FAILED_SPELL_IN_PROGRESS
        Interrupted => 0x23,          // SPELL_FAILED_INTERRUPTED
        WrongShapeshift => 0x56,      // SPELL_FAILED_ONLY_SHAPESHIFT
        CasterAuraState => 0x12,      // SPELL_FAILED_CASTER_AURASTATE
        TargetAuraState => 0x67,      // SPELL_FAILED_TARGET_AURASTATE
    }
}
