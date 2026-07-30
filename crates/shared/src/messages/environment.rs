//! Environment-related server messages
//!
//! These messages handle:
//! - Mirror timer updates (breath, fatigue)
//! - Environmental damage logging
//! - Exploration experience

use crate::messages::update::DEFAULT_REALM_ID;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::ObjectGuid;
use crate::messages::ToWorldPacket;
use crate::protocol::{Opcode, WorldPacket};

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
    /// `StartMirrorTimer::Write` for build 42597.
    ///
    /// Same five values, but the spell id moves **before** the paused flag and paused becomes a bit.
    /// Vanilla writes paused as a byte then the spell id; sending that order leaves the client reading
    /// the spell id as the pause state.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_i32(self.timer_type as i32);
        writer.write_i32(self.current as i32);
        writer.write_i32(self.max as i32);
        writer.write_i32(self.scale);
        writer.write_i32(self.spell_id as i32);
        writer.write_bit(self.paused != 0);
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_START_MIRROR_TIMER))
    }

    fn to_vanilla(&self) -> WorldPacket {
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
    /// `StopMirrorTimer::Write` for build 42597:
    /// identical to vanilla, one i32.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
    }

    fn to_vanilla(&self) -> WorldPacket {
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
    /// `EnvironmentalDamageLog::Write` for build 42597.
    ///
    /// Resisted and absorbed **swap order** relative to vanilla, and a log-data bit is appended.
    /// Swapping them back is not cosmetic: fire damage that was fully resisted would display as fully
    /// absorbed.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = ObjectGuid::from_raw(self.guid).to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u8(self.damage_type);
        writer.write_i32(self.damage as i32);
        writer.write_i32(self.resist as i32); // Resisted comes first in 1.14
        writer.write_i32(self.absorb as i32);
        writer.write_bit(false); // HasLogData
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_ENVIRONMENTALDAMAGELOG))
    }

    fn to_vanilla(&self) -> WorldPacket {
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
    /// `ExplorationExperience::Write` for build 42597:
    /// identical to vanilla, area then experience.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_EXPLORATION_EXPERIENCE);
        packet.write_u32(self.area_id);
        packet.write_u32(self.xp);
        packet
    }
}
