use crate::shared::protocol::{WorldPacket, packet::WorldPacketGuidExt};
use std::collections::HashMap;

/// Internal enum for field value types
#[derive(Debug, Clone, Copy)]
enum FieldValue {
    U32(u32),
    Float(f32),
    Bytes([u8; 4]),
}

/// UpdateMask helper for building update masks and writing them to packets.
///
/// An UpdateMask is a variable-length way of sending known fields to the client.
/// It consists of:
/// 1. A u8 indicating how many u32 mask blocks follow
/// 2. The mask blocks (u32s) where each bit indicates if a field is present
/// 3. The field values in ascending field index order (u32, f32, or 4 bytes)
#[derive(Debug, Clone)]
pub struct UpdateMask {
    /// Map of field index -> field value (u32)
    fields: HashMap<u32, u32>,
    /// Map of field index -> float value (for fields like OBJECT_FIELD_SCALE_X)
    float_fields: HashMap<u32, f32>,
    /// Map of field index -> 4 bytes (for fields like UNIT_FIELD_BYTES_0)
    bytes_fields: HashMap<u32, [u8; 4]>,
}

impl UpdateMask {
    /// Create a new empty UpdateMask
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            float_fields: HashMap::new(),
            bytes_fields: HashMap::new(),
        }
    }

    /// Set a field value by field index
    /// If value is 0, the field is not added to the mask (matching reference core behavior)
    /// Use set_field_required() if you need to include 0 values (e.g., OBJECT_FIELD_ENTRY for players)
    pub fn set_field(&mut self, field_index: u32, value: u32) {
        // Reference core's _SetCreateBits only sets bits if value != 0
        // Skip 0 values to match reference core behavior
        if value != 0 {
            self.fields.insert(field_index, value);
        }
    }

    /// Set a field value by field index, even if it's 0
    /// Use this for required fields that must be included even when 0 (e.g., OBJECT_FIELD_ENTRY)
    pub fn set_field_required(&mut self, field_index: u32, value: u32) {
        self.fields.insert(field_index, value);
    }

    /// Set a GUID field (takes 2 consecutive field indices)
    /// Low part is always included. High part is only included if non-zero (matching reference core behavior)
    pub fn set_guid(&mut self, field_index: u32, guid_low: u32, guid_high: u32) {
        // Low part is always included (required field)
        self.fields.insert(field_index, guid_low);
        // High part is only included if non-zero (matching reference core behavior)
        // This matches how the working core handles GUID fields - it skips the high part if it's 0
        if guid_high != 0 {
            self.fields.insert(field_index + 1, guid_high);
        }
    }

    /// Set a float field (e.g., OBJECT_FIELD_SCALE_X)
    /// The value will be written as f32 instead of u32
    pub fn set_float_field(&mut self, field_index: u32, value: f32) {
        self.float_fields.insert(field_index, value);
    }

    /// Set a bytes field (e.g., UNIT_FIELD_BYTES_0)
    /// The value will be written as 4 separate bytes instead of u32
    pub fn set_bytes_field(&mut self, field_index: u32, bytes: [u8; 4]) {
        self.bytes_fields.insert(field_index, bytes);
    }

    /// Get a field value by field index
    pub fn get_field(&self, field_index: u32) -> Option<u32> {
        self.fields.get(&field_index).copied()
    }

    /// Check if a field is set
    pub fn has_field(&self, field_index: u32) -> bool {
        self.fields.contains_key(&field_index)
    }

    /// Get the number of fields set (including u32, float, and bytes fields)
    pub fn field_count(&self) -> usize {
        self.fields.len() + self.float_fields.len() + self.bytes_fields.len()
    }

    /// Calculate the number of mask blocks needed
    pub fn block_count(&self) -> u8 {
        let all_indices: Vec<u32> = self
            .fields
            .keys()
            .chain(self.float_fields.keys())
            .chain(self.bytes_fields.keys())
            .copied()
            .collect();

        if all_indices.is_empty() {
            return 0;
        }

        let highest_key = *all_indices.iter().max().unwrap();
        let blocks = highest_key / 32;
        let extra = if highest_key % 32 != 0 { 1 } else { 0 };
        (blocks + extra) as u8
    }

    /// Build the mask blocks from the fields
    ///
    /// `min_blocks`: Minimum number of blocks to write (for units, should be 6 to match UNIT_END)
    pub fn build_mask_blocks_with_min(&self, min_blocks: u8) -> Vec<u32> {
        let calculated_blocks = self.block_count() as usize;
        let block_count = calculated_blocks.max(min_blocks as usize);
        let mut blocks = vec![0u32; block_count];

        // Add bits for all field types
        for &field_index in self.fields.keys() {
            let block_index = (field_index / 32) as usize;
            let bit_index = (field_index % 32) as u32;
            if block_index < blocks.len() {
                blocks[block_index] |= 1 << bit_index;
            }
        }

        for &field_index in self.float_fields.keys() {
            let block_index = (field_index / 32) as usize;
            let bit_index = (field_index % 32) as u32;
            if block_index < blocks.len() {
                blocks[block_index] |= 1 << bit_index;
            }
        }

        for &field_index in self.bytes_fields.keys() {
            let block_index = (field_index / 32) as usize;
            let bit_index = (field_index % 32) as u32;
            if block_index < blocks.len() {
                blocks[block_index] |= 1 << bit_index;
            }
        }

        blocks
    }

    /// Build the mask blocks from the fields (uses calculated block count)
    pub fn build_mask_blocks(&self) -> Vec<u32> {
        self.build_mask_blocks_with_min(0)
    }

    /// Write the UpdateMask to a packet
    ///
    /// Writes:
    /// 1. u8: number of mask blocks
    /// 2. u32[]: mask blocks
    /// 3. Field values in ascending field index order (u32, f32, or 4 bytes)
    pub fn write_to_packet(&self, packet: &mut WorldPacket) {
        self.write_to_packet_with_min_blocks(packet, 0)
    }

    /// Write the UpdateMask to a packet with a minimum block count
    ///
    /// `min_blocks`: Minimum number of blocks to write (for units, use 6 to match UNIT_END)
    pub fn write_to_packet_with_min_blocks(&self, packet: &mut WorldPacket, min_blocks: u8) {
        let total_fields = self.fields.len() + self.float_fields.len() + self.bytes_fields.len();
        if total_fields == 0 {
            packet.write_u8(0);
            return;
        }

        let calculated_blocks = self.block_count();
        let block_count = calculated_blocks.max(min_blocks);
        let mask_blocks = self.build_mask_blocks_with_min(min_blocks);

        let mut all_field_indices: Vec<u32> = self
            .fields
            .keys()
            .chain(self.float_fields.keys())
            .chain(self.bytes_fields.keys())
            .copied()
            .collect();
        all_field_indices.sort();

        // Write number of mask blocks
        packet.write_u8(block_count);

        // Write mask blocks
        for (_i, block) in mask_blocks.iter().enumerate() {
            packet.write_u32(*block);
        }

        // Collect all fields and sort by index
        let mut all_fields: Vec<(u32, FieldValue)> = Vec::new();
        for (idx, val) in &self.fields {
            all_fields.push((*idx, FieldValue::U32(*val)));
        }
        for (idx, val) in &self.float_fields {
            all_fields.push((*idx, FieldValue::Float(*val)));
        }
        for (idx, val) in &self.bytes_fields {
            all_fields.push((*idx, FieldValue::Bytes(*val)));
        }
        all_fields.sort_by_key(|(k, _)| *k);

        for (_field_idx, value) in &all_fields {
            match value {
                FieldValue::U32(v) => {
                    packet.write_u32(*v);
                }
                FieldValue::Float(v) => {
                    packet.write_f32(*v);
                }
                FieldValue::Bytes(bytes) => {
                    packet.write_u8(bytes[0]);
                    packet.write_u8(bytes[1]);
                    packet.write_u8(bytes[2]);
                    packet.write_u8(bytes[3]);
                }
            }
        }
    }

    /// Get sorted field indices and values (for manual writing)
    /// Returns a vector of (field_index, value) tuples sorted by field index
    /// Note: This only returns u32 fields. Use get_all_sorted_fields() for all field types.
    pub fn get_sorted_fields(&self) -> Vec<(u32, u32)> {
        let mut sorted: Vec<(u32, u32)> = self.fields.iter().map(|(k, v)| (*k, *v)).collect();
        sorted.sort_by_key(|(k, _)| *k);
        sorted
    }

    /// Get all field indices (u32, float, and bytes fields)
    /// Returns a sorted vector of all field indices that are set
    pub fn get_all_field_indices(&self) -> Vec<u32> {
        let mut indices: Vec<u32> = self
            .fields
            .keys()
            .chain(self.float_fields.keys())
            .chain(self.bytes_fields.keys())
            .copied()
            .collect();
        indices.sort();
        indices
    }

    /// Calculate the size in bytes of this UpdateMask when written to a packet
    pub fn size(&self) -> usize {
        let total_fields = self.fields.len() + self.float_fields.len() + self.bytes_fields.len();
        if total_fields == 0 {
            return 1; // Just the u8 block count
        }

        let block_count = self.block_count() as usize;
        let u32_size = self.fields.len() * 4;
        let float_size = self.float_fields.len() * 4;
        let bytes_size = self.bytes_fields.len() * 4;

        // u8 block count + mask blocks (u32 each) + field values
        1 + (block_count * 4) + u32_size + float_size + bytes_size
    }

    /// Check if the mask is empty
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty() && self.float_fields.is_empty() && self.bytes_fields.is_empty()
    }

    /// Clear all fields
    pub fn clear(&mut self) {
        self.fields.clear();
        self.float_fields.clear();
        self.bytes_fields.clear();
    }
}

