use crate::dbc::structures::SpellEntry;
use crate::game::player::auras::aura::Aura;
use crate::game::player::auras::effects::{
    AURA_MELEE_ATTACK_POWER_ATTACKER_BONUS, AURA_MOD_DAMAGE_DONE, AURA_MOD_DAMAGE_DONE_CREATURE,
    AURA_MOD_DAMAGE_DONE_VERSUS, AURA_MOD_DAMAGE_PERCENT_DONE, AURA_MOD_FLAT_SPELL_DAMAGE_VERSUS,
    AURA_MOD_HEALING_DONE, AURA_MOD_HEALING_DONE_PERCENT, AURA_MOD_MELEE_ATTACK_POWER_VERSUS,
    AURA_MOD_OFFHAND_DAMAGE_PCT, AURA_MOD_RANGED_ATTACK_POWER_VERSUS,
    AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT, AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT,
    AURA_OVERRIDE_CLASS_SCRIPTS, AURA_RANGED_ATTACK_POWER_ATTACKER_BONUS,
};
use crate::game::player::player::Player;
use crate::game::player::spells::state::SpellModOp;
use crate::World;
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::ObjectGuid;

const WORLD_BOSS_LEVEL_DIFF: u32 = 3;
const SPELL_SCHOOL_MASK_NORMAL: u32 = 0x01;
const SPELL_ATTR_EX3_IGNORE_CASTER_MODIFIERS: u32 = 0x0002_0000;
const SPELL_EFFECT_SCHOOL_DAMAGE: u32 = 2;
const SPELL_EFFECT_NORMALIZED_WEAPON_DMG: u32 = 121;
const SPELL_DAMAGE_CLASS_NONE: u32 = 0;
const SPELL_DAMAGE_CLASS_MAGIC: u32 = 1;
const SPELL_DAMAGE_CLASS_MELEE: u32 = 2;
const SPELL_DAMAGE_CLASS_RANGED: u32 = 3;
const BASE_ATTACK: u32 = 0;
const OFF_ATTACK: u32 = 1;
const RANGED_ATTACK: u32 = 2;
const DIRECT_DAMAGE: u32 = 0;
const SPELL_DIRECT_DAMAGE: u32 = 1;
const DOT: u32 = 2;
const SPELL_HIT_TYPE_CRIT: u32 = 0x02;
const SPELL_ATTR_EX3_TREAT_AS_PERIODIC: u32 = 0x02000000;
const MELEE_HIT_NORMAL: u32 = 0;
const MELEE_HIT_CRIT: u32 = 1;
const SPELL_CUSTOM_FIXED_DAMAGE: u32 = 0x0000_0004;
const SPELLFAMILY_MAGE: u32 = 3;
const SPELLFAMILY_PALADIN: u32 = 10;
const CF_MAGE_IGNITE: u64 = 1 << 27; // 0x0800_0000
const CF_PALADIN_SEALS: u64 = 1 << 27; // 0x0800_0000

fn get_unit_level(guid: ObjectGuid, world: &World) -> Option<u32> {
    if guid.is_player() {
        return world
            .managers
            .player_mgr
            .with_player(guid, |player: &Player| player.level as u32);
    }

    if guid.is_creature() || guid.is_pet() {
        return world
            .managers
            .creature_mgr
            .with_creature(guid, |creature| creature.level as u32);
    }

    None
}

/// Get the effective level for target-dependent calculations.
pub fn get_level_for_target(
    source_guid: ObjectGuid,
    target_guid: Option<ObjectGuid>,
    world: &World,
) -> u32 {
    if let Some((creature_level, creature_entry)) = world
        .managers
        .creature_mgr
        .with_creature(source_guid, |creature| {
            (creature.level as u32, creature.entry)
        })
    {
        if world
            .managers
            .creature_mgr
            .get_template(creature_entry)
            .is_some_and(|template| template.rank == 3)
        {
            if let Some(target_guid) = target_guid.filter(|guid| guid.is_unit()) {
                if let Some(target_level) = get_unit_level(target_guid, world) {
                    return target_level
                        .saturating_add(WORLD_BOSS_LEVEL_DIFF)
                        .clamp(1, 255);
                }
            }
        }

        return creature_level;
    }

    if let Some(level) = world
        .managers
        .player_mgr
        .with_player(source_guid, |player: &Player| player.level as u32)
    {
        return level;
    }

    if let Some(level) = world
        .managers
        .gameobject_mgr
        .with_gameobject(source_guid, |go| go.level)
    {
        if level != 0 {
            return level;
        }
    }

    if let Some(target_guid) = target_guid.filter(|guid| guid.is_unit()) {
        if let Some(level) = get_unit_level(target_guid, world) {
            return level;
        }
    }

    world.config.max_player_level.clamp(1, 255)
}

/// Get the melee damage school mask.
pub fn get_melee_damage_school_mask(
    _source_guid: ObjectGuid,
    _attack_type: u8,
    _world: &World,
) -> u32 {
    SPELL_SCHOOL_MASK_NORMAL
}

/// Creature rank spell damage modifier.
/// Returns configurable multiplier (defaults to 1.0 for all ranks).
pub fn get_spell_damage_mod(rank: u8, world: &World) -> f32 {
    match rank {
        0 => 1.0, // CREATURE_ELITE_NORMAL
        1 => 1.0, // CREATURE_ELITE_ELITE
        2 => 1.0, // CREATURE_ELITE_RAREELITE
        3 => 1.0, // CREATURE_ELITE_WORLDBOSS
        4 => 1.0, // CREATURE_ELITE_RARE
        _ => 1.0,
    }
}

/// AP multiplier for weapon-damage-based spells.
///
/// `att_type`: 0 = BASE_ATTACK, 1 = OFF_ATTACK, 2 = RANGED_ATTACK
/// `normalized`: when true uses fixed normalized speeds (dagger=1.7, 1H=2.4, 2H=3.3, ranged=2.8)
pub fn get_ap_multiplier(
    att_type: u32,
    normalized: bool,
    weapon_speed: f32,
    inventory_type: u32,
    weapon_subclass: u32,
) -> f32 {
    if !normalized {
        return weapon_speed / 1000.0;
    }

    match att_type {
        RANGED_ATTACK => 2.8,
        _ => {
            match inventory_type {
                17 | 15 | 16 if weapon_subclass == 15 => 1.7, // Dagger in weapon slot
                17 | 15 | 16 => 2.4,                          // 1H weapon
                10 | 5 => 2.8,                                // Ranged/thrown
                _ if inventory_type == 14 || inventory_type == 19 => 3.3, // 2H weapon
                _ => 2.4,                                     // Default (fist weapon, etc.)
            }
        }
    }
}

