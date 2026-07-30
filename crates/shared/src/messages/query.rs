//! Query message structs - name, creature, item queries

use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::{ObjectGuid, Opcode, WorldPacket};

// =========================================================================
// CREATURE QUERY RESPONSE
// =========================================================================

/// SMSG_CREATURE_QUERY_RESPONSE - Response to creature template query
///
/// Sent when a client requests creature info (name, type, family, etc.)
/// The client sends CMSG_CREATURE_QUERY when it needs to display a creature.
///
/// ## Packet Format (Vanilla 1.12.1)
/// - entry (u32) - Creature entry (high bit set if not found)
/// - name (cstring) - Creature name
/// - name2 (u8) - Always 0 (null terminator)
/// - name3 (u8) - Always 0 (null terminator)
/// - name4 (u8) - Always 0 (null terminator)
/// - subname (cstring) - Creature subname/title (e.g. "General Goods")
/// - type_flags (u32) - Static flags
/// - creature_type (u32) - Beast, Humanoid, Undead, etc.
/// - creature_family (u32) - For hunter pets (Wolf, Cat, etc.)
/// - rank (u32) - Normal, Elite, Rare, Rare Elite, Boss
/// - unknown (u32) - Always 0
/// - pet_spell_data_id (u32) - For pets with spells
/// - display_id (u32) - Model to display
/// - civilian (u8) - Civilian flag (no PvP flagging)
/// - racial_leader (u8) - Racial leader flag
#[derive(Debug, Clone)]
pub struct SmsgCreatureQueryResponse<'a> {
    /// Creature entry ID
    pub entry: u32,
    /// Creature name
    pub name: &'a str,
    /// Creature subname/title (e.g. "General Goods", "Quest Giver")
    pub subname: &'a str,
    /// Type flags (static_flags)
    pub type_flags: u32,
    /// Creature type (0=None, 1=Beast, 2=Dragon, 3=Demon, 4=Elemental, 5=Giant,
    /// 6=Undead, 7=Humanoid, 8=Critter, 9=Mechanical, 10=Not specified)
    pub creature_type: u8,
    /// Creature family for hunter pets (0=None, 1=Wolf, 2=Cat, etc.)
    pub creature_family: u8,
    /// Creature rank (0=Normal, 1=Elite, 2=Rare Elite, 3=Boss, 4=Rare)
    pub rank: u8,
    /// Pet spell data ID (for pets with special abilities)
    pub pet_spell_data_id: u32,
    /// Display ID (model to render)
    pub display_id: u32,
    /// Civilian flag (prevents PvP flagging)
    pub civilian: u8,
    /// Racial leader flag
    pub racial_leader: u8,
}

impl<'a> SmsgCreatureQueryResponse<'a> {
    /// Create a new creature query response
    pub fn new(
        entry: u32,
        name: &'a str,
        subname: &'a str,
        type_flags: u32,
        creature_type: u8,
        display_id: u32,
    ) -> Self {
        Self {
            entry,
            name,
            subname,
            type_flags,
            creature_type,
            creature_family: 0,
            rank: 0,
            pet_spell_data_id: 0,
            display_id,
            civilian: 0,
            racial_leader: 0,
        }
    }

    /// Create a "not found" response
    ///
    /// When entry is not found, we set the high bit (0x80000000) and send minimal data.
    pub fn not_found(entry: u32) -> Self {
        Self {
            entry: entry | 0x80000000, // High bit indicates not found
            name: "",
            subname: "",
            type_flags: 0,
            creature_type: 0,
            creature_family: 0,
            rank: 0,
            pet_spell_data_id: 0,
            display_id: 0,
            civilian: 0,
            racial_leader: 0,
        }
    }
}

