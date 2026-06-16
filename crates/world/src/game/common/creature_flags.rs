pub const CREATURE_STATIC_FLAG_IMMUNE_TO_PC: u32 = 0x00000001;
pub const CREATURE_STATIC_FLAG_IMMUNE_TO_NPC: u32 = 0x00000002;
pub const CREATURE_STATIC_FLAG_UNINTERACTIBLE: u32 = 0x00000004;

/// Spirit healers have this flag in static_flags1 - only visible to dead/ghost players
pub const CREATURE_STATIC_FLAG_VISIBLE_TO_GHOSTS: u32 = 0x00000100;
