//! Login message structs
//!
//! This module contains type-safe message structures for login-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgAuthChallenge`] - Authentication challenge with server seed
//! - [`SmsgAuthResponse`] - Authentication result
//! - [`SmsgCharEnum`] - Character list enumeration
//! - [`SmsgLoginVerifyWorld`] - Initial world verification after login
//! - [`SmsgAccountDataMd5`] - Account data MD5 hashes
//! - [`SmsgBindPointUpdate`] - Hearthstone bind location
//! - [`SmsgSetRestStart`] - Rest state timer
//! - [`SmsgInitialSpellsRef`] - Initial spell list (reference-based version)
//! - [`SmsgActionButtons`] - Action bar configuration
//! - [`SmsgInitializeFactionsEmpty`] - Empty faction/reputation data (convenience)

use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::position::Position;
use crate::protocol::{Opcode, WorldPacket};

/// Authentication error codes for SMSG_AUTH_RESPONSE
pub enum AuthErrorCode {
    Ok = 0x0C,              // AUTH_OK
    Failed = 0x0D,          // AUTH_FAILED
    UnknownAccount = 0x15,  // AUTH_UNKNOWN_ACCOUNT
    AlreadyOnline = 0x06,   // AUTH_ALREADY_ONLINE
    NoTime = 0x17,          // AUTH_NO_TIME
    DbBusy = 0x18,          // AUTH_DB_BUSY
    VersionInvalid = 0x1A,  // AUTH_VERSION_INVALID
    VersionMismatch = 0x1B, // AUTH_VERSION_MISMATCH
    AccountBanned = 0x1C,   // AUTH_BANNED
}

/// SMSG_AUTH_CHALLENGE - Authentication challenge
///
/// Sent immediately after connection to initiate SRP6 session handshake.
/// Contains server seed for digest calculation.
#[derive(Debug, Clone)]
pub struct SmsgAuthChallenge {
    pub seed: u32,
}

impl ToWorldPacket for SmsgAuthChallenge {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AUTH_CHALLENGE);
        packet.write_u32(self.seed);
        // Add padding to match expected packet size (vanilla expects specific size)
        for _ in 0..4 {
            packet.write_u8(0);
        }
        packet
    }
}

/// SMSG_AUTH_RESPONSE - Authentication response
///
/// Sent in response to CMSG_AUTH_SESSION to indicate success or failure.
#[derive(Debug, Clone)]
pub struct SmsgAuthResponse {
    pub error_code: u8,
}

impl ToWorldPacket for SmsgAuthResponse {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AUTH_RESPONSE);
        // For vanilla: error_code is 4 bytes (u32), but only low byte is used
        packet.write_u32(self.error_code as u32);
        // Success case includes additional billing fields (unused in vanilla)
        if self.error_code == AuthErrorCode::Ok as u8 {
            packet.write_u32(0); // billing_time (4 bytes)
            packet.write_u8(0); // billing_flags (1 byte)
            packet.write_u32(0); // billing_rested (4 bytes)
        }
        packet
    }
}

/// Equipment slot data for character enumeration
#[derive(Debug, Clone, Copy, Default)]
pub struct EquipmentSlot {
    /// Item display ID
    pub display_id: u32,
    /// Inventory type
    pub inventory_type: u8,
}

/// Character data for enumeration
#[derive(Debug, Clone)]
pub struct CharacterEnumEntry {
    pub guid: u32,
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub skin: u8,
    pub face: u8,
    pub hair_style: u8,
    pub hair_color: u8,
    pub facial_hair: u8,
    pub level: u8,
    pub zone: u32,
    pub map: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub guild_id: u32,
    pub character_flags: u32,
    pub first_login: bool,
    /// Pet info: (display_id, level, family)
    pub pet_info: Option<(u32, u32, u32)>,
    /// Equipment slots (19 slots)
    pub equipment: [EquipmentSlot; 19],
}

/// SMSG_CHAR_ENUM - Character list enumeration
///
/// Sent in response to CMSG_CHAR_ENUM to provide the list of characters
/// for the authenticated account.
#[derive(Debug, Clone)]
pub struct SmsgCharEnum<'a> {
    pub characters: &'a [CharacterEnumEntry],
    pub realm_id: u16,
}

