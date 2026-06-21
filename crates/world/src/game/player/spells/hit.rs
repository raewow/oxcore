//! Spell Hit Resolution
//!
//! Determines whether a spell hits, misses, or is resisted.
//! Equivalent to MaNGOS Unit::SpellHitResult() / MagicSpellHitResult().
//!
//! Vanilla WoW spell hit mechanics:
//! - Base miss rate: 4% for same-level targets
//! - +1% per level difference (target higher than caster)
//! - Spell hit rating from gear/talents reduces miss chance
//! - Binary spells: full resist or full land
//! - Non-binary spells: partial resist (0/25/50/75/100%)
//! - Physical spells use melee miss table instead

use crate::game::common::creature_flags::CREATURE_STATIC_FLAG_NO_DEFENSE;
use crate::game::player::spells::caster::get_level_for_target;
use crate::game::player::skills::{SkillSaveState, SKILL_DEFENSE};
use crate::World;
use oxcore_shared::game::inventory::{EquipmentSlot, INVENTORY_SLOT_BAG_0};
use oxcore_shared::protocol::ObjectGuid;

/// Result of a spell hit roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellHitOutcome {
    /// Spell hits normally
    Hit,
    /// Spell misses entirely
    Miss,
    /// Full resist (binary spells)
    Resist,
    /// Partial resist: percentage of damage resisted (25, 50, or 75)
    PartialResist(u8),
    /// Target is immune to the spell's school or mechanic
    Immune,
    /// Spell reflected back at caster
    Reflect,
}

impl SpellHitOutcome {
    pub fn is_hit(&self) -> bool {
        matches!(self, Self::Hit | Self::PartialResist(_))
    }
}

/// Wire-level miss reason sent in `SMSG_SPELLLOGMISS` (client `SpellMissInfo` enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpellMissInfo {
    None = 0,
    Miss = 1,
    Resist = 2,
    Dodge = 3,
    Parry = 4,
    Block = 5,
    Evade = 6,
    Immune = 7,
    Immune2 = 8,
    Deflect = 9,
    Absorb = 10,
    Reflect = 11,
}

/// Spell damage classes (SpellEntry.dmg_class), used to route the hit roll.
const SPELL_DAMAGE_CLASS_NONE: u32 = 0;
const SPELL_DAMAGE_CLASS_MELEE: u32 = 2;
const SPELL_DAMAGE_CLASS_RANGED: u32 = 3;

/// Magic spell hit chance as a percentage in [1, 99] (MaNGOS `MagicSpellHitChance`).
///
/// Base hit by level difference: 96% at equal level, −1%/level up to +2, then a steeper
/// −`lchance`/level (7 vs players, 11 vs creatures). Floored at 22% before adding the
/// caster's spell-hit bonus. For binary spells the average resist is folded into the hit
/// chance. Returns the final hit percentage (the caller derives miss = 100 − hit).
fn magic_hit_chance_pct(
    caster_level: i32,
    target_level: i32,
    target_is_player: bool,
    spell_hit_bonus: i32,
    binary_resist_avg: Option<f32>,
) -> f32 {
    let leveldif = target_level - caster_level;
    let lchance = if target_is_player { 7 } else { 11 };

    let mut mod_hit = if leveldif < 3 {
        96.0 - leveldif as f32
    } else {
        94.0 - (leveldif - 2) as f32 * lchance as f32
    };

    // Miss chance from level difference is capped (classic test: lvl1 vs lvl60 → ~22% hit).
    if mod_hit < 22.0 {
        mod_hit = 22.0;
    }

    // Caster spell-hit bonus from gear/talents/auras.
    mod_hit += spell_hit_bonus as f32;

    // Binary spells fold the average resist into the hit chance.
    if let Some(resist_avg) = binary_resist_avg {
        mod_hit *= 1.0 - resist_avg;
    }

    mod_hit.clamp(1.0, 99.0)
}

/// Average resist fraction in [0, 0.75] (MaNGOS `GetSpellResistChance`, player-vs-target
/// case without innate level resist or spell penetration): `resistance * 0.15 / caster_level`.
fn average_resist_fraction(caster_level: i32, resistance: u32, school: u8) -> f32 {
    if school == 0 || resistance == 0 || caster_level <= 0 {
        return 0.0;
    }
    (resistance as f32 * 0.15 / caster_level as f32).clamp(0.0, 0.75)
}