/// Calculate spell bonus with coefficient scaling and level penalty.
///
/// `total` is running total, `benefit` is the flat bonus being scaled.
pub fn spell_bonus_with_coeffs(
    spell_proto: Option<&SpellEntry>,
    effect_index: u8,
    mut total: f32,
    benefit: f32,
    damage_type: u32,
    done_part: bool,
    world: &World,
) -> f32 {
    if benefit == 0.0 {
        return total;
    }

    let ei = effect_index as usize;

    // 1. Use DBC coefficient if available, else compute default
    let has_dbc_coeff = spell_proto
        .map(|s| ei < 3 && s.effect_bonus_coefficient[ei] >= 0.0)
        .unwrap_or(false);

    let coeff = if let Some(proto) = spell_proto {
        if has_dbc_coeff {
            proto.effect_bonus_coefficient[ei]
        } else {
            proto.calculate_default_coefficient() as f32
        }
    } else {
        1.0
    };

    // 2. Level penalty only applies to default (non-DBC) coefficients
    let lvl_penalty = if has_dbc_coeff {
        1.0
    } else if let Some(proto) = spell_proto {
        crate::game::player::spells::effects::calculate_level_penalty(proto.spell_level)
    } else {
        1.0
    };

    // 3. Custom coefficient (from scripts) — currently no-op
    let coeff = coeff;

    total += benefit * coeff * lvl_penalty;
    total
}

/// Caster-side melee damage bonus calculation.
///
/// Calculates flat and percent damage bonuses from caster auras,
/// victim auras, AP bonuses, creature type modifiers, and pet bonuses.
///
/// Only the **player** aura path is implemented; creature auras use defaults.
pub fn melee_damage_bonus_done(
    caster_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    pdamage: f32,
    att_type: u32,
    spell_proto: Option<&SpellEntry>,
    effect_index: u8,
    damage_type: u32,
    stack: u32,
    flat: bool,
    world: &World,
) -> f32 {
    // Null victim or zero damage → return as-is
    if pdamage == 0.0 {
        return 0.0;
    }

    // Spells with IGNORE_CASTER_MODIFIERS bypass done bonuses
    if let Some(proto) = spell_proto {
        if proto.has_attribute_ex3(SPELL_ATTR_EX3_IGNORE_CASTER_MODIFIERS) {
            return pdamage;
        }
    }

    // Differentiate weapon-damage-based spells
    let is_weapon_damage_based = !(spell_proto
        .is_some_and(|s| damage_type == DOT || s.has_effect(SPELL_EFFECT_SCHOOL_DAMAGE)));

    // Creature type mask: 1 << (creature_type - 1), 0 if unknown
    let creature_type_mask = world
        .managers
        .creature_mgr
        .with_creature(victim_guid, |c| {
            let ct = c.creature_type;
            if ct > 0 {
                1u32 << (ct - 1)
            } else {
                0
            }
        })
        .unwrap_or(0);

    // School mask
    let school_mask = spell_proto
        .map(|s| 1u32 << s.school)
        .unwrap_or(SPELL_SCHOOL_MASK_NORMAL);

    // === FLAT damage bonus auras ===
    let mut done_flat = 0i32;

    // ..done flat, already included in weapon-damage-based spells
    let caster_is_player = caster_guid.is_player();
    let victim_is_player = victim_guid.is_player();

    if !is_weapon_damage_based && caster_is_player {
        let mod_damage_done = world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                player
                    .auras
                    .container
                    .get_total_aura_modifier_by_misc_mask(AURA_MOD_DAMAGE_DONE, school_mask)
            })
            .unwrap_or(0);
        done_flat += mod_damage_done;
    }

    // ..done flat (by creature type mask)
    if creature_type_mask != 0 && caster_is_player {
        done_flat += world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                player.auras.container.get_total_aura_modifier_by_misc_mask(
                    AURA_MOD_DAMAGE_DONE_CREATURE,
                    creature_type_mask,
                )
            })
            .unwrap_or(0);
    }

    // ..AP bonuses from victim and caster auras
    let mut ap_bonus = 0i32;
    if att_type == RANGED_ATTACK {
        if victim_is_player {
            ap_bonus += world
                .systems
                .player
                .manager()
                .with_player(victim_guid, |player| {
                    player
                        .auras
                        .container
                        .get_total_aura_modifier(AURA_RANGED_ATTACK_POWER_ATTACKER_BONUS)
                })
                .unwrap_or(0);
        }
        if creature_type_mask != 0 && caster_is_player {
            ap_bonus += world
                .systems
                .player
                .manager()
                .with_player(caster_guid, |player| {
                    player.auras.container.get_total_aura_modifier_by_misc_mask(
                        AURA_MOD_RANGED_ATTACK_POWER_VERSUS,
                        creature_type_mask,
                    )
                })
                .unwrap_or(0);
        }
    } else {
        if victim_is_player {
            ap_bonus += world
                .systems
                .player
                .manager()
                .with_player(victim_guid, |player| {
                    player
                        .auras
                        .container
                        .get_total_aura_modifier(AURA_MELEE_ATTACK_POWER_ATTACKER_BONUS)
                })
                .unwrap_or(0);
        }
        if creature_type_mask != 0 && caster_is_player {
            ap_bonus += world
                .systems
                .player
                .manager()
                .with_player(caster_guid, |player| {
                    player.auras.container.get_total_aura_modifier_by_misc_mask(
                        AURA_MOD_MELEE_ATTACK_POWER_VERSUS,
                        creature_type_mask,
                    )
                })
                .unwrap_or(0);
        }
    }

    // === PERCENT damage auras ===
    let mut done_percent = 1.0f32;

    // Creature rank damage mod (non-player, non-hunter-pet)
    if !is_weapon_damage_based && !caster_is_player {
        if let Some(template) = world
            .managers
            .creature_mgr
            .with_creature(caster_guid, |c| {
                world.managers.creature_mgr.get_template(c.entry)
            })
            .flatten()
        {
            done_percent *= get_spell_damage_mod(template.rank, world);
        }
    }

    // ..done pct, already included in weapon-damage-based spells
    if !is_weapon_damage_based && caster_is_player {
        let pct_done = world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                player
                    .auras
                    .container
                    .get_total_aura_multiplier_by_misc_mask(
                        AURA_MOD_DAMAGE_PERCENT_DONE,
                        school_mask,
                    )
            })
            .unwrap_or(1.0);
        done_percent *= pct_done;

        // Off-hand penalty
        if att_type == OFF_ATTACK {
            let offhand_mod = world
                .systems
                .player
                .manager()
                .with_player(caster_guid, |player| {
                    player
                        .auras
                        .container
                        .get_total_aura_modifier(AURA_MOD_OFFHAND_DAMAGE_PCT)
                })
                .unwrap_or(0);
            done_percent *= (100.0 + offhand_mod as f32) / 100.0;
        }
    }

    // ..done pct (by creature type mask)
    if creature_type_mask != 0 && caster_is_player {
        done_percent *= world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                player
                    .auras
                    .container
                    .get_total_aura_multiplier_by_misc_mask(
                        AURA_MOD_DAMAGE_DONE_VERSUS,
                        creature_type_mask,
                    )
            })
            .unwrap_or(1.0);
    }

    // === Final calculation ===
    let mut done_total = 0.0f32;

    // Scaling of non-weapon-based spells
    if !is_weapon_damage_based {
        done_total = spell_bonus_with_coeffs(
            spell_proto,
            effect_index,
            done_total,
            done_flat as f32,
            damage_type,
            true,
            world,
        );
    }
    // Weapon-damage-based spells
    else if ap_bonus != 0 || done_flat != 0 {
        let normalized =
            spell_proto.is_some_and(|s| s.has_effect(SPELL_EFFECT_NORMALIZED_WEAPON_DMG));

        let weapon_speed = match att_type {
            OFF_ATTACK => world
                .systems
                .player
                .manager()
                .with_player(caster_guid, |player| player.combat.off_hand_speed)
                .unwrap_or(2000),
            RANGED_ATTACK => world
                .systems
                .player
                .manager()
                .with_player(caster_guid, |player| player.combat.ranged_speed)
                .unwrap_or(2000),
            _ => world
                .systems
                .player
                .manager()
                .with_player(caster_guid, |player| player.combat.main_hand_speed)
                .unwrap_or(2000),
        } as f32;

        let ap_mult = if normalized {
            match att_type {
                RANGED_ATTACK => 2.8,
                _ if weapon_speed <= 1800.0 => 1.7,
                _ if weapon_speed <= 2900.0 => 2.4,
                _ => 3.3,
            }
        } else {
            weapon_speed / 1000.0
        };
        done_total += ap_bonus as f32 / 14.0 * ap_mult;
        done_total += done_flat as f32;

        // Apply weapon damage percent mods (TOTAL_PCT)
        let unit_mod = match att_type {
            OFF_ATTACK => crate::game::player::stats::modifiers::UnitMods::DamageOffhand,
            RANGED_ATTACK => crate::game::player::stats::modifiers::UnitMods::DamageRanged,
            _ => crate::game::player::stats::modifiers::UnitMods::DamageMainhand,
        };
        if caster_is_player {
            done_total *= world
                .systems
                .player
                .manager()
                .with_player(caster_guid, |player| {
                    player.stats.unit_mods.get_modifier_value(
                        unit_mod,
                        crate::game::player::stats::modifiers::UnitModifierType::TotalPct,
                    )
                })
                .unwrap_or(1.0);
        }
    }

    if !flat {
        done_total = 0.0;
    }

    let mut tmp_damage = (pdamage + done_total * stack as f32) * done_percent;

    // Apply spell mod to done damage
    if spell_proto.is_some() && caster_is_player {
        let mod_op = if damage_type == DOT {
            SpellModOp::Dot
        } else {
            SpellModOp::Damage
        };
        tmp_damage = world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                crate::game::player::spells::modifiers::apply_spell_modifiers_to_value_f32(
                    &player.spells.spell_modifiers,
                    mod_op,
                    tmp_damage,
                    spell_proto.map(|s| s.spell_family_name).unwrap_or(0),
                    spell_proto.map(|s| s.spell_family_flags).unwrap_or(0),
                )
            })
            .unwrap_or(tmp_damage);
    }

    // Clamp negative to zero
    if tmp_damage > 0.0 {
        tmp_damage
    } else {
        0.0
    }
}

