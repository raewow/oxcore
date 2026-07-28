//! Weather messages
//!
//! The client renders zone weather (rain, snow, sandstorm) from a single
//! server packet. There is no client request — the server pushes weather on
//! zone entry and whenever the zone weather changes.

use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::{Opcode, WorldPacket};

/// SMSG_WEATHER (0x02F4)
///
/// Packet layout (1.12):
///   weather_type:   u32 - WeatherType (0=fine, 1=rain, 2=snow, 3=storm)
///   grade:          f32 - Intensity, 0.0 .. 1.0 (exclusive)
///   sound_id:       u32 - Ambience sound to play (0 = silent)
///   instant_change: u8  - 1 = snap to the new weather, 0 = fade into it
#[derive(Debug, Clone)]
pub struct SmsgWeather {
    pub weather_type: u32,
    pub grade: f32,
    pub sound_id: u32,
    pub instant_change: bool,
}

impl ToWorldPacket for SmsgWeather {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_WEATHER);
        packet.write_u32(self.weather_type);
        packet.write_f32(self.grade);
        packet.write_u32(self.sound_id);
        packet.write_u8(u8::from(self.instant_change));
        packet
    }

    /// The modern body drops the sound id and turns the abrupt flag into a single flushed bit:
    /// weather id, intensity, then one bit. There is nowhere to carry `sound_id`, so it is simply
    /// not sent — the modern client derives ambience from the weather id itself.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.weather_type);
        writer.write_f32(self.grade);
        writer.write_bit(self.instant_change);
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_WEATHER))
    }
}
