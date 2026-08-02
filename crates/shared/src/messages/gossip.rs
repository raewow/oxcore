//! Gossip system message structs
//!
//! This module contains type-safe message structures for all gossip-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgGossipMessage`] - Main gossip menu message with options and quests
//! - [`SmsgGossipComplete`] - Close gossip window
//! - [`SmsgGossipPoi`] - Point of Interest map marker
//! - [`SmsgNpcTextUpdate`] - NPC text/greeting response
//! - [`SmsgShowBank`] - Open bank window

use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::packet::WorldPacketGuidExt;
use crate::protocol::ObjectGuid;
use crate::protocol::{Opcode, WorldPacket};

/// Gossip option data for SMSG_GOSSIP_MESSAGE
#[derive(Debug, Clone)]
pub struct GossipOptionData {
    /// Option index (0-based)
    pub index: u32,
    /// Icon ID (0-15, see GossipIcon enum)
    pub icon: u8,
    /// Whether option requires text input (coded)
    pub coded: bool,
    /// Money cost in copper (for input box)
    pub money: u32,
    /// Option display text
    pub text: String,
}

/// Quest data for gossip menu
#[derive(Debug, Clone)]
pub struct GossipQuestData {
    /// Quest ID
    pub quest_id: u32,
    /// Quest icon (0=available, 1=completed, etc.)
    pub icon: u32,
    /// Quest level
    pub level: u32,
    /// Quest title
    pub title: String,
}

/// SMSG_GOSSIP_MESSAGE (0x17D) - Main gossip menu
///
/// Sent when player right-clicks an NPC with gossip options.
/// Contains the menu text, gossip options, and available quests.
#[derive(Debug, Clone)]
pub struct SmsgGossipMessage {
    /// NPC or GameObject GUID
    pub source_guid: ObjectGuid,
    /// Menu ID
    pub menu_id: u32,
    /// Text ID (from npc_text table)
    pub text_id: u32,
    /// Gossip options
    pub options: Vec<GossipOptionData>,
    /// Available quests
    pub quests: Vec<GossipQuestData>,
}

impl ToWorldPacket for SmsgGossipMessage {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GOSSIP_MESSAGE);

        // Write source GUID (unpacked, fixed 8 bytes — Vanilla 1.12.1 protocol)
        packet.write_guid(self.source_guid);

        // Write text ID (NOT menu_id - that's server-side only!)
        packet.write_u32(self.text_id);

        // Write gossip options count and data
        packet.write_u32(self.options.len() as u32);
        for opt in &self.options {
            packet.write_u32(opt.index);
            packet.write_u8(opt.icon);
            packet.write_u8(if opt.coded { 1 } else { 0 });
            // Note: No money field in 1.12.1 - it was added in later versions
            packet.write_cstring(&opt.text);
        }

        // Write quest count and data
        packet.write_u32(self.quests.len() as u32);
        for quest in &self.quests {
            packet.write_u32(quest.quest_id);
            packet.write_u32(quest.icon);
            packet.write_u32(quest.level);
            packet.write_cstring(&quest.title);
        }

        packet
    }

    /// `GossipMessagePkt::Write`, per the 1.14 wire format.
    ///
    /// A reshape, not a renumber. Both counts move ahead of *both* lists; the option strings become
    /// 12-bit length prefixes inside a bit run rather than null-terminated; and each option gains a
    /// confirm string, a status field, an optional spell id and a treasure list. `GossipID` is a real
    /// wire field here, where vanilla keeps the menu id server-side and sends only the text id.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.source_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);

        writer.write_i32(self.menu_id as i32); // GossipID
        writer.write_i32(0); // FriendshipFactionID -- no friendship factions in Classic Era
        writer.write_i32(self.text_id as i32);

        // Both counts precede both lists, where vanilla interleaves count-then-list twice.
        writer.write_i32(self.options.len() as i32);
        writer.write_i32(self.quests.len() as i32);

        for option in &self.options {
            writer.write_i32(option.index as i32);
            writer.write_u8(option.icon);
            // 1.14's `OptionFlags` occupies the byte vanilla used for the coded flag, and bit 0
            // still means "prompt for text", so the value carries over.
            writer.write_u8(u8::from(option.coded));
            writer.write_i32(option.money as i32); // OptionCost
                                                   // Added in 1.14.1; build 42597 is 1.14.2, so the field is present. Zero is the
                                                   // client's "use the option text as written" default.
            writer.write_u32(0); // Language

            let text = option.text.as_bytes();
            writer.write_bits(text.len() as u32, 12);
            writer.write_bits(0, 12); // Confirm length -- no confirmation prompts yet
            writer.write_bits(0, 2); // Status: 0 = available
            writer.write_bit(false); // HasSpellID
            writer.flush_bits();

            writer.write_i32(0); // Treasure.Items count

            writer.write_bytes(text);
            // Confirm string omitted: its length was written as zero above.
        }

        for quest in &self.quests {
            write_modern_gossip_quest(&mut writer, quest);
        }

        Some(writer.finish(Opcode::SMSG_GOSSIP_MESSAGE))
    }
}