impl ToWorldPacket for SmsgCreatureQueryResponse<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CREATURE_QUERY_RESPONSE);
        packet.write_u32(self.entry);

        // Only write remaining fields if entry high bit is not set (found)
        if (self.entry & 0x80000000) == 0 {
            packet.write_cstring(self.name);
            packet.write_u8(0); // name2 (null terminator)
            packet.write_u8(0); // name3 (null terminator)
            packet.write_u8(0); // name4 (null terminator)
            packet.write_cstring(self.subname);
            packet.write_u32(self.type_flags);
            packet.write_u32(self.creature_type as u32);
            packet.write_u32(self.creature_family as u32);
            packet.write_u32(self.rank as u32);
            packet.write_u32(0); // unknown (always 0)
            packet.write_u32(self.pet_spell_data_id);
            packet.write_u32(self.display_id);
            packet.write_u8(self.civilian);
            packet.write_u8(self.racial_leader);
        }

        packet
    }

    /// `SMSG_QUERY_CREATURE_RESPONSE`, per HermesProxy `QueryPackets.cs:365`.
    ///
    /// Restructured rather than renumbered. The four name slots (localisations) come first as
    /// bit-packed *lengths*, then the strings, then the flat fields. Display info moved into a
    /// counted list with per-entry scale and probability, where vanilla had a single display id.
    ///
    /// String lengths here include the terminator -- `len + 1` -- and an empty optional string is
    /// length 0 rather than 1, so it is skipped entirely in the string run. Getting that wrong
    /// shifts every following field.
    fn to_modern(&self) -> Option<WorldPacket> {
        /// Name localisation slots the client always reads lengths for.
        const MAX_CREATURE_NAMES: usize = 4;
        /// Proxy creature ids (kill credit) the client always reads.
        const MAX_KILL_CREDIT: usize = 2;

        let mut writer = BitWriter::new();
        let found = (self.entry & 0x8000_0000) == 0;
        writer.write_u32(self.entry & 0x7FFF_FFFF);
        writer.write_bit(found);
        writer.flush_bits();

        if !found {
            return Some(writer.finish(Opcode::SMSG_CREATURE_QUERY_RESPONSE));
        }

        // Optional strings: length 0 means absent, and absent strings are not written below.
        let title_len = cstring_bits(self.subname);
        writer.write_bits(title_len, 11); // Title
        writer.write_bits(0, 11); // TitleAlt
        writer.write_bits(0, 6); // CursorName
        writer.write_bit(self.civilian != 0);
        writer.write_bit(self.racial_leader != 0);

        // Lengths for all four name slots and their alts, each as byte count plus terminator, so an
        // empty slot still declares 1. Note this does *not* mean an empty slot contributes a byte
        // to the string run below.
        for slot in 0..MAX_CREATURE_NAMES {
            let name_len = if slot == 0 {
                self.name.len() as u32 + 1
            } else {
                1
            };
            writer.write_bits(name_len, 11);
            writer.write_bits(1, 11); // NameAlt
        }
        writer.flush_bits();

        // Only *non-empty* strings are written, even though every slot declared a length above.
        // The asymmetry is deliberate and matches the reference: emitting a bare terminator for
        // each empty slot adds seven bytes the client does not read, and it then misparses the rest
        // of the template — and every response after it in the same burst.
        if !self.name.is_empty() {
            writer.write_cstring(self.name);
        }

        writer.write_u32(self.type_flags);
        writer.write_u32(0); // Flags[1]
        writer.write_i32(self.creature_type as i32);
        writer.write_i32(self.creature_family as i32);
        writer.write_i32(self.rank as i32);
        writer.write_u32(self.pet_spell_data_id);

        for _ in 0..MAX_KILL_CREDIT {
            writer.write_u32(0); // ProxyCreatureID
        }

        // One display option, always chosen.
        writer.write_i32(1); // CreatureDisplay.Count
        writer.write_f32(1.0); // TotalProbability
        writer.write_u32(self.display_id);
        writer.write_f32(1.0); // Scale
        writer.write_f32(1.0); // Probability

        writer.write_f32(1.0); // HpMulti
        writer.write_f32(1.0); // EnergyMulti

        writer.write_i32(0); // QuestItems.Count
        writer.write_u32(0); // MovementInfoID
        writer.write_i32(0); // HealthScalingExpansion
        writer.write_u32(0); // RequiredExpansion
        writer.write_u32(0); // VignetteID
        writer.write_i32(0); // Class
        writer.write_i32(0); // DifficultyID
        writer.write_i32(0); // WidgetSetID
        writer.write_i32(0); // WidgetSetUnitConditionID

        // Only the optionals that declared a non-zero length above.
        if title_len != 0 {
            writer.write_cstring(self.subname);
        }

        Some(writer.finish(Opcode::SMSG_CREATURE_QUERY_RESPONSE))
    }
}