/// Base healing bonus from +healing gear and stat conversion.
///
/// Sums AURA_MOD_HEALING_DONE values matching school mask,
/// plus AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT * spirit / 100 for players.
pub fn spell_base_healing_bonus_done(
    school_mask: u32,
    caster_guid: ObjectGuid,
    world: &World,
) -> f32 {
    let mut advertised_benefit = 0.0f32;

    if caster_guid.is_player() {
        // Flat healing done by school mask
        advertised_benefit += world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                player
                    .auras
                    .container
                    .get_total_aura_modifier_by_misc_mask(AURA_MOD_HEALING_DONE, school_mask)
            })
            .unwrap_or(0) as f32;

        // Healing bonus from stats (spirit in 1.12)
        let stat_pct = world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                player
                    .auras
                    .container
                    .get_total_aura_modifier(AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT)
            })
            .unwrap_or(0);

        if stat_pct != 0 {
            let spirit = world
                .systems
                .player
                .manager()
                .with_player(caster_guid, |player| player.stats.spirit)
                .unwrap_or(0);
            advertised_benefit += spirit as f32 * stat_pct as f32 / 100.0;
        }
    }

    advertised_benefit
}

/// Caster-side healing bonus calculation.
///
/// Applies percent healing mods, scripted class overrides, base healing bonus
/// from gear, coefficient scaling via `spell_bonus_with_coeffs`, and spell mods.
pub fn spell_healing_bonus_done(
    caster_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    healamount: f32,
    spell_proto: Option<&SpellEntry>,
    effect_index: u8,
    damage_type: u32,
    stack: u32,
    world: &World,
) -> f32 {
    // Early exit: passive/NONE dmg class, fixed damage, or ignore caster mods
    if let Some(proto) = spell_proto {
        if proto.dmg_class == SPELL_DAMAGE_CLASS_NONE && (proto.attributes & 0x0000_0040) != 0 {
            return if healamount < 0.0 { 0.0 } else { healamount };
        }
        if (proto.custom & 0x0000_0004) != 0 {
            return if healamount < 0.0 { 0.0 } else { healamount };
        }
        if proto.has_attribute_ex3(SPELL_ATTR_EX3_IGNORE_CASTER_MODIFIERS) {
            return if healamount < 0.0 { 0.0 } else { healamount };
        }
    }

    let mut done_total_mod = 1.0f32;
    let mut done_total = 0.0f32;

    // Healing done percent auras
    if caster_guid.is_player() {
        done_total_mod *= world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                player
                    .auras
                    .container
                    .get_total_aura_multiplier_by_misc(AURA_MOD_HEALING_DONE_PERCENT, 0)
            })
            .unwrap_or(1.0);

        // Scripted class overrides (AURA_OVERRIDE_CLASS_SCRIPTS)
        let auras_list = world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                player
                    .auras
                    .container
                    .get_auras_by_type(AURA_OVERRIDE_CLASS_SCRIPTS)
                    .into_iter()
                    .map(|a| a.clone())
                    .collect::<Vec<Aura>>()
            })
            .unwrap_or_default();

        for aura in &auras_list {
            let misc = aura.misc_value;
            if !matches!(misc, 4415 | 3736) {
                continue;
            }
            if let Some(proto) = spell_proto {
                // Check IsAffectedOnSpell: look up aura's spell entry to compare family
                let aura_spell = world.managers.spell_mgr.get(aura.spell_id);
                let family_match = aura_spell.as_ref().map_or(true, |s| {
                    s.spell_family_name == 0 || s.spell_family_name == proto.spell_family_name
                });
                let mask_match = aura_spell.as_ref().map_or(true, |s| {
                    s.spell_family_flags == 0
                        || (s.spell_family_flags & proto.spell_family_flags) != 0
                });
                if family_match && mask_match {
                    let amount = aura.current_value() as f32;
                    match misc {
                        4415 => done_total += amount / 4.0,
                        3736 => done_total += amount,
                        _ => {}
                    }
                }
            }
        }
    }

    // Base healing bonus from gear (school mask based on spell school)
    let school_mask = spell_proto.map(|s| 1u32 << s.school).unwrap_or(1);
    let advertised_benefit = spell_base_healing_bonus_done(school_mask, caster_guid, world);

    // Coefficient scaling
    done_total = spell_bonus_with_coeffs(
        spell_proto,
        effect_index,
        done_total,
        advertised_benefit,
        damage_type,
        true,
        world,
    );

    // Final calculation
    let mut heal = (healamount + done_total * stack as f32) * done_total_mod;

    // Apply spell mod to done amount
    if spell_proto.is_some() && caster_guid.is_player() {
        let mod_op = if damage_type == DOT {
            SpellModOp::Dot
        } else {
            SpellModOp::Damage
        };
        heal = world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                crate::game::player::spells::modifiers::apply_spell_modifiers_to_value_f32(
                    &player.spells.spell_modifiers,
                    mod_op,
                    heal,
                    spell_proto.map(|s| s.spell_family_name).unwrap_or(0),
                    spell_proto.map(|s| s.spell_family_flags).unwrap_or(0),
                )
            })
            .unwrap_or(heal);
    }

    if heal < 0.0 {
        0.0
    } else {
        heal
    }
}