/// `ClientGossipQuest::Write`, per the 1.14 wire format.
///
/// Shared by the gossip menu and the quest-giver list, which send the same entry shape.
pub(crate) fn write_modern_gossip_quest(writer: &mut BitWriter, quest: &GossipQuestData) {
    writer.write_u32(quest.quest_id);
    writer.write_u32(0); // ContentTuningID -- a retail scaling concept with no Classic source
                         // Vanilla uses dialog-status values here (5 = available, 3/4 = current quest), whereas
                         // 1.14's ClientGossipQuest::QuestType uses 2 for an offer and 4 for a current quest.
    writer.write_i32(modern_quest_type(quest.icon));
    writer.write_i32(quest.level as i32);
    writer.write_i32(255); // QuestMaxLevel
    writer.write_u32(8); // QuestFlags
    writer.write_u32(0); // QuestFlagsEx

    let title = quest.title.as_bytes();
    writer.write_bit(false); // Repeatable
                             // 9 bits, not 12: quest titles have their own narrower length field.
    writer.write_bits(title.len() as u32, 9);
    writer.flush_bits();
    writer.write_bytes(title);
}

fn modern_quest_type(vanilla_icon: u32) -> i32 {
    match vanilla_icon {
        5 => 2,         // DIALOG_STATUS_AVAILABLE
        3 | 4 | 7 => 4, // incomplete or ready to turn in
        icon => icon as i32,
    }
}

/// SMSG_GOSSIP_COMPLETE (0x17E) - Close gossip window
///
/// Sent to close the gossip dialog on the client.
#[derive(Debug, Clone)]
pub struct SmsgGossipComplete;

impl ToWorldPacket for SmsgGossipComplete {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_GOSSIP_COMPLETE)
    }

    /// Empty in vanilla, one flushed bit from 1.14.2 on — gated on `AddedInVersion(9,2,0, 1,14,2, ...)`
    /// and our target build 42597 *is* 1.14.2, so the bit is present.
    ///
    /// Without it the client is left with an unclosed gossip window: the packet is one byte short of
    /// what it reads.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_bit(false); // SuppressSound
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_GOSSIP_COMPLETE))
    }
}

/// SMSG_SHOW_BANK (0x1B8) - Open bank window
///
/// Sent when a player interacts with a banker.
#[derive(Debug, Clone)]
pub struct SmsgShowBank {
    /// Banker NPC GUID
    pub banker_guid: ObjectGuid,
}

impl ToWorldPacket for SmsgShowBank {
    /// The same single GUID in 1.14, packed as a guid128.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.banker_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        Some(writer.finish(Opcode::SMSG_SHOW_BANK))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SHOW_BANK);
        packet.write_guid(self.banker_guid);
        packet
    }
}

/// SMSG_GOSSIP_POI (0x223) - Point of Interest
///
/// Displays a map marker on the client's world map.
#[derive(Debug, Clone)]
pub struct SmsgGossipPoi {
    /// POI ID
    pub poi_id: u32,
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
    /// Icon type
    pub icon: u32,
    /// Flags
    pub flags: u32,
    /// Data field (usually 0)
    pub data: u32,
    /// POI name
    pub name: String,
}

/// Longest POI name 1.14 can express: the length field is 6 bits.
const MODERN_POI_NAME_MAX: usize = 63;

