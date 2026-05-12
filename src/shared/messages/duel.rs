//! Duel system message structs
//!
//! This module contains type-safe message structures for all duel-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgDuelRequested`] - Notify players that a duel has been requested
//! - [`SmsgDuelCountdown`] - Countdown before a duel begins
//! - [`SmsgDuelOutOfBounds`] - Notify player they've left the duel boundary
//! - [`SmsgDuelInBounds`] - Notify player they've returned to the duel boundary
//! - [`SmsgDuelComplete`] - Notify players that a duel has completed
//! - [`SmsgDuelWinner`] - Announce the duel winner

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;
use crate::shared::protocol::guid::ObjectGuid;

/// SMSG_DUEL_REQUESTED - Notify players that a duel has been requested
///
/// Sent to both players when a duel is initiated.
#[derive(Debug, Clone)]
pub struct SmsgDuelRequested {
    /// GUID of the duel arbiter (flag)
    pub arbiter_guid: ObjectGuid,
    /// GUID of the duel initiator
    pub initiator_guid: ObjectGuid,
}

impl ToWorldPacket for SmsgDuelRequested {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_DUEL_REQUESTED);
        packet.write_guid_raw(self.arbiter_guid.raw());
        packet.write_guid_raw(self.initiator_guid.raw());
        packet
    }
}

/// SMSG_DUEL_COUNTDOWN - Countdown before a duel begins
///
/// Sent to both players when the duel countdown starts.
#[derive(Debug, Clone)]
pub struct SmsgDuelCountdown {
    /// Countdown time in seconds
    pub countdown: u32,
}

impl ToWorldPacket for SmsgDuelCountdown {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_DUEL_COUNTDOWN);
        packet.write_u32(self.countdown);
        packet
    }
}

/// SMSG_DUEL_OUTOFBOUNDS - Notify player they've left the duel boundary
///
/// Sent when a player moves out of the duel boundary.
#[derive(Debug, Clone)]
pub struct SmsgDuelOutOfBounds {}

impl ToWorldPacket for SmsgDuelOutOfBounds {
    fn to_world_packet(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_DUEL_OUTOFBOUNDS)
    }
}

/// SMSG_DUEL_INBOUNDS - Notify player they've returned to the duel boundary
///
/// Sent when a player returns to the duel boundary.
#[derive(Debug, Clone)]
pub struct SmsgDuelInBounds {}

impl ToWorldPacket for SmsgDuelInBounds {
    fn to_world_packet(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_DUEL_INBOUNDS)
    }
}

/// SMSG_DUEL_COMPLETE - Notify players that a duel has completed
///
/// Sent to both players when the duel ends.
#[derive(Debug, Clone)]
pub struct SmsgDuelComplete {
    /// Whether the duel completed normally
    pub completed: bool,
}

impl ToWorldPacket for SmsgDuelComplete {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_DUEL_COMPLETE);
        packet.write_u8(if self.completed { 1 } else { 0 });
        packet
    }
}

/// SMSG_DUEL_WINNER - Announce the duel winner
///
/// Sent to nearby players to announce the duel winner.
#[derive(Debug, Clone)]
pub struct SmsgDuelWinner<'a> {
    /// Whether the winner fled or won normally
    pub won: bool,
    /// Name of the winner
    pub winner_name: &'a str,
    /// Name of the loser
    pub loser_name: &'a str,
}

impl ToWorldPacket for SmsgDuelWinner<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_DUEL_WINNER);
        packet.write_u8(if self.won { 0 } else { 1 });
        packet.write_string(self.winner_name);
        packet.write_string(self.loser_name);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::Opcode;

    #[test]
    fn test_smsg_duel_requested() {
        let msg = SmsgDuelRequested {
            arbiter_guid: ObjectGuid::from_low(123),
            initiator_guid: ObjectGuid::from_low(456),
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_DUEL_REQUESTED);
    }

    #[test]
    fn test_smsg_duel_countdown() {
        let msg = SmsgDuelCountdown { countdown: 3 };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_DUEL_COUNTDOWN);
    }

    #[test]
    fn test_smsg_duel_complete() {
        let msg = SmsgDuelComplete { completed: true };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_DUEL_COMPLETE);
    }

    #[test]
    fn test_smsg_duel_winner() {
        let msg = SmsgDuelWinner {
            won: true,
            winner_name: "Player1",
            loser_name: "Player2",
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_DUEL_WINNER);
    }
}