/// Base damage bonus from +spell damage gear and stat conversion.
///
/// Sums AURA_MOD_DAMAGE_DONE values matching school mask and item class constraints,
/// plus AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT * spirit / 100 for players.
pub fn spell_base_damage_bonus_done(
    school_mask: u32,
    caster_guid: ObjectGuid,
    world: &World,
) -> i32 {
    if !caster_guid.is_player() {
        return 0;
    }

    world
        .systems
        .player
        .manager()
        .with_player(caster_guid, |player| {
            // Flat damage-done auras matching school mask, excluding wand-only auras
            // (EquippedItemClass == -1 && EquippedItemInventoryTypeMask == 0).
            let mut benefit = 0i32;
            for aura in player
                .auras
                .container
                .get_auras_by_type(AURA_MOD_DAMAGE_DONE)
            {
                if (aura.misc_value as u32 & school_mask) == 0 {
                    continue;
                }
                let aura_spell = world.managers.spell_mgr.get(aura.spell_id);
                let is_generic_item = aura_spell.as_deref().map_or(true, |s| {
                    s.equipped_item_class == -1 && s.equipped_item_inventory_type_mask == 0
                });
                if is_generic_item {
                    benefit += aura.current_value();
                }
            }

            // Damage bonus from stats (spirit in 1.12)
            let stat_auras = player
                .auras
                .container
                .get_auras_by_type(AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT)
                .clone();
            for aura in stat_auras {
                if (aura.misc_value as u32 & school_mask) != 0 {
                    let spirit = player.stats.spirit as f32;
                    let pct = aura.current_value() as f32;
                    benefit += (spirit * pct / 100.0) as i32;
                }
            }

            benefit
        })
        .unwrap_or(0)
}

/// Caster-side spell damage bonus calculation.
///
/// Applies percent damage mods, creature rank mod, versus/creature type bonuses,
/// override class scripts, pet happiness, base damage bonus, coefficient scaling,
/// and spell mods.
pub fn spell_damage_bonus_done(
    caster_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    spell_proto: Option<&SpellEntry>,
    effect_index: u8,
    pdamage: f32,
    damage_type: u32,
    stack: u32,
    world: &World,
) -> f32 {
    // Early exits
    if spell_proto.is_none() || damage_type == DIRECT_DAMAGE {
        return pdamage;
    }
    let proto = spell_proto.unwrap();

    if (proto.custom & SPELL_CUSTOM_FIXED_DAMAGE) != 0 {
        return pdamage;
    }
    if proto.has_attribute_ex3(SPELL_ATTR_EX3_IGNORE_CASTER_MODIFIERS) {
        return pdamage;
    }
    // Mage Ignite already includes modifiers
    if proto.spell_family_name == SPELLFAMILY_MAGE
        && (proto.spell_family_flags & CF_MAGE_IGNITE) != 0
    {
        return pdamage;
    }

    // For totems get damage bonus from owner (placeholder)
    if caster_guid.is_creature() {
        // Totem delegation — not yet implemented; pass through
    }

    let mut done_total_mod = 1.0f32;
    let mut done_total = 0.0f32;

    // Creature rank damage mod (non-player, non-hunter-pet)
    if caster_guid.is_creature() {
        if let Some(template) = world
            .managers
            .creature_mgr
            .with_creature(caster_guid, |c| {
                world.managers.creature_mgr.get_template(c.entry)
            })
            .flatten()
        {
            done_total_mod *= get_spell_damage_mod(template.rank, world);
        }
    }

    // AURA_MOD_DAMAGE_PERCENT_DONE — with item class constraints
    if caster_guid.is_player() {
        let school_mask = spell_proto
            .map(|s| 1u32 << s.school)
            .unwrap_or(SPELL_SCHOOL_MASK_NORMAL);
        let pct_auras = world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                player
                    .auras
                    .container
                    .get_auras_by_type(AURA_MOD_DAMAGE_PERCENT_DONE)
                    .iter()
                    .map(|a| (*a).clone())
                    .collect::<Vec<Aura>>()
            })
            .unwrap_or_default();

        for aura in &pct_auras {
            let misc_mask = aura.misc_value as u32;
            let aura_spell = world.managers.spell_mgr.get(aura.spell_id);
            let aura_spell_info = aura_spell.as_deref();
            let pct = (aura.current_value() + 100) as f32 / 100.0;

            // Normal case: school matches, both item classes == -1, both inv types == 0
            if (misc_mask & school_mask) != 0
                && aura_spell_info.map_or(true, |s| s.equipped_item_class == -1)
                && proto.equipped_item_class == -1
                && aura_spell_info.map_or(true, |s| s.equipped_item_inventory_type_mask == 0)
            {
                done_total_mod *= pct;
            }
            // Paladin seals: melee school, paladin seal family, item class check
            else if (misc_mask & SPELL_SCHOOL_MASK_NORMAL) != 0
                && proto.spell_family_name == SPELLFAMILY_PALADIN
                && (proto.spell_family_flags & CF_PALADIN_SEALS) != 0
                && aura_spell_info.map_or(true, |s| s.equipped_item_class == -1)
            {
                done_total_mod *= pct;
            }
        }
    }

    // Creature type mask for victim
    let creature_type_mask = world
        .managers
        .creature_mgr
        .with_creature(victim_guid, |c| {
            let ct = c.creature_type;
            if ct > 0 {
                1u32 << (ct - 1)
            } else {
                0
            }
        })
        .unwrap_or(0);

    // Damage versus (pct) and flat damage creature
    if caster_guid.is_player() {
        if creature_type_mask != 0 {
            done_total_mod *= world
                .systems
                .player
                .manager()
                .with_player(caster_guid, |player| {
                    player
                        .auras
                        .container
                        .get_total_aura_multiplier_by_misc_mask(
                            AURA_MOD_DAMAGE_DONE_VERSUS,
                            creature_type_mask,
                        )
                })
                .unwrap_or(1.0);

            done_total += world
                .systems
                .player
                .manager()
                .with_player(caster_guid, |player| {
                    player.auras.container.get_total_aura_modifier_by_misc_mask(
                        AURA_MOD_DAMAGE_DONE_CREATURE,
                        creature_type_mask,
                    ) as f32
                })
                .unwrap_or(0.0);
        }
    }

    // Override class scripts (take from owner)
    if caster_guid.is_player() {
        let scripts = world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                player
                    .auras
                    .container
                    .get_auras_by_type(AURA_OVERRIDE_CLASS_SCRIPTS)
                    .iter()
                    .map(|a| (*a).clone())
                    .collect::<Vec<Aura>>()
            })
            .unwrap_or_default();

        for aura in &scripts {
            // Check IsAffectedOnSpell
            let aura_spell = world.managers.spell_mgr.get(aura.spell_id);
            let family_match = aura_spell.as_ref().map_or(true, |s| {
                s.spell_family_name == 0 || s.spell_family_name == proto.spell_family_name
            });
            let mask_match = aura_spell.as_ref().map_or(true, |s| {
                s.spell_family_flags == 0 || (s.spell_family_flags & proto.spell_family_flags) != 0
            });
            if !family_match || !mask_match {
                continue;
            }

            match aura.misc_value {
                4418 | 4554 => {
                    // Increased Shock/Lightning Damage
                    done_total += aura.current_value() as f32;
                }
                4555 => {
                    // Improved Moonfire (Idol of the moon)
                    let divisor = if damage_type == DOT {
                        let max_ticks = proto.get_aura_max_ticks(&*world.dbc.read());
                        (100 * max_ticks) as f32
                    } else {
                        800.0
                    };
                    done_total += aura.current_value() as f32 * pdamage / divisor;
                }
                _ => {}
            }
        }
    }

    // Pet happiness (placeholder — not yet implemented)

    // Base damage bonus from gear
    let school_mask = proto.school;
    let bit_mask = 1u32 << school_mask;
    let mut advertised_benefit = spell_base_damage_bonus_done(bit_mask, caster_guid, world);

    // Flat spell damage versus
    if creature_type_mask != 0 && caster_guid.is_player() {
        advertised_benefit += world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                player.auras.container.get_total_aura_modifier_by_misc_mask(
                    AURA_MOD_FLAT_SPELL_DAMAGE_VERSUS,
                    creature_type_mask,
                )
            })
            .unwrap_or(0);
    }

    // Pet bonus damage (placeholder — not yet implemented)

    // Coefficient scaling via spell_bonus_with_coeffs
    done_total = spell_bonus_with_coeffs(
        spell_proto,
        effect_index,
        done_total,
        advertised_benefit as f32,
        damage_type,
        true,
        world,
    );

    // Final calculation
    let mut tmp_damage = (pdamage + done_total * stack as f32) * done_total_mod;

    // Apply spell mod to done damage
    if caster_guid.is_player() {
        let mod_op = if damage_type == DOT {
            SpellModOp::Dot
        } else {
            SpellModOp::Damage
        };
        tmp_damage = world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                crate::game::player::spells::modifiers::apply_spell_modifiers_to_value_f32(
                    &player.spells.spell_modifiers,
                    mod_op,
                    tmp_damage,
                    proto.spell_family_name,
                    proto.spell_family_flags,
                )
            })
            .unwrap_or(tmp_damage);
    }

    if tmp_damage > 0.0 {
        tmp_damage
    } else {
        0.0
    }
}