impl ToWorldPacket for SmsgGossipPoi {
    /// The 1.14 layout adds a height, moves the flags to the end and packs them.
    ///
    /// Field by field against vanilla:
    ///
    /// * The position gains a **Z**. Vanilla map pins are two-dimensional, so it is zero — but the
    ///   field is read unconditionally, and omitting it slides the icon into the Z and the
    ///   importance into the icon, planting a quest-marker pin as a flight point.
    /// * `flags` no longer sits between the icon and the data word; it moves to the end as a 14-bit
    ///   field. Vanilla POI flags are small values, so the width is not a practical limit, but the
    ///   *position* change means writing them where vanilla did corrupts everything after.
    /// * `data` is the 1.14 `Importance` field, which is where it already ended up in vanilla's
    ///   ordering — only its neighbours moved.
    /// * The name length is 6 bits and precedes the string bytes rather than terminating them.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.poi_id);
        writer.write_f32(self.x);
        writer.write_f32(self.y);
        writer.write_f32(0.0); // Z -- vanilla POIs are flat map pins
        writer.write_u32(self.icon);
        writer.write_u32(self.data); // Importance
                                     // A later addition with no vanilla source; the client accepts zero.
        writer.write_u32(0);

        // Clamped rather than masked: a length that disagrees with the bytes that follow would
        // leave the client reading the tail of the name as the next packet.
        let name = self.name.as_bytes();
        let name = &name[..name.len().min(MODERN_POI_NAME_MAX)];

        writer.write_bits(self.flags, 14);
        writer.write_bits(name.len() as u32, 6);
        writer.flush_bits();

        writer.write_bytes(name);
        Some(writer.finish(Opcode::SMSG_GOSSIP_POI))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GOSSIP_POI);
        packet.write_u32(self.poi_id);
        packet.write_f32(self.x);
        packet.write_f32(self.y);
        packet.write_u32(self.icon);
        packet.write_u32(self.flags);
        packet.write_u32(self.data);
        packet.write_cstring(&self.name);
        packet
    }
}

/// NPC text option (one of 8 possible variants)
#[derive(Debug, Clone)]
pub struct NpcTextOption {
    /// Probability weight for random selection
    pub probability: f32,
    /// Broadcast text id. Vanilla resolves it server-side into the strings below; 1.14 sends the id
    /// itself and asks for the row separately.
    pub broadcast_text_id: u32,
    /// Male text
    pub male_text: String,
    /// Female text
    pub female_text: String,
    /// Language ID
    pub language_id: u32,
    /// Emote delays [delay1, delay2, delay3]
    pub emote_delays: [u32; 3],
    /// Emote IDs [id1, id2, id3]
    pub emote_ids: [u32; 3],
}

impl Default for NpcTextOption {
    fn default() -> Self {
        Self {
            probability: 0.0,
            broadcast_text_id: 0,
            male_text: String::new(),
            female_text: String::new(),
            language_id: 0,
            emote_delays: [0; 3],
            emote_ids: [0; 3],
        }
    }
}

// Manual Copy not possible with String fields; keep Clone only.

/// SMSG_NPC_TEXT_UPDATE (0x180) - NPC text/greeting
///
/// Sent in response to CMSG_NPC_TEXT_QUERY.
/// Contains up to 8 text variants with probabilities.
#[derive(Debug, Clone)]
pub struct SmsgNpcTextUpdate {
    /// Text ID being queried
    pub text_id: u32,
    /// Text options (up to 8)
    pub options: [NpcTextOption; 8],
}