impl ToWorldPacket for SmsgCharEnum<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CHAR_ENUM);

        // Character count
        packet.write_u8(self.characters.len() as u8);

        for character in self.characters {
            // GUID (u64, little endian)
            packet.write_u64(character.guid as u64);

            // Name (null-terminated)
            packet.write_cstring(&character.name);

            // Race, class, gender
            packet.write_u8(character.race);
            packet.write_u8(character.class);
            packet.write_u8(character.gender);

            // Appearance: skin, face, hair style, hair color, facial hair
            packet.write_u8(character.skin);
            packet.write_u8(character.face);
            packet.write_u8(character.hair_style);
            packet.write_u8(character.hair_color);
            packet.write_u8(character.facial_hair);

            // Level
            packet.write_u8(character.level);

            // Zone, map
            packet.write_u32(character.zone);
            packet.write_u32(character.map);

            // Position
            packet.write_f32(character.position_x);
            packet.write_f32(character.position_y);
            packet.write_f32(character.position_z);

            // Guild ID
            packet.write_u32(character.guild_id);

            // Character flags
            packet.write_u32(character.character_flags);

            // First login flag
            packet.write_u8(if character.first_login { 1 } else { 0 });

            // Pet info (display_id, level, family)
            if let Some((display_id, level, family)) = character.pet_info {
                packet.write_u32(display_id);
                packet.write_u32(level);
                packet.write_u32(family);
            } else {
                packet.write_u32(0);
                packet.write_u32(0);
                packet.write_u32(0);
            }

            // Equipment (19 slots) - display_id (u32) + inventory_type (u8) per slot
            for slot in &character.equipment {
                packet.write_u32(slot.display_id);
                packet.write_u8(slot.inventory_type);
            }

            // First bag slot (20th equipment entry)
            packet.write_u32(0); // display_id (0 = no bag)
            packet.write_u8(0); // inventory_type
        }

        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        const CLASSIC_RACES: [i32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        const VISUAL_ITEM_COUNT: usize = 23;
        let mut writer = BitWriter::new();
        writer.write_bit(true); // Success
        writer.write_bit(false); // IsDeletedCharacters
        writer.write_bit(false); // IsNewPlayerRestrictionSkipped
        writer.write_bit(false); // IsNewPlayerRestricted
        writer.write_bit(true); // IsNewPlayer
        writer.write_bit(false); // DisabledClassesMask.has_value
        writer.write_bit(false); // IsAlliedRacesCreationAllowed

        // The highest level among the listed characters, floored at 1. The client uses this to
        // decide what the character list is allowed to offer, so a hardcoded 1 mislabels every
        // account that has actually played.
        let max_character_level = self
            .characters
            .iter()
            .map(|character| character.level as i32)
            .max()
            .unwrap_or(1)
            .max(1);

        writer.write_i32(self.characters.len() as i32);
        writer.write_i32(max_character_level);
        writer.write_i32(CLASSIC_RACES.len() as i32); // RaceUnlockData.Count
        writer.write_i32(0); // UnlockedConditionalAppearances.Count
        for (position, character) in self.characters.iter().enumerate() {
            let guid =
                crate::protocol::ObjectGuid::new_player(character.guid).to_guid128(self.realm_id);
            writer.write_packed_guid_128(guid.0, guid.1);
            writer.write_u64(0); // GuildClubMemberID
            writer.write_u8(position as u8);
            writer.write_u8(character.race);
            writer.write_u8(character.class);
            writer.write_u8(character.gender);
            // The legacy schema has no ChrCustomization IDs. Do not invent DB2 choice IDs: an
            // empty list leaves the client with a valid, if default-looking, selection entry.
            writer.write_i32(0);
            writer.write_u8(character.level);
            writer.write_u32(character.zone);
            writer.write_u32(character.map);
            writer.write_f32(character.position_x);
            writer.write_f32(character.position_y);
            writer.write_f32(character.position_z);
            writer.write_packed_guid_128(0, 0); // GuildGuid
            writer.write_u32(character.character_flags); // Flags
                                                         // Flags2/Flags3 sit immediately after Flags, *before* the pet triple. These two
                                                         // constants are placeholder values for Classic; they are opaque to us.
            writer.write_u32(402_685_956); // Flags2
            writer.write_u32(855_688_192); // Flags3
            writer.write_u32(character.pet_info.map_or(0, |pet| pet.0));
            writer.write_u32(character.pet_info.map_or(0, |pet| pet.1));
            writer.write_u32(character.pet_info.map_or(0, |pet| pet.2));
            writer.write_u32(0); // ProfessionIds[0]
            writer.write_u32(0); // ProfessionIds[1]
            for index in 0..VISUAL_ITEM_COUNT {
                let slot = character.equipment.get(index).copied().unwrap_or_default();
                writer.write_u32(slot.display_id);
                writer.write_u32(0);
                writer.write_u32(0);
                writer.write_u8(slot.inventory_type);
                writer.write_u8(0);
            }
            writer.write_u64(chrono::Utc::now().timestamp() as u64);
            writer.write_u16(0);
            writer.write_u32(55); // Unknown703 placeholder
            writer.write_u32(11_400); // LastLoginVersion, Classic Era
            writer.write_u32(0);
            writer.write_i32(0);
            writer.write_i32(0);
            writer.write_u32(0);
            writer.write_bits(character.name.len() as u32, 6);
            writer.write_bit(character.first_login);
            // BoostInProgress must be false and ExpansionChosen true, and they are five bits
            // apart — swapping them marks every character as mid-boost, which the client renders
            // as unselectable.
            writer.write_bit(false); // BoostInProgress
            writer.write_bits(0, 5); // unkWod61x
            writer.write_bit(false);
            writer.write_bit(true); // ExpansionChosen
            writer.flush_bits();
            writer.write_string_raw(&character.name);
        }
        for race_id in CLASSIC_RACES {
            writer.write_i32(race_id);
            writer.write_bit(true); // HasExpansion
            writer.write_bit(false); // HasAchievement
            writer.write_bit(false); // HasHeritageArmor
            writer.flush_bits();
        }
        Some(writer.finish(Opcode::SMSG_CHAR_ENUM))
    }
}

