//! Environment-related server messages
//!
//! These messages handle:
//! - Mirror timer updates (breath, fatigue)
//! - Environmental damage logging
//! - Exploration experience

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{Opcode, WorldPacket};

/// SMSG_START_MIRROR_TIMER - Start or update a client-visible timer bar
///
/// Packet layout:
///   timer_type: u32   - 0=Fatigue, 1=Breath, 2=FeignDeath
///   current:    u32   - Remaining time in milliseconds
///   max:        u32   - Maximum duration in milliseconds
///   scale:      i32   - Rate of change (-1=depleting, +1=recovering, 0=frozen)
///   paused:     u8    - 1 if timer is paused, 0 otherwise
///   spell_id:   u32   - Associated spell (0 for none)
pub struct SmsgStartMirrorTimer {
    pub timer_type: u32,
    pub current: u32,
    pub max: u32,
    pub scale: i32,
    pub paused: u8,
    pub spell_id: u32,
}

impl ToWorldPacket for SmsgStartMirrorTimer {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_START_MIRROR_TIMER);
        packet.write_u32(self.timer_type);
        packet.write_u32(self.current);
        packet.write_u32(self.max);
        packet.write_i32(self.scale);
        packet.write_u8(self.paused);
        packet.write_u32(self.spell_id);
        packet
    }
}

/// SMSG_STOP_MIRROR_TIMER - Remove a timer bar from the client UI
///
/// Packet layout:
///   timer_type: u32   - Which timer to stop
pub struct SmsgStopMirrorTimer {
    pub timer_type: u32,
}

impl ToWorldPacket for SmsgStopMirrorTimer {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_STOP_MIRROR_TIMER);
        packet.write_u32(self.timer_type);
        packet
    }
}

/// SMSG_ENVIRONMENTALDAMAGELOG - Environmental damage combat log entry
///
/// Packet layout:
///   guid:        u64  - Target player GUID
///   damage_type: u8   - EnvironmentalDamageType enum value
///   damage:      u32  - Raw damage dealt
///   absorb:      u32  - Damage absorbed by shields
///   resist:      u32  - Damage resisted by resistances
pub struct SmsgEnvironmentalDamageLog {
    pub guid: u64,
    pub damage_type: u8,
    pub damage: u32,
    pub absorb: u32,
    pub resist: u32,
}

impl ToWorldPacket for SmsgEnvironmentalDamageLog {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ENVIRONMENTALDAMAGELOG);
        packet.write_u64(self.guid);
        packet.write_u8(self.damage_type);
        packet.write_u32(self.damage);
        packet.write_u32(self.absorb);
        packet.write_u32(self.resist);
        packet
    }
}

/// SMSG_EXPLORATION_EXPERIENCE - Area exploration XP award
///
/// Packet layout:
///   area_id: u32  - Explored area ID
///   xp:      u32  - XP awarded for discovery
pub struct SmsgExplorationExperience {
    pub area_id: u32,
    pub xp: u32,
}

impl ToWorldPacket for SmsgExplorationExperience {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_EXPLORATION_EXPERIENCE);
        packet.write_u32(self.area_id);
        packet.write_u32(self.xp);
        packet
    }
}
