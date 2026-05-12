//! Aggro Detection System
//!
//! Handles aggro-on-sight behavior with faction hostility checks.

use crate::shared::protocol::{ObjectGuid, Position};

/// Base aggro range in yards
pub const BASE_AGGRO_RANGE: f32 = 20.0;
/// Maximum aggro range in yards
pub const MAX_AGGRO_RANGE: f32 = 45.0;
/// Minimum aggro range in yards
pub const MIN_AGGRO_RANGE: f32 = 5.0;

/// Unit flags that prevent aggro
pub const UNIT_FLAG_PASSIVE: u32 = 0x00000200;
pub const UNIT_FLAG_NON_ATTACKABLE: u32 = 0x00000002;
pub const UNIT_FLAG_NOT_SELECTABLE: u32 = 0x02000000;

/// NPC flags (NPCs don't aggro)
pub const NPC_FLAG_GOSSIP: u32 = 0x00000001;
pub const NPC_FLAG_QUEST_GIVER: u32 = 0x00000002;
pub const NPC_FLAG_VENDOR: u32 = 0x00000080;
pub const NPC_FLAG_TRAINER: u32 = 0x00000010;

/// Factions that are hostile to players (simplified list)
/// Full implementation would use FactionTemplate DBC
const HOSTILE_TO_PLAYERS: [u32; 10] = [
    14, // Monster
    16, // Monster, Predator
    17, // Monster, Prey
    21, // Undead, Scourge
    28, // Blackfathom
    38, // Murloc
    40, // Dark Iron Dwarf
    73, // Syndicate
    87, // Bloodsail Buccaneers
    91, // Hatefury
];

/// Calculate aggro range based on level difference
pub fn calculate_aggro_range(creature_level: u8, target_level: u8) -> f32 {
    let level_diff = creature_level as i32 - target_level as i32;

    // +1 yard per level higher, -1 yard per level lower
    let range = BASE_AGGRO_RANGE + level_diff as f32;

    range.clamp(MIN_AGGRO_RANGE, MAX_AGGRO_RANGE)
}

/// Check if creature should aggro on target based on faction
pub fn should_aggro_creature(
    creature_faction: u32,
    creature_flags: u32,
    target_faction: u32,
    is_player: bool,
) -> bool {
    // Check if creature can aggro
    if creature_flags & UNIT_FLAG_PASSIVE != 0 {
        return false;
    }

    if creature_flags & UNIT_FLAG_NON_ATTACKABLE != 0 {
        return false;
    }

    if creature_flags & UNIT_FLAG_NOT_SELECTABLE != 0 {
        return false;
    }

    // Check faction hostility
    is_hostile_faction(creature_faction, target_faction, is_player)
}

/// Check if creature is an NPC (shouldn't aggro)
pub fn is_npc(npc_flags: u32) -> bool {
    npc_flags & (NPC_FLAG_GOSSIP | NPC_FLAG_QUEST_GIVER | NPC_FLAG_VENDOR | NPC_FLAG_TRAINER) != 0
}

/// Simplified faction hostility check
fn is_hostile_faction(faction_a: u32, faction_b: u32, target_is_player: bool) -> bool {
    if target_is_player {
        HOSTILE_TO_PLAYERS.contains(&faction_a)
    } else {
        // Creature vs creature hostility
        // Different factions are hostile (simplified)
        faction_a != faction_b
    }
}

/// Check if a target is valid for aggro
pub fn is_valid_aggro_target(
    target_guid: ObjectGuid,
    target_flags: u32,
    target_is_alive: bool,
) -> bool {
    // Must be a player
    if !target_guid.is_player() {
        return false;
    }

    // Must be alive
    if !target_is_alive {
        return false;
    }

    // Check if target is attackable
    if target_flags & UNIT_FLAG_NON_ATTACKABLE != 0 {
        return false;
    }

    true
}

/// Calculate distance squared between two positions (2D, ignoring Z)
pub fn distance_squared_2d(a: &Position, b: &Position) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggro_range_level_difference() {
        // Same level
        assert_eq!(calculate_aggro_range(10, 10), BASE_AGGRO_RANGE);

        // Creature 5 levels higher
        assert_eq!(calculate_aggro_range(15, 10), BASE_AGGRO_RANGE + 5.0);

        // Creature 5 levels lower
        assert_eq!(calculate_aggro_range(5, 10), BASE_AGGRO_RANGE - 5.0);
    }

    #[test]
    fn test_aggro_range_clamping() {
        // Very high level difference (clamped to max)
        assert_eq!(calculate_aggro_range(100, 1), MAX_AGGRO_RANGE);

        // Very low level difference (clamped to min)
        assert_eq!(calculate_aggro_range(1, 100), MIN_AGGRO_RANGE);
    }

    #[test]
    fn test_passive_creature_no_aggro() {
        assert!(!should_aggro_creature(14, UNIT_FLAG_PASSIVE, 1, true));
    }

    #[test]
    fn test_non_attackable_no_aggro() {
        assert!(!should_aggro_creature(
            14,
            UNIT_FLAG_NON_ATTACKABLE,
            1,
            true
        ));
    }

    #[test]
    fn test_hostile_faction_aggro() {
        // Monster faction should aggro on player
        assert!(should_aggro_creature(14, 0, 1, true));
    }

    #[test]
    fn test_friendly_faction_no_aggro() {
        // Player faction (1) should not aggro on player
        assert!(!should_aggro_creature(1, 0, 1, true));
    }

    #[test]
    fn test_npc_detection() {
        assert!(is_npc(NPC_FLAG_GOSSIP));
        assert!(is_npc(NPC_FLAG_VENDOR));
        assert!(!is_npc(0));
    }

    #[test]
    fn test_distance_squared() {
        let pos1 = Position::new(0.0, 0.0, 0.0, 0.0);
        let pos2 = Position::new(3.0, 4.0, 0.0, 0.0);
        assert_eq!(distance_squared_2d(&pos1, &pos2), 25.0); // 3-4-5 triangle
    }
}