/// Melee/ranged spell miss chance percent (MaNGOS `MeleeSpellMissChance`).
///
/// `skill_diff` = attacker weapon skill − victim defense skill (positive favours the attacker).
/// Base 5% miss, adjusted by skill difference (PvP 0.04/pt; PvE 0.1/pt, or 0.2/pt when the
/// attacker is 10+ skill below), reduced by the attacker's +hit, clamped to [0, 60].
fn melee_spell_miss_chance(
    victim_is_player: bool,
    victim_level: u8,
    skill_diff: i32,
    hit_bonus: f32,
) -> f32 {
    let mut miss = if victim_is_player {
        5.0 - skill_diff as f32 * 0.04
    } else if skill_diff < -10 {
        5.0 - skill_diff as f32 * 0.2
    } else {
        5.0 - skill_diff as f32 * 0.1
    };

    // Low-level creature reduction.
    if !victim_is_player && victim_level < 10 {
        miss *= victim_level as f32 / 10.0;
    }

    // The first 1% of +hit is ignored against targets 10+ defense above the attacker.
    let mut hit = hit_bonus;
    if skill_diff < -10 && hit > 0.0 {
        hit -= 1.0;
    }
    miss -= hit;

    miss.clamp(0.0, 60.0)
}

fn unit_melee_skill(level: u8) -> u16 {
    level as u16 * 5
}

fn weapon_skill_from_subclass(item_subclass: u32) -> Option<u16> {
    use crate::game::player::skills::{
        SKILL_2H_AXES, SKILL_2H_MACES, SKILL_2H_SWORDS, SKILL_AXES, SKILL_BOWS, SKILL_CROSSBOWS,
        SKILL_DAGGERS, SKILL_FIST_WEAPONS, SKILL_GUNS, SKILL_MACES, SKILL_POLEARMS, SKILL_STAVES,
        SKILL_SWORDS, SKILL_THROWN, SKILL_WANDS,
    };

    match item_subclass {
        0 => Some(SKILL_AXES),
        1 => Some(SKILL_2H_AXES),
        2 => Some(SKILL_BOWS),
        3 => Some(SKILL_GUNS),
        4 => Some(SKILL_MACES),
        5 => Some(SKILL_2H_MACES),
        6 => Some(SKILL_POLEARMS),
        7 => Some(SKILL_SWORDS),
        8 => Some(SKILL_2H_SWORDS),
        10 => Some(SKILL_STAVES),
        13 => Some(SKILL_FIST_WEAPONS),
        15 => Some(SKILL_DAGGERS),
        16 => Some(SKILL_THROWN),
        18 => Some(SKILL_CROSSBOWS),
        19 => Some(SKILL_WANDS),
        _ => None,
    }
}

fn equipped_weapon_skill_value(
    guid: ObjectGuid,
    slot: EquipmentSlot,
    world: &World,
) -> Option<u16> {
    let item_guid = world
        .systems
        .inventory
        .get_item_at(guid, INVENTORY_SLOT_BAG_0, slot as u8)?;
    let item = world.systems.inventory.cache().get_item(guid, item_guid)?;
    let entry = item.read().entry;
    let template = world.managers.item_mgr.get_template(entry)?;

    if template.item_class != 2 {
        return None;
    }

    let skill_id = weapon_skill_from_subclass(template.item_subclass)?;
    world
        .systems
        .player
        .manager()
        .with_player(guid, |player| {
            player
                .skills
                .skills
                .get(&skill_id)
                .filter(|skill| skill.state != SkillSaveState::Deleted)
                .map(|skill| skill.current_value)
        })
        .flatten()
}

fn player_defense_skill_value(
    skill: Option<&crate::game::player::skills::SkillData>,
    target_is_player: bool,
) -> u16 {
    let Some(skill) = skill.filter(|skill| skill.state != SkillSaveState::Deleted) else {
        return 0;
    };

    if target_is_player {
        skill.max_value
    } else {
        skill.current_value
    }
}

