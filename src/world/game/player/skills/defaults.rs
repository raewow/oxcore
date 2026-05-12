use super::constants::*;

/// Weapon proficiency skill IDs for each class at character creation.
///
/// Data sourced from SkillRaceClassInfo.dbc / MaNGOS Player.cpp CreatePlayer().
/// Classes may learn additional proficiencies from trainers later.
pub fn get_class_default_weapon_skills(class: u8) -> &'static [u16] {
    match class {
        // Warrior: all melee weapons, bows, guns, crossbows, thrown
        1 => &[
            SKILL_SWORDS,
            SKILL_2H_SWORDS,
            SKILL_AXES,
            SKILL_2H_AXES,
            SKILL_MACES,
            SKILL_2H_MACES,
            SKILL_DAGGERS,
            SKILL_UNARMED,
            SKILL_POLEARMS,
            SKILL_STAVES,
            SKILL_BOWS,
            SKILL_GUNS,
            SKILL_CROSSBOWS,
            SKILL_THROWN,
            SKILL_FIST_WEAPONS,
            SKILL_DUAL_WIELD,
        ],
        // Paladin: swords, 2h swords, maces, 2h maces, axes, 2h axes, polearms
        2 => &[
            SKILL_SWORDS,
            SKILL_2H_SWORDS,
            SKILL_MACES,
            SKILL_2H_MACES,
            SKILL_AXES,
            SKILL_2H_AXES,
            SKILL_POLEARMS,
            SKILL_UNARMED,
        ],
        // Hunter: axes, 2h axes, swords, 2h swords, daggers, polearms, staves, bows, guns, crossbows, thrown, fist, dual wield
        3 => &[
            SKILL_AXES,
            SKILL_2H_AXES,
            SKILL_SWORDS,
            SKILL_2H_SWORDS,
            SKILL_DAGGERS,
            SKILL_POLEARMS,
            SKILL_STAVES,
            SKILL_BOWS,
            SKILL_GUNS,
            SKILL_CROSSBOWS,
            SKILL_THROWN,
            SKILL_FIST_WEAPONS,
            SKILL_UNARMED,
            SKILL_DUAL_WIELD,
        ],
        // Rogue: swords, daggers, maces, fist, bows, guns, crossbows, thrown, dual wield
        4 => &[
            SKILL_SWORDS,
            SKILL_DAGGERS,
            SKILL_MACES,
            SKILL_FIST_WEAPONS,
            SKILL_BOWS,
            SKILL_GUNS,
            SKILL_CROSSBOWS,
            SKILL_THROWN,
            SKILL_UNARMED,
            SKILL_DUAL_WIELD,
        ],
        // Priest: maces, daggers, staves, wands
        5 => &[
            SKILL_MACES,
            SKILL_DAGGERS,
            SKILL_STAVES,
            SKILL_WANDS,
            SKILL_UNARMED,
        ],
        // Shaman: maces, 2h maces, axes, 2h axes, daggers, staves, fist
        7 => &[
            SKILL_MACES,
            SKILL_2H_MACES,
            SKILL_AXES,
            SKILL_2H_AXES,
            SKILL_DAGGERS,
            SKILL_STAVES,
            SKILL_FIST_WEAPONS,
            SKILL_UNARMED,
        ],
        // Mage: swords, daggers, staves, wands
        8 => &[
            SKILL_SWORDS,
            SKILL_DAGGERS,
            SKILL_STAVES,
            SKILL_WANDS,
            SKILL_UNARMED,
        ],
        // Warlock: swords, daggers, staves, wands
        9 => &[
            SKILL_SWORDS,
            SKILL_DAGGERS,
            SKILL_STAVES,
            SKILL_WANDS,
            SKILL_UNARMED,
        ],
        // Druid: maces, 2h maces, daggers, staves, fist
        11 => &[
            SKILL_MACES,
            SKILL_2H_MACES,
            SKILL_DAGGERS,
            SKILL_STAVES,
            SKILL_FIST_WEAPONS,
            SKILL_UNARMED,
        ],
        _ => &[SKILL_UNARMED],
    }
}