#[cfg(test)]
mod modern_tests {
    use super::*;

    #[test]
    fn char_enum_modern_empty_list_matches_enum_characters_result() {
        let packet = SmsgCharEnum {
            characters: &[],
            realm_id: 1,
        }
        .to_modern()
        .expect("empty character list has a modern encoding");
        assert_eq!(packet.opcode(), Opcode::SMSG_CHAR_ENUM);
        assert_eq!(
            packet.contents(),
            &[
                // Header bits, MSB-first: Success | IsNewPlayer. IsNewPlayer is set on
                // every enumeration, so this byte is 0x88, not 0x80.
                0x88, // Characters.Count = 0
                0, 0, 0, 0, // MaxCharacterLevel = 1 (floor, no characters to raise it)
                1, 0, 0, 0, // RaceUnlockData.Count = 8
                8, 0, 0, 0, // UnlockedConditionalAppearances.Count = 0
                0, 0, 0, 0, // Eight RaceUnlock entries: i32 race id, then HasExpansion set.
                1, 0, 0, 0, 0x80, 2, 0, 0, 0, 0x80, 3, 0, 0, 0, 0x80, 4, 0, 0, 0, 0x80, 5, 0, 0, 0,
                0x80, 6, 0, 0, 0, 0x80, 7, 0, 0, 0, 0x80, 8, 0, 0, 0, 0x80,
            ]
        );
    }

    #[test]
    fn bind_point_update_modern_matches_vanilla_layout() {
        let msg = SmsgBindPointUpdate {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            map_id: 0,
            zone_id: 12,
        };
        let modern = msg.to_modern().expect("ported");
        assert_eq!(modern.opcode(), Opcode::SMSG_BINDPOINTUPDATE);
        // Vector3, map, area — identical bytes to vanilla, only the wire opcode differs.
        assert_eq!(modern.contents(), msg.to_vanilla().contents());
    }

    #[test]
    fn tutorial_flags_modern_is_eight_u32s() {
        let msg = SmsgTutorialFlags::default();
        let modern = msg.to_modern().expect("ported");
        assert_eq!(modern.opcode(), Opcode::SMSG_TUTORIAL_FLAGS);
        assert_eq!(modern.contents().len(), 32, "Tutorials::Max is 8 u32s");
        assert_eq!(modern.contents(), &[0xFF; 32]);
    }

    #[test]
    fn login_set_time_speed_modern_adds_server_time_and_holiday_offsets() {
        let msg = SmsgLoginSetTimeSpeed {
            game_time: 0x1122_3344,
            game_speed: 0.016_666_67,
        };
        let modern = msg.to_modern().expect("ported");
        assert_eq!(modern.opcode(), Opcode::SMSG_LOGIN_SETTIMESPEED);

        let body = modern.contents();
        assert_eq!(body.len(), 20, "u32 + u32 + f32 + i32 + i32");
        // ServerTime and GameTime carry the same packed value.
        assert_eq!(&body[0..4], &0x1122_3344u32.to_le_bytes());
        assert_eq!(&body[4..8], &0x1122_3344u32.to_le_bytes());
        assert_eq!(&body[8..12], &0.016_666_67f32.to_le_bytes());
        assert_eq!(&body[12..20], &[0u8; 8], "both holiday offsets are zero");
    }

