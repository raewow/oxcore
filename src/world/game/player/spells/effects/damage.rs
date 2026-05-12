//! Spell Damage Effects
//!
//! Handles school damage, weapon damage, health leech, environmental damage, and normalized weapon damage.
//! Formulas ported from old system (world/game/spell/effects/damage.rs).

use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};
use crate::shared::messages::ToWorldPacket;
use crate::world::game::player::spells::effects::{EffectInput, EffectResult};
use crate::world::World;
use anyhow::Result;

/// Environmental damage types (from misc_value)
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentalDamageType {
    Fire = 0,
    Lava = 1,
    Drowning = 2,
    Falling = 3,
}

impl EnvironmentalDamageType {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Fire),
            1 => Some(Self::Lava),
            2 => Some(Self::Drowning),
            3 => Some(Self::Falling),
            _ => None,
        }
    }
}

/// Calculate resistance reduction percentage.
/// Vanilla formula: resistance / (caster_level * 5), capped at 75%.
/// Physical school (0) is exempt from spell resistance.
fn calculate_resistance_reduction(caster_level: u8, resistance: u32, school: u8) -> f32 {
    if school == 0 || resistance == 0 {
        return 0.0;
    }

    let resist_pct = resistance as f32 / (caster_level as f32 * 5.0);
    resist_pct.min(0.75).max(0.0)
}

/// Calculate armor reduction percentage.
/// Vanilla formula: armor / (armor + 400 + 85 * attacker_level), capped at 75%.
fn calculate_armor_reduction(attacker_level: u8, armor: u32) -> f32 {
    if armor == 0 {
        return 0.0;
    }

    let armor_f = armor as f32;
    let level_f = attacker_level as f32;
    let armor_constant = 400.0 + 85.0 * level_f;
    let reduction = armor_f / (armor_f + armor_constant);
    reduction.min(0.75)
}

/// SPELL_EFFECT_SCHOOL_DAMAGE (2)
///
/// Direct spell damage (Fireball, Frostbolt, Shadow Bolt, etc.)
///
/// Calculation (ported from old system):
/// 1. Base damage with dice roll + level scaling via calculate_base_value()
/// 2. + spell_power[school] * coefficient (from DBC or cast_time / 3500)
/// 3. Roll crit (spell_crit_pct)
/// 4. If crit: * 1.5
/// 5. Resistance reduction: resistance / (caster_level * 5), physical exempt
/// 6. Armor reduction for physical school: armor / (armor + 400 + 85 * level)
pub async fn effect_school_damage(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let school = input.spell_school;

    // Get caster stats
    let caster_stats = world.systems.player.manager().with_player(input.caster_guid, |player| {
        let sp = if (school as usize) < 7 {
            player.stats.spell_power[school as usize]
        } else {
            0
        };
        (
            sp,
            player.stats.spell_crit_pct,
            player.level,
        )
    });

    // Step 1: Base damage with dice roll + level scaling
    let caster_level = caster_stats.as_ref().map(|s| s.2).unwrap_or(1);
    let base_damage = input.calculate_base_value(caster_level).max(0) as f32;
    let mut final_damage = base_damage;

    // Step 2: Spell power scaling with coefficient
    if let Some((spell_power, _, _)) = caster_stats {
        let coefficient = input.get_spell_coefficient();
        let spell_power_bonus = spell_power as f32 * coefficient;
        final_damage += spell_power_bonus;

        tracing::debug!(
            "[SPELL-DAMAGE] spell {} school={}: base={:.1}, SP={}, coeff={:.3}, SP_bonus={:.1}, after_SP={:.1}",
            input.spell_id, school, base_damage, spell_power, coefficient, spell_power_bonus, final_damage
        );
    }

    // Step 3: Roll for crit (spell crit = 150% damage)
    let is_crit = if let Some((_, crit_pct, _)) = caster_stats {
        let crit_roll = rand::random::<f32>() * 100.0;
        crit_roll < crit_pct
    } else {
        false
    };

    if is_crit {
        final_damage *= 1.5;
    }

    // Step 4: Resistance reduction (non-physical schools only)
    let (damage_after_mitigation, resisted) = if let Some((_, _, level)) = caster_stats {
        apply_target_mitigation(target_guid, final_damage, school, level, world)
    } else {
        (final_damage, 0u32)
    };

    let damage = damage_after_mitigation.max(0.0) as u32;

    // Apply damage
    apply_damage(input.caster_guid, target_guid, damage, input.spell_id, is_crit, school, resisted, world).await?;

    Ok(EffectResult::with_damage(damage))
}

