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

use super::guid::ObjectGuid;

/// Extension trait for WorldPacket to add ObjectGuid support
pub trait WorldPacketGuidExt {
    /// Read a GUID from the packet (u64, little-endian, unpacked)
    fn read_guid(&mut self) -> Option<ObjectGuid>;

    /// Read a packed GUID from the packet
    fn read_packed_guid(&mut self) -> Option<ObjectGuid>;

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

    fn write_guid(&mut self, guid: ObjectGuid) {
        self.write_guid_raw(guid.raw());
    }

    fn write_packed_guid(&mut self, guid: ObjectGuid) {
        self.write_packed_guid_raw(guid.raw());
    }
}
