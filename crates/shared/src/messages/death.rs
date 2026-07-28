//! Death and resurrection messages
//!
//! All packets involved in the death and resurrection cycle.

use crate::messages::ToWorldPacket;
use crate::protocol::{ObjectGuid, Opcode, WorldPacket};

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
    fn to_vanilla(&self) -> WorldPacket {
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
    fn to_vanilla(&self) -> WorldPacket {
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SPIRIT_HEALER_CONFIRM);
        packet.write_u64(self.healer_guid.raw());
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resurrect_request_encodes_name_and_flag_bytes() {
        let msg = SmsgResurrectRequest {
            caster_guid: ObjectGuid::from_raw(0x1234),
            caster_name: String::new(),
            causes_sickness: false,
            use_corpse_timer: true,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_RESURRECT_REQUEST);

        let bytes = packet.contents();
        // guid (8) + name len u32 (4) + name (0) + null (1) + sickness (1) + timer (1)
        assert_eq!(bytes.len(), 8 + 4 + 0 + 1 + 1 + 1);
        assert_eq!(u64::from_le_bytes(bytes[0..8].try_into().unwrap()), 0x1234);
        assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 1); // empty name -> just null terminator
        assert_eq!(bytes[12], 0); // null terminator
        assert_eq!(bytes[13], 0); // causes_sickness = false
        assert_eq!(bytes[14], 1); // use_corpse_timer = true
    }

    #[test]
    fn resurrect_request_sickness_and_timer_bytes_reflect_flags() {
        let msg = SmsgResurrectRequest {
            caster_guid: ObjectGuid::from_raw(0),
            caster_name: "Bob".to_string(),
            causes_sickness: true,
            use_corpse_timer: false,
        };
        let bytes = msg.to_vanilla().contents().to_vec();
        assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 4); // "Bob" + null
        assert_eq!(&bytes[12..15], b"Bob");
        assert_eq!(bytes[15], 0); // null terminator
        assert_eq!(bytes[16], 1); // causes_sickness = true
        assert_eq!(bytes[17], 0); // use_corpse_timer = false
    }
}