/// SPELL_EFFECT_WEAPON_DAMAGE (58) / WEAPON_DAMAGE_NOSCHOOL (17)
///
/// Weapon-based abilities (Mortal Strike, Sinister Strike, etc.)
///
/// Calculation:
/// 1. Weapon damage roll (min_dmg..max_dmg from equipped weapon)
/// 2. + base_value (flat bonus from spell, e.g., Heroic Strike +138)
/// 3. + Attack Power contribution (AP / 14 * weapon_speed)
/// 4. Roll crit (melee_crit_chance)
/// 5. If crit: * 2.0 (melee crit = 200%, not 150% like spells)
/// 6. Armor reduction using actual caster level
pub async fn effect_weapon_damage(input: &EffectInput, world: &World) -> Result<EffectResult> {
    effect_weapon_damage_internal(input, world, false).await
}

/// SPELL_EFFECT_NORMALIZED_WEAPON_DMG (121)
///
/// Same as weapon damage but uses normalized weapon speed for AP scaling.
/// Normalized speeds: Dagger=1.7s, Other 1H=2.4s, 2H=3.3s, Ranged=2.8s
pub async fn effect_normalized_weapon_dmg(input: &EffectInput, world: &World) -> Result<EffectResult> {
    effect_weapon_damage_internal(input, world, true).await
}

/// Internal weapon damage implementation shared by normal and normalized variants.
async fn effect_weapon_damage_internal(input: &EffectInput, world: &World, normalized: bool) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let caster_level = world.systems.player.manager().with_player(input.caster_guid, |p| p.level).unwrap_or(1);
    let bonus_damage = input.calculate_base_value(caster_level).max(0) as f32;

    // Get caster weapon stats, AP, crit, and level
    let caster_data = world.systems.player.manager().with_player(input.caster_guid, |player| {
        // Determine normalized speed based on weapon type
        // TODO: Read actual weapon subclass from equipment for proper categorization
        // For now, use main hand speed to guess: <=1800ms = dagger, <=2900ms = 1H, else 2H
        let normalized_speed_ms = if normalized {
            if player.combat.main_hand_speed <= 1800 {
                1700u32 // Dagger: 1.7s
            } else if player.combat.main_hand_speed <= 2900 {
                2400u32 // Other 1H: 2.4s
            } else {
                3300u32 // 2H: 3.3s
            }
        } else {
            player.combat.main_hand_speed
        };

        (
            player.combat.main_hand_min_dmg,
            player.combat.main_hand_max_dmg,
            player.combat.main_hand_speed,
            normalized_speed_ms,
            player.stats.melee_attack_power as f32,
            player.stats.melee_crit_pct,
            player.level,
        )
    });

    let mut total_damage = bonus_damage;

    let mut is_crit = false;
    let mut attacker_level = 60u8;

    if let Some((min_dmg, max_dmg, _weapon_speed, ap_speed, ap, crit_pct, level)) = caster_data {
        attacker_level = level;

        // 1. Roll weapon damage
        let weapon_damage = if max_dmg > min_dmg {
            min_dmg + rand::random::<f32>() * (max_dmg - min_dmg)
        } else {
            min_dmg
        };
        total_damage += weapon_damage;

        // 2. Add AP contribution using appropriate speed (normalized or actual)
        let ap_contribution = (ap / 14.0) * (ap_speed as f32 / 1000.0);
        total_damage += ap_contribution;

        // 3. Roll for crit (melee crit = 200% damage)
        let crit_roll = rand::random::<f32>() * 100.0;
        is_crit = crit_roll < crit_pct;
        if is_crit {
            total_damage *= 2.0;
        }

        tracing::debug!(
            "[SPELL-WEAPON-DMG] spell {}: weapon={:.1}-{:.1}, bonus={:.1}, AP_contrib={:.1}, total={:.1}, crit={}, normalized={}",
            input.spell_id, min_dmg, max_dmg, bonus_damage, ap_contribution, total_damage, is_crit, normalized
        );
    }

    // 4. Apply target armor reduction using actual caster level (supports creatures)
    let (damage_after_armor, armor_resisted) = apply_target_mitigation(
        target_guid, total_damage, 0, attacker_level, world,
    );

    let damage = damage_after_armor as u32;

    apply_damage(input.caster_guid, target_guid, damage, input.spell_id, is_crit, 0, armor_resisted, world).await?;

    Ok(EffectResult::with_damage(damage))
}