/// Deal spell damage wrapper.
///
/// Validates the victim (alive, not taxi flying, not in evade mode),
/// looks up the spell entry, creates clean damage info with crit, and
/// delegates to `deal_damage`.
pub async fn deal_spell_damage(
    caster_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    spell_id: u32,
    damage: u32,
    school: u8,
    hit_info: u32,
    resisted: u32,
    spell_proto: Option<&SpellEntry>,
    world: &World,
) -> u32 {
    // Validate victim
    let victim_alive = is_unit_alive(victim_guid, world);
    if !victim_alive {
        return 0;
    }

    // Taxi flying and evade mode checks are skipped for now (no Unit type).

    let is_crit = (hit_info & SPELL_HIT_TYPE_CRIT) != 0;
    let dmg_class = spell_proto
        .map(|s| s.dmg_class)
        .unwrap_or(SPELL_DAMAGE_CLASS_NONE);

    deal_damage(
        caster_guid,
        victim_guid,
        damage,
        school,
        spell_id,
        is_crit,
        resisted,
        dmg_class,
        world,
    )
    .await
}

/// Check whether a unit (player or creature) is alive, by GUID.
fn is_unit_alive(guid: ObjectGuid, world: &World) -> bool {
    if guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player(guid, |p| p.stats.health > 0)
            .unwrap_or(true)
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(guid, |c| c.current_health > 0)
            .unwrap_or(true)
    }
}

/// Roll a partial block against a player victim for melee/ranged-class spell damage.
///
/// Full blocks (spells with the completely-blocked `attributes_ex3` flag) are already
/// resolved as a miss during [`crate::game::player::spells::hit::roll_spell_hit`]; this only
/// covers the partial block that vanilla applies to melee/ranged-class spell damage that
/// landed. Non-melee/ranged spells and creature victims never block.
fn roll_partial_block(victim_guid: ObjectGuid, damage: u32, dmg_class: u32, world: &World) -> u32 {
    if !matches!(
        dmg_class,
        SPELL_DAMAGE_CLASS_MELEE | SPELL_DAMAGE_CLASS_RANGED
    ) {
        return 0;
    }
    if !victim_guid.is_player() {
        return 0;
    }

    let (can_block, block_pct, block_value) = world
        .systems
        .player
        .manager()
        .with_player(victim_guid, |p| {
            (p.combat.can_block, p.stats.block_pct, p.stats.block_value)
        })
        .unwrap_or((false, 0.0, 0));

    if !can_block {
        return 0;
    }

    let roll = rand::random::<f32>() * 100.0;
    if roll >= block_pct {
        return 0;
    }

    block_value.min(damage)
}

/// Deal damage to a unit.
///
/// Rolls partial block, applies absorb shields, mutates target health, sends the combat
/// log, fires procs, and hands off to death processing on a killing blow. This is the
/// single damage-application path for spell damage; effect handlers compute the
/// pre-mitigation damage and call this to apply it.
pub async fn deal_damage(
    caster_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    damage: u32,
    school: u8,
    spell_id: u32,
    is_crit: bool,
    resisted: u32,
    dmg_class: u32,
    world: &World,
) -> u32 {
    // Self-damage guard
    if caster_guid == victim_guid {
        return 0;
    }

    let blocked = roll_partial_block(victim_guid, damage, dmg_class, world);
    let damage_after_block = damage.saturating_sub(blocked);

    if victim_guid.is_player() {
        deal_damage_to_player(
            caster_guid,
            victim_guid,
            damage_after_block,
            school,
            spell_id,
            is_crit,
            resisted,
            blocked,
            world,
        )
        .await
    } else if victim_guid.is_creature() {
        deal_damage_to_creature(
            caster_guid,
            victim_guid,
            damage_after_block,
            school,
            spell_id,
            is_crit,
            resisted,
            blocked,
            world,
        )
        .await
    } else {
        damage_after_block
    }
}

