use super::constants::{SkillRangeType, *};

/// Get the maximum weapon/defense skill value for a given level.
///
/// In vanilla WoW 1.12, all level-based skills (weapons, defense) use:
///   max_skill = level * 5
///
/// Examples:
/// - Level 1:  max = 5
/// - Level 10: max = 50
/// - Level 40: max = 200
/// - Level 60: max = 300
pub fn get_skill_max_for_level(level: u8) -> u16 {
    (level as u16) * 5
}

/// Get the skill range type for a given skill, using DBC data.
///
/// The range type determines how max_value is calculated:
/// - Level: max = level * 5 (weapon skills, defense)
/// - Language: fixed 300/300
/// - Mono: fixed 1/1 (armor proficiencies)
/// - Rank: tiered from SkillTiers.dbc (professions)
///
/// Logic matches MaNGOS ObjectMgr::GetSkillRangeType():
/// 1. If the skill's SkillRaceClassInfo entry references a SkillTiers entry -> Rank
/// 2. If the skill category is Armor -> Mono
/// 3. If the skill category is Languages -> Language
/// 4. Otherwise -> Level
pub fn get_skill_range_type(category_id: u32, has_skill_tier: bool) -> SkillRangeType {
    if has_skill_tier {
        return SkillRangeType::Rank;
    }
    match category_id {
        SKILL_CATEGORY_ARMOR => SkillRangeType::Mono,
        SKILL_CATEGORY_LANGUAGES => SkillRangeType::Language,
        _ => SkillRangeType::Level,
    }
}

/// Get the starting skill value when a new weapon skill is learned.
/// Weapon skills always start at 1 (you must use the weapon to improve).
/// Defense skill starts at level * 5 (always maxed on creation).
pub fn get_initial_skill_value(skill_id: u16, level: u8) -> (u16, u16) {
    match skill_id {
        SKILL_DEFENSE => {
            let max = get_skill_max_for_level(level);
            (max, max) // Defense starts maxed
        }
        // Armor proficiencies: always 1/1 (mono)
        SKILL_CLOTH | SKILL_LEATHER | SKILL_MAIL | SKILL_PLATE_MAIL | SKILL_SHIELD => (1, 1),
        // Weapon skills: start at 1, max = level * 5
        _ => {
            let max = get_skill_max_for_level(level);
            (1, max)
        }
    }
}