/// SPELL_EFFECT_HEALTH_LEECH (9)
///
/// Drain Life, Death Coil, etc.
/// Damages target and heals caster for the damage dealt.
/// Uses school damage calculation for the damage portion.
pub async fn effect_health_leech(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let school = input.spell_school;

    // Get caster stats for spell power scaling
    let caster_stats = world.systems.player.manager().with_player(input.caster_guid, |player| {
        let sp = if (school as usize) < 7 {
            player.stats.spell_power[school as usize]
        } else {
            0
        };
        (sp, player.level)
    });

    let caster_level = caster_stats.as_ref().map(|s| s.1).unwrap_or(1);
    let base_damage = input.calculate_base_value(caster_level).max(0) as f32;
    let mut leech_damage = base_damage;

    // Add spell power scaling
    if let Some((spell_power, _)) = caster_stats {
        let coefficient = input.get_spell_coefficient();
        leech_damage += spell_power as f32 * coefficient;
    }

    // Apply resistance
    let (damage_after_resist, _) = if let Some((_, level)) = caster_stats {
        apply_target_mitigation(target_guid, leech_damage, school, level, world)
    } else {
        (leech_damage, 0)
    };

    let damage = damage_after_resist.max(0.0) as u32;

    // Damage target
    apply_damage(input.caster_guid, target_guid, damage, input.spell_id, false, school, 0, world).await?;

    // Heal caster for the same amount
    heal_target(input.caster_guid, damage, world).await?;

    Ok(EffectResult {
        damage,
        healing: damage,
        success: true,
    })
}

/// SPELL_EFFECT_WEAPON_PERCENT_DAMAGE (31)
///
/// Deals a percentage of weapon damage.
/// Used by abilities like Backstab (150%), Ambush (250%).
pub async fn effect_weapon_percent_damage(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    // base_value contains the percentage (e.g., 150 = 150% weapon damage)
    let caster_level = world.systems.player.manager().with_player(input.caster_guid, |p| p.level).unwrap_or(1);
    let percent = input.calculate_base_value(caster_level).max(0) as f32 / 100.0;

    // Get caster weapon stats and AP
    let caster_data = world.systems.player.manager().with_player(input.caster_guid, |player| {
        (
            player.combat.main_hand_min_dmg,
            player.combat.main_hand_max_dmg,
            player.combat.main_hand_speed,
            player.stats.melee_attack_power as f32,
            player.stats.melee_crit_pct,
            player.level,
        )
    });

    let mut total_damage = 0.0f32;
    let mut is_crit = false;
    let mut attacker_level = 60u8;

    if let Some((min_dmg, max_dmg, weapon_speed, ap, crit_pct, level)) = caster_data {
        attacker_level = level;

        // Roll weapon damage
        let weapon_damage = if max_dmg > min_dmg {
            min_dmg + rand::random::<f32>() * (max_dmg - min_dmg)
        } else {
            min_dmg
        };

        // AP contribution
        let ap_contribution = (ap / 14.0) * (weapon_speed as f32 / 1000.0);

        // Apply percentage to weapon damage + AP
        total_damage = (weapon_damage + ap_contribution) * percent;

        // Roll for crit (melee crit = 200% damage)
        let crit_roll = rand::random::<f32>() * 100.0;
        is_crit = crit_roll < crit_pct;
        if is_crit {
            total_damage *= 2.0;
        }
    }

    // Apply armor reduction (supports creatures)
    let (damage_after_armor, armor_resisted) = apply_target_mitigation(
        target_guid, total_damage, 0, attacker_level, world,
    );

    let damage = damage_after_armor as u32;

    apply_damage(input.caster_guid, target_guid, damage, input.spell_id, is_crit, 0, armor_resisted, world).await?;

    Ok(EffectResult::with_damage(damage))
}