/// Bit-length of an optional C string in a modern query response: zero when absent, otherwise the
/// byte count plus its terminator.
fn cstring_bits(value: &str) -> u32 {
    if value.is_empty() {
        0
    } else {
        value.len() as u32 + 1
    }
}

// =========================================================================
// NAME QUERY RESPONSE
// =========================================================================

/// SMSG_NAME_QUERY_RESPONSE - Response to name query
///
/// Sent when a client requests the name/info for a player GUID.
/// Used for chat messages, target frames, etc.
///
/// ## Packet Format (Vanilla 1.12.1)
/// - guid (u64) - Player GUID (NOT packed)
/// - name (cstring) - Player name
/// - realm (u8) - Realm name (0 for same realm)
/// - race (u32) - Player race
/// - gender (u32) - Player gender
/// - class (u32) - Player class
#[derive(Debug, Clone)]
pub struct SmsgNameQueryResponse<'a> {
    /// Player GUID
    pub guid: ObjectGuid,
    /// Player name
    pub name: &'a str,
    /// Realm name (empty string for same realm)
    pub realm: &'a str,
    /// Player race
    pub race: u8,
    /// Player gender
    pub gender: u8,
    /// Player class
    pub class: u8,
}

impl<'a> SmsgNameQueryResponse<'a> {
    /// Create a new name query response
    pub fn new(guid: ObjectGuid, name: &'a str, race: u8, gender: u8, class: u8) -> Self {
        Self {
            guid,
            name,
            realm: "",
            race,
            gender,
            class,
        }
    }
}

impl ToWorldPacket for SmsgNameQueryResponse<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_NAME_QUERY_RESPONSE);

        // Write GUID as u64 (NOT packed - vanilla 1.12.1 format)
        packet.write_u64(self.guid.raw());

        // Write name (null-terminated string)
        packet.write_cstring(self.name);

        // Write realm name
        if self.realm.is_empty() {
            packet.write_u8(0); // Empty string (null terminator only)
        } else {
            packet.write_cstring(self.realm);
        }

        // Write race, gender, class (for cross-realm support)
        packet.write_u32(self.race as u32);
        packet.write_u32(self.gender as u32);
        packet.write_u32(self.class as u32);

        packet
    }

    /// A much larger message than vanilla's: a result byte, the GUID as a packed 128-bit value,
    /// then a lookup block carrying bit-length-prefixed names, three more GUIDs and the virtual
    /// realm address.
    ///
    /// Several fields have no vanilla source and are sent empty: declined names (a Cyrillic-locale
    /// feature), the account and bnet-account GUIDs, and the guild club member id. The client uses
    /// them for social features, not for rendering a name, so zeros are safe here.
    fn to_modern(&self) -> Option<WorldPacket> {
        /// Declined-name cases the client always expects, even when unused.
        const DECLINED_NAME_CASES: usize = 5;

        let mut writer = BitWriter::new();
        writer.write_u8(0); // Result: 0 = full data follows
        let (high, low) = self.guid.to_guid128(REALM_ID);
        writer.write_packed_guid_128(high, low);

        // --- PlayerGuidLookupData ---
        writer.write_bit(false); // IsDeleted
        writer.write_bits(self.name.len() as u32, 6);
        for _ in 0..DECLINED_NAME_CASES {
            writer.write_bits(0, 7); // each declined name's length
        }
        writer.flush_bits();
        // Declined names themselves would follow here; all are empty.

        writer.write_packed_guid_128(0, 0); // AccountID
        writer.write_packed_guid_128(0, 0); // BnetAccountID
        writer.write_packed_guid_128(high, low); // GuidActual
        writer.write_u64(0); // GuildClubMemberID
        writer.write_u32(VIRTUAL_REALM_ADDRESS);
        writer.write_u8(self.race);
        writer.write_u8(self.gender);
        writer.write_u8(self.class);
        writer.write_u8(0); // Level — not carried by the vanilla message
        writer.write_u8(0); // Unused915
        writer.write_string_raw(self.name);

        Some(writer.finish(Opcode::SMSG_NAME_QUERY_RESPONSE))
    }
}

