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
    MODERN_OBJECT_DYNAMIC_FLAGS, MODERN_PLAYER_QUEST_LOG, MODERN_PLAYER_VISIBLE_ITEM,
    MODERN_UNIT_FIELD_ATTACK_POWER_MOD_NEG, MODERN_UNIT_FIELD_ATTACK_POWER_MOD_POS,
    MODERN_UNIT_FIELD_BYTES_0, MODERN_UNIT_FIELD_BYTES_1, MODERN_UNIT_FIELD_BYTES_2,
    MODERN_UNIT_FIELD_DISPLAY_POWER, MODERN_UNIT_FIELD_RANGED_ATTACK_POWER_MOD_NEG,
    MODERN_UNIT_FIELD_RANGED_ATTACK_POWER_MOD_POS, MODERN_UNIT_NPC_FLAGS,
};
use super::fields::ModernObjectType;
use crate::protocol::update_fields::{
    PLAYER_QUEST_LOG_1_1, PLAYER_VISIBLE_ITEM_1_0, UNIT_DYNAMIC_FLAGS,
    UNIT_FIELD_ATTACK_POWER_MODS, UNIT_FIELD_BYTES_0, UNIT_FIELD_BYTES_1, UNIT_FIELD_BYTES_2,
    UNIT_FIELD_RANGED_ATTACK_POWER_MODS, UNIT_NPC_FLAGS,
};

/// Modern slot writes produced from one vanilla field write.
///
/// A fixed buffer rather than a `Vec`: this runs for every field of every object in every update, so
/// the allocation would be on the hot path. Three is the widest transform so far -- the quest log's
/// state-and-counters slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Writes {
    slots: [(u16, u32); MAX_WRITES],
    /// Per-slot write mask, `u32::MAX` (the whole word) for every write made through `new`. A
    /// narrower mask means the write shares its modern slot with another vanilla field and must
    /// not clobber the bytes that field owns -- see `set_modern_masked`.
    masks: [u32; MAX_WRITES],
    len: usize,
}

const MAX_WRITES: usize = 3;

impl Writes {
    fn new(slots: &[(u16, u32)]) -> Self {
        let mut buffer = [(0u16, 0u32); MAX_WRITES];
        buffer[..slots.len()].copy_from_slice(slots);
        Self {
            slots: buffer,
            masks: [u32::MAX; MAX_WRITES],
            len: slots.len(),
        }
    }

    /// Like `new`, but each write only replaces the bits set in its mask, preserving the rest of
    /// the modern slot's current value. See the `masks` field doc for why this exists.
    fn new_masked(slots: &[(u16, u32, u32)]) -> Self {
        let mut buffer = [(0u16, 0u32); MAX_WRITES];
        let mut masks = [u32::MAX; MAX_WRITES];
        for (i, &(index, value, mask)) in slots.iter().enumerate() {
            buffer[i] = (index, value);
            masks[i] = mask;
        }
        Self {
            slots: buffer,
            masks,
            len: slots.len(),
        }
    }

    pub fn as_slice(&self) -> &[(u16, u32)] {
        &self.slots[..self.len]
    }

    pub fn masks(&self) -> &[u32] {
        &self.masks[..self.len]
    }
}

/// Vanilla quest-log geometry: 20 slots of `(quest id, counters+state, timer)`.
const VANILLA_QUEST_LOG_SLOTS: u32 = 20;
const VANILLA_QUEST_LOG_STRIDE: u32 = 3;

/// 1.14 quest-log geometry: 25 slots of 16, per the 1.14 wire format.
const MODERN_QUEST_LOG_STRIDE: u16 = 16;
/// Offset of `EndTime` within a modern quest-log slot (`+2 + 12`).
const MODERN_QUEST_LOG_END_TIME: u16 = 14;

