//! Spell Modifiers
//!
//! Spell modifiers come from talents (SPELL_AURA_ADD_FLAT_MODIFIER / ADD_PCT_MODIFIER)
//! and some auras. They modify properties of spells the player casts.

use crate::dbc::structures::SpellEntry;
use crate::game::player::spells::state::{SpellMod, SpellModOp, SpellModType};
use crate::World;
use anyhow::Result;
use oxcore_shared::protocol::ObjectGuid;

/// Spell attribute: damage/cost scales with creature level (`SPELL_ATTR_SCALES_WITH_CREATURE_LEVEL`).
const SPELL_ATTR_SCALES_WITH_CREATURE_LEVEL: u32 = 0x0008_0000;
/// AttributesEx: spell drains the caster's entire power pool (`SPELL_ATTR_EX_USE_ALL_MANA`).
const SPELL_ATTR_EX_USE_ALL_MANA: u32 = 0x0000_0002;
/// `POWER_HEALTH` sentinel value as stored in `SpellEntry::power_type` (-2 as unsigned).
const POWER_HEALTH: u32 = 0xFFFF_FFFE;
/// Number of real power types (mana, rage, focus, energy, happiness).
const MAX_POWERS: u32 = 5;

/// Caster state needed to compute a spell's power cost.
///
/// Mirrors the `Unit*` getters read by C++ `Spell::CalculatePowerCost`.
pub struct PowerCostContext {
    /// `caster->GetHealth()` — used by USE_ALL_MANA health spells.
    pub health: u32,
    /// `caster->GetCreateHealth()` — base (create) health for percentage-of-health costs.
    pub create_health: u32,
    /// `caster->GetCreateMana()` — base (create) mana for percentage-of-mana costs.
    pub create_mana: u32,
    /// `caster->GetPower(powerType)` — current pool for USE_ALL_MANA non-health spells.
    pub current_power: u32,
    /// `caster->GetMaxPower(powerType)` — max pool for percentage costs on rage/focus/energy/happiness.
    pub max_power: u32,
    /// `caster->GetLevel()` — used by creature-level scaling.
    pub level: u32,
    /// `caster->GetSpellRank(spellInfo)` — drives the per-level cost term.
    pub spell_rank: u32,
}

/// Calculate the power cost of a spell.
///
/// Faithful port of C++ `Spell::CalculatePowerCost`. Returns the amount of the
/// spell's `power_type` that the cast will consume.
///
/// `is_item_cast` is true when the spell is cast from an item (charges/use), which
/// always costs no power. `cost_modifiers` are the caster's active spell modifiers,
/// from which `SpellModOp::Cost` entries are applied (`SPELLMOD_COST`).
///
/// The school-indexed percentage multiplier defaults to zero and is supplied by
/// [`calculate_power_cost_with_school_multiplier`] when active auras affect the spell school.
pub fn calculate_power_cost(
    spell: &SpellEntry,
    ctx: &PowerCostContext,
    is_item_cast: bool,
    cost_modifiers: &[SpellMod],
) -> u32 {
    calculate_power_cost_with_school_multiplier(spell, ctx, is_item_cast, cost_modifiers, 0)
}

