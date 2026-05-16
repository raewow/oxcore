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

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::position::Position;
use crate::shared::protocol::{Opcode, WorldPacket};

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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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
}

impl ToWorldPacket for SmsgCharEnum<'_> {
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOGIN_VERIFY_WORLD);
        packet.write_u32(self.map_id);
        packet.write_f32(self.position.x);
        packet.write_f32(self.position.y);
        packet.write_f32(self.position.z);
        packet.write_f32(self.position.o);
        packet
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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_BINDPOINTUPDATE);
        packet.write_f32(self.x);
        packet.write_f32(self.y);
        packet.write_f32(self.z);
        packet.write_u32(self.map_id);
        packet.write_u32(self.zone_id);
        packet
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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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
}

/// Empty initial spells (convenience)
#[derive(Debug, Clone, Default)]
pub struct SmsgInitialSpellsEmpty;

impl ToWorldPacket for SmsgInitialSpellsEmpty {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_INITIAL_SPELLS);
        packet.write_u8(0); // Talent spec count
        packet.write_u16(0); // Spell count
        packet.write_u16(0); // Cooldown count
        packet
    }
}

/// Action button data
///
/// Packed format: action (bits 0-23) | type (bits 24-31)
/// Matches MaNGOS: `action | (type << 24)`
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

impl ToWorldPacket for SmsgActionButtons<'_> {
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_INITIALIZE_FACTIONS);
        packet.write_u32(0);
        packet.write_u8(64);
        for _ in 0..64 {
            packet.write_u8(0);
            packet.write_u32(0);
        }
        packet
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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_TUTORIAL_FLAGS);
        for flag in &self.flags {
            packet.write_u32(*flag);
        }
        packet
    }
}

/// SMSG_LOGIN_SETTIMESPEED - Game time and speed
///
/// Sent during login to synchronize game time with the client.
/// Critical for client stability.
#[derive(Debug, Clone)]
pub struct SmsgLoginSetTimeSpeed {
    /// Game time as MaNGOS-style packed bitfield (minutes/hours/weekday/day/month/year)
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

/// Pack current UTC time into MaNGOS-style bitfield for SMSG_LOGIN_SETTIMESPEED.
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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOGIN_SETTIMESPEED);
        packet.write_u32(self.game_time);
        packet.write_f32(self.game_speed);
        packet
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

impl ToWorldPacket for SmsgInitWorldStates {
    fn to_world_packet(&self) -> WorldPacket {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_verify_world() {
        let msg = SmsgLoginVerifyWorld {
            map_id: 0,
            position: Position::new(100.0, 200.0, 300.0, 1.5),
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_LOGIN_VERIFY_WORLD);
    }

    #[test]
    fn test_account_data_md5() {
        let msg = SmsgAccountDataMd5::default();
        let packet = msg.to_world_packet();
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
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_BINDPOINTUPDATE);
    }

    #[test]
    fn test_initial_spells_empty() {
        let msg = SmsgInitialSpellsEmpty;
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_INITIAL_SPELLS);
    }

    #[test]
    fn test_action_buttons_empty() {
        let msg = SmsgActionButtonsEmpty;
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_ACTION_BUTTONS);
    }

    #[test]
    fn test_initialize_factions_empty() {
        let msg = SmsgInitializeFactionsEmpty;
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_INITIALIZE_FACTIONS);
    }
}
