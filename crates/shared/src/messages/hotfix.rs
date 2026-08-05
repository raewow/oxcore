//! DB2 hotfix replies (modern only)
//!
//! 1.14 has no item query. `CMSG_ITEM_QUERY_SINGLE` and `SMSG_ITEM_QUERY_SINGLE_RESPONSE` are simply
//! absent from its opcode table, so the route vanilla uses to send an item's name, icon and stats
//! does not exist. Instead the client treats the server as a source of DB2 *hotfix* records: it sends
//! [`Opcode::CMSG_DB_QUERY_BULK`] naming a table and a batch of record ids, and expects one
//! [`SmsgDbReply`] per id carrying that row's fields inline, in the exact column order the client's
//! schema for that table expects.
//!
//! Three tables carry everything a 1.12 server has to say:
//!
//! * [`Db2Hash::Item`] — the small "what kind of thing is this" row: class, inventory type, icon.
//! * [`Db2Hash::ItemSparse`] — everything else, including the name and description strings.
//! * [`Db2Hash::BroadcastText`] — NPC dialogue. Gossip and quest text reference these by id rather
//!   than carrying strings, which is why [`crate::messages::gossip::SmsgNpcTextUpdate`] sends ids.
//!
//! A row the server cannot supply is answered with [`HotfixStatus::Invalid`] and an empty body rather
//! than left unanswered — the client blocks the frame that asked until *some* reply arrives. This
//! holds for a whole *table* the server has never heard of too, not just a missing row in a known
//! one: every id in the batch gets an `Invalid` reply, echoing the table hash straight off the wire
//! rather than requiring [`Db2Hash`] to name it first. Dropping the batch silently — the original
//! bug here — left the client waiting on a query it would never see answered.

use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::{Opcode, WorldPacket};

/// Identifies which DB2 table a query or reply concerns.
///
/// The values are the client's own table-name hashes; they are not sequential and must be sent
/// verbatim. Only the tables a 1.12 world can populate are listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Db2Hash {
    BroadcastText = 0x0218_26BB,
    Item = 0x5023_8EC2,
    ItemSparse = 0x919B_E54E,
}

impl Db2Hash {
    /// Recognise a table hash off the wire, or `None` for a table we do not serve.
    pub fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0x0218_26BB => Some(Self::BroadcastText),
            0x5023_8EC2 => Some(Self::Item),
            0x919B_E54E => Some(Self::ItemSparse),
            _ => None,
        }
    }

    pub fn raw(self) -> u32 {
        self as u32
    }
}

/// What the client should do with a replied record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HotfixStatus {
    /// The body holds a record; use it.
    Valid = 1,
    /// The row was deleted; drop any cached copy.
    RecordRemoved = 2,
    /// No such record. The body is empty and the client stops waiting on it.
    Invalid = 3,
    NotPublic = 4,
}

/// SMSG_DB_REPLY - one DB2 record, or a negative answer for one record id.
#[derive(Debug, Clone)]
pub struct SmsgDbReply {
    /// The table hash straight off the client's query. A raw `u32` rather than [`Db2Hash`] so a
    /// table we cannot name can still be echoed back verbatim -- the client only needs its own hash
    /// reflected at it to stop waiting, not a name we recognise.
    pub table_hash: u32,
    pub record_id: u32,
    /// The hotfix generation the client caches this under. Any stable value works for a server that
    /// never revises a row mid-session; the world uses its start time.
    pub timestamp: u32,
    pub status: HotfixStatus,
    /// The record body, built by one of the `write_*_record` functions below. Empty for a status
    /// other than [`HotfixStatus::Valid`].
    pub data: Vec<u8>,
}

impl SmsgDbReply {
    /// A negative answer: the client stops waiting on this id and shows its own fallback. Takes the
    /// raw table hash so this covers a table [`Db2Hash`] does not name too.
    pub fn unknown(table_hash: u32, record_id: u32, timestamp: u32) -> Self {
        Self {
            table_hash,
            record_id,
            timestamp,
            status: HotfixStatus::Invalid,
            data: Vec::new(),
        }
    }
}

impl ToWorldPacket for SmsgDbReply {
    /// Modern-only: vanilla has no hotfix mechanism, so there is no `to_vanilla` body to speak of.
    ///
    /// The 3-bit status is followed immediately by a `u32` length, whose write flushes the partial
    /// byte — so the status occupies a byte of its own. That is the layout the client reads, not an
    /// accident of our writer.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.table_hash);
        writer.write_u32(self.record_id);
        writer.write_u32(self.timestamp);
        writer.write_bits(self.status as u32, 3);
        writer.write_u32(self.data.len() as u32);
        writer.write_bytes(&self.data);
        Some(writer.finish(Opcode::SMSG_DB_REPLY))
    }

    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_DB_REPLY)
    }
}