/// Calculate spell cost with the total `AURA_MOD_POWER_COST_PCT` amount for its school.
pub fn calculate_power_cost_with_school_multiplier(
    spell: &SpellEntry,
    ctx: &PowerCostContext,
    is_item_cast: bool,
    cost_modifiers: &[SpellMod],
    school_cost_pct: i32,
) -> u32 {
    // Item casts use no power.
    if is_item_cast {
        return 0;
    }

    let power_type = spell.power_type;

    // Drain-all spells (e.g. Lay on Hands): cost is the entire current pool.
    if spell.attributes_ex & SPELL_ATTR_EX_USE_ALL_MANA != 0 {
        if power_type == POWER_HEALTH {
            return ctx.health;
        }
        if power_type < MAX_POWERS {
            return ctx.current_power;
        }
        // Unknown power type — no cost.
        return 0;
    }

    // Base cost: flat cost + per-level scaling.
    let per_level =
        spell.mana_cost_per_level as i32 * (ctx.spell_rank as i32 / 5 - spell.base_level as i32);
    let mut power_cost = spell.mana_cost as i32 + per_level;

    // Percentage cost from create/max pools.
    if spell.mana_cost_percentage != 0 {
        let pct = spell.mana_cost_percentage as i32;
        match power_type {
            POWER_HEALTH => power_cost += pct * ctx.create_health as i32 / 100,
            0 => power_cost += pct * ctx.create_mana as i32 / 100, // POWER_MANA
            1 | 2 | 3 | 4 => power_cost += pct * ctx.max_power as i32 / 100, // RAGE/FOCUS/ENERGY/HAPPINESS
            _ => return 0,                                                   // unknown power type
        }
    }

    // SPELLMOD_COST from talents/auras. (School-indexed flat unit-mod deferred.)
    power_cost = apply_spell_modifiers_to_value(
        cost_modifiers,
        SpellModOp::Cost,
        power_cost,
        spell.spell_family_name,
        spell.spell_family_flags,
    );

    // Creature-level scaling (mob spells whose cost shrinks with level).
    if spell.attributes & SPELL_ATTR_SCALES_WITH_CREATURE_LEVEL != 0 && ctx.level > 0 {
        let denom = 1.117_f32 * spell.spell_level as f32 / ctx.level as f32 - 0.1327_f32;
        if denom != 0.0 {
            power_cost = (power_cost as f32 / denom) as i32;
        }
    }

    // `UNIT_FIELD_POWER_COST_MULTIPLIER` stores the active aura total divided by 100;
    // Spell::CalculatePowerCost applies it as `1 + multiplier`.
    power_cost = (power_cost as f32 * (1.0 + school_cost_pct as f32 / 100.0)) as i32;

    power_cost.max(0) as u32
}

/// Add a spell modifier (from a talent or aura being applied).
///
/// Called by AuraSystem when applying SPELL_AURA_ADD_FLAT_MODIFIER (107)
/// or SPELL_AURA_ADD_PCT_MODIFIER (108) auras.
///
/// Parameters:
/// - `op`: Which property to modify (from spell DBC effect_misc_value)
/// - `mod_type`: Flat or Pct (from aura type: 107=Flat, 108=Pct)
/// - `value`: Modifier value (from aura current_value)
/// - `spell_family_mask`: Which spells are affected (from spell DBC spell_family_flags)
/// - `spell_family_name`: Which spell family (from spell DBC spell_family_name)
/// - `source_spell_id`: The talent/aura spell providing this modifier
/// - `source_aura_slot`: The aura slot for removal tracking
///
/// `spell_family_mask` is the source spell effect's class mask
/// (`EffectItemType[effect_index]`), matching `SpellModifier` construction.
pub fn add_spell_modifier(
    player_guid: ObjectGuid,
    op: SpellModOp,
    mod_type: SpellModType,
    value: i32,
    spell_family_mask: u64,
    spell_family_name: u32,
    source_spell_id: u32,
    source_aura_slot: Option<u8>,
    world: &World,
) -> Result<()> {
    world
        .systems
        .player
        .manager()
        .with_player_mut(player_guid, |player| {
            player.spells.spell_modifiers.push(SpellMod {
                op,
                mod_type,
                value,
                spell_family_mask,
                spell_family_name,
                source_spell_id,
                source_aura_slot,
            });
        });

    Ok(())
}

/// Remove all spell modifiers from a specific source spell.
///
/// Called by AuraSystem when removing a talent/aura that provided spell modifiers.
pub fn remove_spell_modifier(
    player_guid: ObjectGuid,
    source_spell_id: u32,
    world: &World,
) -> Result<()> {
    world
        .systems
        .player
        .manager()
        .with_player_mut(player_guid, |player| {
            player
                .spells
                .spell_modifiers
                .retain(|m| m.source_spell_id != source_spell_id);
        });

    Ok(())
}

pub fn spell_affect_mask(effect_item_types: &[u64; 3], effect_index: u8) -> u64 {
    effect_item_types
        .get(effect_index as usize)
        .copied()
        .unwrap_or(0)
}

