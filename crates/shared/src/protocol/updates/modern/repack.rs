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
    MODERN_PLAYER_QUEST_LOG, MODERN_UNIT_FIELD_ATTACK_POWER_MOD_NEG,
    MODERN_UNIT_FIELD_ATTACK_POWER_MOD_POS, MODERN_UNIT_FIELD_BYTES_0,
    MODERN_UNIT_FIELD_DISPLAY_POWER, MODERN_UNIT_FIELD_RANGED_ATTACK_POWER_MOD_NEG,
    MODERN_UNIT_FIELD_RANGED_ATTACK_POWER_MOD_POS,
};
use super::fields::ModernObjectType;
use crate::protocol::update_fields::{
    PLAYER_QUEST_LOG_1_1, UNIT_FIELD_ATTACK_POWER_MODS, UNIT_FIELD_BYTES_0,
    UNIT_FIELD_RANGED_ATTACK_POWER_MODS,
};

/// Modern slot writes produced from one vanilla field write.
///
/// A fixed buffer rather than a `Vec`: this runs for every field of every object in every update, so
/// the allocation would be on the hot path. Three is the widest transform so far -- the quest log's
/// state-and-counters slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Writes {
    slots: [(u16, u32); MAX_WRITES],
    len: usize,
}

const MAX_WRITES: usize = 3;

impl Writes {
    fn new(slots: &[(u16, u32)]) -> Self {
        let mut buffer = [(0u16, 0u32); MAX_WRITES];
        buffer[..slots.len()].copy_from_slice(slots);
        Self {
            slots: buffer,
            len: slots.len(),
        }
    }

    pub fn as_slice(&self) -> &[(u16, u32)] {
        &self.slots[..self.len]
    }
}

/// Vanilla quest-log geometry: 20 slots of `(quest id, counters+state, timer)`.
const VANILLA_QUEST_LOG_SLOTS: u32 = 20;
const VANILLA_QUEST_LOG_STRIDE: u32 = 3;

/// 1.14 quest-log geometry: 25 slots of 16, per the 1.14 wire format.
const MODERN_QUEST_LOG_STRIDE: u16 = 16;
/// Offset of `EndTime` within a modern quest-log slot (`+2 + 12`).
const MODERN_QUEST_LOG_END_TIME: u16 = 14;

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

            Writes::new(&[
                (
                    MODERN_UNIT_FIELD_BYTES_0,
                    race | (class << 8) | (class << 16) | (gender << 24),
                ),
                (MODERN_UNIT_FIELD_DISPLAY_POWER, power_type),
            ])
        }

        // Vanilla stores the positive and negative modifiers as two u16s sharing one slot; 1.14
        // gives each its own i32 field. Sent as-is, the negative modifier would land in the high
        // half of the positive one.
        UNIT_FIELD_ATTACK_POWER_MODS => Writes::new(&[
            (MODERN_UNIT_FIELD_ATTACK_POWER_MOD_POS, value & 0xFFFF),
            (MODERN_UNIT_FIELD_ATTACK_POWER_MOD_NEG, value >> 16),
        ]),
        UNIT_FIELD_RANGED_ATTACK_POWER_MODS => Writes::new(&[
            (
                MODERN_UNIT_FIELD_RANGED_ATTACK_POWER_MOD_POS,
                value & 0xFFFF,
            ),
            (MODERN_UNIT_FIELD_RANGED_ATTACK_POWER_MOD_NEG, value >> 16),
        ]),

        // The quest log is a *stride* transform, not a slot move: vanilla names 20 x 3 individual
        // fields (`PLAYER_QUEST_LOG_<n>_<field>`), 1.14 has one 25 x 16 array called
        // `PLAYER_QUEST_LOG`. The names never matched, so the generator's name join produced no
        // mapping at all and every quest-log write was dropped -- the server accepted quests and the
        // client's log stayed empty.
        // Restricted to the player chain: a quest log only exists on a player, and these indices
        // are past a creature's field table anyway. Being explicit keeps a future creature field at
        // the same number from being rewritten as quest data.
        index
            if matches!(
                object_type,
                ModernObjectType::Player | ModernObjectType::ActivePlayer
            ) && quest_log_slot(index).is_some() =>
        {
            let (slot, offset) = quest_log_slot(index)?;
            let base = MODERN_PLAYER_QUEST_LOG + slot * MODERN_QUEST_LOG_STRIDE;

            match offset {
                // Quest id: a straight move to the head of the modern slot.
                0 => Writes::new(&[(base, value)]),

                // One vanilla word holds four 6-bit objective counters and a state byte. 1.14 wants
                // the state in its own slot and the counters as u16 pairs, two per slot -- so this
                // single write becomes three.
                1 => {
                    let counter = |index: u32| (value >> (index * 6)) & 0x3F;
                    Writes::new(&[
                        (base + 1, (value >> 24) & 0xFF), // StateFlags
                        (base + 2, counter(0) | (counter(1) << 16)),
                        (base + 3, counter(2) | (counter(3) << 16)),
                    ])
                }

                // The timer moves past the twelve objective-progress slots.
                _ => Writes::new(&[(base + MODERN_QUEST_LOG_END_TIME, value)]),
            }
        }

        _ => return None,
    })
}