impl ToWorldPacket for SmsgNpcTextUpdate {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_NPC_TEXT_UPDATE);
        packet.write_u32(self.text_id);

        // Each option: probability(f32) + maleText(cstring) + femaleText(cstring)
        // + languageId(u32) + emoteDelay1(u32) + emoteId1(u32)
        // + emoteDelay2(u32) + emoteId2(u32) + emoteDelay3(u32) + emoteId3(u32)
        // Standard NPC text format
        for opt in &self.options {
            let male = if opt.male_text.is_empty() && opt.female_text.is_empty() {
                "Greetings $N"
            } else if opt.male_text.is_empty() {
                &opt.female_text
            } else {
                &opt.male_text
            };
            let female = if opt.male_text.is_empty() && opt.female_text.is_empty() {
                "Greetings $N"
            } else if opt.female_text.is_empty() {
                &opt.male_text
            } else {
                &opt.female_text
            };
            packet.write_f32(opt.probability);
            packet.write_cstring(male);
            packet.write_cstring(female);
            packet.write_u32(opt.language_id);
            packet.write_u32(opt.emote_delays[0]);
            packet.write_u32(opt.emote_ids[0]);
            packet.write_u32(opt.emote_delays[1]);
            packet.write_u32(opt.emote_ids[1]);
            packet.write_u32(opt.emote_delays[2]);
            packet.write_u32(opt.emote_ids[2]);
        }

        packet
    }

    /// `QueryNPCTextResponse::Write`, per the 1.14 wire format.
    ///
    /// **1.14 does not carry the text.** It sends eight `BroadcastTextID`s and the client resolves
    /// each against its `BroadcastText` DB2 — where vanilla sends the male and female strings inline.
    /// So the male/female text, language and emote fields on this struct have nowhere to go here.
    ///
    /// The client does not have those rows either: an Era client's DB2 has no vanilla NPC dialogue in
    /// it. It asks for each id it does not know via `CMSG_DB_QUERY_BULK`, and the server answers with
    /// the row, so the id only has to be one *we* can resolve. `npc_text.broadcast_text_id` is exactly
    /// that, and the same value drives the vanilla path's text lookup — the two cannot drift.
    fn to_modern(&self) -> Option<WorldPacket> {
        const TEXT_OPTIONS: usize = 8;

        let mut writer = BitWriter::new();
        writer.write_u32(self.text_id);
        // `Allow` gates the payload. True with zero ids keeps the fixed-size body the client
        // expects; false makes it treat the text id as unknown.
        writer.write_bit(true);
        writer.write_i32((TEXT_OPTIONS * (4 + 4)) as i32);

        for option in &self.options {
            writer.write_f32(option.probability);
        }
        for option in &self.options {
            writer.write_u32(option.broadcast_text_id);
        }

        Some(writer.finish(Opcode::SMSG_NPC_TEXT_UPDATE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ObjectGuid;

    fn read_u32_le(data: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
    }

    fn read_u64_le(data: &[u8], offset: usize) -> u64 {
        u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
    }

    fn read_f32_le(data: &[u8], offset: usize) -> f32 {
        f32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
    }

    fn read_cstring(data: &[u8], offset: usize) -> (String, usize) {
        let end = data[offset..].iter().position(|&b| b == 0).unwrap() + offset;
        let s = std::str::from_utf8(&data[offset..end]).unwrap().to_string();
        (s, end + 1)
    }

    #[test]
    fn modern_quest_type_distinguishes_offers_from_current_quests() {
        assert_eq!(modern_quest_type(5), 2, "available quest");
        assert_eq!(modern_quest_type(3), 4, "incomplete quest");
        assert_eq!(modern_quest_type(4), 4, "rewardable quest");
        assert_eq!(modern_quest_type(7), 4, "completed quest");
    }

    // ---- SMSG_GOSSIP_MESSAGE ----

    #[test]
    fn smsg_gossip_message_guid_is_unpacked() {
        // GUID must be written as fixed 8-byte little-endian u64, NOT packed
        // (packed would have a variable-length mask byte prefix, crashing the client).
        let guid = ObjectGuid::from_raw(0xF130_0000_00C6_0001);
        let msg = SmsgGossipMessage {
            source_guid: guid,
            menu_id: 9999,
            text_id: 0x0000_0539,
            options: vec![],
            quests: vec![],
        };
        let pkt = msg.to_vanilla();
        let data = pkt.data();

        assert_eq!(
            read_u64_le(data, 0),
            0xF130_0000_00C6_0001,
            "GUID must be unpacked (fixed 8 bytes)"
        );
    }

    #[test]
    fn smsg_gossip_message_no_menu_id_in_wire() {
        // menu_id is server-side state only — bytes 8-11 must be text_id, not menu_id.
        let guid = ObjectGuid::from_raw(0xF130_0000_00C6_0001);
        let msg = SmsgGossipMessage {
            source_guid: guid,
            menu_id: 0xDEAD_BEEF,
            text_id: 0x0000_1234,
            options: vec![],
            quests: vec![],
        };
        let pkt = msg.to_vanilla();
        let data = pkt.data();

        let after_guid = read_u32_le(data, 8);
        assert_eq!(
            after_guid, 0x0000_1234,
            "Bytes 8-11 must be text_id, not menu_id"
        );
    }

    #[test]
    fn smsg_gossip_message_field_order() {
        // Full layout: GUID(8) | text_id(4) | count(4) | [index(4) icon(1) coded(1) text\0] | quests(4)
        let guid = ObjectGuid::from_raw(0xF130_0000_00C6_0001);
        let msg = SmsgGossipMessage {
            source_guid: guid,
            menu_id: 0,
            text_id: 538,
            options: vec![GossipOptionData {
                index: 0,
                icon: 3,
                coded: false,
                money: 0,
                text: "Train".to_string(),
            }],
            quests: vec![],
        };
        let pkt = msg.to_vanilla();
        let data = pkt.data();

        let mut pos = 0;
        assert_eq!(read_u64_le(data, pos), guid.raw());
        pos += 8;
        assert_eq!(read_u32_le(data, pos), 538);
        pos += 4; // text_id
        assert_eq!(read_u32_le(data, pos), 1);
        pos += 4; // options count
        assert_eq!(read_u32_le(data, pos), 0);
        pos += 4; // option index
        assert_eq!(data[pos], 3);
        pos += 1; // icon
        assert_eq!(data[pos], 0);
        pos += 1; // coded = false
        let (text, next) = read_cstring(data, pos);
        assert_eq!(text, "Train");
        pos = next;
        assert_eq!(read_u32_le(data, pos), 0); // quests count
    }

    // ---- SMSG_SHOW_BANK ----

    #[test]
    fn smsg_show_bank_writes_raw_banker_guid() {
        let guid = ObjectGuid::from_raw(0xF130_0000_0998_0001);
        let msg = SmsgShowBank { banker_guid: guid };
        let pkt = msg.to_vanilla();
        let data = pkt.data();

        assert_eq!(pkt.opcode(), Opcode::SMSG_SHOW_BANK);
        assert_eq!(data.len(), 8);
        assert_eq!(read_u64_le(data, 0), guid.raw());
    }

    // ---- SMSG_NPC_TEXT_UPDATE ----

    #[test]
    fn smsg_npc_text_update_field_order() {
        // Per-option layout:
        // probability(f32) + maleText(cstring) + femaleText(cstring) + languageId(u32)
        // + emoteDelay1(u32) + emoteId1(u32) + emoteDelay2(u32) + emoteId2(u32)
        // + emoteDelay3(u32) + emoteId3(u32)
        let mut options: [NpcTextOption; 8] = std::array::from_fn(|_| NpcTextOption::default());
        options[0] = NpcTextOption {
            probability: 1.0,
            broadcast_text_id: 0,
            male_text: "Hello lad".to_string(),
            female_text: "Hello lass".to_string(),
            language_id: 7,
            emote_delays: [100, 200, 300],
            emote_ids: [1, 2, 3],
        };
        let msg = SmsgNpcTextUpdate {
            text_id: 538,
            options,
        };
        let pkt = msg.to_vanilla();
        let data = pkt.data();

        let mut pos = 0;
        assert_eq!(read_u32_le(data, pos), 538);
        pos += 4; // text_id

        // option 0 fields
        assert_eq!(read_f32_le(data, pos), 1.0);
        pos += 4;
        let (male, next) = read_cstring(data, pos);
        pos = next;
        assert_eq!(male, "Hello lad");
        let (female, next) = read_cstring(data, pos);
        pos = next;
        assert_eq!(female, "Hello lass");
        assert_eq!(read_u32_le(data, pos), 7);
        pos += 4; // languageId
        assert_eq!(read_u32_le(data, pos), 100);
        pos += 4; // emoteDelay1
        assert_eq!(read_u32_le(data, pos), 1);
        pos += 4; // emoteId1
        assert_eq!(read_u32_le(data, pos), 200);
        pos += 4; // emoteDelay2
        assert_eq!(read_u32_le(data, pos), 2);
        pos += 4; // emoteId2
        assert_eq!(read_u32_le(data, pos), 300);
        pos += 4; // emoteDelay3
        assert_eq!(read_u32_le(data, pos), 3); // emoteId3
    }

    #[test]
    fn smsg_npc_text_update_empty_male_uses_female() {
        // When male_text is empty, female_text is written in the male slot
        let mut options: [NpcTextOption; 8] = std::array::from_fn(|_| NpcTextOption::default());
        options[0] = NpcTextOption {
            probability: 1.0,
            broadcast_text_id: 0,
            male_text: "".to_string(),
            female_text: "Hi there".to_string(),
            language_id: 0,
            emote_delays: [0; 3],
            emote_ids: [0; 3],
        };
        let msg = SmsgNpcTextUpdate {
            text_id: 1,
            options,
        };
        let pkt = msg.to_vanilla();
        let data = pkt.data();

        // text_id(4) + probability(4) = offset 8, then male slot cstring
        let (male_slot, _) = read_cstring(data, 4 + 4);
        assert_eq!(
            male_slot, "Hi there",
            "Empty male_text should fall back to female_text in the male slot"
        );
    }
}