/// Return whether this cast has already applied this exact modifier instance.
///
/// Modifier values can be structurally equal while originating from distinct
/// auras, so membership intentionally uses reference identity.
pub fn has_modifier_applied(applied_modifiers: &[&SpellMod], modifier: &SpellMod) -> bool {
    applied_modifiers
        .iter()
        .any(|applied| std::ptr::eq(*applied, modifier))
}

/// Apply all matching spell modifiers to a value.
///
/// Used during spell calculations to get the modified value of a property.
/// For example, to get modified damage:
///   let damage = apply_spell_modifiers(player, SpellModOp::Damage, base_damage, spell_entry);
pub fn apply_spell_modifiers_to_value(
    modifiers: &[SpellMod],
    op: SpellModOp,
    base_value: i32,
    spell_family_name: u32,
    spell_family_flags: u64,
) -> i32 {
    let mut flat_total = 0i32;
    let mut pct_total = 0i32;

    for spell_mod in modifiers {
        if spell_mod.op != op {
            continue;
        }

        // Check if this modifier applies to the spell
        if !does_modifier_apply(spell_mod, spell_family_name, spell_family_flags) {
            continue;
        }

        match spell_mod.mod_type {
            SpellModType::Flat => {
                flat_total += spell_mod.value;
            }
            SpellModType::Pct => {
                pct_total += spell_mod.value;
            }
        }
    }

    // Apply flat first, then percentage
    let after_flat = base_value + flat_total;
    let after_pct = (after_flat as f32 * (1.0 + pct_total as f32 / 100.0)) as i32;

    after_pct.max(0)
}

/// f32 variant of `apply_spell_modifiers_to_value` for damage calculations.
/// Used by `melee_damage_bonus_done` and similar floating-point pipelines.
pub fn apply_spell_modifiers_to_value_f32(
    modifiers: &[SpellMod],
    op: SpellModOp,
    base_value: f32,
    spell_family_name: u32,
    spell_family_flags: u64,
) -> f32 {
    let mut flat_total = 0i32;
    let mut pct_total = 0i32;

    for spell_mod in modifiers {
        if spell_mod.op != op {
            continue;
        }

        if !does_modifier_apply(spell_mod, spell_family_name, spell_family_flags) {
            continue;
        }

        match spell_mod.mod_type {
            SpellModType::Flat => {
                flat_total += spell_mod.value;
            }
            SpellModType::Pct => {
                pct_total += spell_mod.value;
            }
        }
    }

    let after_flat = base_value + flat_total as f32;
    let after_pct = after_flat * (1.0 + pct_total as f32 / 100.0);

    after_pct.max(0.0)
}

/// Check if a spell modifier applies to a specific spell.
fn does_modifier_apply(
    spell_mod: &SpellMod,
    spell_family_name: u32,
    spell_family_flags: u64,
) -> bool {
    // A zero family is the generic family, not a wildcard.
    if spell_mod.spell_family_name != spell_family_name {
        return false;
    }

    // Must match spell family flags mask
    if spell_mod.spell_family_mask != 0 && (spell_mod.spell_family_mask & spell_family_flags) == 0 {
        return false;
    }

    true
}

/// Calculate modified cast time for a spell.
///
/// Applies SPELLMOD_CASTTIME flat/percentage modifiers from talents and auras,
/// respecting the spell family name/flags filter.
pub fn calculate_modified_cast_time(
    player_guid: ObjectGuid,
    base_cast_time_ms: u32,
    spell_family_name: u32,
    spell_family_flags: u64,
    world: &World,
) -> u32 {
    let mut modified = base_cast_time_ms as i32;

    world
        .systems
        .player
        .manager()
        .with_player(player_guid, |player| {
            modified = apply_spell_modifiers_to_value(
                &player.spells.spell_modifiers,
                SpellModOp::CastTime,
                modified,
                spell_family_name,
                spell_family_flags,
            );
        });

    modified.max(0) as u32
}