    #[test]
    fn set_rest_start_has_no_modern_encoding() {
        // SMSG_SET_REST_START does not exist in the 1.14 opcode table at all, so the defaulted
        // `to_modern` correctly declines rather than inventing a body.
        assert!(SmsgSetRestStart { time: 0 }.to_modern().is_none());
        assert!(!Opcode::SMSG_SET_REST_START.has_modern());
    }

    #[test]
    fn login_verify_world_modern_appends_reason() {
        let packet = SmsgLoginVerifyWorld {
            map_id: 1,
            position: Position::new(2.0, 3.0, 4.0, 5.0),
        }
        .to_modern()
        .unwrap();
        assert_eq!(packet.contents().len(), 24);
        assert_eq!(&packet.contents()[20..], &[0, 0, 0, 0]);
    }
}

/// SMSG_LOGIN_VERIFY_WORLD - Initial world verification after login
///
/// Sent immediately after character login to tell the client where they are.
/// This is the first packet the client expects after selecting a character.
#[derive(Debug, Clone)]
pub struct SmsgLoginVerifyWorld {
    pub map_id: u32,
    pub position: Position,
}

impl ToWorldPacket for SmsgLoginVerifyWorld {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOGIN_VERIFY_WORLD);
        packet.write_u32(self.map_id);
        packet.write_f32(self.position.x);
        packet.write_f32(self.position.y);
        packet.write_f32(self.position.z);
        packet.write_f32(self.position.o);
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut packet = self.to_vanilla();
        packet.write_u32(0); // Reason
        Some(packet)
    }
}

/// SMSG_ACCOUNT_DATA_MD5 - Account data MD5 hashes
///
/// Sent after login to provide MD5 hashes for all 8 account data types.
/// The client uses these to determine if it needs to request updated data.
#[derive(Debug, Clone, Default)]
pub struct SmsgAccountDataMd5 {
    /// MD5 hashes for each of the 8 account data types
    /// Each hash is 16 bytes. Empty/default data uses all zeros.
    pub hashes: [[u8; 16]; 8],
}

impl ToWorldPacket for SmsgAccountDataMd5 {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ACCOUNT_DATA_MD5);
        for hash in &self.hashes {
            for byte in hash {
                packet.write_u8(*byte);
            }
        }
        packet
    }
}

/// SMSG_BINDPOINTUPDATE - Hearthstone bind location
///
/// Sent to inform the client of the player's hearthstone bind point.
#[derive(Debug, Clone)]
pub struct SmsgBindPointUpdate {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub map_id: u32,
    pub zone_id: u32,
}

impl ToWorldPacket for SmsgBindPointUpdate {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_BINDPOINTUPDATE);
        packet.write_f32(self.x);
        packet.write_f32(self.y);
        packet.write_f32(self.z);
        packet.write_u32(self.map_id);
        packet.write_u32(self.zone_id);
        packet
    }

    /// Field order is unchanged from vanilla (`Vector3`, map, area) — only the opcode differs.
    /// Written out rather than delegating so it stays honest if either layout moves.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_f32(self.x);
        writer.write_f32(self.y);
        writer.write_f32(self.z);
        writer.write_u32(self.map_id);
        writer.write_u32(self.zone_id);
        Some(writer.finish(Opcode::SMSG_BINDPOINTUPDATE))
    }
}

/// SMSG_SET_REST_START - Rest state timer
///
/// Sent to set when the player started resting (for rest XP calculation).
#[derive(Debug, Clone)]
pub struct SmsgSetRestStart {
    pub time: u32,
}

impl ToWorldPacket for SmsgSetRestStart {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SET_REST_START);
        packet.write_u32(self.time);
        packet
    }
}

/// SMSG_INITIAL_SPELLS - Initial spell list (reference-based version)
///
/// Sent during login to provide the player's known spells and cooldowns.
/// Note: This is the reference-based version. For the owned version, use
/// `SmsgInitialSpells` from the `spells` module.
#[derive(Debug, Clone)]
pub struct SmsgInitialSpellsRef<'a> {
    /// List of known spell IDs
    pub spells: &'a [u32],
    /// List of spell cooldowns (spell_id, category_id, cooldown_ms, category_cooldown_ms)
    pub cooldowns: &'a [(u32, u16, u32, u32)],
}

