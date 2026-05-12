//! Reputation system message structs
//!
//! This module contains type-safe message structures for all reputation-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgInitializeFactions`] - Initializes all reputation factions for the player
//! - [`SmsgSetFactionStanding`] - Updates faction standing for specific factions
//! - [`SmsgSetForcedReactions`] - Sets forced reactions for specific factions
//! - [`SmsgSetFactionVisible`] - Makes a faction visible to the player

use crate::shared::game::reputation::{ReputationListID, MAX_REPUTATION_LIST_SLOTS};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;
use std::collections::HashMap;

/// SMSG_INITIALIZE_FACTIONS - Initializes all reputation factions for the player
///
/// Sent to the player upon login to initialize all reputation factions.
#[derive(Debug, Clone)]
pub struct SmsgInitializeFactions {
    /// Map of reputation list IDs to their state
    pub factions: HashMap<ReputationListID, (u8, i32)>, // flags, absolute standing
}

impl ToWorldPacket for SmsgInitializeFactions {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_INITIALIZE_FACTIONS);
        packet.write_u32(0x00000040); // Flags

        let mut rep_list_ids: Vec<ReputationListID> = self.factions.keys().copied().collect();
        rep_list_ids.sort();

        let mut current_id = 0u32;
        for rep_list_id in rep_list_ids {
            while current_id < rep_list_id {
                packet.write_u8(0x00);
                packet.write_u32(0x00000000);
                current_id += 1;
            }

            let (flags, absolute_standing) = self.factions.get(&rep_list_id).unwrap();
            packet.write_u8(*flags);
            packet.write_u32(*absolute_standing as u32);
            current_id += 1;
        }

        while current_id < MAX_REPUTATION_LIST_SLOTS as u32 {
            packet.write_u8(0x00);
            packet.write_u32(0x00000000);
            current_id += 1;
        }

        packet
    }
}

/// SMSG_SET_FACTION_STANDING - Updates faction standing for specific factions
///
/// Sent to the player to update faction standing for specific factions.
#[derive(Debug, Clone)]
pub struct SmsgSetFactionStanding {
    /// List of (ReputationListID, absolute_standing) pairs
    pub factions: Vec<(ReputationListID, i32)>,
}

impl ToWorldPacket for SmsgSetFactionStanding {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SET_FACTION_STANDING);
        packet.write_u32(self.factions.len() as u32);

        for (rep_list_id, absolute_standing) in &self.factions {
            packet.write_u32(*rep_list_id);
            packet.write_u32(*absolute_standing as u32);
        }

        packet
    }
}

/// SMSG_SET_FORCED_REACTIONS - Sets forced reactions for specific factions
///
/// Sent to the player to set forced reactions for specific factions.
#[derive(Debug, Clone)]
pub struct SmsgSetForcedReactions {
    /// Map of faction IDs to forced reaction ranks
    pub forced_reactions: HashMap<u32, u32>, // faction_id, rank
}

impl ToWorldPacket for SmsgSetForcedReactions {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SET_FORCED_REACTIONS);
        packet.write_u32(self.forced_reactions.len() as u32);

        for (faction_id, rank) in &self.forced_reactions {
            packet.write_u32(*faction_id);
            packet.write_u32(*rank);
        }

        packet
    }
}

/// SMSG_SET_FACTION_VISIBLE - Makes a faction visible to the player
///
/// Sent to the player to make a faction visible on their reputation bar.
#[derive(Debug, Clone)]
pub struct SmsgSetFactionVisible {
    /// Reputation list ID of the faction to make visible
    pub reputation_list_id: ReputationListID,
}

impl ToWorldPacket for SmsgSetFactionVisible {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SET_FACTION_VISIBLE);
        packet.write_u32(self.reputation_list_id);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::game::reputation::MAX_REPUTATION_LIST_SLOTS;
    use crate::shared::protocol::Opcode;

    #[test]
    fn test_smsg_initialize_factions() {
        let mut factions = HashMap::new();
        factions.insert(0, (0x01, 1200));
        factions.insert(1, (0x02, 500));
        factions.insert(3, (0x04, -200));

        let msg = SmsgInitializeFactions { factions };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_INITIALIZE_FACTIONS);
    }

    #[test]
    fn test_smsg_set_faction_standing() {
        let factions = vec![(0, 1200), (1, 500)];
        let msg = SmsgSetFactionStanding { factions };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_SET_FACTION_STANDING);
    }

    #[test]
    fn test_smsg_set_forced_reactions() {
        let mut forced_reactions = HashMap::new();
        forced_reactions.insert(1, 2);
        forced_reactions.insert(2, 3);

        let msg = SmsgSetForcedReactions { forced_reactions };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_SET_FORCED_REACTIONS);
    }

    #[test]
    fn test_smsg_set_faction_visible() {
        let msg = SmsgSetFactionVisible {
            reputation_list_id: 5,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_SET_FACTION_VISIBLE);
    }
}