/// Vanilla visible-item geometry: 19 equipment slots of 12 named fields each (creator guid,
/// entry+enchants, properties, pad). oxcore only ever populates the first three -- entry, perm
/// enchant, temp enchant -- of which `visible_item_slot` maps the first two.
const VANILLA_VISIBLE_ITEM_SLOTS: u32 = 19;
const VANILLA_VISIBLE_ITEM_STRIDE: u32 = 12;

/// 1.14 visible-item geometry: 19 slots of 2 (ItemID, enchant/appearance), per the 1.14 wire
/// format -- `PLAYER_VISIBLE_ITEM` is size 38.
const MODERN_VISIBLE_ITEM_STRIDE: u16 = 2;

/// Translate vanilla NPC-flag bits to their modern positions.
///
/// The bits renumber from `Vendor` onward: vanilla `Vendor = 0x4` is 1.14's `Unk1`, vanilla
/// `Innkeeper = 0x80` is 1.14's `Vendor`, and so on for every flag past `Trainer`. A raw copy does
/// not merely mislabel one flag -- it sets a *different* interaction icon than the one the NPC
/// actually has (or none at all for `Vendor`), which is why vendor windows never opened: the
/// client never saw the `Vendor` bit at the position it checks. `Gossip` and `QuestGiver` happen
/// to share their vanilla and modern bit positions, which is why gossip-only NPCs were unaffected.
/// Translated by name, the way `to_modern_movement_flags` and `to_modern_hit_info` translate their
/// own renumbered flag words.
fn to_modern_npc_flags(vanilla: u32) -> u32 {
    const BITS: &[(u32, u32)] = &[
        (0x0000_0001, 0x0000_0001), // Gossip
        (0x0000_0002, 0x0000_0002), // QuestGiver
        (0x0000_0004, 0x0000_0080), // Vendor
        (0x0000_0008, 0x0000_2000), // FlightMaster
        (0x0000_0010, 0x0000_0010), // Trainer
        (0x0000_0020, 0x0000_4000), // SpiritHealer
        (0x0000_0040, 0x0000_8000), // SpiritGuide
        (0x0000_0080, 0x0001_0000), // Innkeeper
        (0x0000_0100, 0x0002_0000), // Banker
        (0x0000_0200, 0x0004_0000), // Petitioner
        (0x0000_0400, 0x0008_0000), // TabardDesigner
        (0x0000_0800, 0x0010_0000), // BattleMaster
        (0x0000_1000, 0x0020_0000), // Auctioneer
        (0x0000_2000, 0x0040_0000), // StableMaster
        (0x0000_4000, 0x0000_1000), // Repair
    ];
    BITS.iter().fold(
        0,
        |acc, &(v, m)| if vanilla & v != 0 { acc | m } else { acc },
    )
}

/// Translate vanilla unit dynamic-flag bits to their modern positions.
///
/// 1.14 renumbers these too, not just relocates the field: vanilla `Lootable = 0x1` is 1.14's
/// `HideModel`, and 1.14's `Lootable` is `0x4`. A raw copy leaves a dead, lootable creature's
/// `Lootable` bit unset on the client, so no loot cue or window ever appears even though the
/// server correctly marked the corpse lootable. `TappedByPlayer` has no modern counterpart at all
/// and is dropped rather than guessed at.
fn to_modern_dynamic_flags(vanilla: u32) -> u32 {
    const BITS: &[(u32, u32)] = &[
        (0x01, 0x04), // Lootable
        (0x02, 0x08), // TrackUnit
        (0x04, 0x10), // Tapped
        (0x10, 0x20), // EmpathyInfo
        (0x20, 0x40), // AppearDead
        (0x40, 0x80), // ReferAFriendLinked
    ];
    BITS.iter().fold(
        0,
        |acc, &(v, m)| if vanilla & v != 0 { acc | m } else { acc },
    )
}

