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

use crate::game::reputation::{ReputationListID, MAX_REPUTATION_LIST_SLOTS};
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;
use std::collections::HashMap;

/// Faction slots the modern client expects in `SMSG_INITIALIZE_FACTIONS`.
///
/// Fixed at 400 regardless of how many the server actually tracks — the body has no count field,
/// so the client reads exactly this many and anything shorter desynchronises the stream.
const MODERN_FACTION_COUNT: usize = 400;

/// SMSG_INITIALIZE_FACTIONS - Initializes all reputation factions for the player
///
/// Sent to the player upon login to initialize all reputation factions.
#[derive(Debug, Clone)]
pub struct SmsgInitializeFactions {
    /// Map of reputation list IDs to their state
    pub factions: HashMap<ReputationListID, (u8, i32)>, // flags, absolute standing
}

impl ToWorldPacket for SmsgInitializeFactions {
    fn to_vanilla(&self) -> WorldPacket {
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

    /// The modern body drops the leading flags word and splits the record: all 400 `(flags,
    /// standing)` pairs first, then 400 "has bonus" bits packed together at the end. Standing is
    /// signed here rather than reinterpreted as `u32`.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        for slot in 0..MODERN_FACTION_COUNT {
            let (flags, standing) = self
                .factions
                .get(&(slot as ReputationListID))
                .copied()
                .unwrap_or((0, 0));
            writer.write_u8(flags);
            writer.write_i32(standing);
        }
        // No faction bonuses are modelled yet; the client still needs all 400 bits.
        for _ in 0..MODERN_FACTION_COUNT {
            writer.write_bit(false);
        }
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_INITIALIZE_FACTIONS))
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SET_FACTION_STANDING);
        packet.write_u32(self.factions.len() as u32);

        for (rep_list_id, absolute_standing) in &self.factions {
            packet.write_u32(*rep_list_id);
            packet.write_u32(*absolute_standing as u32);
        }

        packet
    }

    /// The modern body brackets the same list with two bonus multipliers up front and a
    /// "play the reputation-gain animation" bit at the end.
    ///
    /// The standings themselves need no arithmetic: both protocols carry the **absolute** standing,
    /// not a delta against the faction's base value, the same convention `SmsgInitializeFactions`
    /// uses above. Subtracting a base here would drop every reputation to near zero on the client
    /// while the server still thought it was correct.
    ///
    /// Skipping the two leading floats is the dangerous omission — the client would read the first
    /// four bytes of the pair list as a bonus multiplier and the entry count from a standing.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_f32(0.0); // ReferAFriendBonus -- no recruit-a-friend in vanilla
        writer.write_f32(0.0); // BonusFromAchievementSystem -- no achievements in vanilla

        writer.write_i32(self.factions.len() as i32);
        for (rep_list_id, absolute_standing) in &self.factions {
            writer.write_i32(*rep_list_id as i32);
            writer.write_i32(*absolute_standing);
        }

        // Vanilla has no way to say "update this quietly", and every send here is a real gain or
        // loss the player should see animate on the reputation bar.
        writer.write_bit(true); // ShowVisual
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_SET_FACTION_STANDING))
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
    /// 1.14 reads the same count-then-pairs body, only as signed words, so the bytes are identical.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
    }

    fn to_vanilla(&self) -> WorldPacket {
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
    /// Identical to vanilla in 1.14: one u32 reputation list index.
    ///
    /// 1.14 hides a faction with a separate opcode rather than a flag in this one, so this message
    /// only ever means "show it" — which is all vanilla can say here anyway.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SET_FACTION_VISIBLE);
        packet.write_u32(self.reputation_list_id);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Opcode;

    #[test]
    fn test_smsg_initialize_factions() {
        let mut factions = HashMap::new();
        factions.insert(0, (0x01, 1200));
        factions.insert(1, (0x02, 500));
        factions.insert(3, (0x04, -200));

        let msg = SmsgInitializeFactions { factions };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_INITIALIZE_FACTIONS);
    }

    #[test]
    fn test_smsg_set_faction_standing() {
        let factions = vec![(0, 1200), (1, 500)];
        let msg = SmsgSetFactionStanding { factions };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_SET_FACTION_STANDING);
    }

    #[test]
    fn test_smsg_set_forced_reactions() {
        let mut forced_reactions = HashMap::new();
        forced_reactions.insert(1, 2);
        forced_reactions.insert(2, 3);

        let msg = SmsgSetForcedReactions { forced_reactions };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_SET_FORCED_REACTIONS);
    }

    #[test]
    fn test_smsg_set_faction_visible() {
        let msg = SmsgSetFactionVisible {
            reputation_list_id: 5,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_SET_FACTION_VISIBLE);
    }
}
