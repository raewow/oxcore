//! Unit - shared fields for Player and Creature
//!
//! In the new architecture, unit-level data (health, level, etc.)
//! is typically stored in the StatsSystem rather than on the object.
//! This module provides shared constants and helper functions.

/// Unit type mask bits
pub const TYPEMASK_OBJECT: u16 = 0x0001;
pub const TYPEMASK_ITEM: u16 = 0x0002;
pub const TYPEMASK_CONTAINER: u16 = 0x0004;
pub const TYPEMASK_UNIT: u16 = 0x0008;
pub const TYPEMASK_PLAYER: u16 = 0x0010;
pub const TYPEMASK_GAMEOBJECT: u16 = 0x0020;
pub const TYPEMASK_DYNAMICOBJECT: u16 = 0x0040;
pub const TYPEMASK_CORPSE: u16 = 0x0080;

/// Type IDs
pub const TYPEID_OBJECT: u8 = 0;
pub const TYPEID_ITEM: u8 = 1;
pub const TYPEID_CONTAINER: u8 = 2;
pub const TYPEID_UNIT: u8 = 3;
pub const TYPEID_PLAYER: u8 = 4;
pub const TYPEID_GAMEOBJECT: u8 = 5;
pub const TYPEID_DYNAMICOBJECT: u8 = 6;
pub const TYPEID_CORPSE: u8 = 7;

/// Unit races
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Race {
    Human = 1,
    Orc = 2,
    Dwarf = 3,
    NightElf = 4,
    Undead = 5,
    Tauren = 6,
    Gnome = 7,
    Troll = 8,
}

/// Unit classes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Class {
    Warrior = 1,
    Paladin = 2,
    Hunter = 3,
    Rogue = 4,
    Priest = 5,
    Shaman = 7,
    Mage = 8,
    Warlock = 9,
    Druid = 11,
}

/// Get faction template for a race
pub fn get_faction_template(race: u8) -> u32 {
    match race {
        1 => 1,   // Human
        2 => 2,   // Orc
        3 => 3,   // Dwarf
        4 => 4,   // Night Elf
        5 => 5,   // Undead
        6 => 6,   // Tauren
        7 => 115, // Gnome
        8 => 116, // Troll
        _ => 0,
    }
}

/// Get display ID for race/gender
pub fn get_display_id(race: u8, gender: u8) -> u32 {
    // Simplified - real implementation would look up in DBC
    match (race, gender) {
        (1, 0) => 49,   // Human male
        (1, 1) => 50,   // Human female
        (2, 0) => 51,   // Orc male
        (2, 1) => 52,   // Orc female
        (3, 0) => 53,   // Dwarf male
        (3, 1) => 54,   // Dwarf female
        (4, 0) => 55,   // Night Elf male
        (4, 1) => 56,   // Night Elf female
        (5, 0) => 57,   // Undead male
        (5, 1) => 58,   // Undead female
        (6, 0) => 59,   // Tauren male
        (6, 1) => 60,   // Tauren female
        (7, 0) => 1563, // Gnome male
        (7, 1) => 1564, // Gnome female
        (8, 0) => 1478, // Troll male
        (8, 1) => 1479, // Troll female
        _ => 49,
    }
}
