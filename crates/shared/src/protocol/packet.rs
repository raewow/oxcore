//! WorldPacket - network packet for world server communication
//!
//! This is the shared packet type used by both world and world.

use super::Opcode;
use bytes::{Buf, BufMut, BytesMut};

#[derive(Debug, Clone)]
pub struct WorldPacket {
    opcode: Opcode,
    data: BytesMut,
}

impl WorldPacket {
    pub fn new(opcode: Opcode) -> Self {
        Self {
            opcode,
            data: BytesMut::new(),
        }
    }

    pub fn with_capacity(opcode: Opcode, capacity: usize) -> Self {
        Self {
            opcode,
            data: BytesMut::with_capacity(capacity),
        }
    }

    pub fn from_data(opcode: Opcode, data: BytesMut) -> Self {
        Self { opcode, data }
    }

    pub fn opcode(&self) -> Opcode {
        self.opcode
    }

    pub fn data(&self) -> &BytesMut {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut BytesMut {
        &mut self.data
    }

    /// Get packet contents as a slice (alias for data().as_ref())
    pub fn contents(&self) -> &[u8] {
        &self.data
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Drop `count` bytes from the front of the unread body.
    ///
    /// For parsers that read from [`Self::contents`] directly -- bit-packed modern bodies cannot go
    /// through the byte-oriented `read_*` methods -- so that the packet cursor still ends up where
    /// the parse finished.
    pub fn advance(&mut self, count: usize) {
        let count = count.min(self.data.len());
        let _ = self.data.split_to(count);
    }

    // ========================================================================
    // Read methods
    // ========================================================================

    pub fn read_u8(&mut self) -> Option<u8> {
        if self.data.remaining() >= 1 {
            Some(self.data.get_u8())
        } else {
            None
        }
    }

    pub fn read_u16(&mut self) -> Option<u16> {
        if self.data.remaining() >= 2 {
            Some(self.data.get_u16_le())
        } else {
            None
        }
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        if self.data.remaining() >= 4 {
            Some(self.data.get_u32_le())
        } else {
            None
        }
    }

    pub fn read_u64(&mut self) -> Option<u64> {
        if self.data.remaining() >= 8 {
            Some(self.data.get_u64_le())
        } else {
            None
        }
    }

    pub fn read_i8(&mut self) -> Option<i8> {
        if self.data.remaining() >= 1 {
            Some(self.data.get_i8())
        } else {
            None
        }
    }

    pub fn read_f32(&mut self) -> Option<f32> {
        if self.data.remaining() >= 4 {
            Some(self.data.get_f32_le())
        } else {
            None
        }
    }

    pub fn read_string(&mut self) -> Option<String> {
        let mut bytes = Vec::new();
        while let Some(byte) = self.read_u8() {
            if byte == 0 {
                return String::from_utf8(bytes).ok();
            }
            bytes.push(byte);
        }
        None
    }

    pub fn read_cstring(&mut self) -> Option<String> {
        self.read_string()
    }

    /// Read a raw GUID (u64, little-endian)
    pub fn read_guid_raw(&mut self) -> Option<u64> {
        self.read_u64()
    }

    /// Read a packed GUID from the packet (returns raw u64)
    ///
    /// Packed GUID format: [mask byte] [non-zero GUID bytes]
    pub fn read_packed_guid_raw(&mut self) -> Option<u64> {
        let mask = self.read_u8()?;

        let mut guid: u64 = 0;
        for i in 0..8 {
            if (mask & (1 << i)) != 0 {
                let byte = self.read_u8()?;
                guid |= (byte as u64) << (i * 8);
            }
        }

        Some(guid)
    }

    /// Skip n bytes in the packet
    pub fn read_skip(&mut self, n: usize) -> anyhow::Result<()> {
        if self.data.remaining() >= n {
            self.data.advance(n);
            Ok(())
        } else {
            anyhow::bail!(
                "Not enough data to skip {} bytes (remaining: {})",
                n,
                self.data.remaining()
            )
        }
    }

    // ========================================================================
    // Write methods
    // ========================================================================

    pub fn write_u8(&mut self, value: u8) {
        self.data.put_u8(value);
    }

    pub fn write_u16(&mut self, value: u16) {
        self.data.put_u16_le(value);
    }

    pub fn write_u32(&mut self, value: u32) {
        self.data.put_u32_le(value);
    }

    pub fn write_i32(&mut self, value: i32) {
        self.data.put_i32_le(value);
    }

    pub fn write_u64(&mut self, value: u64) {
        self.data.put_u64_le(value);
    }

    pub fn write_i8(&mut self, value: i8) {
        self.data.put_i8(value);
    }

    pub fn write_f32(&mut self, value: f32) {
        self.data.put_f32_le(value);
    }

    pub fn write_string(&mut self, value: &str) {
        self.data.put_slice(value.as_bytes());
        self.data.put_u8(0);
    }

    pub fn write_cstring(&mut self, value: &str) {
        self.write_string(value);
    }

    /// Write a raw GUID (u64, little-endian)
    pub fn write_guid_raw(&mut self, guid: u64) {
        self.write_u64(guid);
    }

    /// Write a packed GUID to the packet
    ///
    /// Packed GUID format: [mask byte] [non-zero GUID bytes]
    pub fn write_packed_guid_raw(&mut self, guid: u64) {
        let mut mask: u8 = 0;
        let mut guid_bytes = Vec::new();

        let mut temp_guid = guid;
        for i in 0..8 {
            if temp_guid == 0 {
                break;
            }

            let byte = (temp_guid & 0xFF) as u8;
            if byte != 0 {
                mask |= 1 << i;
                guid_bytes.push(byte);
            }

            temp_guid >>= 8;
        }

        self.write_u8(mask);
        for byte in guid_bytes {
            self.write_u8(byte);
        }
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.data.put_slice(bytes);
    }

    /// Write packed XYZ coordinates (used in SMSG_MONSTER_MOVE for intermediate waypoints)
    ///
    /// MaNGOS format: 11 bits X, 11 bits Y, 10 bits Z, packed into a single u32.
    /// Values are quantized to 0.25 unit resolution.
    pub fn write_pack_xyz(&mut self, x: f32, y: f32, z: f32) {
        let packed: u32 = ((x / 0.25) as i32 as u32 & 0x7FF)
            | (((y / 0.25) as i32 as u32 & 0x7FF) << 11)
            | (((z / 0.25) as i32 as u32 & 0x3FF) << 22);
        self.write_u32(packed);
    }
}

use super::bitbuf::BitReader;
use super::guid::ObjectGuid;
use super::Protocol;

/// Extension trait for WorldPacket to add ObjectGuid support
pub trait WorldPacketGuidExt {
    /// Read a GUID from the packet (u64, little-endian, unpacked)
    fn read_guid(&mut self) -> Option<ObjectGuid>;

    /// Read a packed GUID from the packet
    fn read_packed_guid(&mut self) -> Option<ObjectGuid>;

    /// Read a GUID in whichever form the session's protocol uses.
    ///
    /// The two are unrelated encodings, so this is a branch rather than a translation: 1.14 names
    /// every object with a packed 128-bit GUID, which then has to be folded back to the legacy
    /// 64-bit form the whole server is keyed on. Named and shaped like
    /// [`MovementInfo::read_for`](super::movement::MovementInfo::read_for), which solved the same
    /// problem for movement bodies.
    ///
    /// Use this at **every** inbound GUID read. Reading a modern body with [`Self::read_guid`]
    /// does not fail loudly -- it returns a plausible-looking GUID built from the wrong bytes and
    /// leaves the cursor in the wrong place, so the field after it is wrong too.
    fn read_guid_for(&mut self, protocol: Protocol) -> Option<ObjectGuid>;

    /// [`Self::read_guid_for`] for bodies whose vanilla form uses a *packed* GUID.
    ///
    /// Modern has only one GUID encoding, so the modern arm is identical; the vanilla arm is what
    /// differs. Kept separate so each call site keeps documenting which vanilla layout it expects.
    fn read_packed_guid_for(&mut self, protocol: Protocol) -> Option<ObjectGuid>;

    /// Write a GUID to the packet (u64, little-endian)
    fn write_guid(&mut self, guid: ObjectGuid);

    /// Write a packed GUID to the packet
    fn write_packed_guid(&mut self, guid: ObjectGuid);
}

impl WorldPacketGuidExt for WorldPacket {
    fn read_guid(&mut self) -> Option<ObjectGuid> {
        self.read_guid_raw().map(|v| {
            let mut guid = ObjectGuid::from_raw(v);
            guid.clamp_player_guid();
            guid
        })
    }

    fn read_packed_guid(&mut self) -> Option<ObjectGuid> {
        self.read_packed_guid_raw().map(|v| {
            let mut guid = ObjectGuid::from_raw(v);
            guid.clamp_player_guid();
            guid
        })
    }

    fn read_guid_for(&mut self, protocol: Protocol) -> Option<ObjectGuid> {
        match protocol {
            Protocol::Vanilla => self.read_guid(),
            Protocol::Modern => read_modern_guid(self),
        }
    }

    fn read_packed_guid_for(&mut self, protocol: Protocol) -> Option<ObjectGuid> {
        match protocol {
            Protocol::Vanilla => self.read_packed_guid(),
            Protocol::Modern => read_modern_guid(self),
        }
    }

    fn write_guid(&mut self, guid: ObjectGuid) {
        self.write_guid_raw(guid.raw());
    }

    fn write_packed_guid(&mut self, guid: ObjectGuid) {
        self.write_packed_guid_raw(guid.raw());
    }
}

/// Read one packed guid128 from the packet's cursor and advance past it.
///
/// A packed guid128 is byte-oriented but variable-length, so it cannot go through the fixed-width
/// `read_*` methods. Going via [`BitReader`] over [`WorldPacket::contents`] and then
/// [`WorldPacket::advance`] is the pattern that keeps the cursor correct -- without the advance the
/// guid parses but every field after it re-reads the guid's own bytes.
fn read_modern_guid(packet: &mut WorldPacket) -> Option<ObjectGuid> {
    let mut reader = BitReader::new(packet.contents());
    let (high, low) = reader.read_packed_guid_128()?;
    let consumed = reader.consumed();
    packet.advance(consumed);
    Some(ObjectGuid::from_guid128(high, low))
}

#[cfg(test)]
mod guid_read_tests {
    use super::*;
    use crate::protocol::bitbuf::BitWriter;
    use crate::protocol::guid::ObjectGuid;
    use crate::protocol::Opcode;

    fn modern_body(guid: ObjectGuid, trailing: u32) -> WorldPacket {
        let mut writer = BitWriter::new();
        let (high, low) = guid.to_guid128(1);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(trailing);
        writer.finish(Opcode::CMSG_QUESTGIVER_HELLO)
    }

    #[test]
    fn modern_read_recovers_the_creature_and_leaves_the_cursor_after_the_guid() {
        let creature = ObjectGuid::new_creature(299, 464);
        let mut packet = modern_body(creature, 0xDEAD_BEEF);

        assert_eq!(packet.read_guid_for(Protocol::Modern), Some(creature));
        // The reason the cursor matters: without the advance this reads the guid's own bytes back.
        assert_eq!(packet.read_u32(), Some(0xDEAD_BEEF));
    }

    #[test]
    fn vanilla_read_is_unchanged() {
        let creature = ObjectGuid::new_creature(299, 464);
        let mut packet = WorldPacket::new(Opcode::CMSG_QUESTGIVER_HELLO);
        packet.write_guid(creature);
        packet.write_u32(0xDEAD_BEEF);

        assert_eq!(packet.read_guid_for(Protocol::Vanilla), Some(creature));
        assert_eq!(packet.read_u32(), Some(0xDEAD_BEEF));
    }

    /// Attacker-controlled input: a truncated body must return `None`, not panic, and must not
    /// consume anything it did not parse.
    #[test]
    fn truncated_modern_guid_errors_rather_than_panicking() {
        let full = modern_body(ObjectGuid::new_creature(299, 464), 0);
        let bytes = full.contents().to_vec();

        for len in 0..bytes.len() {
            let mut packet = WorldPacket::from_data(
                Opcode::CMSG_QUESTGIVER_HELLO,
                bytes[..len].to_vec().as_slice().into(),
            );
            // Either it parses (a short prefix can still be a valid guid) or it declines. Neither
            // may panic.
            let _ = packet.read_guid_for(Protocol::Modern);
        }
    }

    #[test]
    fn empty_guid_survives_both_directions() {
        let mut packet = modern_body(ObjectGuid::empty(), 7);
        assert_eq!(
            packet.read_guid_for(Protocol::Modern),
            Some(ObjectGuid::empty())
        );
        assert_eq!(packet.read_u32(), Some(7));
    }
}