/// Defense skill for melee-based spell hit checks.
fn get_defense_skill_value(
    defender_guid: ObjectGuid,
    attacker_guid: Option<ObjectGuid>,
    world: &World,
) -> u16 {
    if defender_guid.is_player() {
        let target_is_player = attacker_guid.is_some_and(|guid| guid.is_player());
        return world
            .managers
            .player_mgr
            .with_player(defender_guid, |player| {
                player_defense_skill_value(
                    player.skills.skills.get(&SKILL_DEFENSE),
                    target_is_player,
                )
            })
            .unwrap_or(0);
    }

    world
        .managers
        .creature_mgr
        .with_creature(defender_guid, |creature| {
            if creature.static_flags1 & CREATURE_STATIC_FLAG_NO_DEFENSE != 0 {
                0
            } else {
                unit_melee_skill(creature.level)
            }
        })
        .unwrap_or(0)
}

/// Melee/ranged spell hit roll (MaNGOS `MeleeSpellHitResult`): a single-roll table of
/// miss → dodge → parry → block. Crit is rolled separately during damage. Dodge/parry/block
/// are read from real player victims; creature avoidance is not modelled yet (treated as 0),
/// so creature victims face only the skill-based miss. Outcomes collapse to Hit/Miss because
/// SpellHitOutcome has no dodge/parry/block variants (not surfaced to the client yet).
fn roll_melee_spell_hit(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    spell: &crate::dbc::structures::SpellEntry,
    world: &World,
) -> SpellHitOutcome {
    let is_ranged = spell.dmg_class == SPELL_DAMAGE_CLASS_RANGED;

    // Caster level + melee/ranged +hit from auras.
    let (_caster_level, hit_bonus) = if caster_guid.is_player() {
        world
            .managers
            .player_mgr
            .with_player(caster_guid, |p| {
                use crate::game::player::auras::effects::AURA_MOD_HIT_CHANCE;
                let mut bonus = 0i32;
                for aura in p.auras.container.all_auras() {
                    if aura.aura_type == AURA_MOD_HIT_CHANCE {
                        bonus += aura.current_value();
                    }
                }
                (p.level as i32, bonus as f32)
            })
            .unwrap_or((60, 0.0))
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(caster_guid, |c| (c.level as i32, 0.0f32))
            .unwrap_or((60, 0.0))
    };

    let caster_level_for_target = get_level_for_target(caster_guid, Some(target_guid), world) as i32;

    // Victim level + avoidance (real for players, unmodelled for creatures).
    let victim_level_for_hit = get_level_for_target(target_guid, Some(caster_guid), world) as i32;
    let target_is_player = target_guid.is_player();
    let (victim_level, dodge, parry, block, can_parry, can_block) = if target_is_player {
        world
            .managers
            .player_mgr
            .with_player(target_guid, |p| {
                (
                    victim_level_for_hit,
                    p.stats.dodge_pct,
                    p.stats.parry_pct,
                    p.stats.block_pct,
                    p.combat.can_parry,
                    p.combat.can_block,
                )
            })
            .unwrap_or((60, 0.0, 0.0, 0.0, false, false))
    } else {
        let lvl = world
            .managers
            .creature_mgr
            .with_creature(target_guid, |_| victim_level_for_hit)
            .unwrap_or(60);
        (lvl, 0.0, 0.0, 0.0, false, false)
    };

    let attacker_weapon_skill = if caster_guid.is_player() {
        let weapon_skill = equipped_weapon_skill_value(
            caster_guid,
            if is_ranged {
                EquipmentSlot::Ranged
            } else {
                EquipmentSlot::Mainhand
            },
            world,
        )
        .map(i32::from)
        .unwrap_or_else(|| caster_level_for_target * 5);
        weapon_skill
    } else {
        caster_level_for_target * 5
    };
    let defender_defense_skill = get_defense_skill_value(target_guid, Some(caster_guid), world);
    let skill_diff = attacker_weapon_skill - defender_defense_skill as i32;

    let miss = melee_spell_miss_chance(target_is_player, victim_level as u8, skill_diff, hit_bonus);

    let roll: f32 = rand::random::<f32>() * 100.0;
    let mut threshold = miss;
    if roll < threshold {
        return SpellHitOutcome::Miss;
    }

    // SPELL_ATTR_NO_ACTIVE_DEFENSE (0x00200000): cannot be dodged/parried/blocked.
    if spell.attributes & 0x0020_0000 != 0 {
        return SpellHitOutcome::Hit;
    }
    // Ranged spells cannot be dodged/parried/blocked.
    if is_ranged {
        return SpellHitOutcome::Hit;
    }

    // Dodge: avoidance reduced by skill difference (0.04/pt vs players, 0.10/pt vs creatures).
    let dodge_mod = if target_is_player {
        skill_diff as f32 * 0.04
    } else {
        skill_diff as f32 * 0.10
    };
    let mut dodge_chance = (dodge - dodge_mod).max(0.0);
    if !target_is_player && (victim_level as u8) < 10 {
        dodge_chance *= victim_level as f32 / 10.0;
    }
    threshold += dodge_chance;
    if roll < threshold {
        return SpellHitOutcome::Miss; // dodged
    }

    // Parry.
    if can_parry {
        let parry_mod = if target_is_player {
            skill_diff as f32 * 0.04
        } else if skill_diff < -10 {
            skill_diff as f32 * 0.60
        } else {
            skill_diff as f32 * 0.20
        };
        let mut parry_chance = (parry - parry_mod).max(0.0);
        if !target_is_player && (victim_level as u8) < 10 {
            parry_chance *= victim_level as f32 / 10.0;
        }
        threshold += parry_chance;
        if roll < threshold {
            return SpellHitOutcome::Miss; // parried
        }
    }

    // Full block only applies to spells flagged SPELL_ATTR_EX3_COMPLETELY_BLOCKED (0x08);
    // other spells take a partial block during damage calculation instead.
    if can_block && spell.attributes_ex3 & 0x0000_0008 != 0 {
        threshold += block.max(0.0);
        if roll < threshold {
            return SpellHitOutcome::Miss; // fully blocked
        }
    }

    SpellHitOutcome::Hit
}