/// SPELL_EFFECT_ENVIRONMENTAL_DAMAGE (7)
///
/// Damage from environmental sources (fire, lava, drowning, falling).
/// Bypasses armor but affected by resistance.
/// No threat generation.
pub async fn effect_environmental_damage(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let damage = input.base_value.max(0) as u32;
    let damage_type = EnvironmentalDamageType::from_u32(input.misc_value as u32)
        .unwrap_or(EnvironmentalDamageType::Fire);

    // Apply environmental damage
    world.systems.player.manager().with_player_mut(target_guid, |player| {
        let current_health = player.stats.health;
        let new_health = current_health.saturating_sub(damage);
        player.stats.health = new_health;

        tracing::debug!(
            "Environmental damage: {} took {} {:?} damage, health: {} -> {}",
            player.name, damage, damage_type, current_health, new_health
        );
    });

    Ok(EffectResult::with_damage(damage))
}

/// Apply resistance and/or armor mitigation to damage based on spell school.
/// Supports both player and creature targets.
/// Returns (mitigated_damage, resisted_amount).
fn apply_target_mitigation(
    target_guid: ObjectGuid,
    damage: f32,
    school: u8,
    caster_level: u8,
    world: &World,
) -> (f32, u32) {
    // Get target's armor and resistance values
    let (armor, resistance) = if target_guid.is_player() {
        world.systems.player.manager().with_player(target_guid, |player| {
            let resist = if school != 0 && (school as usize) < 7 {
                player.stats.resistances[school as usize]
            } else {
                0
            };
            (player.stats.armor, resist)
        }).unwrap_or((0, 0))
    } else if target_guid.is_creature() {
        world.managers.creature_mgr.with_creature(target_guid, |creature| {
            // Creatures have armor but no per-school resistance fields yet
            // TODO: Add per-school resistances to Creature struct from template
            (creature.armor, 0u32)
        }).unwrap_or((0, 0))
    } else {
        (0, 0)
    };

    let mut mitigated = damage;
    let mut total_resisted = 0u32;

    if school != 0 {
        let resist_pct = calculate_resistance_reduction(caster_level, resistance, school);
        let resisted = (mitigated * resist_pct) as u32;
        total_resisted += resisted;
        mitigated *= 1.0 - resist_pct;
    } else {
        let reduction = calculate_armor_reduction(caster_level, armor);
        let armor_reduced = (mitigated * reduction) as u32;
        total_resisted += armor_reduced;
        mitigated *= 1.0 - reduction;
    }

    (mitigated, total_resisted)
}