async fn deal_damage_to_player(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    damage: u32,
    school: u8,
    spell_id: u32,
    is_crit: bool,
    resisted: u32,
    blocked: u32,
    world: &World,
) -> u32 {
    // Absorb shields.
    let (damage_after_absorb, absorbed) = world
        .systems
        .auras
        .absorb_damage(target_guid, damage, school, world)
        .await
        .unwrap_or((damage, 0));

    let died = world
        .systems
        .player
        .manager()
        .with_player_mut(target_guid, |player| {
            let current_health = player.stats.health;
            player.apply_damage(damage_after_absorb);
            let new_health = player.stats.health;
            new_health == 0 && current_health > 0
        })
        .unwrap_or(false);

    send_spell_damage_log(
        caster_guid,
        target_guid,
        spell_id,
        damage_after_absorb,
        school,
        resisted,
        absorbed,
        blocked,
        is_crit,
        world,
    );

    // Cast pushback triggers even if damage was fully absorbed.
    if damage > 0 && !died {
        let _ = world.systems.spells.apply_cast_pushback(target_guid, world);
    }

    // Interrupt auras with the DAMAGE flag on the target (triggers even if absorbed).
    if damage > 0 {
        let _ = world
            .systems
            .auras
            .remove_auras_with_interrupt_flag(
                target_guid,
                0x0000_0002, // AURA_INTERRUPT_FLAG_DAMAGE (bit 1)
                world,
            )
            .await;
    }

    if damage > 0 && !died {
        stand_player_up_on_damage(target_guid, world);
    }

    if damage > 0 {
        use crate::game::player::auras::proc::{proc_flags, proc_flags_ex};
        let proc_ex = if is_crit {
            proc_flags_ex::CRITICAL_HIT
        } else {
            proc_flags_ex::NORMAL_HIT
        };
        let _ = world
            .systems
            .auras
            .check_procs(
                caster_guid,
                proc_flags::DEAL_HARMFUL_SPELL,
                proc_ex,
                Some(spell_id),
                damage_after_absorb,
                world,
            )
            .await;
        if target_guid.is_player() {
            let _ = world
                .systems
                .auras
                .check_procs(
                    target_guid,
                    proc_flags::TAKE_HARMFUL_SPELL | proc_flags::TAKEN_ANY_DAMAGE,
                    proc_ex,
                    Some(spell_id),
                    damage_after_absorb,
                    world,
                )
                .await;
        }
    }

    if died {
        if let Err(e) =
            world
                .systems
                .death
                .on_killed(target_guid, Some(caster_guid), Some(spell_id), world)
        {
            tracing::error!("Failed to handle player death: {}", e);
        }
    }

    damage_after_absorb
}

async fn deal_damage_to_creature(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    damage: u32,
    school: u8,
    spell_id: u32,
    is_crit: bool,
    resisted: u32,
    blocked: u32,
    world: &World,
) -> u32 {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let Some((actual_damage, is_dead)) =
        world
            .managers
            .creature_mgr
            .apply_damage(target_guid, damage, caster_guid, timestamp)
    else {
        tracing::warn!(
            "[SPELL_DAMAGE] creature {:?} not found for spell {}",
            target_guid,
            spell_id
        );
        return 0;
    };

    send_spell_damage_log(
        caster_guid,
        target_guid,
        spell_id,
        damage,
        school,
        resisted,
        0,
        blocked,
        is_crit,
        world,
    );

    if is_dead {
        let death_info = world
            .managers
            .creature_mgr
            .handle_death(target_guid, Some(caster_guid));

        if let Some(ref info) = death_info {
            world
                .systems
                .creature_movement
                .send_stop_packet(info.guid, info.position, world);
        }

        send_creature_killed_update(caster_guid, target_guid, world);

        crate::game::creature::ai::queue_event(
            world,
            target_guid,
            crate::game::creature::ai::AIEvent::Died {
                killer_guid: Some(caster_guid),
            },
        );
    } else if actual_damage > 0 {
        send_creature_health_update(target_guid, world);

        crate::game::creature::ai::queue_event(
            world,
            target_guid,
            crate::game::creature::ai::AIEvent::DamageTaken {
                attacker_guid: caster_guid,
                damage: actual_damage,
                spell_id: Some(spell_id),
                school,
            },
        );
    }

    // Caster-side "dealt harmful spell" procs (target-side procs are player-only today).
    if actual_damage > 0 {
        use crate::game::player::auras::proc::{proc_flags, proc_flags_ex};
        let proc_ex = if is_crit {
            proc_flags_ex::CRITICAL_HIT
        } else {
            proc_flags_ex::NORMAL_HIT
        };
        let _ = world
            .systems
            .auras
            .check_procs(
                caster_guid,
                proc_flags::DEAL_HARMFUL_SPELL,
                proc_ex,
                Some(spell_id),
                actual_damage,
                world,
            )
            .await;
    }

    actual_damage
}

/// Wake a sitting player when they take damage.
fn stand_player_up_on_damage(target_guid: ObjectGuid, world: &World) {
    use oxcore_shared::protocol::{Opcode, WorldPacket};

    let should_send = world
        .systems
        .player
        .manager()
        .with_player_mut(target_guid, |player| {
            if player.stand_state == 0 {
                return false;
            }
            player.stand_state = 0;
            true
        })
        .unwrap_or(false);

    if should_send {
        let mut packet = WorldPacket::new(Opcode::SMSG_STANDSTATE_UPDATE);
        packet.write_u8(0);
        world
            .managers
            .broadcast_mgr
            .send_msg_to_player(target_guid, packet);
    }
}

/// Build and broadcast SMSG_SPELLNONMELEEDAMAGELOG.
#[allow(clippy::too_many_arguments)]
fn send_spell_damage_log(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    spell_id: u32,
    damage: u32,
    school: u8,
    resisted: u32,
    absorbed: u32,
    blocked: u32,
    is_crit: bool,
    world: &World,
) {
    use oxcore_shared::protocol::{Opcode, WorldPacket};

    let mut packet = WorldPacket::new(Opcode::SMSG_SPELLNONMELEEDAMAGELOG);
    packet.write_packed_guid_raw(target_guid.raw());
    packet.write_packed_guid_raw(caster_guid.raw());
    packet.write_u32(spell_id);
    packet.write_u32(damage);
    packet.write_u8(school);
    packet.write_u32(absorbed);
    packet.write_u32(resisted);
    packet.write_u8(0); // periodicLog (0 = not periodic)
    packet.write_u8(0); // unused
    packet.write_u32(blocked);
    let mut hit_info = 0u32;
    if is_crit {
        hit_info |= 0x02; // HITINFO_CRITICALHIT
    }
    packet.write_u32(hit_info);
    packet.write_u8(0); // debug info flag

    world
        .managers
        .broadcast_mgr
        .broadcast_nearby_packet(caster_guid, &packet, true);
}

/// Send creature health update to nearby players via SMSG_UPDATE_OBJECT.
fn send_creature_health_update(creature_guid: ObjectGuid, world: &World) {
    use crate::core::common::guid::ObjectGuid as WorldObjectGuid;
    use crate::game::broadcast_mgr::broadcast_around_creature;
    use crate::game::common::update_fields::{UNIT_FIELD_HEALTH, UNIT_FIELD_MAXHEALTH};
    use oxcore_shared::messages::{
        ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
    };

    if let Some((current, max)) = world.managers.creature_mgr.get_health(creature_guid) {
        let world_guid =
            WorldObjectGuid::new_creature(creature_guid.entry(), creature_guid.counter());
        let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(world_guid, ObjectType::Unit)
                .set_field(UNIT_FIELD_HEALTH, current)
                .set_field(UNIT_FIELD_MAXHEALTH, max),
        ));
        broadcast_around_creature(world, creature_guid, &update);
    }
}