impl ToWorldPacket for SmsgInitialSpellsRef<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_INITIAL_SPELLS);
        packet.write_u8(0); // Talent spec count (0 for vanilla)
        packet.write_u16(self.spells.len() as u16);

        for &spell_id in self.spells {
            packet.write_u16(spell_id as u16);
            packet.write_u16(0); // Slot (unused in vanilla)
        }

        packet.write_u16(self.cooldowns.len() as u16);
        for &(spell_id, category, cooldown_ms, category_cooldown_ms) in self.cooldowns {
            packet.write_u16(spell_id as u16);
            packet.write_u16(0); // Item ID (for item spells)
            packet.write_u16(category);
            packet.write_u32(cooldown_ms);
            packet.write_u32(category_cooldown_ms);
        }

        packet
    }

    /// A different message in all but name: spell ids widen to `u32`, a favourites list appears,
    /// and **cooldowns are not carried here at all** — the modern client learns them from
    /// `SMSG_SPELL_COOLDOWN` instead. Anything in `self.cooldowns` is silently not sent.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(write_modern_known_spells(self.spells))
    }
}

/// Shared body for the modern `SMSG_SEND_KNOWN_SPELLS`, which all three initial-spell messages
/// serialize to.
///
/// `InitialLogin` is always true: every message that reaches here is part of the login sequence.
/// The favourites list is tradeskill recipes, which are not modelled, so it is always empty.
pub(crate) fn write_modern_known_spells(spells: &[u32]) -> WorldPacket {
    let mut writer = BitWriter::new();
    writer.write_bit(true); // InitialLogin
    writer.write_i32(spells.len() as i32);
    writer.write_i32(0); // FavoriteSpells.Count
    for &spell_id in spells {
        writer.write_u32(spell_id);
    }
    writer.finish(Opcode::SMSG_INITIAL_SPELLS)
}

/// Empty initial spells (convenience)
#[derive(Debug, Clone, Default)]
pub struct SmsgInitialSpellsEmpty;

impl ToWorldPacket for SmsgInitialSpellsEmpty {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_INITIAL_SPELLS);
        packet.write_u8(0); // Talent spec count
        packet.write_u16(0); // Spell count
        packet.write_u16(0); // Cooldown count
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        Some(write_modern_known_spells(&[]))
    }
}

/// Action button data
///
/// Packed format: action (bits 0-23) | type (bits 24-31)
#[derive(Debug, Clone, Copy, Default)]
pub struct ActionButton {
    /// Action ID (spell ID, item ID, macro ID, etc.) - uses lower 24 bits
    pub action: u32,
    /// Type (0 = spell, 64 = macro, 128 = item)
    pub action_type: u8,
}

impl ActionButton {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn spell(spell_id: u32) -> Self {
        Self {
            action: spell_id,
            action_type: 0,
        }
    }

    pub fn to_u32(&self) -> u32 {
        (self.action & 0xFFFFFF) | ((self.action_type as u32) << 24)
    }
}

/// SMSG_ACTION_BUTTONS - Action bar configuration
///
/// Sent during login to provide the player's action bar setup.
/// Contains 120 buttons (10 bars * 12 buttons each).
#[derive(Debug, Clone)]
pub struct SmsgActionButtons<'a> {
    pub buttons: &'a [ActionButton; 120],
}

/// No `to_modern`, and there never will be: **1.14 has no action-buttons packet.**
///
/// The bar is part of the `ActivePlayer` create block instead — 132 × `i32` behind a
/// `HasActionButtons` bit in its tail. The packed `action | type << 24` word is identical to
/// vanilla's, so the values pass through untouched; only the slot count differs, 120 to 132.
///
/// So the modern bar is populated by `CreateObjectBlock::with_action_buttons` at the self-create,
/// and the drop logged here is correct rather than a gap.
impl ToWorldPacket for SmsgActionButtons<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ACTION_BUTTONS);
        for button in self.buttons {
            packet.write_u32(button.to_u32());
        }
        packet
    }
}

/// Empty action buttons (convenience)
#[derive(Debug, Clone, Default)]
pub struct SmsgActionButtonsEmpty;

impl ToWorldPacket for SmsgActionButtonsEmpty {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ACTION_BUTTONS);
        for _ in 0..120 {
            packet.write_u32(0);
        }
        packet
    }
}

