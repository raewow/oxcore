//! A bit-packed little-endian byte buffer for modern world packet bodies.

use super::{Opcode, WorldPacket};

/// A growable buffer that supports both bit and byte writes.
#[derive(Debug, Default)]
pub struct BitWriter {
    buf: Vec<u8>,
    cur: u8,
    bits: u8,
}

impl BitWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Write a single bit, most-significant first within its byte.
    pub fn write_bit(&mut self, bit: bool) {
        if bit {
            self.cur |= 1 << (7 - self.bits);
        }
        self.bits += 1;
        if self.bits == 8 {
            self.buf.push(self.cur);
            self.cur = 0;
            self.bits = 0;
        }
    }

    /// Write the low `count` bits of `value`, most-significant first.
    pub fn write_bits(&mut self, value: u32, count: u8) {
        for i in (0..count).rev() {
            self.write_bit((value >> i) & 1 != 0);
        }
    }

    /// Commit a partial byte, padding its unused low bits with zero.
    pub fn flush_bits(&mut self) {
        if self.bits > 0 {
            self.buf.push(self.cur);
            self.cur = 0;
            self.bits = 0;
        }
    }

    fn put(&mut self, bytes: &[u8]) {
        self.flush_bits();
        self.buf.extend_from_slice(bytes);
    }

    pub fn write_u8(&mut self, value: u8) {
        self.put(&[value]);
    }
    pub fn write_i16(&mut self, value: i16) {
        self.put(&value.to_le_bytes());
    }
    pub fn write_u16(&mut self, value: u16) {
        self.put(&value.to_le_bytes());
    }
    pub fn write_u32(&mut self, value: u32) {
        self.put(&value.to_le_bytes());
    }
    pub fn write_u64(&mut self, value: u64) {
        self.put(&value.to_le_bytes());
    }
    pub fn write_i32(&mut self, value: i32) {
        self.put(&value.to_le_bytes());
    }
    pub fn write_i64(&mut self, value: i64) {
        self.put(&value.to_le_bytes());
    }
    pub fn write_f32(&mut self, value: f32) {
        self.put(&value.to_le_bytes());
    }
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.put(bytes);
    }
    pub fn write_string_raw(&mut self, value: &str) {
        self.put(value.as_bytes());
    }
    pub fn write_cstring(&mut self, value: &str) {
        self.write_string_raw(value);
        self.write_u8(0);
    }

    /// Write a modern packed 128-bit GUID as `(high, low)`.
    pub fn write_packed_guid_128(&mut self, high: u64, low: u64) {
        let (low_mask, low_bytes) = pack_u64(low);
        let (high_mask, high_bytes) = pack_u64(high);
        self.write_u8(low_mask);
        self.write_u8(high_mask);
        self.write_bytes(&low_bytes);
        self.write_bytes(&high_bytes);
    }

    /// Finish writing and attach the supplied logical opcode.
    pub fn finish(mut self, opcode: Opcode) -> WorldPacket {
        self.flush_bits();
        let mut packet = WorldPacket::with_capacity(opcode, self.buf.len());
        packet.write_bytes(&self.buf);
        packet
    }

    pub fn into_bytes(mut self) -> Vec<u8> {
        self.flush_bits();
        self.buf
    }
}

fn pack_u64(value: u64) -> (u8, Vec<u8>) {
    let mut mask = 0;
    let mut bytes = Vec::new();
    for index in 0..8 {
        let byte = (value >> (index * 8)) as u8;
        if byte != 0 {
            mask |= 1 << index;
            bytes.push(byte);
        }
    }
    (mask, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_writes_flush_pending_bits_first() {
        let mut writer = BitWriter::new();
        writer.write_bit(true);
        writer.write_u32(0x11223344);
        assert_eq!(writer.into_bytes(), vec![0x80, 0x44, 0x33, 0x22, 0x11]);
    }

    #[test]
    fn packed_guid_omits_zero_bytes() {
        let mut writer = BitWriter::new();
        writer.write_packed_guid_128(0x0100, 0x020001);
        assert_eq!(writer.into_bytes(), vec![0x05, 0x02, 0x01, 0x02, 0x01]);
    }

    #[test]
    fn finish_flushes_bits_into_a_packet() {
        let mut writer = BitWriter::new();
        writer.write_bit(true);
        let packet = writer.finish(Opcode::SMSG_CHAR_ENUM);
        assert_eq!(packet.contents(), &[0x80]);
    }
}