/// Send creature death update to nearby players via SMSG_UPDATE_OBJECT.
fn send_creature_killed_update(caster_guid: ObjectGuid, creature_guid: ObjectGuid, world: &World) {
    use crate::core::common::guid::ObjectGuid as WorldObjectGuid;
    use crate::game::broadcast_mgr::broadcast_around_creature;
    use crate::game::common::update_fields::*;
    use oxcore_shared::messages::{
        ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
    };

    let (max_health, unit_flags) = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| (c.max_health, c.unit_flags))
        .unwrap_or((1, 0));

    let cleared_flags = unit_flags & !crate::game::common::unit_flags::IN_COMBAT;

    let world_guid = WorldObjectGuid::new_creature(creature_guid.entry(), creature_guid.counter());
    let empty_guid = WorldObjectGuid::from_raw(0);
    let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
        ValuesUpdateBlock::new(world_guid, ObjectType::Unit)
            .set_guid_field(UNIT_FIELD_TARGET, empty_guid)
            .set_field(UNIT_FIELD_HEALTH, 0u32)
            .set_field(UNIT_FIELD_MAXHEALTH, max_health)
            .set_field(UNIT_FIELD_FLAGS, cleared_flags)
            .set_field(
                UNIT_DYNAMIC_FLAGS,
                crate::game::creature::death::UNIT_DYNFLAG_DEAD,
            )
            .set_field(UNIT_FIELD_BYTES_1, 7u32) // Stand state Dead
            .set_field(UNIT_NPC_FLAGS, 0u32),
    ));
    broadcast_around_creature(world, creature_guid, &update);

    let stop_packet = oxcore_shared::messages::combat::SmsgAttackStop {
        attacker_guid: caster_guid,
        target_guid: creature_guid,
        unk: 1, // target is dead
    };
    world
        .managers
        .broadcast_mgr
        .broadcast_msg_nearby(caster_guid, &stop_packet, true);
}

/// Deal healing to a target unit.
///
/// Applies the heal (clamped to max health), sends the combat log, and fires procs.
/// Player targets only — creatures are not healed by player spells in vanilla.
/// Returns the actual amount healed (post-overheal clamp).
pub async fn deal_heal(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    heal_amount: u32,
    spell_id: u32,
    is_crit: bool,
    world: &World,
) -> u32 {
    if !target_guid.is_player() {
        return 0;
    }

    let healed = world
        .systems
        .player
        .manager()
        .with_player_mut(target_guid, |player| {
            player.stats.modify_health(heal_amount as i32).max(0) as u32
        })
        .unwrap_or(0);

    let overheal = heal_amount.saturating_sub(healed);
    send_spell_heal_log(
        caster_guid,
        target_guid,
        spell_id,
        healed,
        overheal,
        is_crit,
        world,
    );

    if healed > 0 {
        use crate::game::player::auras::proc::{proc_flags, proc_flags_ex};
        let proc_ex = if is_crit {
            proc_flags_ex::CRITICAL_HIT
        } else {
            proc_flags_ex::NORMAL_HIT
        };
        let _ = world
            .systems
            .auras
            .check_procs(
                caster_guid,
                proc_flags::DEAL_HELPFUL_SPELL,
                proc_ex,
                Some(spell_id),
                healed,
                world,
            )
            .await;
        let _ = world
            .systems
            .auras
            .check_procs(
                target_guid,
                proc_flags::TAKE_HELPFUL_SPELL,
                proc_ex,
                Some(spell_id),
                healed,
                world,
            )
            .await;
    }

    healed
}

/// Build and broadcast SMSG_SPELLHEALLOG.
fn send_spell_heal_log(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    spell_id: u32,
    heal_amount: u32,
    overheal: u32,
    is_crit: bool,
    world: &World,
) {
    use oxcore_shared::protocol::{Opcode, WorldPacket};

    let mut packet = WorldPacket::new(Opcode::SMSG_SPELLHEALLOG);
    packet.write_packed_guid_raw(target_guid.raw());
    packet.write_packed_guid_raw(caster_guid.raw());
    packet.write_u32(spell_id);
    packet.write_u32(heal_amount);
    packet.write_u32(overheal);
    packet.write_u8(if is_crit { 1 } else { 0 });
    packet.write_u8(0); // unused

    world
        .managers
        .broadcast_mgr
        .broadcast_nearby_packet(caster_guid, &packet, true);
}

/// Stochastic rounding of a float to an integer (faithful `rand_dither`).
///
/// Preserves fractional expectation: `2.3` rounds down to `2` 70% of the time and up
/// to `3` 30% of the time, rather than always truncating or always rounding.
fn rand_dither(v: f32) -> i32 {
    use rand::Rng;
    let frac = rand::thread_rng().gen_range(0.0..1.0);
    let magnitude = (v.abs() + frac).floor();
    if v < 0.0 {
        -(magnitude as i32)
    } else {
        magnitude as i32
    }
}

/// Extra "done" bonus added to a school-absorb shield's magnitude on real apply.
/// Faithful to the school-absorb handling (apply branch only; nothing happens on
/// remove).
///
/// Certain absorb shields (Power Word: Shield, Frost/Fire Ward, Shadow Ward) get a
/// flat 10% bonus from the caster's +healing (PW:S) or +spell damage (wards) for the
/// aura's school, scaled by the caster's level-downranking penalty and then
/// stochastically rounded (`rand_dither`) to avoid always favoring one side of the
/// fraction. Returns 0 for spells that don't match one of the hardcoded family/mask
/// checks.
pub fn school_absorb_bonus_done(
    caster_guid: ObjectGuid,
    spell_proto: &SpellEntry,
    world: &World,
) -> i32 {
    const CF_PRIEST_POWER_WORD_SHIELD: u64 = 1 << 0;
    const CF_MAGE_FIRE_WARD: u64 = 1 << 3;
    const CF_MAGE_FROST_WARD: u64 = 1 << 8;
    const SPELLFAMILY_PRIEST: u32 = 6;
    const SPELLFAMILY_WARLOCK: u32 = 5;
    const SPELL_ICON_SHADOW_WARD: u32 = 207;
    const CATEGORY_SHADOW_WARD: u32 = 56;

    let school_mask = 1u32 << spell_proto.school;

    let mut done_actual_benefit = if spell_proto.spell_family_name == SPELLFAMILY_PRIEST
        && (spell_proto.spell_family_flags & CF_PRIEST_POWER_WORD_SHIELD) != 0
    {
        // +10% from +healing bonus
        spell_base_healing_bonus_done(school_mask, caster_guid, world) * 0.1
    } else if spell_proto.spell_family_name == SPELLFAMILY_MAGE
        && (spell_proto.spell_family_flags & (CF_MAGE_FIRE_WARD | CF_MAGE_FROST_WARD)) != 0
    {
        // +10% from +spell damage bonus
        spell_base_damage_bonus_done(school_mask, caster_guid, world) as f32 * 0.1
    } else if spell_proto.spell_family_name == SPELLFAMILY_WARLOCK
        && spell_proto.spell_icon_id == SPELL_ICON_SHADOW_WARD
        && spell_proto.category == CATEGORY_SHADOW_WARD
    {
        // +10% from +spell damage bonus
        spell_base_damage_bonus_done(school_mask, caster_guid, world) as f32 * 0.1
    } else {
        0.0
    };

    done_actual_benefit *=
        crate::game::player::spells::effects::calculate_level_penalty(spell_proto.spell_level);

    rand_dither(done_actual_benefit)
}