/// Split a vanilla field index into its quest-log `(slot, offset)`, or `None` if it is not one.
///
/// Kept separate so the match arm above and its guard cannot disagree about the range.
fn quest_log_slot(vanilla_index: u32) -> Option<(u16, u32)> {
    let relative = vanilla_index.checked_sub(PLAYER_QUEST_LOG_1_1)?;
    if relative >= VANILLA_QUEST_LOG_SLOTS * VANILLA_QUEST_LOG_STRIDE {
        return None;
    }
    Some((
        (relative / VANILLA_QUEST_LOG_STRIDE) as u16,
        relative % VANILLA_QUEST_LOG_STRIDE,
    ))
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
            writes.as_slice(),
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

        assert_eq!(writes.as_slice()[0].1 >> 24, 0, "sex, not the power type");
        assert_eq!(
            writes.as_slice()[1],
            (MODERN_UNIT_FIELD_DISPLAY_POWER, 1)
        );
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
            writes.as_slice(),
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

    /// The bug the user saw as "I can see quests but can't accept them": the accept succeeded on the
    /// server and the client's quest log stayed empty, because the vanilla field name
    /// (`PLAYER_QUEST_LOG_1_1`) never matched 1.14's array name (`PLAYER_QUEST_LOG`), so the
    /// generator produced no mapping and every write was silently dropped.
    #[test]
    fn quest_log_fields_are_no_longer_dropped() {
        let writes = repack(ModernObjectType::ActivePlayer, PLAYER_QUEST_LOG_1_1, 783)
            .expect("the quest id must map somewhere");
        assert_eq!(writes.as_slice(), [(MODERN_PLAYER_QUEST_LOG, 783)]);
    }

    /// Vanilla's second slot crams four 6-bit counters under a state byte; 1.14 wants the state
    /// alone and the counters as u16 pairs. One write becomes three.
    #[test]
    fn quest_log_counters_unpack_into_state_and_u16_pairs() {
        // counters 1, 2, 3, 4 and state 0x01 (complete)
        let packed = 1 | (2 << 6) | (3 << 12) | (4 << 18) | (0x01 << 24);
        let writes = repack(
            ModernObjectType::ActivePlayer,
            PLAYER_QUEST_LOG_1_1 + 1,
            packed,
        )
        .expect("transformed");

        assert_eq!(
            writes.as_slice(),
            [
                (MODERN_PLAYER_QUEST_LOG + 1, 0x01),
                (MODERN_PLAYER_QUEST_LOG + 2, 1 | (2 << 16)),
                (MODERN_PLAYER_QUEST_LOG + 3, 3 | (4 << 16)),
            ]
        );
    }

    /// The strides differ (3 vanilla, 16 modern), so slot n does not land at n * 3.
    #[test]
    fn quest_log_slots_use_the_modern_stride() {
        let slot_3_id = PLAYER_QUEST_LOG_1_1 + 3 * VANILLA_QUEST_LOG_STRIDE;
        let writes =
            repack(ModernObjectType::ActivePlayer, slot_3_id, 42).expect("slot 3 must map");
        assert_eq!(
            writes.as_slice(),
            [(MODERN_PLAYER_QUEST_LOG + 3 * MODERN_QUEST_LOG_STRIDE, 42)]
        );
    }

    /// The timer sits past the twelve objective-progress slots, not immediately after the state.
    #[test]
    fn the_quest_timer_clears_the_objective_slots() {
        let writes = repack(
            ModernObjectType::ActivePlayer,
            PLAYER_QUEST_LOG_1_1 + 2,
            12345,
        )
        .expect("transformed");
        assert_eq!(
            writes.as_slice(),
            [(MODERN_PLAYER_QUEST_LOG + MODERN_QUEST_LOG_END_TIME, 12345)]
        );
    }

    /// One past the last vanilla quest slot must fall through to the ordinary map, or an unrelated
    /// player field would be rewritten as quest data.
    #[test]
    fn the_field_after_the_quest_log_is_not_treated_as_quest_data() {
        let past_end = PLAYER_QUEST_LOG_1_1 + VANILLA_QUEST_LOG_SLOTS * VANILLA_QUEST_LOG_STRIDE;
        assert!(repack(ModernObjectType::ActivePlayer, past_end, 1).is_none());
    }

    /// A creature field at the same index must not be rewritten as quest data.
    #[test]
    fn quest_log_indices_are_player_only() {
        assert!(repack(ModernObjectType::Unit, PLAYER_QUEST_LOG_1_1, 783).is_none());
        assert!(repack(ModernObjectType::Unit, PLAYER_QUEST_LOG_1_1 + 1, 783).is_none());
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
