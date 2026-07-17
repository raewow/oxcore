//! Spell Damage Effects
//!
//! Handles school damage, weapon damage, health leech, environmental damage, and normalized weapon damage.
//! Formulas ported from old system (world/game/spell/effects/damage.rs).

use crate::dbc::structures::SpellEntry;
use crate::game::player::auras::effects::{
    AURA_MOD_DAMAGE_PERCENT_DONE, AURA_MOD_DAMAGE_PERCENT_TAKEN,
};
use crate::game::player::player::Player;
use crate::game::player::spells::effects::{EffectInput, EffectResult};
use crate::World;
use anyhow::Result;
use oxcore_shared::protocol::ObjectGuid;

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

/// Apply a critical-strike bonus to spell damage (faithful `SpellCaster::SpellCriticalDamageBonus`).
///
/// Melee/ranged-class spells crit for +100% (double); all other classes crit for +50%.
/// The SPELLMOD_CRIT_DAMAGE_BONUS talent mod and the MOD_CRIT_PERCENT_VERSUS aura
/// multiplier are deferred until those systems are ported.
fn spell_critical_damage_bonus(dmg_class: u32, damage: f32) -> f32 {
    const SPELL_DAMAGE_CLASS_MELEE: u32 = 2;
    const SPELL_DAMAGE_CLASS_RANGED: u32 = 3;

    let crit_bonus = match dmg_class {
        SPELL_DAMAGE_CLASS_MELEE | SPELL_DAMAGE_CLASS_RANGED => damage,
        _ => damage / 2.0,
    };

    damage + crit_bonus
}

fn select_weapon_stats(
    player: &Player,
    spell_entry: Option<&SpellEntry>,
    normalized: bool,
) -> (f32, f32, u32) {
    let attack_type = spell_entry
        .map(|entry| entry.get_weapon_attack_type())
        .unwrap_or(0);

    let (min_dmg, max_dmg, weapon_speed) = match attack_type {
        1 if player.combat.can_dual_wield => (
            player.combat.off_hand_min_dmg,
            player.combat.off_hand_max_dmg,
            player.combat.off_hand_speed,
        ),
        2 if player.combat.has_ranged_weapon => (
            player.combat.ranged_min_dmg,
            player.combat.ranged_max_dmg,
            player.combat.ranged_speed,
        ),
        _ => (
            player.combat.main_hand_min_dmg,
            player.combat.main_hand_max_dmg,
            player.combat.main_hand_speed,
        ),
    };

    let normalized_speed = if normalized {
        match attack_type {
            2 if player.combat.has_ranged_weapon => 2800,
            _ if weapon_speed <= 1800 => 1700,
            _ if weapon_speed <= 2900 => 2400,
            _ => 3300,
        }
    } else {
        weapon_speed
    };

    (min_dmg, max_dmg, normalized_speed)
}

#[inline]
fn spell_school_mask(school: u8) -> u32 {
    1u32.checked_shl(school as u32).unwrap_or(0)
}

fn apply_damage_percent_modifiers(
    damage: f32,
    damage_percent_done: i32,
    damage_percent_taken: i32,
) -> f32 {
    let modifier = 1.0 + (damage_percent_done + damage_percent_taken) as f32 / 100.0;
    (damage * modifier).max(0.0)
}