/// The fields of one `npc_text`/`broadcast_text` line, as the client's `BroadcastText` row.
#[derive(Debug, Clone, Default)]
pub struct BroadcastTextRecord {
    pub entry: u32,
    pub male_text: String,
    pub female_text: String,
    pub language: u32,
    pub emotes: [u16; 3],
    pub emote_delays: [u16; 3],
}

/// Serialise a `BroadcastText` row.
///
/// `VoiceOverPriorityID` was added in 1.14.1 and our target build is 1.14.2, so it is present.
pub fn write_broadcast_text_record(record: &BroadcastTextRecord) -> Vec<u8> {
    let mut buf = RecordBuf::new();
    buf.cstring(&record.male_text);
    buf.cstring(&record.female_text);
    buf.u32(record.entry);
    buf.u32(record.language);
    buf.u32(0); // ConditionID
    buf.u16(0); // EmotesID
    buf.u8(0); // Flags
    buf.u32(0); // ChatBubbleDurationMs
    buf.u32(0); // VoiceOverPriorityID
    for _ in 0..2 {
        buf.u32(0); // SoundEntriesID
    }
    for emote in record.emotes {
        buf.u16(emote);
    }
    for delay in record.emote_delays {
        buf.u16(delay);
    }
    buf.finish()
}

/// The subset of a 1.12 `item_template` row that the client's two item tables need.
///
/// One struct feeds both [`write_item_record`] and [`write_item_sparse_record`] because the tables
/// overlap heavily and splitting them would mean two lookups per item. Fields with no 1.12 source are
/// absent here and written as zero by the serialisers.
#[derive(Debug, Clone, Default)]
pub struct ItemHotfixRecord {
    pub entry: u32,
    pub name: String,
    pub description: String,
    pub class: u8,
    pub subclass: u8,
    pub material: u8,
    pub inventory_type: u8,
    pub required_level: i32,
    pub sheath_type: u8,
    pub random_property: u16,
    pub random_suffix: u16,
    /// Resolved from the item's display id; 0 leaves the client showing a blank icon.
    pub icon_file_data_id: i32,
    pub max_durability: u32,
    pub ammo_type: u8,
    pub damage_types: [u8; 5],
    pub armor: i16,
    /// Holy, fire, nature, frost, shadow, arcane — the order the client reads them in.
    pub resistances: [i16; 6],
    pub damage_min: [u16; 5],
    pub damage_max: [u16; 5],

    // ItemSparse-only fields.
    pub allowed_races: i64,
    pub allowed_classes: i16,
    pub duration: u32,
    pub bag_family: u32,
    pub ranged_mod: f32,
    pub max_stack_size: i32,
    pub max_count: i32,
    pub required_spell: u32,
    pub sell_price: u32,
    pub buy_price: u32,
    pub buy_count: u32,
    pub flags: u32,
    pub flags_extra: u32,
    pub holiday_id: u16,
    pub item_limit_category: u16,
    pub gem_properties: u16,
    pub socket_bonus: u16,
    pub totem_category: u16,
    pub map_id: u16,
    pub area_id: u16,
    pub item_set: u16,
    pub lock_id: u16,
    pub start_quest_id: u16,
    pub page_text: u16,
    pub delay: u16,
    pub required_rep_faction: u16,
    pub required_skill_level: u16,
    pub required_skill_id: u16,
    pub item_level: u16,
    pub socket_colors: [u8; 3],
    pub page_material: u8,
    pub language: u8,
    pub bonding: u8,
    pub stat_types: [i8; 10],
    pub stat_values: [i8; 10],
    pub container_slots: u8,
    pub required_rep_value: u8,
    pub required_city_rank: u8,
    pub required_honor_rank: u8,
    pub quality: u8,
}