/// Calculate modified cooldown for a spell.
///
/// Applies SPELLMOD_COOLDOWN flat/percentage modifiers from talents and auras,
/// respecting the spell family name/flags filter.
pub fn calculate_modified_cooldown(
    player_guid: ObjectGuid,
    base_cooldown_ms: u32,
    spell_family_name: u32,
    spell_family_flags: u64,
    world: &World,
) -> u32 {
    let mut modified = base_cooldown_ms as i32;

    world
        .systems
        .player
        .manager()
        .with_player(player_guid, |player| {
            modified = apply_spell_modifiers_to_value(
                &player.spells.spell_modifiers,
                SpellModOp::Cooldown,
                modified,
                spell_family_name,
                spell_family_flags,
            );
        });

    modified.max(0) as u32
}

/// Calculate modified GCD for a spell.
///
/// Applies SPELLMOD_GLOBAL_COOLDOWN flat/percentage modifiers from talents and
/// auras, respecting the spell family name/flags filter. The caller clamps the
/// hasted 1.5s GCD to [1000, 1500] separately.
pub fn calculate_modified_gcd(
    player_guid: ObjectGuid,
    base_gcd_ms: u32,
    spell_family_name: u32,
    spell_family_flags: u64,
    world: &World,
) -> u32 {
    let mut modified = base_gcd_ms as i32;

    world
        .systems
        .player
        .manager()
        .with_player(player_guid, |player| {
            modified = apply_spell_modifiers_to_value(
                &player.spells.spell_modifiers,
                SpellModOp::GlobalCooldown,
                modified,
                spell_family_name,
                spell_family_flags,
            );
        });

    modified.max(0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::player::spells::state::{SpellMod, SpellModOp, SpellModType};

    fn make_spell_entry() -> SpellEntry {
        SpellEntry {
            id: 1,
            name: String::new(),
            rank_text: String::new(),
            school: 0,
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
            effect: [0; 3],
            effect_die_sides: [0; 3],
            effect_base_dice: [0; 3],
            effect_dice_per_level: [0.0; 3],
            effect_real_points_per_level: [0.0; 3],
            effect_base_points: [0; 3],
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
            dmg_multiplier: [1.0; 3],
        }
    }

    fn ctx() -> PowerCostContext {
        PowerCostContext {
            health: 1000,
            create_health: 2000,
            create_mana: 3000,
            current_power: 500,
            max_power: 800,
            level: 60,
            spell_rank: 60,
        }
    }

    fn cost_mod(mod_type: SpellModType, value: i32, family: u32) -> SpellMod {
        SpellMod {
            op: SpellModOp::Cost,
            mod_type,
            value,
            spell_family_mask: 0,
            spell_family_name: family,
            source_spell_id: 1,
            source_aura_slot: None,
        }
    }

    #[test]
    fn modifier_membership_matches_the_same_modifier_instance() {
        let modifier = cost_mod(SpellModType::Flat, 10, 3);
        assert!(has_modifier_applied(&[&modifier], &modifier));
    }

    #[test]
    fn modifier_membership_rejects_equal_but_distinct_instances() {
        let applied = cost_mod(SpellModType::Flat, 10, 3);
        let modifier = cost_mod(SpellModType::Flat, 10, 3);
        assert!(!has_modifier_applied(&[&applied], &modifier));
    }

    #[test]
    fn modifier_membership_returns_false_when_absent() {
        let applied = cost_mod(SpellModType::Flat, 10, 3);
        let modifier = cost_mod(SpellModType::Pct, 20, 3);
        assert!(!has_modifier_applied(&[&applied], &modifier));
    }

    #[test]
    fn modifier_membership_finds_a_later_modifier() {
        let first = cost_mod(SpellModType::Flat, 10, 3);
        let modifier = cost_mod(SpellModType::Pct, 20, 3);
        assert!(has_modifier_applied(&[&first, &modifier], &modifier));
    }

    #[test]
    fn spell_modifier_requires_an_exact_family_match() {
        let modifier = cost_mod(SpellModType::Flat, 10, 0);
        assert!(!does_modifier_apply(&modifier, 3, 0));
    }

    #[test]
    fn spell_modifier_with_zero_mask_affects_its_entire_family() {
        let modifier = cost_mod(SpellModType::Flat, 10, 3);
        assert!(does_modifier_apply(&modifier, 3, 0x400));
    }

    #[test]
    fn spell_modifier_mask_must_overlap_spell_flags() {
        let mut modifier = cost_mod(SpellModType::Flat, 10, 3);
        modifier.spell_family_mask = 0x200;
        assert!(does_modifier_apply(&modifier, 3, 0x200));
        assert!(!does_modifier_apply(&modifier, 3, 0x100));
    }

    #[test]
    fn spell_affect_mask_uses_the_source_effect_index() {
        let masks = [0x10, 0x20, 0x40];
        assert_eq!(spell_affect_mask(&masks, 1), 0x20);
        assert_eq!(spell_affect_mask(&masks, 3), 0);
    }

    #[test]
    fn item_cast_costs_nothing() {
        let mut spell = make_spell_entry();
        spell.mana_cost = 100;
        assert_eq!(calculate_power_cost(&spell, &ctx(), true, &[]), 0);
    }

    #[test]
    fn flat_mana_cost() {
        let mut spell = make_spell_entry();
        spell.mana_cost = 100;
        assert_eq!(calculate_power_cost(&spell, &ctx(), false, &[]), 100);
    }

    #[test]
    fn use_all_mana_health_returns_current_health() {
        let mut spell = make_spell_entry();
        spell.attributes_ex = SPELL_ATTR_EX_USE_ALL_MANA;
        spell.power_type = POWER_HEALTH;
        spell.mana_cost = 50; // ignored
        assert_eq!(calculate_power_cost(&spell, &ctx(), false, &[]), 1000);
    }

    #[test]
    fn use_all_mana_returns_current_power() {
        let mut spell = make_spell_entry();
        spell.attributes_ex = SPELL_ATTR_EX_USE_ALL_MANA;
        spell.power_type = 0; // mana
        assert_eq!(calculate_power_cost(&spell, &ctx(), false, &[]), 500);
    }

    #[test]
    fn use_all_mana_unknown_power_is_zero() {
        let mut spell = make_spell_entry();
        spell.attributes_ex = SPELL_ATTR_EX_USE_ALL_MANA;
        spell.power_type = 99; // not health, not < MAX_POWERS
        assert_eq!(calculate_power_cost(&spell, &ctx(), false, &[]), 0);
    }

    #[test]
    fn percentage_of_create_mana() {
        let mut spell = make_spell_entry();
        spell.power_type = 0; // mana
        spell.mana_cost_percentage = 10; // 10% of create_mana(3000) = 300
        assert_eq!(calculate_power_cost(&spell, &ctx(), false, &[]), 300);
    }

    #[test]
    fn percentage_of_create_health() {
        let mut spell = make_spell_entry();
        spell.power_type = POWER_HEALTH;
        spell.mana_cost_percentage = 5; // 5% of create_health(2000) = 100
        assert_eq!(calculate_power_cost(&spell, &ctx(), false, &[]), 100);
    }

    #[test]
    fn percentage_of_max_power_for_energy() {
        let mut spell = make_spell_entry();
        spell.power_type = 3; // energy
        spell.mana_cost_percentage = 25; // 25% of max_power(800) = 200
        assert_eq!(calculate_power_cost(&spell, &ctx(), false, &[]), 200);
    }

    #[test]
    fn per_level_cost_scaling() {
        let mut spell = make_spell_entry();
        spell.mana_cost = 100;
        spell.mana_cost_per_level = 2;
        spell.base_level = 1;
        // spell_rank 60 -> 60/5 - 1 = 11; 100 + 2*11 = 122
        assert_eq!(calculate_power_cost(&spell, &ctx(), false, &[]), 122);
    }

    #[test]
    fn spellmod_cost_flat_and_pct() {
        let mut spell = make_spell_entry();
        spell.mana_cost = 100;
        spell.spell_family_name = 3;
        let mods = vec![
            cost_mod(SpellModType::Flat, -20, 3),
            cost_mod(SpellModType::Pct, -50, 3),
        ];
        // (100 - 20) * (1 - 0.5) = 40
        assert_eq!(calculate_power_cost(&spell, &ctx(), false, &mods), 40);
    }

    #[test]
    fn school_power_cost_aura_percentage_applies_after_spell_modifiers() {
        let mut spell = make_spell_entry();
        spell.mana_cost = 100;
        assert_eq!(
            calculate_power_cost_with_school_multiplier(&spell, &ctx(), false, &[], -20),
            80
        );
    }

    #[test]
    fn school_power_cost_aura_percentage_clamps_negative_cost() {
        let mut spell = make_spell_entry();
        spell.mana_cost = 100;
        assert_eq!(
            calculate_power_cost_with_school_multiplier(&spell, &ctx(), false, &[], -200),
            0
        );
    }

    #[test]
    fn cost_clamped_to_zero() {
        let mut spell = make_spell_entry();
        spell.mana_cost = 10;
        spell.spell_family_name = 3;
        let mods = vec![cost_mod(SpellModType::Flat, -999, 3)];
        assert_eq!(calculate_power_cost(&spell, &ctx(), false, &mods), 0);
    }

    #[test]
    fn creature_level_scaling_applies() {
        let mut spell = make_spell_entry();
        spell.mana_cost = 100;
        spell.attributes = SPELL_ATTR_SCALES_WITH_CREATURE_LEVEL;
        spell.spell_level = 30;
        let mut c = ctx();
        c.level = 60;
        let expected = (100.0_f32 / (1.117_f32 * 30.0 / 60.0 - 0.1327_f32)) as i32 as u32;
        assert_eq!(calculate_power_cost(&spell, &c, false, &[]), expected);
        assert!(calculate_power_cost(&spell, &c, false, &[]) > 100);
    }

    #[test]
    fn cooldown_modifier_respects_family_match() {
        // Matching family: flat -500ms + pct -50% halves and subtracts.
        let mods = vec![
            SpellMod {
                op: SpellModOp::Cooldown,
                mod_type: SpellModType::Flat,
                value: -500,
                spell_family_mask: 0,
                spell_family_name: 3,
                source_spell_id: 1,
                source_aura_slot: None,
            },
            SpellMod {
                op: SpellModOp::Cooldown,
                mod_type: SpellModType::Pct,
                value: -50,
                spell_family_mask: 0,
                spell_family_name: 3,
                source_spell_id: 2,
                source_aura_slot: None,
            },
        ];
        // (2000 - 500) * 0.5 = 750
        assert_eq!(
            apply_spell_modifiers_to_value(&mods, SpellModOp::Cooldown, 2000, 3, 0),
            750
        );

        // Wrong family name: no modifier applies.
        assert_eq!(
            apply_spell_modifiers_to_value(&mods, SpellModOp::Cooldown, 2000, 7, 0),
            2000
        );
    }

    #[tokio::test]
    async fn calculate_modified_cooldown_wired_to_player_modifiers() {
        let world = test_world();
        let guid = ObjectGuid::new_player(1);
        add_player(&world, guid);

        world
            .systems
            .player
            .manager()
            .with_player_mut(guid, |player| {
                player.spells.spell_modifiers.push(SpellMod {
                    op: SpellModOp::Cooldown,
                    mod_type: SpellModType::Flat,
                    value: -1000,
                    spell_family_mask: 0,
                    spell_family_name: 3,
                    source_spell_id: 1,
                    source_aura_slot: None,
                });
            });

        assert_eq!(calculate_modified_cooldown(guid, 3000, 3, 0, &world), 2000);

        // Wrong family name returns the base value unchanged.
        assert_eq!(calculate_modified_cooldown(guid, 3000, 7, 0, &world), 3000);
    }

    fn test_world() -> World {
        use crate::config::Config;
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
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    fn add_player(world: &World, guid: ObjectGuid) {
        use crate::game::player::Player;
        world.managers.player_mgr.add_player(
            Player::new(guid, format!("P{}", guid.counter()), 1, 0, 0, 60, 1, 1, 0),
            guid.counter(),
        );
    }
}