/// Empty factions (convenience for login)
#[derive(Debug, Clone, Default)]
pub struct SmsgInitializeFactionsEmpty;

impl ToWorldPacket for SmsgInitializeFactionsEmpty {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_INITIALIZE_FACTIONS);
        packet.write_u32(0);
        packet.write_u8(64);
        for _ in 0..64 {
            packet.write_u8(0);
            packet.write_u32(0);
        }
        packet
    }

    /// Defers to the populated message with no factions, so the two cannot drift.
    fn to_modern(&self) -> Option<WorldPacket> {
        crate::messages::reputation::SmsgInitializeFactions {
            factions: Default::default(),
        }
        .to_modern()
    }
}

/// SMSG_TUTORIAL_FLAGS - Tutorial completion flags
///
/// Sent during login to provide the player's tutorial progress.
/// 8 u32 values (32 bytes total), each bit represents one tutorial.
/// Set to 0xFFFFFFFF to disable all tutorials.
#[derive(Debug, Clone)]
pub struct SmsgTutorialFlags {
    pub flags: [u32; 8],
}

impl Default for SmsgTutorialFlags {
    fn default() -> Self {
        // All bits set = all tutorials completed/disabled
        Self {
            flags: [0xFFFFFFFF; 8],
        }
    }
}

impl ToWorldPacket for SmsgTutorialFlags {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_TUTORIAL_FLAGS);
        for flag in &self.flags {
            packet.write_u32(*flag);
        }
        packet
    }

    /// The modern client also expects eight `u32`s (`Tutorials::Max` is 8 in Classic), so the body
    /// is the same size and shape as vanilla's.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        for flag in &self.flags {
            writer.write_u32(*flag);
        }
        Some(writer.finish(Opcode::SMSG_TUTORIAL_FLAGS))
    }
}

/// SMSG_LOGIN_SETTIMESPEED - Game time and speed
///
/// Sent during login to synchronize game time with the client.
/// Critical for client stability.
#[derive(Debug, Clone)]
pub struct SmsgLoginSetTimeSpeed {
    /// Game time as packed bitfield (minutes/hours/weekday/day/month/year)
    pub game_time: u32,
    /// Game speed (default 0.01666667 = 1/60)
    pub game_speed: f32,
}

impl Default for SmsgLoginSetTimeSpeed {
    fn default() -> Self {
        Self {
            game_time: pack_game_time(),
            game_speed: 0.01666667, // Normal game speed
        }
    }
}

/// Pack current UTC time into bitfield for SMSG_LOGIN_SETTIMESPEED.
///
/// Format:
/// - bits 0-5: minutes (0-59)
/// - bits 6-10: hours (0-23)
/// - bits 11-13: weekday (0=Sun..6=Sat)
/// - bits 14-19: day of month (0-based)
/// - bits 20-23: month (0-based, 0-11)
/// - bits 24-28: year (since 2000)
fn pack_game_time() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let time_of_day = (secs % 86400) as u32;
    let minutes = (time_of_day / 60) % 60;
    let hours = time_of_day / 3600;

    let days_since_epoch = (secs / 86400) as i64;
    // Jan 1 1970 was Thursday (4)
    let weekday = ((days_since_epoch + 4) % 7) as u32;

    let (year, month, day) = civil_from_days(days_since_epoch);

    let mut packed: u32 = 0;
    packed |= minutes & 0x3F;
    packed |= (hours & 0x1F) << 6;
    packed |= (weekday & 0x07) << 11;
    packed |= ((day as u32) & 0x3F) << 14;
    packed |= ((month as u32) & 0x0F) << 20;
    packed |= (((year - 2000) as u32) & 0x1F) << 24;
    packed
}

/// Convert days since Unix epoch to (year, month-0based, day-0based).
/// Uses Howard Hinnant's civil_from_days algorithm.
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64 + era * 400) as i32;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5; // 0-based day
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // 1-based month
    let y = if m <= 2 { y + 1 } else { y };
    (y, m - 1, d) // month 0-based, day 0-based
}

impl ToWorldPacket for SmsgLoginSetTimeSpeed {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOGIN_SETTIMESPEED);
        packet.write_u32(self.game_time);
        packet.write_f32(self.game_speed);
        packet
    }

    /// The modern body gained a separate server time and two holiday offsets. The
    /// single legacy time is mirrored into both fields and the offsets are left at zero — the legacy
    /// protocol has nothing to fill them from, and the packed time format itself is unchanged.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.game_time); // ServerTime
        writer.write_u32(self.game_time); // GameTime
        writer.write_f32(self.game_speed); // NewSpeed
        writer.write_i32(0); // ServerTimeHolidayOffset
        writer.write_i32(0); // GameTimeHolidayOffset
        Some(writer.finish(Opcode::SMSG_LOGIN_SETTIMESPEED))
    }
}