/// SPELL_EFFECT_SCHOOL_DAMAGE (2)
///
/// Direct spell damage (Fireball, Frostbolt, Shadow Bolt, etc.)
///
/// Calculation (ported from old system):
/// 1. Base damage with dice roll + level scaling via calculate_base_value()
/// 2. + spell_power[school] * coefficient (from DBC or cast_time / 3500)
/// 3. Roll crit (spell_crit_pct)
/// 4. If crit: apply SpellCriticalDamageBonus (+50%, or +100% for melee/ranged-class spells)
/// 5. Resistance reduction: resistance / (caster_level * 5), physical exempt
/// 6. Armor reduction for physical school (vanilla CalcArmorReducedDamage formula)
pub async fn effect_school_damage(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let school = input.spell_school;

    // Get caster stats
    let caster_stats = world
        .systems
        .player
        .manager()
        .with_player(input.caster_guid, |player| {
            let sp = if (school as usize) < 7 {
                player.stats.spell_power[school as usize]
            } else {
                0
            };
            (sp, player.stats.spell_crit_pct, player.level)
        });

    // Step 1: Base damage with dice roll + level scaling
    let caster_level = caster_stats.as_ref().map(|s| s.2).unwrap_or(1);
    let base_damage = input.calculate_base_value(caster_level).max(0) as f32;
    let mut final_damage = base_damage;

    // Step 2: Spell power scaling with coefficient
    if let Some((spell_power, _, _)) = caster_stats {
        let coefficient = input.get_spell_coefficient();
        let mut spell_power_bonus = spell_power as f32 * coefficient;
        // Downranking penalty applies only to the computed (non-DBC) coefficient.
        if input.spell_coefficient <= 0.01 {
            let spell_level = world
                .managers
                .spell_mgr
                .get(input.spell_id)
                .map(|e| e.spell_level)
                .unwrap_or(0);
            spell_power_bonus *= super::calculate_level_penalty(spell_level);
        }
        final_damage += spell_power_bonus;

        tracing::debug!(
            "[SPELL-DAMAGE] spell {} school={}: base={:.1}, SP={}, coeff={:.3}, SP_bonus={:.1}, after_SP={:.1}",
            input.spell_id, school, base_damage, spell_power, coefficient, spell_power_bonus, final_damage
        );
    }

    // Step 3: Apply damage percent bonuses/penalties from caster and target auras.
    // School masks are bitmasks, not indexes: school 0 => 0x01, school 6 => 0x40.
    let school_mask = spell_school_mask(school);
    if school_mask != 0 {
        let caster_damage_bonus = world
            .systems
            .player
            .manager()
            .with_player(input.caster_guid, |player| {
                player
                    .auras
                    .container
                    .get_total_aura_modifier_by_misc_mask(AURA_MOD_DAMAGE_PERCENT_DONE, school_mask)
            })
            .unwrap_or(0);

        let target_damage_taken = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |player| {
                player.auras.container.get_total_aura_modifier_by_misc_mask(
                    AURA_MOD_DAMAGE_PERCENT_TAKEN,
                    school_mask,
                )
            })
            .unwrap_or(0);

        final_damage =
            apply_damage_percent_modifiers(final_damage, caster_damage_bonus, target_damage_taken);
    }

    // Step 4: Roll for crit (spell crit = 150% damage)
    let is_crit = if let Some((_, crit_pct, _)) = caster_stats {
        let crit_roll = rand::random::<f32>() * 100.0;
        crit_roll < crit_pct
    } else {
        false
    };

    if is_crit {
        let dmg_class = world
            .managers
            .spell_mgr
            .get(input.spell_id)
            .map(|e| e.dmg_class)
            .unwrap_or(0);
        final_damage = spell_critical_damage_bonus(dmg_class, final_damage);
    }

    // Step 5: Resistance reduction (non-physical schools only)
    let (damage_after_mitigation, resisted) = if let Some((_, _, level)) = caster_stats {
        apply_target_mitigation(target_guid, final_damage, school, level, world)
    } else {
        (final_damage, 0u32)
    };

    let damage = damage_after_mitigation.max(0.0) as u32;

    let dmg_class = world
        .managers
        .spell_mgr
        .get(input.spell_id)
        .map(|e| e.dmg_class)
        .unwrap_or(0);

    // Apply damage (absorb, health mutation, combat log, procs, death)
    super::super::caster::deal_damage(
        input.caster_guid,
        target_guid,
        damage,
        school,
        input.spell_id,
        is_crit,
        resisted,
        dmg_class,
        world,
    )
    .await;

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
pub async fn effect_normalized_weapon_dmg(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    effect_weapon_damage_internal(input, world, true).await
}

