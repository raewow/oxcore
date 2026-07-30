//! Modern update masks and field arrays.

use super::field_map::{self, ModernSlot};
use super::repack::repack;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::ObjectGuid;
use crate::protocol::updates::update_types::ObjectTypeId;
use std::collections::HashMap;

/// Object type as 1.14 numbers them.
///
/// Not the same as vanilla's `TYPEID_*`: 1.14 inserts `ActivePlayer` at 5, shifting `GameObject`
/// and everything after it up by one. Converting with `as u8` between the two would put creatures
/// and gameobjects in each other's slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModernObjectType {
    Object = 0,
    Item = 1,
    Container = 2,
    Unit = 3,
    Player = 4,
    /// The client's own player. Carries the private fields nobody else may see.
    ActivePlayer = 5,
    GameObject = 6,
    DynamicObject = 7,
    Corpse = 8,
}

impl ModernObjectType {
    /// Translate a vanilla type id, promoting the recipient's own player to `ActivePlayer`.
    ///
    /// The promotion matters: `ActivePlayer` selects a much larger field table, and sending a
    /// player their own object as plain `Player` costs them every self-only field (xp, coinage,
    /// inventory, skills).
    pub fn from_vanilla(type_id: ObjectTypeId, is_self: bool) -> Self {
        match type_id {
            ObjectTypeId::Object => Self::Object,
            ObjectTypeId::Item => Self::Item,
            ObjectTypeId::Container => Self::Container,
            ObjectTypeId::Unit => Self::Unit,
            ObjectTypeId::Player if is_self => Self::ActivePlayer,
            ObjectTypeId::Player => Self::Player,
            ObjectTypeId::GameObject => Self::GameObject,
            ObjectTypeId::DynamicObject => Self::DynamicObject,
            ObjectTypeId::Corpse => Self::Corpse,
        }
    }

    /// The `HeirFlags` word: one bit per type in this type's inheritance chain.
    ///
    /// Every object is at least an `Object`, and the chain accumulates -- an `ActivePlayer` is also
    /// a `Player`, a `Unit` and an `Object`.
    pub fn type_mask(self) -> i32 {
        const OBJECT: i32 = 0x001;
        const ITEM: i32 = 0x002;
        const CONTAINER: i32 = 0x004;
        const UNIT: i32 = 0x008;
        const PLAYER: i32 = 0x010;
        const ACTIVE_PLAYER: i32 = 0x020;
        const GAME_OBJECT: i32 = 0x040;
        const DYNAMIC_OBJECT: i32 = 0x080;
        const CORPSE: i32 = 0x100;

        OBJECT
            | match self {
                Self::Object => 0,
                Self::Item => ITEM,
                Self::Container => ITEM | CONTAINER,
                Self::Unit => UNIT,
                Self::Player => UNIT | PLAYER,
                Self::ActivePlayer => UNIT | PLAYER | ACTIVE_PLAYER,
                Self::GameObject => GAME_OBJECT,
                Self::DynamicObject => DYNAMIC_OBJECT,
                Self::Corpse => CORPSE,
            }
    }

    /// Whether this type inherits from `Unit`, which decides between a movement block and a
    /// stationary position in create blocks.
    pub fn is_unit(self) -> bool {
        matches!(self, Self::Unit | Self::Player | Self::ActivePlayer)
    }

