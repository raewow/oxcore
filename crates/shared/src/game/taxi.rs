/// Number of `u32` words in a taxi mask. Vanilla stores the mask as `uint32[8]`, so this is a word
/// count, not a byte count -- 256 node bits in total.
pub const TAXI_MASK_SIZE: usize = 8;

/// Bitmap of the flight points a player has discovered, one bit per taxi node.
///
/// Stored as `u32` words to match the vanilla wire format, which sends the mask as eight `u32`s.
/// It was previously `[u8; 8]`, which had two consequences: only 64 of the ~250 vanilla nodes could
/// be represented at all, and the serializer widened each *byte* to a `u32` -- putting node 8 at
/// wire bit 32, so every node past the first eight was reported to the client as a different one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxiMask(pub [u32; TAXI_MASK_SIZE]);

/// Node bits per stored word.
const BITS_PER_WORD: usize = 32;

impl TaxiMask {
    pub fn new() -> Self {
        Self([0u32; TAXI_MASK_SIZE])
    }

    pub fn clear(&mut self) {
        self.0 = [0u32; TAXI_MASK_SIZE];
    }

    pub fn set(&mut self, index: usize) {
        if index < Self::capacity() {
            self.0[index / BITS_PER_WORD] |= 1 << (index % BITS_PER_WORD);
        }
    }

    pub fn unset(&mut self, index: usize) {
        if index < Self::capacity() {
            self.0[index / BITS_PER_WORD] &= !(1 << (index % BITS_PER_WORD));
        }
    }

    pub fn is_set(&self, index: usize) -> bool {
        if index < Self::capacity() {
            (self.0[index / BITS_PER_WORD] & (1 << (index % BITS_PER_WORD))) != 0
        } else {
            false
        }
    }

    pub fn has_any(&self) -> bool {
        self.0.iter().any(|&word| word != 0)
    }

    /// How many nodes the mask can hold.
    pub const fn capacity() -> usize {
        TAXI_MASK_SIZE * BITS_PER_WORD
    }

    /// The mask as the `u32` words the vanilla body writes.
    pub fn as_array(&self) -> [u32; TAXI_MASK_SIZE] {
        self.0
    }

    /// The mask as a flat little-endian byte bitmap, which is what 1.14 sends.
    ///
    /// Little-endian is what makes the two formats agree: node `n` lands at bit `n % 8` of byte
    /// `n / 8` here, and at bit `n % 32` of word `n / 32` in the vanilla body -- the same wire
    /// position either way.
    pub fn as_bytes(&self) -> [u8; TAXI_MASK_SIZE * 4] {
        let mut out = [0u8; TAXI_MASK_SIZE * 4];
        for (word, chunk) in self.0.iter().zip(out.chunks_exact_mut(4)) {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        out
    }
}

impl Default for TaxiMask {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaxiNode {
    pub id: u32,
    pub name: String,
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub next_nodes: Vec<u32>,
    pub mount_creature_id: u32,
    pub cost: u32,
}

impl TaxiNode {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: String::new(),
            map_id: 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            next_nodes: Vec::new(),
            mount_creature_id: 0,
            cost: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaxiPath {
    pub from_node: u32,
    pub to_node: u32,
    pub id: u32,
    pub cost: u32,
    pub length: f32,
}

impl TaxiPath {
    pub fn new(from_node: u32, to_node: u32) -> Self {
        Self {
            from_node,
            to_node,
            id: 0,
            cost: 0,
            length: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaxiRoute {
    pub path: TaxiPath,
    pub nodes: Vec<TaxiNode>,
    pub total_cost: u32,
    pub total_length: f32,
}

impl TaxiRoute {
    pub fn new(path: TaxiPath) -> Self {
        Self {
            path,
            nodes: Vec::new(),
            total_cost: 0,
            total_length: 0.0,
        }
    }
}

#[cfg(test)]
mod taxi_mask_tests {
    use super::*;

    /// Vanilla has around 250 flight points. The mask was `[u8; 8]` — 64 bits — so more than half of
    /// them could not be recorded at all, and a player who discovered one silently kept losing it.
    #[test]
    fn the_mask_holds_every_vanilla_node() {
        assert_eq!(TaxiMask::capacity(), 256);

        let mut mask = TaxiMask::new();
        mask.set(200);
        assert!(mask.is_set(200), "a high node id must be storable");
    }

    /// The bug this fixes: a node lands at the same wire bit under both encodings. Previously the
    /// serializer widened each stored *byte* to a `u32`, so node 8 went out at bit 32 and the client
    /// read it as node 32.
    #[test]
    fn a_node_lands_at_the_same_wire_bit_in_both_encodings() {
        for node in [0usize, 7, 8, 31, 32, 63, 64, 200] {
            let mut mask = TaxiMask::new();
            mask.set(node);

            // Modern: a flat byte bitmap.
            let bytes = mask.as_bytes();
            assert_eq!(
                bytes[node / 8],
                1 << (node % 8),
                "node {node} in the byte bitmap"
            );

            // Vanilla: the same bits as u32 words, which serialise to the same bytes.
            let words = mask.as_array();
            assert_eq!(
                words[node / 32],
                1 << (node % 32),
                "node {node} in the word array"
            );

            let from_words: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            assert_eq!(
                from_words.as_slice(),
                bytes.as_slice(),
                "the two encodings must describe the same wire bytes"
            );
        }
    }

    /// Node 8 is the first one the old code got wrong, so it is worth an explicit guard.
    #[test]
    fn node_eight_is_not_reported_as_node_thirty_two() {
        let mut mask = TaxiMask::new();
        mask.set(8);

        let bytes = mask.as_bytes();
        assert_eq!(bytes[1], 0x01, "node 8 is bit 0 of byte 1");
        assert_eq!(bytes[4], 0x00, "and must not appear at bit 32");
    }
}
