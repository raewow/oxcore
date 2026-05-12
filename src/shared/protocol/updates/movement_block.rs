use crate::shared::protocol::{WorldPacket, packet::WorldPacketGuidExt};
use crate::shared::protocol::position::Position;
use anyhow::Result;
use tracing::{info, warn};

/// Update flags for movement blocks
pub mod update_flags {
    /// No special flags
    pub const UPDATEFLAG_NONE: u8 = 0x00;

    /// Building packet for yourself
    pub const UPDATEFLAG_SELF: u8 = 0x01;

    /// Unit is alive (includes movement block with position and speeds)
    pub const UPDATEFLAG_LIVING: u8 = 0x20;

    /// Has position data
    pub const UPDATEFLAG_HAS_POSITION: u8 = 0x40;

    /// All fields (used for items - writes uint32(1))
    pub const UPDATEFLAG_ALL: u8 = 0x10;

    /// Melee attacking
    pub const UPDATEFLAG_MELEE_ATTACKING: u8 = 0x04;

    /// High GUID
    pub const UPDATEFLAG_HIGHGUID: u8 = 0x08;

    /// Transport
    pub const UPDATEFLAG_TRANSPORT: u8 = 0x02;
}

/// Movement block helper for SMSG_UPDATE_OBJECT packets
///
/// Movement blocks vary based on update flags:
/// - UPDATEFLAG_ALL (0x10): Simple - just writes uint32(1) (used for items)
/// - UPDATEFLAG_LIVING (0x20): Full movement block with position, speeds, etc. (used for units/players)
/// - UPDATEFLAG_HAS_POSITION (0x40): Position only (used for game objects)
pub struct MovementBlock;

impl MovementBlock {
    /// Write a movement block for an item (UPDATEFLAG_ALL)
    ///
    /// For items, the movement block is simple: just writes uint32(1)
    /// See: core/src/game/Objects/Object.cpp:531-534
    pub fn write_for_item(packet: &mut WorldPacket) -> Result<()> {
        packet.write_u32(1);
        Ok(())
    }

    /// Write a full movement block for a living unit (UPDATEFLAG_LIVING)
    ///
    /// Writes:
    /// - Movement flags (u32)
    /// - Server timestamp (u32)
    /// - Position (4 floats: x, y, z, orientation)
    /// - Fall time (u32)
    /// - Movement speeds (6 floats: walk, run, run_back, swim, swim_back, turn_rate)
    /// - Update flag (u32) - typically UPDATEFLAG_ALL
    pub fn write_for_living_unit(
        packet: &mut WorldPacket,
        position: &Position,
        movement_flags: u32,
        speeds: Option<MovementSpeeds>,
    ) -> Result<()> {
        // Movement flags
        packet.write_u32(movement_flags);

        // Server uptime timestamp (matches WoW 1.12 client's time domain)
        let server_time = crate::shared::common::server_mstime();
        packet.write_u32(server_time);

        // Position (4 floats)
        // Validate and normalize orientation before writing
        let mut normalized_position = *position;
        let orientation_valid = normalized_position.validate_orientation();

        if !orientation_valid {
            warn!(
                "[MovementBlock] ⚠️ Invalid orientation in movement block: {:.4} (NaN or infinite). Normalized to {:.4}",
                position.o,
                normalized_position.o
            );
        }

        packet.write_f32(normalized_position.x);
        packet.write_f32(normalized_position.y);
        packet.write_f32(normalized_position.z);
        packet.write_f32(normalized_position.o);

        // Fall time (0 = not falling)
        packet.write_u32(0);

        // Movement speeds (6 floats)
        let speeds = speeds.unwrap_or_default();
        packet.write_f32(speeds.walk);
        packet.write_f32(speeds.run);
        packet.write_f32(speeds.run_back);
        packet.write_f32(speeds.swim);
        packet.write_f32(speeds.swim_back);
        packet.write_f32(speeds.turn_rate);

        // Movement block ends here - no extra u32 after speeds
        // Reference core shows movement blocks are 52 bytes (not 56)

        Ok(())
    }

