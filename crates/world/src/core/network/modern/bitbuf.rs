//! A bit-packed little-endian byte buffer, matching TrinityCore/CypherCore's `ByteBuffer`.
//!
//! Modern packets interleave single bits and bit-fields with byte-aligned values. Bits accumulate
//! MSB-first into a partial byte; a byte-aligned write (or [`BitWriter::flush_bits`]) commits the
//! partial byte first. The client reads with the same convention, so the two stay in step as long
//! as we mirror its write order exactly.

/// A growable buffer that supports both bit and byte writes.
#[derive(Debug, Default)]
pub struct BitWriter {
    buf: Vec<u8>,
    /// The partial byte being assembled from `write_bit`.
    cur: u8,
    /// How many bits of `cur` are used (0..8).
    bits: u8,
}

impl BitWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Write a single bit (MSB-first within the current partial byte).
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

    /// Commit any partial byte (padding the unused low bits with zero). A no-op when byte-aligned.
    pub fn flush_bits(&mut self) {
        if self.bits > 0 {
            self.buf.push(self.cur);
            self.cur = 0;
            self.bits = 0;
        }
    }

    /// Every byte-aligned write first flushes pending bits, so callers never have to remember to.
    fn put(&mut self, bytes: &[u8]) {
        self.flush_bits();
        self.buf.extend_from_slice(bytes);
    }

    pub fn write_u8(&mut self, v: u8) {
        self.put(&[v]);
    }
    pub fn write_u16(&mut self, v: u16) {
        self.put(&v.to_le_bytes());
    }
    pub fn write_u32(&mut self, v: u32) {
        self.put(&v.to_le_bytes());
    }
    pub fn write_i32(&mut self, v: i32) {
        self.put(&v.to_le_bytes());
    }
    pub fn write_i64(&mut self, v: i64) {
        self.put(&v.to_le_bytes());
    }
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.put(bytes);
    }

    /// Finish writing and return the buffer, flushing any trailing bits.
    pub fn into_bytes(mut self) -> Vec<u8> {
        self.flush_bits();
        self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_bit_is_the_high_bit() {
        let mut w = BitWriter::new();
        w.write_bit(true); // bit 7
        w.write_bit(false); // bit 6
        let out = w.into_bytes();
        assert_eq!(out, vec![0x80]);
    }

    #[test]
    fn two_true_bits_pack_into_the_top_two_bits() {
        let mut w = BitWriter::new();
        w.write_bit(true);
        w.write_bit(true);
        assert_eq!(w.into_bytes(), vec![0xC0]);
    }

    #[test]
    fn byte_writes_flush_pending_bits_first() {
        let mut w = BitWriter::new();
        w.write_bit(true); // pending: 0x80
        w.write_u32(0x11223344); // flushes the bit byte, then the u32 (LE)
        assert_eq!(w.into_bytes(), vec![0x80, 0x44, 0x33, 0x22, 0x11]);
    }

    #[test]
    fn write_bits_is_most_significant_first() {
        let mut w = BitWriter::new();
        w.write_bits(0b101, 3); // 1,0,1 into bits 7,6,5 -> 0b1010_0000
        assert_eq!(w.into_bytes(), vec![0xA0]);
    }

    #[test]
    fn a_full_byte_of_bits_commits_without_an_explicit_flush() {
        let mut w = BitWriter::new();
        for _ in 0..8 {
            w.write_bit(true);
        }
        // Already committed; a following byte write appends after it.
        w.write_u8(0x01);
        assert_eq!(w.into_bytes(), vec![0xFF, 0x01]);
    }
}
