// creature_template.static_flags1
pub const CREATURE_STATIC_FLAG_UNKILLABLE: u32 = 0x00000008;
pub const CREATURE_STATIC_FLAG_IMMUNE_TO_PC: u32 = 0x00000020;
pub const CREATURE_STATIC_FLAG_IMMUNE_TO_NPC: u32 = 0x00000040;
/// Cannot move: no chasing, no wandering.
pub const CREATURE_STATIC_FLAG_SESSILE: u32 = 0x00000100;
pub const CREATURE_STATIC_FLAG_UNINTERACTIBLE: u32 = 0x00000200;
pub const CREATURE_STATIC_FLAG_NO_AUTOMATIC_REGEN: u32 = 0x00000400;
pub const CREATURE_STATIC_FLAG_NO_DEFENSE: u32 = 0x00004000;
/// Does not auto attack.
pub const CREATURE_STATIC_FLAG_NO_MELEE: u32 = 0x00100000;
/// Spirit healers have this flag in static_flags1 - only visible to dead/ghost players
pub const CREATURE_STATIC_FLAG_VISIBLE_TO_GHOSTS: u32 = 0x00200000;
/// Passive react state: never starts a fight on its own.
pub const CREATURE_STATIC_FLAG_IGNORE_COMBAT: u32 = 0x02000000;
/// No proximity aggro against players who are not PvP flagged.
pub const CREATURE_STATIC_FLAG_ONLY_ATTACK_PVP_ENABLING: u32 = 0x04000000;
/// Summons a guard when an opposite-faction player gets near or attacks.
pub const CREATURE_STATIC_FLAG_CALLS_GUARDS: u32 = 0x08000000;

// creature_template.static_flags2
/// Enters combat with every player in the dungeon on aggro.
pub const CREATURE_STATIC_FLAG_2_FORCE_RAID_COMBAT: u32 = 0x00000002;

// creature_template.flags_extra
/// Does not evade when its target runs away.
pub const CREATURE_FLAG_EXTRA_NO_LEASH_EVADE: u32 = 0x00000001;
/// Defensive: does not attack nearby hostile targets.
pub const CREATURE_FLAG_EXTRA_NO_AGGRO: u32 = 0x00000002;
/// Does not evade when its target cannot be reached.
pub const CREATURE_FLAG_EXTRA_NO_UNREACHABLE_EVADE: u32 = 0x00000008;
pub const CREATURE_FLAG_EXTRA_GUARD: u32 = 0x00000400;
/// Keeps no threat list; leaves combat on a five second timer like a player.
pub const CREATURE_FLAG_EXTRA_NO_THREAT_LIST: u32 = 0x00000800;
/// Keeps its positive auras when it evades.
pub const CREATURE_FLAG_EXTRA_KEEP_POSITIVE_AURAS_ON_EVADE: u32 = 0x00001000;
/// Does not join when nearby creatures are pulled.
pub const CREATURE_FLAG_EXTRA_NO_ASSIST: u32 = 0x00010000;
/// Passive: never acquires targets.
pub const CREATURE_FLAG_EXTRA_NO_TARGET: u32 = 0x00020000;
