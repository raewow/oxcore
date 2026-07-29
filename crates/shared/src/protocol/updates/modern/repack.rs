//! Value transforms for fields that changed meaning rather than just position.
//!
//! Most of the 1.12 -> 1.14 field migration is a slot move, handled by the generated map in
//! [`super::field_map`]. A handful of byte-packed fields kept their name but repacked their
//! contents, and one vanilla field split into two. Those cannot go through the map -- moving the
//! word intact would produce a packet the client parses happily and renders wrongly.
//!
//! The generator refuses to resolve these by name (its `DENIED` table); this module is where they
//! are handled deliberately instead.

use super::field_map::{
    MODERN_UNIT_FIELD_ATTACK_POWER_MOD_NEG, MODERN_UNIT_FIELD_ATTACK_POWER_MOD_POS,
    MODERN_UNIT_FIELD_BYTES_0, MODERN_UNIT_FIELD_DISPLAY_POWER,
    MODERN_UNIT_FIELD_RANGED_ATTACK_POWER_MOD_NEG, MODERN_UNIT_FIELD_RANGED_ATTACK_POWER_MOD_POS,
};
use super::fields::ModernObjectType;
use crate::protocol::update_fields::{
    UNIT_FIELD_ATTACK_POWER_MODS, UNIT_FIELD_BYTES_0, UNIT_FIELD_RANGED_ATTACK_POWER_MODS,
};

/// Modern slot writes produced from one vanilla field write.
///
/// Every transform so far produces exactly two, so this is a fixed pair rather than a list. Add a
/// length alongside it if a future transform needs one or three.
pub type Writes = [(u16, u32); 2];

/// Translate a vanilla field whose contents, not just position, changed in 1.14.
///
/// `None` means the field needs no transform and should go through the ordinary slot map.
///
/// The object type is not optional. Vanilla field numbers are only unique *within* an inheritance
/// chain: `UNIT_FIELD_BYTES_0` is index 36, and index 36 on an item is inside
/// `ITEM_FIELD_ENCHANTMENT`. Keying on the index alone would repack enchantment data as unit bytes.
pub fn repack(object_type: ModernObjectType, vanilla_index: u32, value: u32) -> Option<Writes> {
    // Every transform so far is a Unit-chain field.
    if !object_type.is_unit() {
        return None;
    }

    Some(match vanilla_index {
        // Vanilla packs (race, class, gender, powerType). 1.14 packs
        // (Race, ClassId, PlayerClassId, Sex) and moved the power type to its own field, so the
        // top two bytes both shift and the power type leaves entirely.
        //
        // PlayerClassId mirrors ClassId: the two differ only for creatures whose displayed class
        // is not their real one, which vanilla cannot express.
        UNIT_FIELD_BYTES_0 => {
            let race = value & 0xFF;
            let class = (value >> 8) & 0xFF;
            let gender = (value >> 16) & 0xFF;
            let power_type = (value >> 24) & 0xFF;

            [
                (
                    MODERN_UNIT_FIELD_BYTES_0,
                    race | (class << 8) | (class << 16) | (gender << 24),
                ),
                (MODERN_UNIT_FIELD_DISPLAY_POWER, power_type),
            ]
        }

        // Vanilla stores the positive and negative modifiers as two u16s sharing one slot; 1.14
        // gives each its own i32 field. Sent as-is, the negative modifier would land in the high
        // half of the positive one.
        UNIT_FIELD_ATTACK_POWER_MODS => [
            (MODERN_UNIT_FIELD_ATTACK_POWER_MOD_POS, value & 0xFFFF),
            (MODERN_UNIT_FIELD_ATTACK_POWER_MOD_NEG, value >> 16),
        ],
        UNIT_FIELD_RANGED_ATTACK_POWER_MODS => [
            (
                MODERN_UNIT_FIELD_RANGED_ATTACK_POWER_MOD_POS,
                value & 0xFFFF,
            ),
            (MODERN_UNIT_FIELD_RANGED_ATTACK_POWER_MOD_NEG, value >> 16),
        ],

        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The class byte is duplicated into PlayerClassId and the power type is evicted to its own
    /// field. Passing the word through unchanged would set the player's sex to their power type.
    #[test]
    fn unit_bytes_0_repacks_and_splits_out_the_power_type() {
        // race 1 (human), class 2 (paladin), gender 1 (female), power type 0 (mana)
        let vanilla = 1 | (2 << 8) | (1 << 16);
        let writes =
            repack(ModernObjectType::Player, UNIT_FIELD_BYTES_0, vanilla).expect("transformed");

        assert_eq!(
            writes,
            [
                (
                    MODERN_UNIT_FIELD_BYTES_0,
                    1 | (2 << 8) | (2 << 16) | (1 << 24)
                ),
                (MODERN_UNIT_FIELD_DISPLAY_POWER, 0),
            ]
        );
    }

    #[test]
    fn unit_bytes_0_carries_a_non_mana_power_type() {
        // race 3 (dwarf), class 1 (warrior), gender 0, power type 1 (rage)
        let vanilla = 3 | (1 << 8) | (1 << 24);
        let writes =
            repack(ModernObjectType::Player, UNIT_FIELD_BYTES_0, vanilla).expect("transformed");

        assert_eq!(writes[0].1 >> 24, 0, "sex, not the power type");
        assert_eq!(writes[1], (MODERN_UNIT_FIELD_DISPLAY_POWER, 1));
    }

    #[test]
    fn attack_power_mods_split_into_two_fields() {
        let vanilla = 50 | (20 << 16); // +50 positive, -20 negative
        let writes = repack(
            ModernObjectType::Unit,
            UNIT_FIELD_ATTACK_POWER_MODS,
            vanilla,
        )
        .expect("transformed");

        assert_eq!(
            writes,
            [
                (MODERN_UNIT_FIELD_ATTACK_POWER_MOD_POS, 50),
                (MODERN_UNIT_FIELD_ATTACK_POWER_MOD_NEG, 20),
            ]
        );
    }

    /// Vanilla field numbers repeat across inheritance chains. `UNIT_FIELD_BYTES_0` is 36, and 36
    /// on an item falls inside `ITEM_FIELD_ENCHANTMENT` -- so an enchantment write must not be
    /// mistaken for unit bytes and scattered across two unrelated slots.
    #[test]
    fn a_matching_index_on_another_object_type_is_not_repacked() {
        assert_eq!(UNIT_FIELD_BYTES_0, 36, "the overlap this guards against");
        assert!(repack(ModernObjectType::Item, UNIT_FIELD_BYTES_0, 1234).is_none());
        assert!(repack(ModernObjectType::GameObject, UNIT_FIELD_BYTES_0, 1234).is_none());
    }

    #[test]
    fn ordinary_fields_are_left_to_the_slot_map() {
        assert!(repack(
            ModernObjectType::Unit,
            crate::protocol::update_fields::UNIT_FIELD_HEALTH,
            100
        )
        .is_none());
    }
}
