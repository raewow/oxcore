//! Death and resurrection messages
//!
//! All packets involved in the death and resurrection cycle.

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};

/// SMSG_CORPSE_RECLAIM_DELAY (0x0269)
///
/// Sent after death to tell the client how long before the corpse can be
/// reclaimed. The client greys out the "Resurrect" button until this timer
/// expires.
#[derive(Debug, Clone)]
pub struct SmsgCorpseReclaimDelay {
    pub delay_ms: u32,
}

impl ToWorldPacket for SmsgCorpseReclaimDelay {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CORPSE_RECLAIM_DELAY);
        packet.write_u32(self.delay_ms);
        packet
    }
}

/// SMSG_RESURRECT_REQUEST
///
/// Displays the "X wants to resurrect you" popup dialog on the dead
/// player's screen.
#[derive(Debug, Clone)]
pub struct SmsgResurrectRequest {
    pub caster_guid: ObjectGuid,
    pub caster_name: String,
    pub causes_sickness: bool,
    pub use_corpse_timer: bool,
}

impl ToWorldPacket for SmsgResurrectRequest {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_RESURRECT_REQUEST);
        packet.write_u64(self.caster_guid.raw());
        let name_bytes = self.caster_name.as_bytes();
        packet.write_u32((name_bytes.len() + 1) as u32);
        for &b in name_bytes {
            packet.write_u8(b);
        }
        packet.write_u8(0); // null terminator
        packet.write_u8(if self.causes_sickness { 1 } else { 0 });
        packet.write_u8(if self.use_corpse_timer { 1 } else { 0 });
        packet
    }
}

/// SMSG_SPIRIT_HEALER_CONFIRM
///
/// Sent when a player interacts with a spirit healer.
/// Shows the confirmation dialog for resurrection.
#[derive(Debug, Clone)]
pub struct SmsgSpiritHealerConfirm {
    pub healer_guid: ObjectGuid,
}

impl ToWorldPacket for SmsgSpiritHealerConfirm {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SPIRIT_HEALER_CONFIRM);
        packet.write_u64(self.healer_guid.raw());
        packet
    }
}
