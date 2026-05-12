// === Defense ===
pub const SKILL_DEFENSE: u16 = 95;

// === Weapon Skills ===
pub const SKILL_SWORDS: u16 = 43;
pub const SKILL_AXES: u16 = 44;
pub const SKILL_BOWS: u16 = 45;
pub const SKILL_GUNS: u16 = 46;
pub const SKILL_MACES: u16 = 54;
pub const SKILL_2H_SWORDS: u16 = 55;
pub const SKILL_DUAL_WIELD: u16 = 118;
pub const SKILL_STAVES: u16 = 136;
pub const SKILL_2H_MACES: u16 = 160;
pub const SKILL_UNARMED: u16 = 162;
pub const SKILL_2H_AXES: u16 = 172;
pub const SKILL_DAGGERS: u16 = 173;
pub const SKILL_THROWN: u16 = 176;
pub const SKILL_POLEARMS: u16 = 229;
pub const SKILL_CROSSBOWS: u16 = 226;
pub const SKILL_WANDS: u16 = 228;
pub const SKILL_FIST_WEAPONS: u16 = 473;

// === Armor Proficiencies ===
pub const SKILL_CLOTH: u16 = 415;
pub const SKILL_LEATHER: u16 = 414;
pub const SKILL_MAIL: u16 = 413;
pub const SKILL_PLATE_MAIL: u16 = 293;
pub const SKILL_SHIELD: u16 = 433;

// === Skill Categories (from SkillLine.dbc category_id) ===
pub const SKILL_CATEGORY_WEAPON: u32 = 6;
pub const SKILL_CATEGORY_ARMOR: u32 = 5;
pub const SKILL_CATEGORY_LANGUAGES: u32 = 9;
pub const SKILL_CATEGORY_CLASS: u32 = 7;
pub const SKILL_CATEGORY_SECONDARY: u32 = 4;
pub const SKILL_CATEGORY_PROFESSION: u32 = 11;

// === Skill Range Types ===
/// How a skill's max value is determined.
/// This affects both skill-up behavior and how max_value is calculated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillRangeType {
    /// Level-dependent: max = level * 5 (weapon skills, defense)
    Level,
    /// Fixed language: always 300/300
    Language,
    /// Mono: always 1/1 (armor proficiencies)
    Mono,
    /// Rank/tier-based: max comes from SkillTiers.dbc (professions)
    Rank,
}

// === Skill Flags (from SkillRaceClassInfo.flags) ===
/// Skill is always set to max value for the player's level.
/// Used for armor proficiencies and some special skills.
pub const SKILL_FLAG_ALWAYS_MAX_VALUE: u32 = 0x0001;

// === Item Classes for Proficiency ===
pub const ITEM_CLASS_WEAPON: u8 = 2;
pub const ITEM_CLASS_ARMOR: u8 = 4;

// === Weapon Subclass Bits ===
pub const ITEM_SUBCLASS_WEAPON_AXE: u32 = 1 << 0;
pub const ITEM_SUBCLASS_WEAPON_AXE2: u32 = 1 << 1;
pub const ITEM_SUBCLASS_WEAPON_BOW: u32 = 1 << 2;
pub const ITEM_SUBCLASS_WEAPON_GUN: u32 = 1 << 3;
pub const ITEM_SUBCLASS_WEAPON_MACE: u32 = 1 << 4;
pub const ITEM_SUBCLASS_WEAPON_MACE2: u32 = 1 << 5;
pub const ITEM_SUBCLASS_WEAPON_POLEARM: u32 = 1 << 6;
pub const ITEM_SUBCLASS_WEAPON_SWORD: u32 = 1 << 7;
pub const ITEM_SUBCLASS_WEAPON_SWORD2: u32 = 1 << 8;
pub const ITEM_SUBCLASS_WEAPON_STAFF: u32 = 1 << 10;
pub const ITEM_SUBCLASS_WEAPON_FIST: u32 = 1 << 13;
pub const ITEM_SUBCLASS_WEAPON_DAGGER: u32 = 1 << 15;
pub const ITEM_SUBCLASS_WEAPON_THROWN: u32 = 1 << 16;
pub const ITEM_SUBCLASS_WEAPON_CROSSBOW: u32 = 1 << 18;
pub const ITEM_SUBCLASS_WEAPON_WAND: u32 = 1 << 19;

// === Armor Subclass Bits ===
pub const ITEM_SUBCLASS_ARMOR_CLOTH: u32 = 1 << 1;
pub const ITEM_SUBCLASS_ARMOR_LEATHER: u32 = 1 << 2;
pub const ITEM_SUBCLASS_ARMOR_MAIL: u32 = 1 << 3;
pub const ITEM_SUBCLASS_ARMOR_PLATE: u32 = 1 << 4;
pub const ITEM_SUBCLASS_ARMOR_SHIELD: u32 = 1 << 6;