/// Realm used to qualify GUIDs in modern bodies built here.
///
/// Single-realm assumption, matching HermesProxy, which hardcodes the same. Thread a real value
/// through before running a second realm — it must agree with what `SmsgCharEnum` sends.
const REALM_ID: u16 = 1;

/// `region << 24 | site << 16 | realm id`, the same scheme the bnet realm list advertises.
const VIRTUAL_REALM_ADDRESS: u32 = 0x0101_0000 | REALM_ID as u32;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::bitbuf::BitReader;

    /// The creature template is what lets the client render a creature it has just been told to
    /// create. Dropping it left the client with 31 unnamed creatures and no template to draw from.
    ///
    /// The fragile part is the length run: four name slots each declare an 11-bit name and alt
    /// length *including* the terminator, and the optional Title uses 0 to mean absent. A wrong
    /// length shifts every field after it.
    #[test]
    fn creature_query_modern_declares_lengths_then_strings() {
        let msg = SmsgCreatureQueryResponse::new(299, "Defias Thug", "", 0, 7, 1706);
        let packet = msg.to_modern().expect("ported");
        let body = packet.contents();

        assert_eq!(&body[0..4], &299u32.to_le_bytes(), "creature id");
        // Allow bit, MSB-first in its own flushed byte.
        assert_eq!(body[4], 0x80, "Allow");

        // The name appears once, and the seven empty slots contribute *nothing* to the string run
        // even though each declared a length of 1. Emitting terminators for them desynchronises
        // the rest of the template.
        let text = String::from_utf8_lossy(body);
        assert_eq!(
            text.matches("Defias Thug").count(),
            1,
            "the name is written exactly once"
        );

        let name_bytes = b"Defias Thug\0";
        let name_at = body
            .windows(name_bytes.len())
            .position(|w| w == name_bytes)
            .expect("the name and its terminator are present");
        // The very next byte begins Flags[0]; seven stray terminators would push it out.
        assert_eq!(
            &body[name_at + name_bytes.len()..name_at + name_bytes.len() + 4],
            &0u32.to_le_bytes(),
            "Flags[0] must follow the name immediately"
        );

        // A display option is always present: count 1, then total probability 1.0.
        let display = 1i32.to_le_bytes();
        assert!(
            body.windows(4).any(|w| w == display),
            "one display entry must be declared"
        );
    }

    /// An unknown entry stops after the id and the cleared Allow bit; sending a body the client
    /// then tries to read as a template is worse than admitting we do not have it.
    #[test]
    fn creature_query_modern_not_found_is_just_the_id() {
        let packet = SmsgCreatureQueryResponse::not_found(1234)
            .to_modern()
            .expect("ported");

        assert_eq!(
            packet.contents(),
            &[0xD2, 0x04, 0x00, 0x00, 0x00][..],
            "entry with the not-found bit stripped, then a cleared Allow bit"
        );
    }

    /// 1.14 reads 35 data ints where 1.12 reads 24, and wraps the template in a length-counted
    /// buffer. A vanilla body here is silent corruption rather than a visible failure.
    #[test]
    fn gameobject_query_modern_length_prefixes_its_template() {
        let data = [0i32; 24];
        let msg = SmsgGameObjectQueryResponse {
            entry: 151971,
            guid: (0x0100, 0x0200_01),
            template: Some(GameObjectTemplateInfo {
                go_type: 3,
                display_id: 456,
                name: "Chest",
                icon_name: "",
                data: &data,
            }),
        };
        let packet = msg.to_modern().expect("ported");
        let body = packet.contents();

        assert_eq!(&body[0..4], &151_971u32.to_le_bytes(), "gameobject id");
        let mut reader = BitReader::new(&body[4..]);
        assert_eq!(
            reader.read_packed_guid_128(),
            Some((0x0100, 0x0200_01)),
            "the response echoes the queried object GUID"
        );
        assert!(reader.read_bit().unwrap(), "Allow");
        let stats_offset = 4 + reader.consumed();

        let declared =
            u32::from_le_bytes(body[stats_offset..stats_offset + 4].try_into().unwrap()) as usize;
        assert_eq!(
            declared,
            body.len() - stats_offset - 4,
            "the declared length must match the template that follows"
        );
    }

    /// A missing template is a zero-length buffer, not an absent one.
    #[test]
    fn gameobject_query_modern_not_found_declares_zero_length() {
        let packet = SmsgGameObjectQueryResponse {
            entry: 7,
            guid: (0, 0),
            template: None,
        }
        .to_modern()
        .expect("ported");
        let body = packet.contents();

        assert_eq!(body[6], 0x00, "Allow cleared");
        assert_eq!(&body[7..11], &0u32.to_le_bytes(), "zero-length template");
        assert_eq!(body.len(), 11, "and nothing after it");
    }

    #[test]
    fn test_smsg_name_query_response() {
        let guid = ObjectGuid::from_low(42);
        let msg = SmsgNameQueryResponse::new(guid, "TestPlayer", 1, 0, 1);
        let packet = msg.to_vanilla();

        assert_eq!(packet.opcode(), Opcode::SMSG_NAME_QUERY_RESPONSE);

        // Verify the packet contains the correct data
        let data = packet.contents();

        // First 8 bytes should be the GUID (little-endian u64)
        // GUID counter is 42, so raw() should be 0x0000000000000042 in little-endian
        assert_eq!(data[0], 0x2A); // 42 in hex = 0x2A
        assert_eq!(data[1], 0x00);
        assert_eq!(data[2], 0x00);
        assert_eq!(data[3], 0x00);
        assert_eq!(data[4], 0x00);
        assert_eq!(data[5], 0x00);
        assert_eq!(data[6], 0x00);
        assert_eq!(data[7], 0x00);
    }

    #[test]
    fn test_smsg_name_query_response_guid_writing() {
        // Test that GUID is written as u64, not packed
        let guid = ObjectGuid::from_low(1000);
        let msg = SmsgNameQueryResponse::new(guid, "Player", 1, 0, 1);
        let packet = msg.to_vanilla();

        let data = packet.contents();

        // Verify that the first 8 bytes are the full u64 GUID (little-endian)
        // 1000 = 0x3E8 in hex, so little-endian is [E8 03 00 00 00 00 00 00]
        // This is critical for things like chat to work.*
        assert_eq!(data[0], 0xE8);
        assert_eq!(data[1], 0x03);
        assert_eq!(data[2], 0x00);
        assert_eq!(data[3], 0x00);
        assert_eq!(data[4], 0x00);
        assert_eq!(data[5], 0x00);
        assert_eq!(data[6], 0x00);
        assert_eq!(data[7], 0x00);
    }

    /// Byte-for-byte against HermesProxy's `PlayerGuidLookupData::Write`
    /// (`World/Server/Packets/QueryPackets.cs:137`).
    ///
    /// The bit block is the fragile part: one `IsDeleted` bit, a 6-bit name length, then five 7-bit
    /// declined-name lengths — 42 bits, so the flush pads out to six bytes. A miscount there shifts
    /// every following field and the client reads garbage rather than failing cleanly.
    #[test]
    fn name_query_modern_body_matches_hermes_layout() {
        let msg = SmsgNameQueryResponse::new(ObjectGuid::from_low(42), "Kris", 1, 0, 1);
        let packet = msg.to_modern().expect("ported to modern");

        assert_eq!(packet.opcode(), Opcode::SMSG_NAME_QUERY_RESPONSE);
        assert_eq!(
            packet.contents(),
            &[
                0x00, // Result: data follows
                // Player: guid128 (low mask, high mask, low bytes, high bytes)
                0x01, 0xA0, 0x2A, 0x04, 0x08, //
                // 42 bits: !IsDeleted, name len 4, five zero-length declined names
                0x08, 0x00, 0x00, 0x00, 0x00, 0x00, //
                0x00, 0x00, // AccountID: empty guid128
                0x00, 0x00, // BnetAccountID: empty guid128
                0x01, 0xA0, 0x2A, 0x04, 0x08, // GuidActual
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // GuildClubMemberID
                0x01, 0x00, 0x01, 0x01, // VirtualRealmAddress
                0x01, // RaceID
                0x00, // Sex
                0x01, // ClassID
                0x00, // Level
                0x00, // Unused915
                b'K', b'r', b'i', b's',
            ][..]
        );
    }

    #[test]
    fn test_smsg_name_query_response_with_realm() {
        let guid = ObjectGuid::from_low(100);
        let mut msg = SmsgNameQueryResponse::new(guid, "TestPlayer", 1, 0, 1);
        msg.realm = "TestRealm";

        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_NAME_QUERY_RESPONSE);
    }
}