/// Serialise an `Item` row — the short table the client consults for kind and icon.
pub fn write_item_record(item: &ItemHotfixRecord) -> Vec<u8> {
    let mut buf = RecordBuf::new();
    buf.u8(item.class);
    buf.u8(item.subclass);
    buf.u8(item.material);
    buf.u8(item.inventory_type);
    buf.i32(item.required_level);
    buf.u8(item.sheath_type);
    buf.u16(item.random_property);
    buf.u16(item.random_suffix);
    buf.i8(-1); // ItemNameDescriptionID
    buf.u16(0); // ModifiedCraftingReagentItemID
    buf.i32(item.icon_file_data_id);
    buf.u8(0); // ContentTuningID
    buf.i32(0); // CraftingQualityID
    buf.u32(item.max_durability);
    buf.u8(item.ammo_type);
    for damage_type in item.damage_types {
        buf.u8(damage_type);
    }
    buf.i16(item.armor);
    for resistance in item.resistances {
        buf.i16(resistance);
    }
    for value in item.damage_min {
        buf.u16(value);
    }
    for value in item.damage_max {
        buf.u16(value);
    }
    buf.finish()
}

/// Serialise an `ItemSparse` row — the long table holding the name, description and every stat.
///
/// The four name strings are the client's name-variant slots, written **high index first**. A 1.12
/// item has one name, so it goes in all four; sending it only in the first leaves the tooltip blank
/// in three of the four contexts the client picks a variant for.
pub fn write_item_sparse_record(item: &ItemHotfixRecord) -> Vec<u8> {
    let mut buf = RecordBuf::new();
    buf.i64(item.allowed_races);
    buf.cstring(&item.description);
    for _ in 0..4 {
        buf.cstring(&item.name);
    }
    buf.f32(1.0); // DmgVariance
    buf.u32(item.duration);
    buf.f32(0.0); // QualityModifier
    buf.u32(item.bag_family);
    buf.f32(item.ranged_mod);
    for _ in 0..10 {
        buf.f32(0.0); // StatPercentageOfSocket
    }
    for _ in 0..10 {
        buf.i32(0); // StatPercentEditor
    }
    buf.i32(item.max_stack_size);
    buf.i32(item.max_count);
    buf.u32(item.required_spell);
    buf.u32(item.sell_price);
    buf.u32(item.buy_price);
    buf.u32(item.buy_count);
    buf.f32(1.0); // VendorStackCount
    buf.f32(1.0); // PriceVariance
    buf.u32(item.flags);
    buf.u32(item.flags_extra);
    for _ in 0..3 {
        buf.i32(0); // remaining Flags words
    }
    buf.u32(item.max_durability);
    buf.u16(0); // ItemNameDescriptionID
    buf.u16(0); // RequiredTransmogHoliday
    buf.u16(item.holiday_id);
    buf.u16(item.item_limit_category);
    buf.u16(item.gem_properties);
    buf.u16(item.socket_bonus);
    buf.u16(item.totem_category);
    buf.u16(item.map_id);
    buf.u16(item.area_id);
    buf.u16(0); // InstanceID
    buf.u16(item.item_set);
    buf.u16(item.lock_id);
    buf.u16(item.start_quest_id);
    buf.u16(item.page_text);
    buf.u16(item.delay);
    buf.u16(item.required_rep_faction);
    buf.u16(item.required_skill_level);
    buf.u16(item.required_skill_id);
    buf.u16(item.item_level);
    buf.i16(item.allowed_classes);
    buf.u16(item.random_suffix);
    buf.u16(item.random_property);
    for value in item.damage_min {
        buf.u16(value);
    }
    for value in item.damage_max {
        buf.u16(value);
    }
    buf.i16(item.armor);
    for resistance in item.resistances {
        buf.i16(resistance);
    }
    buf.u16(0); // ScalingStatDistribution
                // ExpansionID. 254 is "classic", which is what makes the client apply Era item rules rather
                // than treating the row as a modern-expansion item.
    buf.u8(254);
    buf.u8(0); // ArtifactID
    buf.u8(0); // SpellWeight
    buf.u8(0); // SpellWeightCategory
    for color in item.socket_colors {
        buf.u8(color);
    }
    buf.u8(item.sheath_type);
    buf.u8(item.material);
    buf.u8(item.page_material);
    buf.u8(item.language);
    buf.u8(item.bonding);
    buf.u8(item.damage_types[0]);
    for stat_type in item.stat_types {
        buf.i8(stat_type);
    }
    buf.u8(item.container_slots);
    buf.u8(item.required_rep_value);
    buf.u8(item.required_city_rank);
    buf.u8(item.required_honor_rank);
    buf.u8(item.inventory_type);
    buf.u8(item.quality);
    buf.u8(item.ammo_type);
    for stat_value in item.stat_values {
        buf.i8(stat_value);
    }
    buf.i8(item.required_level as i8);
    buf.finish()
}

