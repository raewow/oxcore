use crate::world::dbc::manager::DbcManager;
use crate::world::dbc::structures::SpellEntry;
use crate::world::game::spell::manager::SpellManager;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellSpecific {
    Normal = 0,
    Seal = 1,
    Blessing = 2,
    Aura = 3,
    Sting = 4,
    Curse = 5,
    Aspect = 6,
    Tracker = 7,
    WarlockArmor = 8,
    MageArmor = 9,
    ElementalShield = 10,
    MagePolymorph = 11,
    PositiveShout = 12,
    Judgement = 13,
    BattleElixir = 14,
    GuardianElixir = 15,
    FlaskElixir = 16,
    WellFed = 19,
    Food = 20,
    Drink = 21,
    FoodAndDrink = 22,
    NegativeHaste = 23,
    Snare = 24,
}

const SPELLFAMILY_GENERIC: u32 = 0;
const SPELLFAMILY_MAGE: u32 = 3;
const SPELLFAMILY_WARRIOR: u32 = 4;
const SPELLFAMILY_WARLOCK: u32 = 5;
const SPELLFAMILY_PRIEST: u32 = 6;
const SPELLFAMILY_DRUID: u32 = 7;
const SPELLFAMILY_ROGUE: u32 = 8;
const SPELLFAMILY_HUNTER: u32 = 9;
const SPELLFAMILY_PALADIN: u32 = 10;
const SPELLFAMILY_SHAMAN: u32 = 11;
const SPELLFAMILY_POTION: u32 = 13;

const SPELL_EFFECT_APPLY_AURA: u32 = 6;
const SPELL_EFFECT_TRIGGER_SPELL: u32 = 32;

const AURA_MOD_REGEN: u32 = 84;
const AURA_MOD_POWER_REGEN: u32 = 85;
const AURA_OBS_MOD_HEALTH: u32 = 20;
const AURA_OBS_MOD_MANA: u32 = 21;
const AURA_MOD_MELEE_HASTE: u32 = 138;
const AURA_MOD_DECREASE_SPEED: u32 = 33;
const AURA_TRACK_CREATURES: u32 = 44;
const AURA_TRACK_RESOURCES: u32 = 45;
const AURA_TRACK_STEALTHED: u32 = 46;
const AURA_PERIODIC_DAMAGE: u32 = 3;
const AURA_PERIODIC_HEAL: u32 = 8;

pub fn get_spell_specific(spell: &SpellEntry) -> SpellSpecific {
    match spell.spell_family_name {
        SPELLFAMILY_GENERIC => {
            if spell.id == 13161 {
                return SpellSpecific::Aspect;
            }

            if (spell.aura_interrupt_flags & 0x0004_0000) != 0 {
                let mut food = false;
                let mut drink = false;
                for aura in spell.effect_apply_aura_name {
                    match aura {
                        AURA_MOD_REGEN | AURA_OBS_MOD_HEALTH => food = true,
                        AURA_MOD_POWER_REGEN | AURA_OBS_MOD_MANA => drink = true,
                        _ => {}
                    }
                }
                if food && drink {
                    return SpellSpecific::FoodAndDrink;
                }
                if food {
                    return SpellSpecific::Food;
                }
                if drink {
                    return SpellSpecific::Drink;
                }
            } else if (spell.attributes_ex2 & 0x8000_0000) != 0 {
                return SpellSpecific::WellFed;
            }
        }
        SPELLFAMILY_MAGE => {
            if spell.spell_family_flags & 0x1200_0000 != 0 {
                return SpellSpecific::MageArmor;
            }
            if spell.effect_apply_aura_name[0] == 48 && spell.prevention_type == 1 {
                return SpellSpecific::MagePolymorph;
            }
        }
        SPELLFAMILY_WARRIOR => {
            if spell.spell_family_flags & 0x0000_8000_0100_0000 != 0 {
                return SpellSpecific::PositiveShout;
            }
        }
        SPELLFAMILY_WARLOCK => {
            if spell.dispel == 2 {
                return SpellSpecific::Curse;
            }
        }
        SPELLFAMILY_PRIEST => {
            if (spell.attributes & 0x0800_0000) != 0 && (spell.aura_interrupt_flags & 0x0000_0008) != 0 && (spell.spell_icon_id == 52 || spell.spell_icon_id == 79) {
                return SpellSpecific::WellFed;
            }
        }
        SPELLFAMILY_HUNTER => {
            if spell.dispel == 4 {
                return SpellSpecific::Sting;
            }
            if spell.active_icon_id == 122 && spell.id != 75 {
                return SpellSpecific::Aspect;
            }
        }
        SPELLFAMILY_PALADIN => {
            if spell.is_aura_spell() {
                return SpellSpecific::Aura;
            }
        }
        SPELLFAMILY_SHAMAN => {
            if spell.id == 23552 {
                return SpellSpecific::ElementalShield;
            }
        }
        SPELLFAMILY_POTION => {
            // No dedicated elixir table yet.
        }
        _ => {}
    }

    if spell.spell_visual == 130 && spell.spell_icon_id == 89 {
        return SpellSpecific::WarlockArmor;
    }

    if spell.effect_apply_aura_name.iter().any(|&aura| {
        aura == AURA_TRACK_CREATURES || aura == AURA_TRACK_RESOURCES || aura == AURA_TRACK_STEALTHED
    })
        && ((spell.attributes_ex & 0x0002_0000) != 0 || (spell.attributes & 0x0800_0000) != 0)
    {
        return SpellSpecific::Tracker;
    }

    if spell.effect_apply_aura_name.iter().any(|&aura| aura == AURA_MOD_MELEE_HASTE) && spell.effect_base_points.iter().any(|&points| points < 0) {
        return SpellSpecific::NegativeHaste;
    }

    if spell.effect_apply_aura_name.iter().any(|&aura| aura == AURA_MOD_DECREASE_SPEED) {
        return SpellSpecific::Snare;
    }

    SpellSpecific::Normal
}