// =========================================================================
// GAMEOBJECT QUERY RESPONSE
// =========================================================================

/// `SMSG_GAMEOBJECT_QUERY_RESPONSE` — a gameobject template.
///
/// A message type rather than a hand-built packet because the two protocols disagree about the
/// shape: 1.14 prefixes a GUID and wraps the whole template in a length-counted sub-buffer, and it
/// reads 35 data ints where 1.12 reads 24. A vanilla body on a modern connection is silent
/// corruption — the client accepts it and misreads every field.
#[derive(Debug, Clone)]
pub struct SmsgGameObjectQueryResponse<'a> {
    pub entry: u32,
    /// The object GUID supplied by the query. Modern clients use it to associate this template
    /// response with the object they just created.
    pub guid: (u64, u64),
    /// `None` when the template is unknown; the client is told so via the high bit / `Allow` bit.
    pub template: Option<GameObjectTemplateInfo<'a>>,
}

/// The template payload of a gameobject query response.
#[derive(Debug, Clone)]
pub struct GameObjectTemplateInfo<'a> {
    pub go_type: u32,
    pub display_id: u32,
    pub name: &'a str,
    pub icon_name: &'a str,
    /// Vanilla reads 24 of these, 1.14 reads 35; the extra ones are sent as zero.
    pub data: &'a [i32],
}