/// A plain little-endian byte sink for record bodies.
///
/// Records are pure byte sequences with no bit packing, so they are built here rather than through
/// `BitWriter`: the reply's own length prefix is the only thing that needs to know their size.
struct RecordBuf(Vec<u8>);

impl RecordBuf {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn finish(self) -> Vec<u8> {
        self.0
    }

    fn u8(&mut self, value: u8) {
        self.0.push(value);
    }

    fn i8(&mut self, value: i8) {
        self.0.push(value as u8);
    }

    fn u16(&mut self, value: u16) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    fn i16(&mut self, value: i16) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    fn i32(&mut self, value: i32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    fn f32(&mut self, value: f32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    fn cstring(&mut self, value: &str) {
        self.0.extend_from_slice(value.as_bytes());
        self.0.push(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_hash_survives_a_round_trip() {
        for table in [Db2Hash::BroadcastText, Db2Hash::Item, Db2Hash::ItemSparse] {
            assert_eq!(Db2Hash::from_raw(table.raw()), Some(table));
        }
        assert_eq!(Db2Hash::from_raw(0), None);
    }

    /// `SmsgDbReply::unknown` must accept a table hash `Db2Hash` has never heard of (e.g.
    /// `SpellVisual`, which the client queries but we do not yet serve) and echo it back verbatim.
    /// The bug this guards: `table_hash` used to be typed `Db2Hash`, so a reply could only be built
    /// for a table we recognised -- an unrecognised one had nothing well-formed to send and the
    /// whole query batch got dropped, leaving the client's query permanently unanswered.
    #[test]
    fn an_unrecognised_table_hash_still_produces_a_wire_reply() {
        let spell_visual_hash = 0xF724_96D9; // SpellVisual -- not a Db2Hash variant
        assert_eq!(Db2Hash::from_raw(spell_visual_hash), None);

        let reply = SmsgDbReply::unknown(spell_visual_hash, 67, 7);
        let packet = reply.to_modern().unwrap();
        let body = packet.contents();
        assert_eq!(
            u32::from_le_bytes(body[0..4].try_into().unwrap()),
            spell_visual_hash,
            "the raw hash must round-trip even with no Db2Hash name for it"
        );
        assert_eq!(u32::from_le_bytes(body[4..8].try_into().unwrap()), 67);
    }

    /// The status is three bits, but the length word that follows flushes them, so it costs a whole
    /// byte and the header is a fixed 17 bytes. Reading the length one byte early is the failure this
    /// pins down, and it would make every record body land at the wrong offset.
    #[test]
    fn the_reply_header_is_byte_aligned_after_the_status_bits() {
        let reply = SmsgDbReply {
            data: vec![0xAB, 0xCD],
            status: HotfixStatus::Valid,
            ..SmsgDbReply::unknown(Db2Hash::ItemSparse.raw(), 25, 7)
        };
        let packet = reply.to_modern().unwrap();
        assert_eq!(packet.size(), 17 + 2);

        let body = packet.contents();
        assert_eq!(
            u32::from_le_bytes(body[0..4].try_into().unwrap()),
            Db2Hash::ItemSparse.raw()
        );
        assert_eq!(u32::from_le_bytes(body[4..8].try_into().unwrap()), 25);
        assert_eq!(u32::from_le_bytes(body[8..12].try_into().unwrap()), 7);
        // Bits pack from the top of the byte down, so a 3-bit field sits in bits 7..5.
        assert_eq!(body[12] >> 5, HotfixStatus::Valid as u8);
        assert_eq!(u32::from_le_bytes(body[13..17].try_into().unwrap()), 2);
        assert_eq!(&body[17..], &[0xAB, 0xCD]);
    }

    /// A 1.12 item has one name and the client has four name slots; all four must carry it.
    #[test]
    fn every_name_slot_carries_the_item_name() {
        let item = ItemHotfixRecord {
            name: "Worn Shortsword".to_string(),
            ..Default::default()
        };
        let record = write_item_sparse_record(&item);
        let occurrences = record
            .windows(b"Worn Shortsword".len())
            .filter(|window| *window == b"Worn Shortsword")
            .count();
        assert_eq!(occurrences, 4);
    }

    /// The `Item` row is fixed-width, so a wrong field width shifts the icon and every stat after it.
    #[test]
    fn the_item_row_is_a_fixed_width() {
        let record = write_item_record(&ItemHotfixRecord::default());
        assert_eq!(record.len(), 69);
    }
}