impl Default for UpdateMask {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_mask() {
        let mask = UpdateMask::new();
        assert_eq!(mask.block_count(), 0);
        assert_eq!(mask.size(), 1);
    }

    #[test]
    fn test_multiple_blocks() {
        let mut mask = UpdateMask::new();
        mask.set_field(0, 1);
        mask.set_field(35, 2); // Field 35 is in block 1 (35 / 32 = 1)
        assert_eq!(mask.block_count(), 2);
        assert_eq!(mask.size(), 1 + (2 * 4) + (2 * 4)); // u8 + 2 blocks + 2 values
    }

    #[test]
    fn test_guid_field() {
        let mut mask = UpdateMask::new();
        mask.set_guid(0, 0x12345678, 0x9ABCDEF0);
        assert_eq!(mask.get_field(0), Some(0x12345678));
        assert_eq!(mask.get_field(1), Some(0x9ABCDEF0));
        assert_eq!(mask.field_count(), 2);
    }

    #[test]
    fn test_field_ordering() {
        let mut mask = UpdateMask::new();
        mask.set_field(10, 100);
        mask.set_field(5, 50);
        mask.set_field(1, 10);

        let blocks = mask.build_mask_blocks();
        assert_eq!(blocks.len(), 1);
        // Bits 1, 5, and 10 should be set
        assert_eq!(blocks[0] & (1 << 1), 1 << 1);
        assert_eq!(blocks[0] & (1 << 5), 1 << 5);
        assert_eq!(blocks[0] & (1 << 10), 1 << 10);
    }
}