/// Data ints a 1.14.1+ client reads. Fewer would leave it reading the size float as a data value.
const MODERN_GAMEOBJECT_DATA_FIELDS: usize = 35;
/// Data ints a 1.12 client reads.
const VANILLA_GAMEOBJECT_DATA_FIELDS: usize = 24;

impl ToWorldPacket for SmsgGameObjectQueryResponse<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GAMEOBJECT_QUERY_RESPONSE);

        let Some(template) = &self.template else {
            // High bit signals "not found" and the body stops here.
            packet.write_u32(self.entry | 0x8000_0000);
            return packet;
        };

        packet.write_u32(self.entry);
        packet.write_u32(template.go_type);
        packet.write_u32(template.display_id);
        packet.write_cstring(template.name);
        packet.write_u8(0); // name2
        packet.write_u8(0); // name3
        packet.write_u8(0); // name4
        packet.write_cstring(template.icon_name);
        for index in 0..VANILLA_GAMEOBJECT_DATA_FIELDS {
            packet.write_i32(template.data.get(index).copied().unwrap_or(0));
        }

        packet
    }

    /// `SMSG_QUERY_GAME_OBJECT_RESPONSE`, per HermesProxy `QueryPackets.cs:464`.
    ///
    /// The template goes into a length-counted sub-buffer, so a client that does not recognise the
    /// contents can still skip it. A zero length is how "not found" is expressed, alongside the
    /// `Allow` bit.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.entry);
        writer.write_packed_guid_128(self.guid.0, self.guid.1);
        writer.write_bit(self.template.is_some());
        writer.flush_bits();

        let stats = self.template.as_ref().map(|template| {
            let mut stats = BitWriter::new();
            stats.write_u32(template.go_type);
            stats.write_u32(template.display_id);
            // Four name localisation slots; only the first is populated.
            stats.write_cstring(template.name);
            for _ in 1..4 {
                stats.write_cstring("");
            }
            stats.write_cstring(template.icon_name);
            stats.write_cstring(""); // CastBarCaption
            stats.write_cstring(""); // UnkString
            for index in 0..MODERN_GAMEOBJECT_DATA_FIELDS {
                stats.write_i32(template.data.get(index).copied().unwrap_or(0));
            }
            stats.write_f32(1.0); // Size
            stats.write_u8(0); // QuestItems.Count
            stats.write_u32(0); // ContentTuningId
            stats.into_bytes()
        });

        let stats = stats.unwrap_or_default();
        writer.write_u32(stats.len() as u32);
        if !stats.is_empty() {
            writer.write_bytes(&stats);
        }

        Some(writer.finish(Opcode::SMSG_GAMEOBJECT_QUERY_RESPONSE))
    }
}