/// Roll spell hit for a target.
///
/// Returns the outcome (hit, miss, resist, immune, reflect).
/// Routes by the spell's damage class (MaNGOS `SpellHitResult`): NONE never misses,
/// MELEE/RANGED use the melee table (not yet ported — treated as a hit), MAGIC uses the
/// level-based hit roll plus resistance.
pub fn roll_spell_hit(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    spell_id: u32,
    world: &World,
) -> SpellHitOutcome {
    let spell_entry = match world.managers.spell_mgr.get(spell_id) {
        Some(entry) => entry,
        None => return SpellHitOutcome::Hit, // Unknown spell = auto-hit
    };

    // Self-cast and positive spells can never miss (MaNGOS SpellHitResult early returns).
    if caster_guid == target_guid || spell_entry.is_positive_spell() {
        return SpellHitOutcome::Hit;
    }

    // Route by damage class. NONE never misses; melee/ranged use the (un-ported) melee
    // table — MeleeSpellHitResult (dodge/parry/block) is a separate task, so auto-hit here.
    match spell_entry.dmg_class {
        SPELL_DAMAGE_CLASS_NONE => return SpellHitOutcome::Hit,
        SPELL_DAMAGE_CLASS_MELEE | SPELL_DAMAGE_CLASS_RANGED => {
            return roll_melee_spell_hit(caster_guid, target_guid, &spell_entry, world)
        }
        _ => {}
    }

    let school = spell_entry.school as u8;

    // Get caster level and spell hit bonus
    let (caster_level, spell_hit_bonus) = if caster_guid.is_player() {
        world
            .managers
            .player_mgr
            .with_player(caster_guid, |p| {
                // Sum up hit chance from auras (AURA_MOD_SPELL_HIT_CHANCE + AURA_MOD_HIT_CHANCE)
                use crate::game::player::auras::effects::{
                    AURA_MOD_HIT_CHANCE, AURA_MOD_SPELL_HIT_CHANCE,
                };
                let mut hit_bonus = 0i32;
                for aura in p.auras.container.all_auras() {
                    if aura.aura_type == AURA_MOD_SPELL_HIT_CHANCE
                        || aura.aura_type == AURA_MOD_HIT_CHANCE
                    {
                        hit_bonus += aura.current_value();
                    }
                }
                (p.level as i32, hit_bonus)
            })
            .unwrap_or((60, 0))
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(caster_guid, |c| (c.level as i32, 0i32))
            .unwrap_or((60, 0))
    };

    let caster_level_for_target = get_level_for_target(caster_guid, Some(target_guid), world) as i32;

    // Get target level and resistances
    let target_level = get_level_for_target(target_guid, Some(caster_guid), world) as i32;
    let target_resistance = if target_guid.is_player() {
        world
            .managers
            .player_mgr
            .with_player(target_guid, |p| {
                if school != 0 && (school as usize) < 7 {
                    p.stats.resistances[school as usize]
                } else {
                    0
                }
            })
            .unwrap_or(0)
    } else {
        0u32 // TODO: creature resistances
    };

    // Step 1: Hit roll (level-based, MagicSpellHitChance). Binary spells fold the average
    // resist into the hit chance; non-binary spells roll partial resist on the damage after.
    let is_binary = school != 0 && is_binary_spell(spell_id, world);
    let resist_avg = average_resist_fraction(caster_level, target_resistance, school);
    let binary_resist_avg = if is_binary { Some(resist_avg) } else { None };

    let hit_pct = magic_hit_chance_pct(
        caster_level_for_target,
        target_level,
        target_guid.is_player(),
        spell_hit_bonus,
        binary_resist_avg,
    );

    let roll: f32 = rand::random::<f32>() * 100.0;
    if roll >= hit_pct {
        // A binary spell that fails its (resist-folded) roll is a full resist; otherwise a miss.
        return if is_binary {
            SpellHitOutcome::Resist
        } else {
            SpellHitOutcome::Miss
        };
    }

    // Step 2: Non-binary magic spells roll partial resist on the landed hit (0/25/50/75/100%).
    if school != 0 && !is_binary {
        let partial = roll_partial_resist(caster_level, target_resistance, school);
        if partial == 100 {
            return SpellHitOutcome::Resist;
        }
        if partial > 0 {
            return SpellHitOutcome::PartialResist(partial);
        }
    }

    SpellHitOutcome::Hit
}