/// Split a vanilla field index into its visible-item `(slot, offset)`, or `None` if it is not one
/// with a modern counterpart. Modern has room for two fields per slot (ItemID, one enchant), so
/// only vanilla offset 0 (entry) and offset 1 (perm enchant) map; offset 2 (temp enchant) falls
/// through to the ordinary map and is dropped, the same way the spell-visual flourish is dropped
/// elsewhere in this port rather than guessed at.
fn visible_item_slot(vanilla_index: u32) -> Option<(u16, u32)> {
    let relative = vanilla_index.checked_sub(PLAYER_VISIBLE_ITEM_1_0)?;
    let slot = relative / VANILLA_VISIBLE_ITEM_STRIDE;
    if slot >= VANILLA_VISIBLE_ITEM_SLOTS {
        return None;
    }
    let offset = relative % VANILLA_VISIBLE_ITEM_STRIDE;
    if offset > 1 {
        return None;
    }
    Some((slot as u16, offset))
}

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

        // Vanilla packs [StandState, 0, ShapeshiftForm, 0]. Modern's BYTES_1 has no shapeshift
        // form at all -- it packs (StandState, PetLoyaltyIndex, VisFlags, AnimTier) -- so only
        // StandState (byte 0) goes there. The shapeshift form moves to byte 3 of modern BYTES_2,
        // which vanilla's own UNIT_FIELD_BYTES_2 write also contributes to below; both writes are
        // masked to their own byte so neither can clobber the other regardless of write order.
        UNIT_FIELD_BYTES_1 => {
            let stand_state = value & 0xFF;
            let shapeshift_form = (value >> 16) & 0xFF;
            Writes::new_masked(&[
                (MODERN_UNIT_FIELD_BYTES_1, stand_state, 0x0000_00FF),
                (
                    MODERN_UNIT_FIELD_BYTES_2,
                    shapeshift_form << 24,
                    0xFF00_0000,
                ),
            ])
        }

        // Vanilla packs (SheatheState, misc aura-icon flag, 0, 0); modern packs (SheatheState,
        // PvpFlags, PetFlags, ShapeshiftForm). Only SheatheState (byte 0) has a clean modern
        // counterpart -- the misc flag byte does not correspond to PvpFlags, so it is dropped
        // rather than misencoded. Masked to byte 0 so it cannot clobber the shapeshift form the
        // UNIT_FIELD_BYTES_1 case above writes into byte 3 of this same modern slot.
        UNIT_FIELD_BYTES_2 => {
            let sheathe_state = value & 0xFF;
            Writes::new_masked(&[(MODERN_UNIT_FIELD_BYTES_2, sheathe_state, 0x0000_00FF)])
        }

        // The flag *bits* renumber in 1.14, not just the field position -- see
        // `to_modern_npc_flags` for why a raw copy broke vendor windows specifically.
        UNIT_NPC_FLAGS => Writes::new(&[(MODERN_UNIT_NPC_FLAGS, to_modern_npc_flags(value))]),

        // Same hazard as NPC flags: the bits renumber as well as the field moving to the shared
        // Object block. See `to_modern_dynamic_flags` for why this broke loot visibility.
        UNIT_DYNAMIC_FLAGS => {
            Writes::new(&[(MODERN_OBJECT_DYNAMIC_FLAGS, to_modern_dynamic_flags(value))])
        }

        // Vanilla names 19 x 12 individual fields; 1.14 has one 19 x 2 array. Restricted to the
        // player chain the same way the quest log is: these indices are past a creature's field
        // table, and being explicit keeps a future creature field at the same number from being
        // mistaken for visible-item data.
        index
            if matches!(
                object_type,
                ModernObjectType::Player | ModernObjectType::ActivePlayer
            ) && visible_item_slot(index).is_some() =>
        {
            let (slot, offset) = visible_item_slot(index)?;
            let target =
                MODERN_PLAYER_VISIBLE_ITEM + slot * MODERN_VISIBLE_ITEM_STRIDE + offset as u16;
            Writes::new(&[(target, value)])
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
        assert_eq!(writes.as_slice()[1], (MODERN_UNIT_FIELD_DISPLAY_POWER, 1));
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

    /// The bug this fixes: a vendor's `Vendor` bit (vanilla `0x4`) lands on modern's `Unk1`
    /// instead of modern's `Vendor` bit (`0x80`), so the client never recognizes the NPC as a
    /// vendor and shows no window at all.
    #[test]
    fn npc_flags_move_the_vendor_bit() {
        let writes = repack(ModernObjectType::Unit, UNIT_NPC_FLAGS, 0x4).expect("transformed");
        assert_eq!(writes.as_slice(), [(MODERN_UNIT_NPC_FLAGS, 0x80)]);
    }

    /// Gossip and quest-giver happen to share their vanilla and modern bit positions, so a gossip
    /// NPC combined with a vendor must still carry both correctly rather than only the coincidence.
    #[test]
    fn npc_flags_combine_gossip_and_vendor() {
        let vanilla = 0x1 | 0x4; // Gossip | Vendor
        let writes = repack(ModernObjectType::Unit, UNIT_NPC_FLAGS, vanilla).expect("transformed");
        assert_eq!(writes.as_slice(), [(MODERN_UNIT_NPC_FLAGS, 0x1 | 0x80)]);
    }

    /// The bug this fixes: a dead creature's `Lootable` bit (vanilla `0x1`) lands on modern's
    /// `HideModel` instead of modern's `Lootable` bit (`0x4`), so the client never shows a loot
    /// cue even though the server marked the corpse lootable.
    #[test]
    fn dynamic_flags_move_the_lootable_bit() {
        let writes = repack(ModernObjectType::Unit, UNIT_DYNAMIC_FLAGS, 0x1).expect("transformed");
        assert_eq!(writes.as_slice(), [(MODERN_OBJECT_DYNAMIC_FLAGS, 0x4)]);
    }

    /// `TappedByPlayer` (vanilla `0x8`) has no modern counterpart and must be dropped, not guessed.
    #[test]
    fn dynamic_flags_drop_tapped_by_player() {
        let writes = repack(ModernObjectType::Unit, UNIT_DYNAMIC_FLAGS, 0x8).expect("transformed");
        assert_eq!(writes.as_slice(), [(MODERN_OBJECT_DYNAMIC_FLAGS, 0)]);
    }

    /// The bug this fixes: a warrior's shapeshift form (vanilla `UNIT_FIELD_BYTES_1` byte 2) has
    /// no counterpart in modern `BYTES_1` at all -- it moves to byte 3 of modern `BYTES_2` -- so a
    /// raw copy silently drops it and the client shows no stance selected.
    #[test]
    fn bytes_1_sends_stand_state_and_moves_shapeshift_form_to_bytes_2() {
        let stand_state = 0u32;
        let shapeshift_form = 17u32; // Battle Stance
        let vanilla = stand_state | (shapeshift_form << 16);
        let writes =
            repack(ModernObjectType::Player, UNIT_FIELD_BYTES_1, vanilla).expect("transformed");
        assert_eq!(
            writes.as_slice(),
            [
                (MODERN_UNIT_FIELD_BYTES_1, stand_state),
                (MODERN_UNIT_FIELD_BYTES_2, shapeshift_form << 24),
            ]
        );
        assert_eq!(writes.masks(), [0x0000_00FF, 0xFF00_0000]);
    }

    #[test]
    fn bytes_2_sends_only_sheathe_state_masked_to_byte_0() {
        let vanilla = 1 | (0x10 << 8); // sheathed, plus vanilla's misc aura-icon byte
        let writes =
            repack(ModernObjectType::Player, UNIT_FIELD_BYTES_2, vanilla).expect("transformed");
        assert_eq!(writes.as_slice(), [(MODERN_UNIT_FIELD_BYTES_2, 1)]);
        assert_eq!(writes.masks(), [0x0000_00FF]);
    }

    /// The two vanilla fields share one modern slot and must merge regardless of which arrives
    /// first -- game systems write `UNIT_FIELD_BYTES_1` and `UNIT_FIELD_BYTES_2` independently, so
    /// nothing guarantees an order. This exercises the real merge path through
    /// `ModernFieldsArray::set_vanilla`, not just the two `Writes` in isolation.
    #[test]
    fn shapeshift_form_and_sheathe_state_merge_regardless_of_order() {
        use super::super::fields::ModernFieldsArray;

        let shapeshift_form = 17u32; // Battle Stance
        let bytes_1 = 0u32 | (shapeshift_form << 16);
        let bytes_2 = 1u32; // sheathed

        let mut forward = ModernFieldsArray::new(ModernObjectType::ActivePlayer, 1);
        forward.set_vanilla(UNIT_FIELD_BYTES_1, bytes_1);
        forward.set_vanilla(UNIT_FIELD_BYTES_2, bytes_2);

        let mut reverse = ModernFieldsArray::new(ModernObjectType::ActivePlayer, 1);
        reverse.set_vanilla(UNIT_FIELD_BYTES_2, bytes_2);
        reverse.set_vanilla(UNIT_FIELD_BYTES_1, bytes_1);

        let expected = 1u32 | (shapeshift_form << 24);
        assert_eq!(
            forward.modern_value(MODERN_UNIT_FIELD_BYTES_2),
            Some(expected)
        );
        assert_eq!(
            reverse.modern_value(MODERN_UNIT_FIELD_BYTES_2),
            Some(expected)
        );
    }

    /// The bug this fixes: `PLAYER_VISIBLE_ITEM_<n>_0` (the item entry) never joins modern's
    /// single `PLAYER_VISIBLE_ITEM` array by name, so every equipped item was silently dropped and
    /// no equipped items rendered on other players.
    #[test]
    fn visible_item_entry_maps_into_the_modern_array() {
        let writes = repack(
            ModernObjectType::ActivePlayer,
            PLAYER_VISIBLE_ITEM_1_0,
            12345,
        )
        .expect("slot 0 entry must map");
        assert_eq!(writes.as_slice(), [(MODERN_PLAYER_VISIBLE_ITEM, 12345)]);
    }

    #[test]
    fn visible_item_perm_enchant_maps_to_the_second_modern_field() {
        let writes = repack(
            ModernObjectType::ActivePlayer,
            PLAYER_VISIBLE_ITEM_1_0 + 1,
            999,
        )
        .expect("slot 0 enchant must map");
        assert_eq!(writes.as_slice(), [(MODERN_PLAYER_VISIBLE_ITEM + 1, 999)]);
    }

    /// The strides differ (12 vanilla, 2 modern), so slot n does not land at n * 12.
    #[test]
    fn visible_item_slots_use_the_modern_stride() {
        let slot_3_entry = PLAYER_VISIBLE_ITEM_1_0 + 3 * VANILLA_VISIBLE_ITEM_STRIDE;
        let writes =
            repack(ModernObjectType::ActivePlayer, slot_3_entry, 42).expect("slot 3 must map");
        assert_eq!(
            writes.as_slice(),
            [(
                MODERN_PLAYER_VISIBLE_ITEM + 3 * MODERN_VISIBLE_ITEM_STRIDE,
                42
            )]
        );
    }

    /// Temp enchant (offset 2) has nowhere to go in modern's two-field-per-slot array and must be
    /// dropped, not folded into the perm-enchant field where it could overwrite a real value.
    #[test]
    fn visible_item_temp_enchant_is_not_mapped() {
        assert!(repack(
            ModernObjectType::ActivePlayer,
            PLAYER_VISIBLE_ITEM_1_0 + 2,
            777
        )
        .is_none());
    }

    /// A creature field at the same index must not be rewritten as visible-item data.
    #[test]
    fn visible_item_indices_are_player_only() {
        assert!(repack(ModernObjectType::Unit, PLAYER_VISIBLE_ITEM_1_0, 12345).is_none());
    }
}
