//! Experience system shared types
//!
//! Contains enums and constants shared between the experience system
//! and other systems that interact with it (combat, quest, etc.).

/// XP source - how the experience was earned
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XpSource {
    /// Creature kill XP
    Kill = 0,
    /// Quest completion XP
    Quest = 1,
    /// Area discovery XP
    Discovery = 2,
    /// PvP kill XP
    Pvp = 3,
}

/// XP color code for level difference display
/// Used by client to color-code targets based on difficulty
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XpColor {
    /// Gray - no XP (too low level)
    Gray = 0,
    /// Green - easy
    Green = 1,
    /// Yellow - normal
    Yellow = 2,
    /// Orange - hard
    Orange = 3,
    /// Red - very hard (+5 or more levels)
    Red = 4,
}

/// Maximum player level (Vanilla 1.12.x)
pub const MAX_PLAYER_LEVEL: u8 = 60;

/// XP sharing distance in yards for group XP
pub const XP_SHARING_DISTANCE: f32 = 74.0;

/// Base XP constant used in XP calculations
/// Matches existing codebase formula
pub const BASE_XP: f32 = 400.0;

/// Base XP constant used in creature kill XP calculation
/// Formula: (player_level * 5 + BASE_CREATURE_XP) * level_factor
pub const BASE_CREATURE_XP: f32 = 45.0;