    /// Vanilla slot -> modern slot table for this type.
    pub fn field_map(self) -> &'static [ModernSlot] {
        match self {
            // An `Object`-typed update carries only the shared header fields, which every other
            // table also starts with, so Unit's prefix serves.
            Self::Object | Self::Unit => &field_map::UNIT_MAP,
            Self::Item => &field_map::ITEM_MAP,
            Self::Container => &field_map::CONTAINER_MAP,
            Self::Player => &field_map::PLAYER_MAP,
            Self::ActivePlayer => &field_map::ACTIVE_PLAYER_MAP,
            Self::GameObject => &field_map::GAME_OBJECT_MAP,
            Self::DynamicObject => &field_map::DYNAMIC_OBJECT_MAP,
            Self::Corpse => &field_map::CORPSE_MAP,
        }
    }

    /// Modern field-slot count, which fixes the mask width.
    pub fn field_count(self) -> u16 {
        match self {
            Self::Object | Self::Unit => field_map::UNIT_FIELD_COUNT,
            Self::Item => field_map::ITEM_FIELD_COUNT,
            Self::Container => field_map::CONTAINER_FIELD_COUNT,
            Self::Player => field_map::PLAYER_FIELD_COUNT,
            Self::ActivePlayer => field_map::ACTIVE_PLAYER_FIELD_COUNT,
            Self::GameObject => field_map::GAME_OBJECT_FIELD_COUNT,
            Self::DynamicObject => field_map::DYNAMIC_OBJECT_FIELD_COUNT,
            Self::Corpse => field_map::CORPSE_FIELD_COUNT,
        }
    }

    /// Modern dynamic-field count. We never populate dynamic fields, but the client still reads a
    /// mask of this width after the static block.
    pub fn dynamic_field_count(self) -> u16 {
        match self {
            Self::Object | Self::Unit => field_map::UNIT_DYNAMIC_FIELD_COUNT,
            Self::Item => field_map::ITEM_DYNAMIC_FIELD_COUNT,
            Self::Container => field_map::CONTAINER_DYNAMIC_FIELD_COUNT,
            Self::Player => field_map::PLAYER_DYNAMIC_FIELD_COUNT,
            Self::ActivePlayer => field_map::ACTIVE_PLAYER_DYNAMIC_FIELD_COUNT,
            Self::GameObject => field_map::GAME_OBJECT_DYNAMIC_FIELD_COUNT,
            Self::DynamicObject => field_map::DYNAMIC_OBJECT_DYNAMIC_FIELD_COUNT,
            Self::Corpse => field_map::CORPSE_DYNAMIC_FIELD_COUNT,
        }
    }
}

/// Presence bitmask for modern update fields.
///
/// Unlike vanilla's, this is always written at the object type's full width -- the client sizes its
/// read from the leading block count, so trimming to the highest set bit would still parse, but
/// the mask is always sent at full width.
#[derive(Debug, Clone)]
pub struct ModernUpdateMask {
    blocks: Vec<u32>,
}

impl ModernUpdateMask {
    pub fn new(field_count: u16) -> Self {
        Self {
            blocks: vec![0; block_count(field_count)],
        }
    }

    /// Mark a modern slot present. Out-of-range indices are ignored rather than panicking: they
    /// mean the field map pointed past this type's table, which should drop the field, not kill
    /// the connection.
    pub fn set(&mut self, index: u16) {
        let index = index as usize;
        if let Some(block) = self.blocks.get_mut(index / 32) {
            *block |= 1 << (index % 32);
        }
    }

    pub fn is_set(&self, index: u16) -> bool {
        let index = index as usize;
        self.blocks
            .get(index / 32)
            .is_some_and(|block| block & (1 << (index % 32)) != 0)
    }

    pub fn write_to(&self, writer: &mut BitWriter) {
        writer.write_u8(self.blocks.len() as u8);
        for block in &self.blocks {
            writer.write_u32(*block);
        }
    }
}

/// How many 32-bit blocks cover `field_count` slots.
fn block_count(field_count: u16) -> usize {
    field_count.div_ceil(32) as usize
}

/// A modern object's field values plus the mask saying which are present.
#[derive(Debug, Clone)]
pub struct ModernFieldsArray {
    object_type: ModernObjectType,
    values: Vec<u32>,
    mask: ModernUpdateMask,
    /// Realm used to widen 64-bit GUID fields to 128 bits.
    realm_id: u16,
    /// Vanilla GUID halves seen so far, keyed by the modern slot their widened form starts at.
    ///
    /// A vanilla GUID arrives as two separate `set_vanilla` calls, and neither alone is enough to
    /// widen: the modern high half encodes the object type, which is only known from the whole
    /// 64-bit value. So the halves are held here until both have landed.
    guid_halves: HashMap<u16, (Option<u32>, Option<u32>)>,
}