/// SMSG_INIT_WORLD_STATES - Zone world state data
///
/// Sent after SMSG_UPDATE_OBJECT to provide zone-specific world states.
/// Required for minimap and zone functionality.
#[derive(Debug, Clone)]
pub struct SmsgInitWorldStates {
    pub map_id: u32,
    pub zone_id: u32,
    /// World state entries: (state_id, value)
    pub states: Vec<(u32, u32)>,
}

impl SmsgInitWorldStates {
    pub fn new(map_id: u32, zone_id: u32) -> Self {
        Self {
            map_id,
            zone_id,
            states: Vec::new(),
        }
    }

    pub fn with_state(mut self, state_id: u32, value: u32) -> Self {
        self.states.push((state_id, value));
        self
    }
}

impl SmsgInitWorldStates {
    /// The modern body adds an area id between the zone and the state list, and widens the count
    /// from `u16` to `i32`. Vanilla carries no separate area, so the zone is reused — the client
    /// treats them the same for world-state scoping.
    fn write_modern(&self) -> WorldPacket {
        let mut writer = BitWriter::new();
        writer.write_u32(self.map_id);
        writer.write_u32(self.zone_id);
        writer.write_u32(self.zone_id); // AreaID
        writer.write_i32(self.states.len() as i32);
        for (state_id, value) in &self.states {
            writer.write_u32(*state_id);
            writer.write_i32(*value as i32);
        }
        writer.finish(Opcode::SMSG_INIT_WORLD_STATES)
    }
}

impl ToWorldPacket for SmsgInitWorldStates {
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.write_modern())
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_INIT_WORLD_STATES);
        packet.write_u32(self.map_id);
        packet.write_u32(self.zone_id);
        packet.write_u16(self.states.len() as u16);
        for (state_id, value) in &self.states {
            packet.write_u32(*state_id);
            packet.write_u32(*value);
        }
        // Terminator
        packet.write_u32(0);
        packet.write_u32(0);
        packet
    }
}

/// SMSG_TRIGGER_CINEMATIC - Trigger a cinematic sequence on the client
///
/// Sent to start a cinematic. The cinematic plays entirely client-side.
#[derive(Debug, Clone)]
pub struct SmsgTriggerCinematic {
    /// Cinematic sequence ID (from CinematicSequences.dbc)
    pub cinematic_sequence_id: u32,
}

impl ToWorldPacket for SmsgTriggerCinematic {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_TRIGGER_CINEMATIC);
        packet.write_u32(self.cinematic_sequence_id);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_verify_world() {
        let msg = SmsgLoginVerifyWorld {
            map_id: 0,
            position: Position::new(100.0, 200.0, 300.0, 1.5),
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_LOGIN_VERIFY_WORLD);
    }

    #[test]
    fn test_account_data_md5() {
        let msg = SmsgAccountDataMd5::default();
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_ACCOUNT_DATA_MD5);
    }

    #[test]
    fn test_bind_point_update() {
        let msg = SmsgBindPointUpdate {
            x: 100.0,
            y: 200.0,
            z: 300.0,
            map_id: 0,
            zone_id: 12,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_BINDPOINTUPDATE);
    }

    #[test]
    fn test_initial_spells_empty() {
        let msg = SmsgInitialSpellsEmpty;
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_INITIAL_SPELLS);
    }

    #[test]
    fn test_action_buttons_empty() {
        let msg = SmsgActionButtonsEmpty;
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_ACTION_BUTTONS);
    }

    #[test]
    fn test_initialize_factions_empty() {
        let msg = SmsgInitializeFactionsEmpty;
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_INITIALIZE_FACTIONS);
    }
}

// =========================================================================
// MODERN ENTER-WORLD PACKETS
// =========================================================================
//
// These four have no 1.12 counterpart at all: the vanilla client finishes loading off
// `SMSG_LOGIN_VERIFY_WORLD` alone, while 1.14 waits for them before it will hand control to the
// player. All four are sent immediately after the verify, which is why they are grouped here
// rather than scattered by subject.
//
// Because they are modern-only they implement `to_vanilla` as an empty body that is never sent —
// the trait requires it, and the send path only reaches `to_modern` for a 1.14 session.