/// Apply damage to a target (player or creature).
/// Handles combat log packets (P5), creature targeting (P1), death (P6), and cast pushback (P6).
async fn apply_damage(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    damage: u32,
    spell_id: u32,
    is_crit: bool,
    school: u8,
    resisted: u32,
    world: &World,
) -> Result<()> {
    if target_guid.is_player() {
        // --- Player target ---

        // Process absorb shields before applying damage
        let (damage_after_absorb, absorbed) = world.systems.auras
            .absorb_damage(target_guid, damage, school, world)
            .await?;

        let died = world.systems.player.manager().with_player_mut(target_guid, |player| {
            let current_health = player.stats.health;
            let new_health = current_health.saturating_sub(damage_after_absorb);
            player.stats.health = new_health;
            player.stats.dirty = true;

            tracing::debug!(
                "Spell damage: {} took {} damage (absorbed={}), health: {} -> {}",
                player.name, damage_after_absorb, absorbed, current_health, new_health
            );

            new_health == 0 && current_health > 0
        }).unwrap_or(false);

        // Send SMSG_SPELLNONMELEEDAMAGELOG (report original damage for combat log, absorbed shown separately)
        send_spell_damage_log(caster_guid, target_guid, spell_id, damage_after_absorb, school, resisted, is_crit, world);

        // Cast pushback: if target is casting, push back their cast bar
        // Pushback triggers even if damage was fully absorbed (MaNGOS behavior)
        if damage > 0 && !died {
            let _ = world.systems.spells.apply_cast_pushback(target_guid, world);
        }

        // Interrupt auras with DAMAGE flag on target (triggers even if absorbed)
        if damage > 0 {
            let _ = world.systems.auras.remove_auras_with_interrupt_flag(
                target_guid,
                0x00000002, // AURA_INTERRUPT_FLAG_DAMAGE (bit 1)
                world,
            ).await;
        }

        // Fire proc checks: caster dealt spell damage, target took spell damage
        // Use damage (pre-absorb) for proc triggering, damage_after_absorb for amounts
        if damage > 0 {
            use crate::world::game::player::auras::proc::proc_flags;
            // Caster: spell hit dealt
            let _ = world.systems.auras.check_procs(
                caster_guid,
                proc_flags::SPELL_HIT | proc_flags::DAMAGE_DEALT,
                Some(spell_id),
                damage_after_absorb,
                world,
            ).await;
            // Target: spell hit taken (only if target is a player)
            if target_guid.is_player() {
                let _ = world.systems.auras.check_procs(
                    target_guid,
                    proc_flags::SPELL_HIT_TAKEN | proc_flags::DAMAGE_TAKEN,
                    Some(spell_id),
                    damage_after_absorb,
                    world,
                ).await;
            }
        }

        // P6: Death handling
        if died {
            if let Err(e) = world.systems.death.on_killed(target_guid, Some(caster_guid), Some(spell_id), world) {
                tracing::error!("Failed to handle player death: {}", e);
            }
        }
    } else if target_guid.is_creature() {
        // --- Creature target ---
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let result = world.managers.creature_mgr.apply_damage(
            target_guid,
            damage,
            caster_guid,
            timestamp,
        );

        if let Some((actual_damage, is_dead)) = result {
            tracing::info!(
                "[SPELL_DAMAGE] creature {:?} took {} damage from spell {} (caster {:?}), dead={}",
                target_guid, actual_damage, spell_id, caster_guid, is_dead
            );

            // Send SMSG_SPELLNONMELEEDAMAGELOG (works for creature targets too)
            send_spell_damage_log(caster_guid, target_guid, spell_id, damage, school, resisted, is_crit, world);

            // Send creature health/death update to nearby players (SMSG_UPDATE_OBJECT)
            // Without this, the client never sees the health bar change or death animation
            if is_dead {
                // Mark dead on server first (snaps position to spline location)
                let death_info = world.managers.creature_mgr.handle_death(target_guid, Some(caster_guid));

                // Send stop packet BEFORE death VALUES update so client stops
                // the spline at the correct position before processing death state
                if let Some(ref info) = death_info {
                    world.systems.creature_movement.send_stop_packet(info.guid, info.position, world);
                }

                // Now send death VALUES update (health=0, stand state Dead)
                send_creature_killed_update(caster_guid, target_guid, world);

                crate::world::game::creature::ai::queue_event(
                    world,
                    target_guid,
                    crate::world::game::creature::ai::AIEvent::Died {
                        killer_guid: Some(caster_guid),
                    },
                );

                tracing::info!("Creature {:?} killed by spell {} from {:?}", target_guid, spell_id, caster_guid);
            } else if actual_damage > 0 {
                send_creature_health_update(target_guid, world);

                // Queue AI event so creature enters combat and chases attacker
                crate::world::game::creature::ai::queue_event(
                    world,
                    target_guid,
                    crate::world::game::creature::ai::AIEvent::DamageTaken {
                        attacker_guid: caster_guid,
                        damage: actual_damage,
                        spell_id: Some(spell_id),
                        school,
                    },
                );
            }
        } else {
            tracing::warn!("[SPELL_DAMAGE] creature {:?} not found for spell {}", target_guid, spell_id);
        }
    }

    Ok(())
}

/// Build and broadcast SMSG_SPELLNONMELEEDAMAGELOG packet (P5).
fn send_spell_damage_log(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    spell_id: u32,
    damage: u32,
    school: u8,
    resisted: u32,
    is_crit: bool,
    world: &World,
) {
    let spell_school_mask = if school == 0 { 1u8 } else { 1u8 << school };

    let mut packet = WorldPacket::new(Opcode::SMSG_SPELLNONMELEEDAMAGELOG);
    packet.write_packed_guid_raw(target_guid.raw());
    packet.write_packed_guid_raw(caster_guid.raw());
    packet.write_u32(spell_id);
    packet.write_u32(damage);
    packet.write_u8(spell_school_mask);
    packet.write_u32(0); // absorbed
    packet.write_u32(resisted);
    packet.write_u8(0); // periodicLog (0 = not periodic)
    packet.write_u8(0); // unused
    packet.write_u32(0); // blocked
    let mut hit_info = 0u32;
    if is_crit {
        hit_info |= 0x02; // HITINFO_CRITICALHIT
    }
    packet.write_u32(hit_info);
    packet.write_u8(0); // debug info flag

    world.managers.broadcast_mgr.broadcast_nearby(caster_guid, &packet, true);
}