/// Internal weapon damage implementation shared by normal and normalized variants.
async fn effect_weapon_damage_internal(
    input: &EffectInput,
    world: &World,
    normalized: bool,
) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let caster_level = world
        .systems
        .player
        .manager()
        .with_player(input.caster_guid, |p| p.level)
        .unwrap_or(1);
    let bonus_damage = input.calculate_base_value(caster_level).max(0) as f32;
    let spell_entry = world.managers.spell_mgr.get(input.spell_id);

    // Get caster weapon stats, AP, crit, and level
    let caster_data = world
        .systems
        .player
        .manager()
        .with_player(input.caster_guid, |player| {
            let (min_dmg, max_dmg, ap_speed) =
                select_weapon_stats(player, spell_entry.as_deref(), normalized);

            (
                min_dmg,
                max_dmg,
                ap_speed,
                player.stats.melee_attack_power as f32,
                player.stats.melee_crit_pct,
                player.level,
            )
        });

    let mut total_damage = bonus_damage;

    let mut is_crit = false;
    let mut attacker_level = 60u8;

    if let Some((min_dmg, max_dmg, ap_speed, ap, crit_pct, level)) = caster_data {
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
    let (damage_after_armor, armor_resisted) =
        apply_target_mitigation(target_guid, total_damage, 0, attacker_level, world);

    let damage = damage_after_armor as u32;

    let dmg_class = spell_entry.as_ref().map(|e| e.dmg_class).unwrap_or(0);

    super::super::caster::deal_damage(
        input.caster_guid,
        target_guid,
        damage,
        0,
        input.spell_id,
        is_crit,
        armor_resisted,
        dmg_class,
        world,
    )
    .await;

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
    let caster_stats = world
        .systems
        .player
        .manager()
        .with_player(input.caster_guid, |player| {
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
    super::super::caster::deal_damage(
        input.caster_guid,
        target_guid,
        damage,
        school,
        input.spell_id,
        false,
        0,
        0,
        world,
    )
    .await;

    // Heal caster for the same amount
    super::super::caster::deal_heal(
        input.caster_guid,
        input.caster_guid,
        damage,
        input.spell_id,
        false,
        world,
    )
    .await;

    Ok(EffectResult {
        damage,
        healing: damage,
        success: true,
        target_guid: None,
        effect_index: 0,
        execute_log: None,
    })
}

/// SPELL_EFFECT_WEAPON_PERCENT_DAMAGE (31)
///
/// Deals a percentage of weapon damage.
/// Used by abilities like Backstab (150%), Ambush (250%).
pub async fn effect_weapon_percent_damage(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    // base_value contains the percentage (e.g., 150 = 150% weapon damage)
    let caster_level = world
        .systems
        .player
        .manager()
        .with_player(input.caster_guid, |p| p.level)
        .unwrap_or(1);
    let percent = input.calculate_base_value(caster_level).max(0) as f32 / 100.0;
    let spell_entry = world.managers.spell_mgr.get(input.spell_id);

    // Get caster weapon stats and AP
    let caster_data = world
        .systems
        .player
        .manager()
        .with_player(input.caster_guid, |player| {
            let (min_dmg, max_dmg, weapon_speed) =
                select_weapon_stats(player, spell_entry.as_deref(), false);
            (
                min_dmg,
                max_dmg,
                weapon_speed,
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
    let (damage_after_armor, armor_resisted) =
        apply_target_mitigation(target_guid, total_damage, 0, attacker_level, world);

    let damage = damage_after_armor as u32;

    let dmg_class = spell_entry.as_ref().map(|e| e.dmg_class).unwrap_or(0);

    super::super::caster::deal_damage(
        input.caster_guid,
        target_guid,
        damage,
        0,
        input.spell_id,
        is_crit,
        armor_resisted,
        dmg_class,
        world,
    )
    .await;

    Ok(EffectResult::with_damage(damage))
}

/// SPELL_EFFECT_ENVIRONMENTAL_DAMAGE (7)
///
/// Damage from environmental sources (fire, lava, drowning, falling).
/// Bypasses armor but affected by resistance.
/// No threat generation.
pub async fn effect_environmental_damage(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let damage = input.base_value.max(0) as u32;
    let damage_type = EnvironmentalDamageType::from_u32(input.misc_value as u32)
        .unwrap_or(EnvironmentalDamageType::Fire);

    // Apply environmental damage
    world
        .systems
        .player
        .manager()
        .with_player_mut(target_guid, |player| {
            let current_health = player.stats.health;
            let new_health = current_health.saturating_sub(damage);
            player.stats.health = new_health;

            tracing::debug!(
                "Environmental damage: {} took {} {:?} damage, health: {} -> {}",
                player.name,
                damage,
                damage_type,
                current_health,
                new_health
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
        world
            .systems
            .player
            .manager()
            .with_player(target_guid, |player| {
                let resist = if school != 0 && (school as usize) < 7 {
                    player.stats.resistances[school as usize]
                } else {
                    0
                };
                (player.stats.armor, resist)
            })
            .unwrap_or((0, 0))
    } else if target_guid.is_creature() {
        world
            .managers
            .creature_mgr
            .with_creature(target_guid, |creature| {
                // Creatures have armor but no per-school resistance fields yet
                // TODO: Add per-school resistances to Creature struct from template
                (creature.armor, 0u32)
            })
            .unwrap_or((0, 0))
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
        let reduction = crate::game::combat::armor_reduction_fraction(armor, caster_level);
        let armor_reduced = (mitigated * reduction) as u32;
        total_resisted += armor_reduced;
        mitigated *= 1.0 - reduction;
    }

    (mitigated, total_resisted)
}

#[cfg(test)]
mod tests {
    use super::{apply_damage_percent_modifiers, spell_school_mask};

    #[test]
    fn school_mask_uses_bit_positions() {
        assert_eq!(spell_school_mask(0), 0x01);
        assert_eq!(spell_school_mask(6), 0x40);
    }

    #[test]
    fn school_mask_rejects_invalid_shifts() {
        assert_eq!(spell_school_mask(32), 0);
    }

    #[test]
    fn damage_percent_modifiers_add_and_clamp() {
        assert!((apply_damage_percent_modifiers(100.0, 20, -10) - 110.0).abs() < 0.001);
        assert!((apply_damage_percent_modifiers(100.0, -80, -30) - 0.0).abs() < 0.001);
    }
}
