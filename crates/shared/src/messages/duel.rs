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

use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::ObjectGuid;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;

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
    /// The 1.14 `DuelRequested` body: the two GUIDs become guid128s and a **third** GUID follows,
    /// naming the Battle.net game account of the player who asked for the duel.
    ///
    /// The third GUID is not optional padding -- it is read unconditionally, so omitting it leaves
    /// the client short and it never raises the duel dialog. We send it empty because vanilla has no
    /// Battle.net account behind a character; the client only uses it to group requests by account.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.arbiter_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        let (high, low) = self.initiator_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_packed_guid_128(0, 0); // RequestedByWowAccount -- no bnet account mapping
        Some(writer.finish(Opcode::SMSG_DUEL_REQUESTED))
    }

    fn to_vanilla(&self) -> WorldPacket {
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
    /// The 1.14 `DuelCountdown` body is identical to vanilla: one u32 of milliseconds.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
    }

    fn to_vanilla(&self) -> WorldPacket {
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
    /// Empty body in 1.14 as well, so the vanilla packet is byte-identical.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
    }

    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_DUEL_OUTOFBOUNDS)
    }
}

/// SMSG_DUEL_INBOUNDS - Notify player they've returned to the duel boundary
///
/// Sent when a player returns to the duel boundary.
#[derive(Debug, Clone)]
pub struct SmsgDuelInBounds {}

impl ToWorldPacket for SmsgDuelInBounds {
    /// Empty body in 1.14 as well, so the vanilla packet is byte-identical.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
    }

    fn to_vanilla(&self) -> WorldPacket {
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
    /// 1.14 carries the same flag as a single bit instead of a byte, so the body is one byte either
    /// way -- but the bit sits in the *high* bit of that byte, and a vanilla `1` byte would read as
    /// false. The flag itself keeps its meaning: set means the duel actually started.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_bit(self.completed); // Started
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_DUEL_COMPLETE))
    }

    fn to_vanilla(&self) -> WorldPacket {
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
    /// The 1.14 `DuelWinner` body, which rearranges every part of this message.
    ///
    /// Three traps, in order of how badly they read:
    ///
    /// * The flag is inverted. Vanilla writes `0` for a clean win; 1.14 carries a `Fled` bit, so
    ///   passing `won` through unchanged announces "X has fled from Y" on every honest kill.
    /// * The **beaten** name comes first, and its length is the first field of the bit run. Writing
    ///   the winner first swaps the two names in the announcement.
    /// * Both lengths are 6 bits and precede the two realm addresses; the string bytes come after
    ///   them, not inline.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let beaten = self.loser_name.as_bytes();
        let winner = self.winner_name.as_bytes();

        writer.write_bits(beaten.len() as u32, 6);
        writer.write_bits(winner.len() as u32, 6);
        writer.write_bit(!self.won); // Fled
        writer.flush_bits();

        // Both players are on the realm serving this session; vanilla has no cross-realm duels.
        writer.write_u32(0); // BeatenVirtualRealmAddress
        writer.write_u32(0); // WinnerVirtualRealmAddress

        writer.write_bytes(beaten);
        writer.write_bytes(winner);
        Some(writer.finish(Opcode::SMSG_DUEL_WINNER))
    }

    fn to_vanilla(&self) -> WorldPacket {
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
    use crate::protocol::Opcode;

    #[test]
    fn test_smsg_duel_requested() {
        let msg = SmsgDuelRequested {
            arbiter_guid: ObjectGuid::from_low(123),
            initiator_guid: ObjectGuid::from_low(456),
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_DUEL_REQUESTED);
    }

    #[test]
    fn test_smsg_duel_countdown() {
        let msg = SmsgDuelCountdown { countdown: 3 };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_DUEL_COUNTDOWN);
    }

    #[test]
    fn test_smsg_duel_complete() {
        let msg = SmsgDuelComplete { completed: true };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_DUEL_COMPLETE);
    }

    #[test]
    fn test_smsg_duel_winner() {
        let msg = SmsgDuelWinner {
            won: true,
            winner_name: "Player1",
            loser_name: "Player2",
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_DUEL_WINNER);
    }
}