/// Heal a target (player only for now).
async fn heal_target(
    target_guid: ObjectGuid,
    healing: u32,
    world: &World,
) -> Result<()> {
    if target_guid.is_player() {
        world.systems.player.manager().with_player_mut(target_guid, |player| {
            let max_heal = player.stats.max_health.saturating_sub(player.stats.health);
            let actual_heal = healing.min(max_heal);
            player.stats.health += actual_heal;
            player.stats.dirty = true;

            tracing::debug!(
                "Spell heal: {} healed for {}, health: {} -> {}",
                player.name, actual_heal, player.stats.health - actual_heal, player.stats.health
            );
        });
    }

    Ok(())
}

/// Send creature health update to nearby players via SMSG_UPDATE_OBJECT.
/// Mirrors the send_health_update() in creature_combat handler.
fn send_creature_health_update(creature_guid: ObjectGuid, world: &World) {
    use crate::world::game::common::update_fields::{UNIT_FIELD_HEALTH, UNIT_FIELD_MAXHEALTH};
    use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
    use crate::shared::messages::{SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock, ObjectType};
    use crate::world::game::broadcast_mgr::broadcast_around_creature;

    if let Some((current, max)) = world.managers.creature_mgr.get_health(creature_guid) {
        let world_guid = WorldObjectGuid::new_creature(creature_guid.entry(), creature_guid.counter());
        let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(world_guid, ObjectType::Unit)
                .set_field(UNIT_FIELD_HEALTH, current)
                .set_field(UNIT_FIELD_MAXHEALTH, max)
        ));
        broadcast_around_creature(world, creature_guid, &update.to_world_packet());
    }
}

/// Send creature death update to nearby players via SMSG_UPDATE_OBJECT.
/// Mirrors send_creature_killed_update() in creature_combat handler.
fn send_creature_killed_update(caster_guid: ObjectGuid, creature_guid: ObjectGuid, world: &World) {
    use crate::world::game::common::update_fields::*;
    use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
    use crate::shared::messages::{SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock, ObjectType};
    use crate::world::game::broadcast_mgr::broadcast_around_creature;

    let (max_health, unit_flags) = world.managers.creature_mgr
        .with_creature_mut(creature_guid, |c| (c.max_health, c.unit_flags))
        .unwrap_or((1, 0));

    // Clear IN_COMBAT from unit flags
    let cleared_flags = unit_flags & !crate::world::game::common::unit_flags::IN_COMBAT;

    let world_guid = WorldObjectGuid::new_creature(creature_guid.entry(), creature_guid.counter());
    let empty_guid = WorldObjectGuid::from_raw(0);
    let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
        ValuesUpdateBlock::new(world_guid, ObjectType::Unit)
            .set_guid_field(UNIT_FIELD_TARGET, empty_guid)
            .set_field(UNIT_FIELD_HEALTH, 0u32)
            .set_field(UNIT_FIELD_MAXHEALTH, max_health)
            .set_field(UNIT_FIELD_FLAGS, cleared_flags)
            .set_field(UNIT_DYNAMIC_FLAGS, crate::world::game::creature::death::UNIT_DYNFLAG_DEAD)
            .set_field(UNIT_FIELD_BYTES_1, 7u32) // Stand state Dead
            .set_field(UNIT_NPC_FLAGS, 0u32)
    ));
    broadcast_around_creature(world, creature_guid, &update.to_world_packet());

    // Also send attack stop from both sides
    let stop_packet = crate::shared::messages::combat::SmsgAttackStop {
        attacker_guid: caster_guid,
        target_guid: creature_guid,
        unk: 1, // target is dead
    };
    world.managers.broadcast_mgr.broadcast_nearby(caster_guid, &stop_packet.to_world_packet(), true);
}