impl ModernFieldsArray {
    pub fn new(object_type: ModernObjectType, realm_id: u16) -> Self {
        let field_count = object_type.field_count();
        Self {
            object_type,
            values: vec![0; field_count as usize],
            mask: ModernUpdateMask::new(field_count),
            realm_id,
            guid_halves: HashMap::new(),
        }
    }

    /// Set a field by its *vanilla* slot number, translating through this type's map.
    ///
    /// Returns whether the value was placed. A `false` means the field has no 1.14 counterpart and
    /// was dropped -- normal for the many vanilla-only fields (honor, buyback, aura slots), so
    /// callers should not treat it as an error.
    pub fn set_vanilla(&mut self, vanilla_index: u32, value: u32) -> bool {
        // A few fields repacked their contents rather than just moving, so they bypass the map
        // entirely. See `repack` for why each one cannot be a slot move.
        if let Some(writes) = repack(self.object_type, vanilla_index, value) {
            for &(index, value) in writes.as_slice() {
                self.set_modern(index, value);
            }
            return true;
        }

        let Some(slot) = self
            .object_type
            .field_map()
            .get(vanilla_index as usize)
            .copied()
        else {
            return false;
        };

        let index = match slot {
            ModernSlot::None => return false,
            ModernSlot::Plain(index) => index,
            // GUIDs cannot be written a half at a time: widening 64 bits to 128 needs the whole
            // value, because the modern high half is derived from the object type encoded in it.
            ModernSlot::GuidLow(base) => {
                self.guid_halves.entry(base).or_default().0 = Some(value);
                self.widen_guid(base);
                return true;
            }
            ModernSlot::GuidHigh(base) => {
                self.guid_halves.entry(base).or_default().1 = Some(value);
                self.widen_guid(base);
                return true;
            }
        };

        let Some(cell) = self.values.get_mut(index as usize) else {
            return false;
        };
        *cell = value;
        self.mask.set(index);
        true
    }

    /// Widen a vanilla GUID field into its four modern slots, once both halves have arrived.
    ///
    /// Filling only the two slots vanilla supplies leaves the upper 64 bits zero, which the client
    /// reads as high-type `Null`. The object's own GUID field then disagrees with the GUID in its
    /// block header -- the object claims to be two different things, and the client does not
    /// survive it.
    fn widen_guid(&mut self, base: u16) {
        let Some(&(Some(low), Some(high))) = self.guid_halves.get(&base) else {
            return; // still waiting on the other half
        };

        let raw = u64::from(low) | (u64::from(high) << 32);
        let (guid_high, guid_low) = ObjectGuid::from_raw(raw).to_guid128(self.realm_id);

        self.set_modern(base, guid_low as u32);
        self.set_modern(base + 1, (guid_low >> 32) as u32);
        self.set_modern(base + 2, guid_high as u32);
        self.set_modern(base + 3, (guid_high >> 32) as u32);
    }

    /// Set a field by its modern slot number, for values with no vanilla origin.
    pub fn set_modern(&mut self, index: u16, value: u32) {
        if let Some(cell) = self.values.get_mut(index as usize) {
            *cell = value;
            self.mask.set(index);
        }
    }

    /// Set a float field by its modern slot number.
    ///
    /// Modern stores floats in the same u32 slots as integers; only the interpretation differs.
    pub fn set_modern_f32(&mut self, index: u16, value: f32) {
        self.set_modern(index, value.to_bits());
    }

    /// Read back a modern slot, or `None` if it was never set. For tests and diagnostics.
    pub fn modern_value(&self, index: u16) -> Option<u32> {
        self.mask
            .is_set(index)
            .then(|| self.values.get(index as usize).copied())
            .flatten()
    }

    /// Write the mask followed by every set value in slot order.
    pub fn write_to(&self, writer: &mut BitWriter) {
        self.mask.write_to(writer);
        for (index, value) in self.values.iter().enumerate() {
            if self.mask.is_set(index as u16) {
                writer.write_u32(*value);
            }
        }
    }