/// Armor proficiency skill IDs for each class at character creation.
pub fn get_class_default_armor_skills(class: u8) -> &'static [u16] {
    match class {
        // Warrior: cloth, leather, mail (plate at 40 from trainer)
        1 => &[SKILL_CLOTH, SKILL_LEATHER, SKILL_MAIL, SKILL_SHIELD],
        // Paladin: cloth, leather, mail (plate at 40 from trainer), shield
        2 => &[SKILL_CLOTH, SKILL_LEATHER, SKILL_MAIL, SKILL_SHIELD],
        // Hunter: cloth, leather (mail at 40 from trainer)
        3 => &[SKILL_CLOTH, SKILL_LEATHER],
        // Rogue: cloth, leather
        4 => &[SKILL_CLOTH, SKILL_LEATHER],
        // Priest: cloth
        5 => &[SKILL_CLOTH],
        // Shaman: cloth, leather (mail at 40 from trainer), shield
        7 => &[SKILL_CLOTH, SKILL_LEATHER, SKILL_SHIELD],
        // Mage: cloth
        8 => &[SKILL_CLOTH],
        // Warlock: cloth
        9 => &[SKILL_CLOTH],
        // Druid: cloth, leather
        11 => &[SKILL_CLOTH, SKILL_LEATHER],
        _ => &[SKILL_CLOTH],
    }
}

/// Initialize all default skills for a newly created character.
///
/// Called during character creation (CMSG_CHAR_CREATE handler).
/// Sets up:
/// 1. Defense skill (starts maxed at level * 5)
/// 2. All weapon proficiencies for the class (start at 1)
/// 3. All armor proficiencies for the class (mono: 1/1)
/// 4. Racial language skills (300/300)
/// 5. Unarmed skill (always present)
pub fn initialize_default_skills(
    skills: &mut super::state::SkillState,
    class: u8,
    race: u8,
    level: u8,
) {
    use super::formulas::get_skill_max_for_level;
    use super::state::{SkillData, PLAYER_MAX_SKILLS};

    let max_skill = get_skill_max_for_level(level);
    let mut next_position: usize = 0;

    let mut add_skill = |skill_id: u16, current: u16, max: u16, step: u16| {
        if next_position >= PLAYER_MAX_SKILLS {
            return;
        }
        let pos = next_position;
        next_position += 1;
        skills
            .skills
            .insert(skill_id, SkillData::new(skill_id, current, max, step, pos));
    };

    // Defense (always maxed)
    add_skill(SKILL_DEFENSE, max_skill, max_skill, 0);

    // Weapon skills for class
    for &skill_id in get_class_default_weapon_skills(class) {
        if skill_id == SKILL_DUAL_WIELD {
            // Dual wield is a special skill (mono: 1/1)
            add_skill(skill_id, 1, 1, 0);
        } else {
            add_skill(skill_id, 1, max_skill, 0);
        }
    }

    // Armor proficiencies (mono: 1/1)
    for &skill_id in get_class_default_armor_skills(class) {
        add_skill(skill_id, 1, 1, 0);
    }

    // Racial language
    let language_skill = get_racial_language(race);
    if language_skill > 0 {
        add_skill(language_skill, 300, 300, 0);
    }

    // Common language for Alliance races, Orcish for Horde
    let faction_language = get_faction_language(race);
    if faction_language > 0 && faction_language != language_skill {
        add_skill(faction_language, 300, 300, 0);
    }
}

/// Racial language skill IDs.
const SKILL_LANG_COMMON: u16 = 98;
const SKILL_LANG_ORCISH: u16 = 109;
const SKILL_LANG_DWARVEN: u16 = 111;
const SKILL_LANG_DARNASSIAN: u16 = 113;
const SKILL_LANG_TAURAHE: u16 = 115;
const SKILL_LANG_THALASSIAN: u16 = 137;
const SKILL_LANG_GNOMISH: u16 = 313;
const SKILL_LANG_TROLL: u16 = 315;
const SKILL_LANG_GUTTERSPEAK: u16 = 673;

fn get_racial_language(race: u8) -> u16 {
    match race {
        1 => SKILL_LANG_COMMON,      // Human
        2 => SKILL_LANG_ORCISH,      // Orc
        3 => SKILL_LANG_DWARVEN,     // Dwarf
        4 => SKILL_LANG_DARNASSIAN,  // Night Elf
        5 => SKILL_LANG_GUTTERSPEAK, // Undead
        6 => SKILL_LANG_TAURAHE,     // Tauren
        7 => SKILL_LANG_GNOMISH,     // Gnome
        8 => SKILL_LANG_TROLL,       // Troll
        _ => 0,
    }
}

fn get_faction_language(race: u8) -> u16 {
    match race {
        1 | 3 | 4 | 7 => SKILL_LANG_COMMON, // Alliance
        2 | 5 | 6 | 8 => SKILL_LANG_ORCISH, // Horde
        _ => 0,
    }
}