/// Check if a spell is "binary" (all-or-nothing resist).
///
/// Binary spells are those with non-damage effects that either fully apply or
/// fully resist (like Polymorph, Fear, Silence). Damage-only spells use
/// partial resist.
fn is_binary_spell(spell_id: u32, world: &World) -> bool {
    let spell_entry = match world.managers.spell_mgr.get(spell_id) {
        Some(entry) => entry,
        None => return false,
    };

    // A spell is binary if ALL its effects are non-damage aura effects
    // (Damage spells use partial resist instead)
    for i in 0..3 {
        let effect = spell_entry.effect[i];
        if effect == 0 {
            continue;
        }
        // School damage (2), weapon damage (58, 17, 31, 121), health leech (9) = NOT binary
        if matches!(effect, 2 | 9 | 17 | 31 | 58 | 121) {
            return false;
        }
    }

    true
}

/// Roll for partial resistance.
/// Returns the percentage of damage resisted (0, 25, 50, 75, or 100).
///
/// Centered on the average resist fraction (`GetSpellResistChance`), distributed across the
/// five possible outcomes using a weighted table.
fn roll_partial_resist(caster_level: i32, resistance: u32, school: u8) -> u8 {
    let avg_resist = average_resist_fraction(caster_level, resistance, school);

    // Simplified distribution: roll against average resist
    let roll: f32 = rand::random::<f32>();

    if avg_resist < 0.01 {
        return 0; // Negligible resistance
    }

    // Weight table based on average resistance
    // Higher resistance shifts distribution toward more resist
    let threshold_100 = avg_resist * avg_resist; // Full resist very rare at low levels
    let threshold_75 = avg_resist * 0.5;
    let threshold_50 = avg_resist * 0.8;
    let threshold_25 = avg_resist * 1.2;

    if roll < threshold_100 {
        100
    } else if roll < threshold_100 + threshold_75 {
        75
    } else if roll < threshold_100 + threshold_75 + threshold_50 {
        50
    } else if roll < threshold_100 + threshold_75 + threshold_50 + threshold_25 {
        25
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_level_hit_is_96_percent() {
        // 96% base hit (4% miss) at equal level, no bonus, non-binary.
        let hit = magic_hit_chance_pct(60, 60, false, 0, None);
        assert!((hit - 96.0).abs() < 0.001, "got {hit}");
    }

    #[test]
    fn each_level_above_costs_one_percent_up_to_two() {
        assert!((magic_hit_chance_pct(60, 61, false, 0, None) - 95.0).abs() < 0.001);
        assert!((magic_hit_chance_pct(60, 62, false, 0, None) - 94.0).abs() < 0.001);
    }

    #[test]
    fn pve_level_gap_beyond_two_is_steeper_than_pvp() {
        // leveldif = 5: PvE uses 94 - 3*11 = 61; PvP uses 94 - 3*7 = 73.
        let pve = magic_hit_chance_pct(60, 65, false, 0, None);
        let pvp = magic_hit_chance_pct(60, 65, true, 0, None);
        assert!((pve - 61.0).abs() < 0.001, "pve {pve}");
        assert!((pvp - 73.0).abs() < 0.001, "pvp {pvp}");
        assert!(pve < pvp, "PvE level gaps should miss more than PvP");
    }

    #[test]
    fn huge_level_gap_floors_at_22_before_bonus() {
        // lvl 1 vs lvl 60 creature: floor 22 applies before the (zero) hit bonus.
        let hit = magic_hit_chance_pct(1, 60, false, 0, None);
        assert!((hit - 22.0).abs() < 0.001, "got {hit}");
    }

    #[test]
    fn hit_chance_is_clamped_to_99() {
        // A large hit bonus cannot push hit chance above 99%.
        let hit = magic_hit_chance_pct(60, 60, false, 50, None);
        assert!((hit - 99.0).abs() < 0.001, "got {hit}");
    }

    #[test]
    fn binary_folds_average_resist_into_hit_chance() {
        // 96% base * (1 - 0.25 resist) = 72%.
        let hit = magic_hit_chance_pct(60, 60, false, 0, Some(0.25));
        assert!((hit - 72.0).abs() < 0.001, "got {hit}");
    }

    #[test]
    fn melee_miss_base_is_5_percent_at_equal_skill() {
        assert!((melee_spell_miss_chance(true, 60, 0, 0.0) - 5.0).abs() < 0.001);
        assert!((melee_spell_miss_chance(false, 60, 0, 0.0) - 5.0).abs() < 0.001);
    }

    #[test]
    fn melee_miss_pve_skill_gap_is_steeper_when_far_below() {
        // Attacker 15 skill below a creature: -15 < -10 → 5 - (-15)*0.2 = 8%.
        assert!((melee_spell_miss_chance(false, 60, -15, 0.0) - 8.0).abs() < 0.001);
        // Attacker 5 skill below a creature: 5 - (-5)*0.1 = 5.5%.
        assert!((melee_spell_miss_chance(false, 60, -5, 0.0) - 5.5).abs() < 0.001);
    }

    #[test]
    fn melee_miss_pvp_uses_smaller_skill_factor() {
        // vs player: 5 - (-15)*0.04 = 5.6% (much less than the 8% PvE case).
        assert!((melee_spell_miss_chance(true, 60, -15, 0.0) - 5.6).abs() < 0.001);
    }

    #[test]
    fn melee_miss_reduced_by_hit_and_clamped() {
        // +10 hit on a 5% base → floored at 0.
        assert_eq!(melee_spell_miss_chance(true, 60, 0, 10.0), 0.0);
        // Huge skill deficit vs creature is capped at 60%.
        assert_eq!(melee_spell_miss_chance(false, 60, -1000, 0.0), 60.0);
    }

    #[test]
    fn melee_miss_low_level_creature_reduction() {
        // A level-5 creature halves its miss chance (5 * 5/10 = 2.5%).
        assert!((melee_spell_miss_chance(false, 5, 0, 0.0) - 2.5).abs() < 0.001);
    }

    #[test]
    fn weapon_skill_subclass_mapping_matches_weapon_skills() {
        assert_eq!(weapon_skill_from_subclass(0), Some(crate::game::player::skills::SKILL_AXES));
        assert_eq!(weapon_skill_from_subclass(19), Some(crate::game::player::skills::SKILL_WANDS));
        assert_eq!(weapon_skill_from_subclass(99), None);
    }

    #[test]
    fn average_resist_matches_mangos_formula() {
        // resistance * 0.15 / level: 200 res at lvl 60 → 0.5; physical school exempt.
        assert!((average_resist_fraction(60, 200, 1) - 0.5).abs() < 0.001);
        assert_eq!(average_resist_fraction(60, 200, 0), 0.0); // physical exempt
        assert_eq!(average_resist_fraction(60, 0, 1), 0.0); // no resistance
                                                            // Clamped at 0.75.
        assert!((average_resist_fraction(60, 1000, 1) - 0.75).abs() < 0.001);
    }
}