pub fn compare_aura_ranks(spell1: &SpellEntry, spell2: &SpellEntry) -> bool {
    if spell1.id == spell2.id {
        return false;
    }

    for idx in 0..spell1.effect.len() {
        if spell1.effect[idx] != 0 && spell1.effect[idx] == spell2.effect[idx] {
            let diff = spell1.effect_base_points[idx] - spell2.effect_base_points[idx];
            if diff != 0 {
                return true;
            }
        }
    }

    false
}

pub fn compare_spell_specific_auras(a: &SpellEntry, b: &SpellEntry) -> bool {
    for idx in 0..a.effect.len() {
        for jdx in 0..b.effect.len() {
            if a.effect[idx] == SPELL_EFFECT_APPLY_AURA && a.effect_apply_aura_name[idx] == b.effect_apply_aura_name[jdx] {
                if a.effect_base_points[idx] != b.effect_base_points[jdx] {
                    return true;
                }

                if a.effect_base_points[idx] == b.effect_base_points[jdx] {
                    let dbc = DbcManager::new();
                    if a.get_duration(&dbc) >= b.get_duration(&dbc) {
                        return true;
                    }
                }
            }
        }
    }

    false
}

pub fn is_autocastable(spell: &SpellEntry) -> bool {
    spell.is_autocastable()
}

pub fn is_passive_spell(spell: &SpellEntry) -> bool {
    spell.is_passive_spell()
}

pub fn is_positive_spell(spell: &SpellEntry) -> bool {
    spell.is_positive_spell()
}

pub fn is_single_target_spells(a: &SpellEntry, b: &SpellEntry) -> bool {
    if a.spell_family_name == b.spell_family_name && a.spell_icon_id == b.spell_icon_id {
        return true;
    }

    matches!(get_spell_specific(a), SpellSpecific::Judgement | SpellSpecific::MagePolymorph)
        && get_spell_specific(a) == get_spell_specific(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_spell_entry(id: u32) -> SpellEntry {
        SpellEntry {
            id,
            name: format!("Spell{}", id),
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

    #[test]
    fn passive_and_autocast_checks() {
        let mut passive = make_spell_entry(1);
        passive.attributes = 0x40;
        assert!(is_passive_spell(&passive));
        assert!(!is_autocastable(&passive));

        let mut active = make_spell_entry(2);
        active.attributes_ex = 0x0002_0000;
        assert!(!is_autocastable(&active));
    }

    #[test]
    fn spell_specific_classification() {
        let mut food = make_spell_entry(13161);
        food.spell_family_name = SPELLFAMILY_GENERIC;
        food.aura_interrupt_flags = 0x0004_0000;
        food.effect_apply_aura_name = [AURA_MOD_REGEN, 0, 0];
        assert_eq!(get_spell_specific(&food), SpellSpecific::Aspect);

        let mut well_fed = make_spell_entry(20000);
        well_fed.spell_family_name = SPELLFAMILY_GENERIC;
        well_fed.attributes_ex2 = 0x8000_0000;
        assert_eq!(get_spell_specific(&well_fed), SpellSpecific::WellFed);
    }

    #[test]
    fn aura_rank_and_specific_aura_comparisons() {
        let mut a = make_spell_entry(10);
        let mut b = make_spell_entry(11);
        a.effect = [6, 0, 0];
        b.effect = [6, 0, 0];
        a.effect_base_points = [20, 0, 0];
        b.effect_base_points = [10, 0, 0];
        a.effect_apply_aura_name = [AURA_MOD_REGEN, 0, 0];
        b.effect_apply_aura_name = [AURA_MOD_REGEN, 0, 0];

        assert!(compare_aura_ranks(&a, &b));
        assert!(compare_spell_specific_auras(&a, &b));
    }

    #[test]
    fn single_target_detection_uses_family_and_icon() {
        let mut a = make_spell_entry(20);
        let mut b = make_spell_entry(21);
        a.spell_family_name = 3;
        b.spell_family_name = 3;
        a.spell_icon_id = 125;
        b.spell_icon_id = 125;

        assert!(is_single_target_spells(&a, &b));
    }
}