/// Applies the caster's `ResistMissChance` spell modifiers to a reflect chance value.
///
/// The original implementation calls the spell-modifier apply in-place on the aura's
/// amount, letting talents/items that boost "resist miss chance" scale this aura's own
/// reflect % up or down. Returns the adjusted amount (base_amount + spellmod delta).
pub fn reflect_spells_school_bonus(
    caster_guid: ObjectGuid,
    spell_id: u32,
    base_amount: i32,
    world: &World,
) -> i32 {
    if !caster_guid.is_player() {
        return base_amount;
    }

    let spell_proto = world.managers.spell_mgr.get(spell_id);
    let (family_name, family_flags) = spell_proto
        .as_deref()
        .map(|s| (s.spell_family_name, s.spell_family_flags))
        .unwrap_or((0, 0));

    world
        .systems
        .player
        .manager()
        .with_player(caster_guid, |player| {
            crate::game::player::spells::modifiers::apply_spell_modifiers_to_value(
                &player.spells.spell_modifiers,
                SpellModOp::ResistMissChance,
                base_amount,
                family_name,
                family_flags,
            )
        })
        .unwrap_or(base_amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::game::creature::creature::Creature;
    use crate::game::creature::manager::CreatureTemplate;
    use crate::game::gameobject::gameobject::{GameObject, GameObjectTemplate};
    use crate::game::player::player::Player;
    use crate::World;
    use oxcore_db::database::Databases;
    use oxcore_shared::protocol::{ObjectGuid, Position};
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

    fn creature_template(entry: u32, rank: u8) -> CreatureTemplate {
        CreatureTemplate {
            entry,
            name: format!("Creature{entry}"),
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
            rank,
            gossip_menu_id: 0,
            vendor_id: 0,
            trainer_id: 0,
            trainer_type: 0,
            spells: [0; 4],
            gold_min: 0,
            gold_max: 0,
        }
    }

    fn creature(guid: ObjectGuid, entry: u32, rank: u8) -> Creature {
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
            &creature_template(entry, rank),
            1,
            None,
        )
    }

    fn gameobject(guid: ObjectGuid, level: u32) -> GameObject {
        let template = GameObjectTemplate {
            entry: guid.entry(),
            go_type: 0,
            display_id: 1,
            name: format!("GO{}", guid.entry()),
            icon_name: String::new(),
            cast_bar_caption: String::new(),
            faction: 0,
            flags: 0,
            size: 1.0,
            data: [0; 24],
        };
        let mut go = GameObject::new(
            guid,
            template.entry,
            1,
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                o: 0.0,
            },
            1,
            &template,
            [0.0, 0.0, 0.0, 1.0],
            0,
            0,
        );
        go.level = level;
        go
    }

    fn gameobject_with_template_level(
        guid: ObjectGuid,
        go_type: crate::game::gameobject::GameObjectType,
        level_index: usize,
        level: i32,
    ) -> GameObject {
        let mut data = [0; 24];
        data[level_index] = level;
        let template = GameObjectTemplate {
            entry: guid.entry(),
            go_type: go_type as u32,
            display_id: 1,
            name: format!("GO{}", guid.entry()),
            icon_name: String::new(),
            cast_bar_caption: String::new(),
            faction: 0,
            flags: 0,
            size: 1.0,
            data,
        };
        GameObject::new(
            guid,
            template.entry,
            1,
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                o: 0.0,
            },
            1,
            &template,
            [0.0, 0.0, 0.0, 1.0],
            0,
            0,
        )
    }

    #[tokio::test]
    async fn player_source_returns_player_level() {
        let world = test_world();
        let source = ObjectGuid::new_player(1);
        world
            .managers
            .player_mgr
            .add_player(Player::new(source, "P".into(), 1, 0, 0, 37, 1, 1, 0), 1);

        assert_eq!(get_level_for_target(source, None, &world), 37);
    }

    #[tokio::test]
    async fn creature_source_returns_creature_level() {
        let world = test_world();
        let source = ObjectGuid::new_creature(100, 1);
        world
            .managers
            .creature_mgr
            .add_template(creature_template(100, 0));
        world
            .managers
            .creature_mgr
            .add_creature_for_test(creature(source, 100, 0));

        assert_eq!(get_level_for_target(source, None, &world), 50);
    }

    #[tokio::test]
    async fn world_boss_source_scales_against_unit_targets() {
        let world = test_world();
        let source = ObjectGuid::new_creature(200, 1);
        let target = ObjectGuid::new_player(2);

        world
            .managers
            .creature_mgr
            .add_template(creature_template(200, 3));
        world
            .managers
            .creature_mgr
            .add_creature_for_test(creature(source, 200, 3));
        world
            .managers
            .player_mgr
            .add_player(Player::new(target, "T".into(), 1, 0, 0, 60, 1, 1, 0), 2);

        assert_eq!(get_level_for_target(source, Some(target), &world), 63);
    }

    #[tokio::test]
    async fn gameobject_source_uses_own_level_then_target_then_max_level() {
        let world = test_world();
        let source = ObjectGuid::new_gameobject(300, 1);
        let target = ObjectGuid::new_player(2);

        world
            .managers
            .gameobject_mgr
            .add_gameobject_for_test(gameobject(source, 12));
        world
            .managers
            .player_mgr
            .add_player(Player::new(target, "T".into(), 1, 0, 0, 45, 1, 1, 0), 2);

        assert_eq!(get_level_for_target(source, Some(target), &world), 12);

        world
            .managers
            .gameobject_mgr
            .add_gameobject_for_test(gameobject(source, 0));
        assert_eq!(get_level_for_target(source, Some(target), &world), 45);

        assert_eq!(get_level_for_target(source, None, &world), 60);
    }

    #[tokio::test]
    async fn gameobject_source_uses_chest_and_trap_template_levels() {
        let world = test_world();
        let chest = ObjectGuid::new_gameobject(301, 1);
        let trap = ObjectGuid::new_gameobject(302, 1);

        world
            .managers
            .gameobject_mgr
            .add_gameobject_for_test(gameobject_with_template_level(
                chest,
                crate::game::gameobject::GameObjectType::Chest,
                9,
                18,
            ));
        world
            .managers
            .gameobject_mgr
            .add_gameobject_for_test(gameobject_with_template_level(
                trap,
                crate::game::gameobject::GameObjectType::Trap,
                1,
                24,
            ));

        assert_eq!(get_level_for_target(chest, None, &world), 18);
        assert_eq!(get_level_for_target(trap, None, &world), 24);
    }

    #[tokio::test]
    async fn unknown_source_uses_target_unit_or_max_level() {
        let world = test_world();
        let target = ObjectGuid::new_player(2);
        world
            .managers
            .player_mgr
            .add_player(Player::new(target, "T".into(), 1, 0, 0, 29, 1, 1, 0), 2);

        assert_eq!(
            get_level_for_target(ObjectGuid::new_gameobject(999, 1), Some(target), &world),
            29
        );
        assert_eq!(
            get_level_for_target(ObjectGuid::new_gameobject(999, 1), None, &world),
            60
        );
    }

    #[tokio::test]
    async fn melee_damage_school_mask_is_always_physical() {
        let world = test_world();
        let player = ObjectGuid::new_player(3);

        assert_eq!(
            get_melee_damage_school_mask(player, 0, &world),
            SPELL_SCHOOL_MASK_NORMAL
        );
        assert_eq!(
            get_melee_damage_school_mask(ObjectGuid::new_creature(1, 1), 0, &world),
            SPELL_SCHOOL_MASK_NORMAL
        );
    }
}