    /// Write a position-only movement block (UPDATEFLAG_HAS_POSITION)
    ///
    /// Writes:
    /// - Position (4 floats: x, y, z, orientation)
    pub fn write_for_position(packet: &mut WorldPacket, position: &Position) -> Result<()> {
        // Validate and normalize orientation before writing
        let mut normalized_position = *position;
        let orientation_valid = normalized_position.validate_orientation();

        if !orientation_valid {
            warn!(
                "[MovementBlock] ⚠️ Invalid orientation in position block: {:.4} (NaN or infinite). Normalized to {:.4}",
                position.o,
                normalized_position.o
            );
        }

        packet.write_f32(normalized_position.x);
        packet.write_f32(normalized_position.y);
        packet.write_f32(normalized_position.z);
        packet.write_f32(normalized_position.o);
        Ok(())
    }

    /// Write movement block based on update flags
    ///
    /// Automatically determines which type of movement block to write based on flags
    pub fn write(
        packet: &mut WorldPacket,
        flags: u8,
        position: Option<&Position>,
        movement_flags: Option<u32>,
        speeds: Option<MovementSpeeds>,
    ) -> Result<()> {
        if flags & update_flags::UPDATEFLAG_ALL != 0 {
            // Items use UPDATEFLAG_ALL - simple block
            Self::write_for_item(packet)
        } else if flags & update_flags::UPDATEFLAG_LIVING != 0 {
            // Units/players use UPDATEFLAG_LIVING - full movement block
            let pos = position
                .ok_or_else(|| anyhow::anyhow!("Position required for UPDATEFLAG_LIVING"))?;
            Self::write_for_living_unit(packet, pos, movement_flags.unwrap_or(0), speeds)
        } else if flags & update_flags::UPDATEFLAG_HAS_POSITION != 0 {
            // Game objects use UPDATEFLAG_HAS_POSITION - position only
            let pos = position
                .ok_or_else(|| anyhow::anyhow!("Position required for UPDATEFLAG_HAS_POSITION"))?;
            Self::write_for_position(packet, pos)
        } else {
            // No movement block needed
            Ok(())
        }
    }
}

/// Movement speeds for living units
#[derive(Debug, Clone, Copy)]
pub struct MovementSpeeds {
    pub walk: f32,
    pub run: f32,
    pub run_back: f32,
    pub swim: f32,
    pub swim_back: f32,
    pub turn_rate: f32,
}

impl Default for MovementSpeeds {
    fn default() -> Self {
        Self {
            walk: 2.5f32,
            run: 7.0f32,
            run_back: 4.5f32,
            swim: 4.72f32,
            swim_back: 2.5f32,
            turn_rate: 3.14f32,
        }
    }
}

impl MovementSpeeds {
    /// Create custom movement speeds
    pub fn new(
        walk: f32,
        run: f32,
        run_back: f32,
        swim: f32,
        swim_back: f32,
        turn_rate: f32,
    ) -> Self {
        Self {
            walk,
            run,
            run_back,
            swim,
            swim_back,
            turn_rate,
        }
    }

    /// Create movement speeds for a player
    pub fn player() -> Self {
        Self::default()
    }

    /// Create movement speeds for a creature
    pub fn creature() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::opcodes::Opcode;

    #[test]
    fn test_item_movement_block() {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);
        MovementBlock::write_for_item(&mut packet).unwrap();

        let data = packet.data();
        assert_eq!(data.len(), 4); // Just the uint32(1)
        assert_eq!(data[0], 1);
        assert_eq!(data[1], 0);
        assert_eq!(data[2], 0);
        assert_eq!(data[3], 0);
    }

    #[test]
    fn test_living_unit_movement_block() {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);
        let position = Position {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            o: 4.0,
        };
        MovementBlock::write_for_living_unit(&mut packet, &position, 0, None).unwrap();

        let data = packet.data();
        // Should have: flags(4) + timestamp(4) + position(16) + fall_time(4) + speeds(24) = 52 bytes
        assert_eq!(data.len(), 52);
    }
}