    /// Write the empty dynamic-field block that follows the static one.
    ///
    /// We never populate dynamic fields, but the client unconditionally reads a mask here, so
    /// omitting it desynchronises everything after this object.
    pub fn write_empty_dynamic_to(&self, writer: &mut BitWriter) {
        ModernUpdateMask::new(self.object_type.dynamic_field_count()).write_to(writer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_mask_accumulates_the_inheritance_chain() {
        assert_eq!(ModernObjectType::Object.type_mask(), 0x001);
        assert_eq!(ModernObjectType::Unit.type_mask(), 0x001 | 0x008);
        assert_eq!(ModernObjectType::Player.type_mask(), 0x001 | 0x008 | 0x010);
        assert_eq!(
            ModernObjectType::ActivePlayer.type_mask(),
            0x001 | 0x008 | 0x010 | 0x020
        );
        assert_eq!(ModernObjectType::GameObject.type_mask(), 0x001 | 0x040);
    }

    /// 1.14 inserts ActivePlayer at 5, so every type after it shifts. Casting a vanilla
    /// `ObjectTypeId` straight to u8 would send a gameobject as a player.
    #[test]
    fn gameobject_renumbers_between_protocols() {
        assert_eq!(ObjectTypeId::GameObject as u8, 5);
        assert_eq!(
            ModernObjectType::from_vanilla(ObjectTypeId::GameObject, false) as u8,
            6
        );
    }

    #[test]
    fn own_player_promotes_to_active_player() {
        assert_eq!(
            ModernObjectType::from_vanilla(ObjectTypeId::Player, true),
            ModernObjectType::ActivePlayer
        );
        assert_eq!(
            ModernObjectType::from_vanilla(ObjectTypeId::Player, false),
            ModernObjectType::Player
        );
    }

    /// The mask is little-endian blocks of 32, and is sent at full width for the type -- 218 Unit
    /// slots need 7 blocks even when one field changed.
    #[test]
    fn mask_writes_full_width_little_endian_blocks() {
        let mut mask = ModernUpdateMask::new(ModernObjectType::Unit.field_count());
        mask.set(0);
        mask.set(33);

        let mut writer = BitWriter::new();
        mask.write_to(&mut writer);
        let bytes = writer.into_bytes();

        assert_eq!(bytes[0], 7, "218 fields is 7 blocks of 32");
        assert_eq!(bytes.len(), 1 + 7 * 4);
        assert_eq!(&bytes[1..5], &[0x01, 0x00, 0x00, 0x00], "bit 0");
        assert_eq!(
            &bytes[5..9],
            &[0x02, 0x00, 0x00, 0x00],
            "bit 33 = block 1 bit 1"
        );
    }

    #[test]
    fn values_follow_the_mask_in_slot_order() {
        let mut fields = ModernFieldsArray::new(ModernObjectType::Unit, 1);
        // UNIT_FIELD_LEVEL and UNIT_FIELD_HEALTH, deliberately set out of order.
        assert!(fields.set_vanilla(34, 60));
        assert!(fields.set_vanilla(22, 100));

        let mut writer = BitWriter::new();
        fields.write_to(&mut writer);
        let bytes = writer.into_bytes();

        let values = &bytes[1 + 7 * 4..];
        assert_eq!(values.len(), 8);
        // Health is modern slot 55, level is 80, so health comes first regardless of set order.
        assert_eq!(&values[0..4], &100u32.to_le_bytes());
        assert_eq!(&values[4..8], &60u32.to_le_bytes());
    }

    /// Vanilla-only fields must drop rather than land somewhere arbitrary.
    #[test]
    fn unmapped_vanilla_fields_are_dropped() {
        let mut fields = ModernFieldsArray::new(ModernObjectType::Unit, 1);
        // OBJECT_FIELD_TYPE: 1.14 sends the type as a header byte, so the field is gone.
        assert!(!fields.set_vanilla(2, 25));
        assert!(!fields.set_vanilla(9_999, 1));
    }
}