/// `SMSG_WORLD_SERVER_INFO` — difficulty and realm-wide restrictions.
///
/// Everything except the difficulty is optional and gated behind presence bits; a Classic Era realm
/// sends none of it.
#[derive(Debug, Clone, Default)]
pub struct SmsgWorldServerInfo {
    pub difficulty_id: u32,
    /// Present only inside instances, where 1.14 wants the raid/party size.
    pub instance_group_size: Option<u32>,
}

impl ToWorldPacket for SmsgWorldServerInfo {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_WORLD_SERVER_INFO)
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.difficulty_id);
        writer.write_u8(0); // IsTournamentRealm
        writer.write_bit(false); // XRealmPvpAlert
        writer.write_bit(false); // RestrictedAccountMaxLevel
        writer.write_bit(false); // RestrictedAccountMaxMoney
        writer.write_bit(self.instance_group_size.is_some());
        writer.flush_bits();

        if let Some(size) = self.instance_group_size {
            writer.write_u32(size);
        }
        Some(writer.finish(Opcode::SMSG_WORLD_SERVER_INFO))
    }
}

/// `SMSG_SET_ALL_TASK_PROGRESS` — always empty for Classic Era, which has no tasks.
#[derive(Debug, Clone, Default)]
pub struct SmsgSetAllTaskProgress;

impl ToWorldPacket for SmsgSetAllTaskProgress {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_SET_ALL_TASK_PROGRESS)
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut packet = WorldPacket::new(Opcode::SMSG_SET_ALL_TASK_PROGRESS);
        packet.write_u32(0); // Tasks.Count
        Some(packet)
    }
}

/// `SMSG_INITIAL_SETUP` — the expansion the realm serves.
#[derive(Debug, Clone)]
pub struct SmsgInitialSetup {
    pub expansion_level: u8,
    pub expansion_tier: u8,
}

impl Default for SmsgInitialSetup {
    /// Classic Era: expansion 0, tier 0.
    ///
    /// Computed as `LegacyVersion.ExpansionVersion - 1`, which for a 1.12 realm is 0.
    fn default() -> Self {
        Self {
            expansion_level: 0,
            expansion_tier: 0,
        }
    }
}

impl ToWorldPacket for SmsgInitialSetup {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_INITIAL_SETUP)
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut packet = WorldPacket::new(Opcode::SMSG_INITIAL_SETUP);
        packet.write_u8(self.expansion_level);
        packet.write_u8(self.expansion_tier);
        Some(packet)
    }
}

/// `SMSG_LOAD_CUF_PROFILES` — saved raid-frame layouts, an opaque blob.
///
/// We persist none, so this is the empty profile list the client accepts as "no saved layouts".
#[derive(Debug, Clone, Default)]
pub struct SmsgLoadCufProfiles;

impl ToWorldPacket for SmsgLoadCufProfiles {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_LOAD_CUF_PROFILES)
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOAD_CUF_PROFILES);
        // A zero-length profile list; the client reads a count first.
        packet.write_u32(0);
        Some(packet)
    }
}

/// `SMSG_TIME_SYNC_REQUEST` — asks the client for its tick count so the server can track drift.
///
/// Modern-only: 1.12 has no time sync at all, which is why `to_vanilla` produces a packet with no
/// vanilla opcode and the send layer never delivers it to a 1.12 session.
///
/// The first one is sent as the *first* packet of `SendInitialPacketsBeforeAddToMap`,
/// then again after 5 s and every 10 s thereafter. The body is a single sequence index,
/// per the 1.14 wire format; the client echoes it back in `CMSG_TIME_SYNC_RESPONSE` alongside its own tick count.
#[derive(Debug, Clone, Default)]
pub struct SmsgTimeSyncRequest {
    /// Increments per request, so a response can be matched to the request it answers.
    pub sequence_index: u32,
}

impl ToWorldPacket for SmsgTimeSyncRequest {
    fn to_vanilla(&self) -> WorldPacket {
        // Unreachable in practice -- `Opcode::SMSG_TIME_SYNC_REQUEST` has no vanilla wire value, so
        // the send path drops it for a 1.12 session before it gets here.
        WorldPacket::new(Opcode::SMSG_TIME_SYNC_REQUEST)
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut packet = WorldPacket::new(Opcode::SMSG_TIME_SYNC_REQUEST);
        packet.write_u32(self.sequence_index);
        Some(packet)
    }
}
