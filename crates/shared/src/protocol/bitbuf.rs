//! A bit-packed little-endian byte buffer for modern world packet bodies.

use super::{Opcode, WorldPacket};

/// A growable buffer that supports both bit and byte writes.
#[derive(Debug, Default)]
pub struct BitWriter {
    buf: Vec<u8>,
    cur: u8,
    bits: u8,
}

/// A cursor over a modern bit-packed packet body.
#[derive(Debug)]
pub struct BitReader<'a> {
    buf: &'a [u8],
    byte: usize,
    bit: u8,
}

impl<'a> BitReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            byte: 0,
            bit: 0,
        }
    }

    /// Bytes consumed so far, rounding a partially-read byte up to a whole one.
    ///
    /// Lets a caller that parsed a bit-packed prefix advance the underlying packet past it.
    pub fn consumed(&self) -> usize {
        self.byte + usize::from(self.bit != 0)
    }

    pub fn read_bit(&mut self) -> Option<bool> {
        let value = *self.buf.get(self.byte)? & (1 << (7 - self.bit)) != 0;
        self.bit += 1;
        if self.bit == 8 {
            self.byte += 1;
            self.bit = 0;
        }
        Some(value)
    }

    pub fn read_bits(&mut self, count: u8) -> Option<u32> {
        let mut value = 0;
        for _ in 0..count {
            value = (value << 1) | u32::from(self.read_bit()?);
        }
        Some(value)
    }

    /// Advance to the start of the next byte if a bit run left the cursor mid-byte. A no-op if
    /// already byte-aligned. Byte-level reads call this themselves; expose it for callers that
    /// need to force alignment between two bit runs with no byte read in between (the 1.14 wire
    /// format does this at the top of each `SpellWeight` entry, `ResetBitPos` in the reference).
    pub fn align(&mut self) {
        if self.bit != 0 {
            self.byte += 1;
            self.bit = 0;
        }
    }

    pub fn read_u8(&mut self) -> Option<u8> {
        self.align();
        let value = *self.buf.get(self.byte)?;
        self.byte += 1;
        Some(value)
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        self.read_array().map(u32::from_le_bytes)
    }

    pub fn read_i64(&mut self) -> Option<i64> {
        self.read_array().map(i64::from_le_bytes)
    }

    pub fn read_f32(&mut self) -> Option<f32> {
        self.read_array().map(f32::from_le_bytes)
    }

    pub fn read_bytes(&mut self, count: usize) -> Option<&'a [u8]> {
        self.align();
        let end = self.byte.checked_add(count)?;
        let value = self.buf.get(self.byte..end)?;
        self.byte = end;
        Some(value)
    }

    pub fn read_string(&mut self, count: usize) -> Option<String> {
        String::from_utf8(self.read_bytes(count)?.to_vec()).ok()
    }

    pub fn read_packed_guid_128(&mut self) -> Option<(u64, u64)> {
        let low_mask = self.read_u8()?;
        let high_mask = self.read_u8()?;
        let low = self.read_packed_u64(low_mask)?;
        let high = self.read_packed_u64(high_mask)?;
        Some((high, low))
    }

    fn read_array<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.read_bytes(N)?.try_into().ok()
    }

    fn read_packed_u64(&mut self, mask: u8) -> Option<u64> {
        let mut value = 0;
        for index in 0..8 {
            if mask & (1 << index) != 0 {
                value |= u64::from(self.read_u8()?) << (index * 8);
            }
        }
        Some(value)
    }
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

    #[test]
    fn reader_round_trips_bits_and_packed_guid() {
        let mut writer = BitWriter::new();
        writer.write_bits(17, 6);
        writer.write_bit(true);
        writer.write_packed_guid_128(0x0100, 0x020001);
        let bytes = writer.into_bytes();
        let mut reader = BitReader::new(&bytes);
        assert_eq!(reader.read_bits(6), Some(17));
        assert_eq!(reader.read_bit(), Some(true));
        assert_eq!(reader.read_packed_guid_128(), Some((0x0100, 0x020001)));
    }
}
