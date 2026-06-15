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
