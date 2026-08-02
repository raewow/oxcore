//! Chat system message structs
//!
//! This module contains type-safe message structures for all chat-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgMessageChat`] - Main chat message (say, yell, whisper, etc.)
//! - [`SmsgChatWrongFaction`] - Cross-faction whisper blocked
//! - [`SmsgChatPlayerNotFound`] - Whisper target not found
//! - [`SmsgChatRestricted`] - Player is muted/chat restricted
//! - [`SmsgChatPlayerAmbiguous`] - Multiple players match whisper target

use crate::game::chat::{to_modern_chat_type, ChatMsg, ChatTag, Language};
use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::packet::WorldPacketGuidExt;
use crate::protocol::ObjectGuid;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;

/// SMSG_MESSAGECHAT - Main chat message packet
///
/// Used for all chat message types: say, yell, whisper, party, guild, etc.
/// The packet format varies based on the message type.
///
/// ## Packet Format (Vanilla 1.12.1)
/// - msgtype (u8) - Chat message type
/// - language (u32) - Language ID
/// - [type-specific fields] - Varies by message type
/// - message_length (u32) - Length of message + 1
/// - message (cstring) - The actual message
/// - chat_tag (u8) - AFK/DND/GM tag
#[derive(Debug, Clone)]
pub struct SmsgMessageChat<'a> {
    /// Chat message type (say, yell, whisper, etc.)
    pub msgtype: ChatMsg,
    /// Language of the message
    pub language: Language,
    /// Sender's GUID
    pub sender_guid: ObjectGuid,
    /// Sender's name (for monster messages)
    pub sender_name: Option<&'a str>,
    /// Target's GUID (for whispers, monster whispers)
    pub target_guid: Option<ObjectGuid>,
    /// Channel name (for channel messages)
    pub channel_name: Option<&'a str>,
    /// Player rank in channel
    pub player_rank: Option<u8>,
    /// The message content
    pub message: &'a str,
    /// Chat tag (AFK, DND, GM)
    pub chat_tag: ChatTag,
}

impl<'a> SmsgMessageChat<'a> {
    /// Create a new whisper message from sender to target
    pub fn whisper(
        sender_guid: ObjectGuid,
        sender_name: &'a str,
        target_guid: ObjectGuid,
        message: &'a str,
    ) -> Self {
        Self {
            msgtype: ChatMsg::Whisper,
            language: Language::Universal,
            sender_guid,
            sender_name: Some(sender_name),
            target_guid: Some(target_guid),
            channel_name: None,
            player_rank: None,
            message,
            chat_tag: ChatTag::None,
        }
    }

    /// Create a whisper inform message (sent back to sender)
    pub fn whisper_inform(target_guid: ObjectGuid, target_name: &'a str, message: &'a str) -> Self {
        Self {
            msgtype: ChatMsg::WhisperInform,
            language: Language::Universal,
            sender_guid: target_guid,
            sender_name: Some(target_name),
            target_guid: Some(target_guid),
            channel_name: None,
            player_rank: None,
            message,
            chat_tag: ChatTag::None,
        }
    }

    /// Create an "ignored" message (when target has sender on ignore list)
    pub fn ignored(target_guid: ObjectGuid, target_name: &'a str) -> Self {
        Self {
            msgtype: ChatMsg::Ignored,
            language: Language::Universal,
            sender_guid: target_guid,
            sender_name: Some(target_name),
            target_guid: None,
            channel_name: None,
            player_rank: None,
            message: target_name,
            chat_tag: ChatTag::None,
        }
    }

    /// Create a system message
    pub fn system(sender_guid: ObjectGuid, message: &'a str) -> Self {
        Self {
            msgtype: ChatMsg::System,
            language: Language::Universal,
            sender_guid,
            sender_name: None,
            target_guid: None,
            channel_name: None,
            player_rank: None,
            message,
            chat_tag: ChatTag::None,
        }
    }

    /// Create a system message with a chat tag (for AFK/DND status)
    pub fn system_with_tag(
        sender_guid: ObjectGuid,
        sender_name: &'a str,
        message: &'a str,
        chat_tag: ChatTag,
    ) -> Self {
        Self {
            msgtype: ChatMsg::System,
            language: Language::Universal,
            sender_guid,
            sender_name: Some(sender_name),
            target_guid: None,
            channel_name: None,
            player_rank: None,
            message,
            chat_tag,
        }
    }
}

impl ToWorldPacket for SmsgMessageChat<'_> {
    /// `ChatPkt::Write` for build 42597.
    ///
    /// One flat layout replaces vanilla's per-type branching. Vanilla writes a different field set for
    /// a monster whisper than for a say; 1.14 writes **six GUIDs and five strings every time** and
    /// lets the empty ones be empty. So the `match self.msgtype` above has no counterpart here.
    ///
    /// Two hazards:
    ///
    /// * the chat type is **renumbered**, not extended — see [`to_modern_chat_type`]. Vanilla `Say` is
    ///   0 and 1.14 reads 0 as `System`.
    /// * every string length is bit-packed at a *different* width (11/11/5/7/12), and the flags are 14
    ///   bits. Getting one wrong shifts all five strings.
    fn to_modern(&self) -> Option<WorldPacket> {
        /// `Language::AddonBfA` — 1.14's addon channel. Vanilla signals addon traffic with a chat
        /// *type* of 0xFF; 1.14 signals it with a language instead, so the type becomes an ordinary
        /// Say and the language carries the meaning.
        const LANGUAGE_ADDON_MODERN: u32 = 35;

        let is_addon = matches!(self.msgtype, ChatMsg::Addon);
        let slash_cmd = to_modern_chat_type(self.msgtype)?;
        let language = if is_addon {
            LANGUAGE_ADDON_MODERN
        } else {
            self.language as u32
        };

        let sender_name = self.sender_name.unwrap_or("");
        let channel = self.channel_name.unwrap_or("");

        let mut writer = BitWriter::new();
        writer.write_u8(slash_cmd);
        writer.write_u32(language);

        let (high, low) = self.sender_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // SenderGUID
        writer.write_packed_guid_128(0, 0); // SenderGuildGUID
        writer.write_packed_guid_128(0, 0); // SenderAccountGUID -- no bnet account mapping
        let (high, low) = self
            .target_guid
            .unwrap_or_default()
            .to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // TargetGUID
        writer.write_u32(0); // TargetVirtualAddress
        writer.write_u32(0); // SenderVirtualAddress
        writer.write_packed_guid_128(0, 0); // PartyGUID
        writer.write_u32(0); // AchievementID
        writer.write_f32(0.0); // DisplayTime

        // Five string lengths at five different widths, then the flags, all in one bit run.
        writer.write_bits(sender_name.len() as u32, 11);
        writer.write_bits(0, 11); // TargetName -- resolved client-side from TargetGUID
        writer.write_bits(0, 5); // Prefix (addon) length
        writer.write_bits(channel.len() as u32, 7);
        writer.write_bits(self.message.len() as u32, 12);
        writer.write_bits(u32::from(self.chat_tag as u8), 14); // ChatFlags
        writer.write_bit(false); // HideChatLog
        writer.write_bit(false); // FakeSenderName
        writer.write_bit(false); // HasUnused_801
        writer.write_bit(false); // HasChannelGUID
        writer.flush_bits();

        writer.write_bytes(sender_name.as_bytes());
        // TargetName and Prefix declared zero-length above, so nothing goes here.
        writer.write_bytes(channel.as_bytes());
        writer.write_bytes(self.message.as_bytes());

        Some(writer.finish(Opcode::SMSG_MESSAGECHAT))
    }
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_MESSAGECHAT);

        // Write message type (as u8, Addon is special - 0xFF)
        let msgtype_byte = if matches!(self.msgtype, ChatMsg::Addon) {
            0xFF
        } else {
            self.msgtype as u8
        };
        packet.write_u8(msgtype_byte);

        // Write language
        packet.write_u32(self.language as u32);

        // Write sender/target info based on message type
        match self.msgtype {
            ChatMsg::MonsterWhisper | ChatMsg::MonsterEmote => {
                // For monster messages: senderName, targetGuid (u64)
                if let Some(name) = self.sender_name {
                    packet.write_u32(name.len() as u32 + 1);
                    packet.write_cstring(name);
                } else {
                    packet.write_u32(1);
                    packet.write_u8(0);
                }
                if let Some(guid) = self.target_guid {
                    packet.write_guid_raw(guid.raw());
                } else {
                    packet.write_u64(0);
                }
            }
            ChatMsg::Say | ChatMsg::Party | ChatMsg::Yell => {
                // For player messages: senderGuid (twice, full u64 format)
                packet.write_guid_raw(self.sender_guid.raw());
                packet.write_guid_raw(self.sender_guid.raw());
            }
            ChatMsg::MonsterSay | ChatMsg::MonsterYell => {
                // For monster say/yell: senderGuid (u64), senderName, targetGuid (u64)
                packet.write_guid_raw(self.sender_guid.raw());
                if let Some(name) = self.sender_name {
                    packet.write_u32(name.len() as u32 + 1);
                    packet.write_cstring(name);
                } else {
                    packet.write_u32(1);
                    packet.write_u8(0);
                }
                if let Some(guid) = self.target_guid {
                    packet.write_guid_raw(guid.raw());
                } else {
                    packet.write_u64(0);
                }
            }
            ChatMsg::Channel => {
                // For channel messages: channelName (cstring), playerRank (u32), senderGuid (u64)
                if let Some(name) = self.channel_name {
                    packet.write_cstring(name);
                } else {
                    packet.write_u8(0);
                }
                packet.write_u32(self.player_rank.unwrap_or(0) as u32);
                packet.write_guid_raw(self.sender_guid.raw());
            }
            _ => {
                // Default: just senderGuid (u64 format)
                packet.write_guid_raw(self.sender_guid.raw());
            }
        }

        // Write message
        packet.write_u32(self.message.len() as u32 + 1);
        packet.write_cstring(self.message);

        // Write chat tag
        packet.write_u8(self.chat_tag as u8);

        packet
    }
}

/// SMSG_CHAT_WRONG_FACTION - Cross-faction whisper blocked
///
/// Sent when a player tries to whisper someone of the opposite faction
/// and cross-faction whispers are disabled.
///
/// ## Packet Format
/// Empty packet (opcode only)
#[derive(Debug, Clone, Copy)]
pub struct SmsgChatWrongFaction;

/// No `to_modern`: the 1.14 opcode table has no cross-faction-whisper error.
///
/// The opcode was removed: the 1.14 table has every other chat error in this family -- not found,
/// ambiguous, restricted, not in party -- but nothing for a cross-faction whisper, so there is no
/// opcode to send a body under. A 1.14 client can only be told the target does not exist, which is
/// [`SmsgChatPlayerNotFound`]; callers that want any feedback here must send that instead.
impl ToWorldPacket for SmsgChatWrongFaction {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_CHAT_WRONG_FACTION)
    }
}

/// SMSG_CHAT_PLAYER_NOT_FOUND - Whisper target not found
///
/// Sent when the whisper target player is not online or doesn't exist.
///
/// ## Packet Format
/// - name (cstring) - The name that was searched for
#[derive(Debug, Clone)]
pub struct SmsgChatPlayerNotFound<'a> {
    /// The name that was searched for
    pub name: &'a str,
}

impl ToWorldPacket for SmsgChatPlayerNotFound<'_> {
    /// The name loses its null terminator and gains a 9-bit length prefix.
    ///
    /// The prefix is a byte *count*, not a character count, and it is the whole body's framing: a
    /// stray trailing NUL would be counted as part of the name and the client would render the
    /// error with an invisible character appended, so `write_string_raw` is used rather than
    /// `write_cstring`.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_bits(self.name.len() as u32, 9);
        writer.flush_bits();
        writer.write_string_raw(self.name);
        Some(writer.finish(Opcode::SMSG_CHAT_PLAYER_NOT_FOUND))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CHAT_PLAYER_NOT_FOUND);
        packet.write_cstring(self.name);
        packet
    }
}

/// SMSG_CHAT_RESTRICTED - Player is muted/chat restricted
///
/// Sent when a muted player tries to chat.
///
/// ## Packet Format
/// Empty packet (opcode only)
#[derive(Debug, Clone, Copy)]
pub struct SmsgChatRestricted;

/// No `to_modern`: the 1.14 body carries a restriction reason this struct does not have.
///
/// The opcode still exists in 1.14, but where vanilla sends an empty packet that means only "you
/// are muted", the 1.14 body selects *which* restriction message to print. This struct is a unit
/// type and the vanilla wire format carries nothing to derive that from, so any value written here
/// would be invented -- and the wrong one tells the player the wrong thing about why they cannot
/// talk. Porting this needs a reason on the struct first, which in turn needs the sending code to
/// distinguish the cases.
impl ToWorldPacket for SmsgChatRestricted {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_CHAT_RESTRICTED)
    }
}

/// SMSG_CHAT_PLAYER_AMBIGUOUS - Multiple players match target name
///
/// Sent when the whisper target name matches multiple online players.
///
/// ## Packet Format
/// - name (cstring) - The ambiguous name
#[derive(Debug, Clone)]
pub struct SmsgChatPlayerAmbiguous<'a> {
    /// The ambiguous name that matched multiple players
    pub name: &'a str,
}

impl ToWorldPacket for SmsgChatPlayerAmbiguous<'_> {
    /// Same shape as [`SmsgChatPlayerNotFound`]: a 9-bit byte count, then the raw name.
    ///
    /// **Unverified.** The 1.14 opcode exists and the body is a single player name, but no
    /// authoritative field list for it was available; this mirrors the sibling name-carrying chat
    /// error, whose 9 bits are sized exactly for a player name. If a 1.14 client mis-renders this
    /// one error toast, this is the reason -- it cannot affect anything else, since each packet is
    /// framed independently.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_bits(self.name.len() as u32, 9);
        writer.flush_bits();
        writer.write_string_raw(self.name);
        Some(writer.finish(Opcode::SMSG_CHAT_PLAYER_AMBIGUOUS))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CHAT_PLAYER_AMBIGUOUS);
        packet.write_cstring(self.name);
        packet
    }
}

/// SMSG_EMOTE - Emote animation broadcast
///
/// Sent when a player performs an emote animation (dance, wave, etc.).
/// Broadcast to nearby players to show the animation.
///
/// ## Packet Format (Vanilla 1.12.1)
/// - emote_id (u32) - Emote animation ID
/// - guid (u64) - Player GUID performing the emote
#[derive(Debug, Clone)]
pub struct SmsgEmote {
    /// Emote animation ID
    pub emote_id: u32,
    /// Player GUID performing the emote
    pub guid: ObjectGuid,
}

impl ToWorldPacket for SmsgEmote {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_EMOTE);
        packet.write_u32(self.emote_id);
        packet.write_guid(self.guid);
        packet
    }

    /// `EmoteMessage::Write`, per the 1.14 wire format.
    ///
    /// The GUID moves *ahead* of the emote id, and two fields are appended: a spell-visual-kit list
    /// (added in 1.14.0) and a sequence variation (added in 1.14.2). Build 42597 is 1.14.2, so both
    /// are present — an empty list and a zero variation.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(self.emote_id);
        writer.write_i32(0); // SpellVisualKitIDs count
        writer.write_i32(0); // SequenceVariation
        Some(writer.finish(Opcode::SMSG_EMOTE))
    }
}

/// SMSG_TEXT_EMOTE - Text emote broadcast
///
/// Sent when a player performs a text emote (/dance, /wave, etc.).
/// Broadcast to nearby players to show the emote text.
///
/// ## Packet Format (Vanilla 1.12.1)
/// - guid (u64) - Player GUID performing the emote
/// - text_emote (u32) - Text emote ID
/// - emote_num (u32) - Emote animation number
/// - name_length (u32) - Length of target name + 1
/// - target_name (cstring) - Target name (or empty if no target)
#[derive(Debug, Clone)]
pub struct SmsgTextEmote<'a> {
    /// Player GUID performing the emote
    pub guid: ObjectGuid,
    /// Text emote ID
    pub text_emote: u32,
    /// Emote animation number
    pub emote_num: u32,
    /// Target name (None if no target)
    ///
    /// Vanilla identifies the target by name; 1.14 sends its GUID and resolves the name locally, so
    /// both are carried here and each protocol writes the one it wants.
    pub target_name: Option<&'a str>,
    /// Target GUID, empty when the emote has no target. Used only by the modern body.
    pub target_guid: ObjectGuid,
}

impl ToWorldPacket for SmsgTextEmote<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_TEXT_EMOTE);
        packet.write_guid(self.guid);
        packet.write_u32(self.text_emote);
        packet.write_u32(self.emote_num);

        // Write target name if present
        if let Some(name) = self.target_name {
            packet.write_u32(name.len() as u32 + 1);
            packet.write_cstring(name);
        } else {
            packet.write_u32(1);
            packet.write_u8(0); // Empty string (null terminator)
        }

        packet
    }

    /// `STextEmote::Write`, per the 1.14 wire format.
    ///
    /// 1.14 names the target by **GUID**, not by name — the client looks the name up itself — and adds
    /// a source account GUID and a sound index. So the name this struct carries for vanilla is unused
    /// here, and `target_guid` is what matters.
    ///
    /// The account GUID is sent empty: it identifies the emoting player's Battle.net account, which
    /// has no 1.12 equivalent and which the client only uses for ignore checks.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // SourceGUID
        writer.write_packed_guid_128(0, 0); // SourceAccountGUID
        writer.write_i32(self.text_emote as i32); // EmoteID
        writer.write_i32(self.emote_num as i32); // SoundIndex
        let (high, low) = self.target_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // TargetGUID
        Some(writer.finish(Opcode::SMSG_TEXT_EMOTE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smsg_message_chat_whisper() {
        let sender_guid = ObjectGuid::from_low(100);
        let target_guid = ObjectGuid::from_low(200);
        let msg = SmsgMessageChat::whisper(sender_guid, "Sender", target_guid, "Hello!");
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_MESSAGECHAT);
    }

    #[test]
    fn test_smsg_message_chat_whisper_inform() {
        let target_guid = ObjectGuid::from_low(200);
        let msg = SmsgMessageChat::whisper_inform(target_guid, "Target", "Hello!");
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_MESSAGECHAT);
    }

    #[test]
    fn test_smsg_message_chat_ignored() {
        let target_guid = ObjectGuid::from_low(200);
        let msg = SmsgMessageChat::ignored(target_guid, "Target");
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_MESSAGECHAT);
    }

    #[test]
    fn test_smsg_chat_wrong_faction() {
        let msg = SmsgChatWrongFaction;
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_CHAT_WRONG_FACTION);
    }

    #[test]
    fn test_smsg_chat_player_not_found() {
        let msg = SmsgChatPlayerNotFound { name: "Unknown" };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_CHAT_PLAYER_NOT_FOUND);
    }

    #[test]
    fn test_smsg_chat_restricted() {
        let msg = SmsgChatRestricted;
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_CHAT_RESTRICTED);
    }

    #[test]
    fn test_smsg_chat_player_ambiguous() {
        let msg = SmsgChatPlayerAmbiguous { name: "Player" };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_CHAT_PLAYER_AMBIGUOUS);
    }

    #[test]
    fn test_smsg_emote() {
        let guid = ObjectGuid::from_low(100);
        let msg = SmsgEmote { emote_id: 1, guid };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_EMOTE);

        // Verify packet structure
        let data = packet.contents();
        // First 4 bytes: emote_id (u32 little-endian)
        assert_eq!(data[0], 0x01);
        assert_eq!(data[1], 0x00);
        assert_eq!(data[2], 0x00);
        assert_eq!(data[3], 0x00);
    }

    #[test]
    fn test_smsg_text_emote_no_target() {
        let guid = ObjectGuid::from_low(100);
        let msg = SmsgTextEmote {
            guid,
            text_emote: 1,
            emote_num: 2,
            target_name: None,
            target_guid: ObjectGuid::empty(),
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_TEXT_EMOTE);
    }

    #[test]
    fn test_smsg_text_emote_with_target() {
        let guid = ObjectGuid::from_low(100);
        let msg = SmsgTextEmote {
            guid,
            text_emote: 1,
            emote_num: 2,
            target_name: Some("Target"),
            target_guid: ObjectGuid::new_player(2),
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_TEXT_EMOTE);
    }

    #[test]
    fn test_monster_say_packet_structure() {
        use crate::game::chat::{ChatMsg, ChatTag, Language};

        let guid = ObjectGuid::from_low(42);
        let msg = SmsgMessageChat {
            msgtype: ChatMsg::MonsterSay,
            language: Language::Universal,
            sender_guid: guid,
            sender_name: Some("Thug"),
            target_guid: None,
            channel_name: None,
            player_rank: None,
            message: "Hello!",
            chat_tag: ChatTag::None,
        };
        let packet = msg.to_vanilla();
        let data = packet.contents();

        // Byte 0: chat type = MonsterSay = 0x0B
        assert_eq!(data[0], 0x0B, "chat type should be MonsterSay (0x0B)");

        // Bytes 1-4: language = 0 (Universal, u32 LE)
        assert_eq!(
            &data[1..5],
            &[0, 0, 0, 0],
            "language should be Universal (0)"
        );

        // Bytes 5-12: sender GUID (u64 LE)
        let sender_raw = u64::from_le_bytes(data[5..13].try_into().unwrap());
        assert_eq!(sender_raw, guid.raw(), "sender GUID mismatch");

        // Bytes 13-16: name length (u32 LE) = "Thug".len() + 1 = 5
        let name_len = u32::from_le_bytes(data[13..17].try_into().unwrap());
        assert_eq!(name_len, 5, "name length should be 5 (4 chars + null)");

        // Bytes 17-20: "Thug" + null
        assert_eq!(&data[17..21], b"Thug");
        assert_eq!(data[21], 0, "name null terminator");

        // Bytes 22-29: target GUID = 0 (u64 LE, no target)
        let target_raw = u64::from_le_bytes(data[22..30].try_into().unwrap());
        assert_eq!(target_raw, 0, "target GUID should be 0 when no target");

        // Bytes 30-33: text length (u32 LE) = "Hello!".len() + 1 = 7
        let text_len = u32::from_le_bytes(data[30..34].try_into().unwrap());
        assert_eq!(text_len, 7, "text length should be 7 (6 chars + null)");

        // Bytes 34-39: "Hello!" + null
        assert_eq!(&data[34..40], b"Hello!");
        assert_eq!(data[40], 0, "text null terminator");

        // Byte 41: chat tag = None = 0
        assert_eq!(data[41], 0, "chat tag should be None (0)");
    }

    #[test]
    fn test_monster_yell_packet_structure() {
        use crate::game::chat::{ChatMsg, ChatTag, Language};

        let guid = ObjectGuid::from_low(99);
        let msg = SmsgMessageChat {
            msgtype: ChatMsg::MonsterYell,
            language: Language::Universal,
            sender_guid: guid,
            sender_name: Some("Boss"),
            target_guid: None,
            channel_name: None,
            player_rank: None,
            message: "Die!",
            chat_tag: ChatTag::None,
        };
        let packet = msg.to_vanilla();
        let data = packet.contents();

        // Byte 0: chat type = MonsterYell = 0x0C
        assert_eq!(data[0], 0x0C, "chat type should be MonsterYell (0x0C)");

        // Bytes 1-4: language
        assert_eq!(&data[1..5], &[0, 0, 0, 0]);

        // Bytes 5-12: sender GUID (no extra flags field between GUID and name)
        let sender_raw = u64::from_le_bytes(data[5..13].try_into().unwrap());
        assert_eq!(sender_raw, guid.raw());

        // Bytes 13-16: name length = "Boss".len() + 1 = 5
        let name_len = u32::from_le_bytes(data[13..17].try_into().unwrap());
        assert_eq!(name_len, 5, "name length should be 5");
    }

    #[test]
    fn test_monster_say_no_spurious_flags_field() {
        // Regression test: the old hand-built packet had a u32 flags field
        // between sender GUID and name length, corrupting the packet.
        use crate::game::chat::{ChatMsg, ChatTag, Language};

        let guid = ObjectGuid::from_low(1);
        let msg = SmsgMessageChat {
            msgtype: ChatMsg::MonsterSay,
            language: Language::Universal,
            sender_guid: guid,
            sender_name: Some("A"),
            target_guid: None,
            channel_name: None,
            player_rank: None,
            message: "B",
            chat_tag: ChatTag::None,
        };
        let packet = msg.to_vanilla();
        let data = packet.contents();

        // Expected layout:
        // [0]    chat_type  (1 byte)
        // [1-4]  language   (4 bytes)
        // [5-12] guid       (8 bytes)
        // [13-16] name_len  (4 bytes) = 2
        // [17]   'A'        (1 byte)
        // [18]   0          (null terminator)
        // [19-26] target    (8 bytes) = 0
        // [27-30] text_len  (4 bytes) = 2
        // [31]   'B'        (1 byte)
        // [32]   0          (null terminator)
        // [33]   chat_tag   (1 byte) = 0
        // Total: 34 bytes
        assert_eq!(
            data.len(),
            34,
            "packet should be exactly 34 bytes (no extra flags field)"
        );

        // Verify name_len is at offset 13 (immediately after GUID), not offset 17
        let name_len = u32::from_le_bytes(data[13..17].try_into().unwrap());
        assert_eq!(name_len, 2, "name_len should be at offset 13 with value 2");
    }
}

#[cfg(test)]
mod modern_chat_tests {
    use super::*;

    fn say(text: &str) -> WorldPacket {
        SmsgMessageChat {
            msgtype: ChatMsg::Say,
            language: Language::Common,
            sender_guid: ObjectGuid::new_player(4),
            sender_name: Some("Tester"),
            target_guid: None,
            channel_name: None,
            player_rank: None,
            message: text,
            chat_tag: ChatTag::None,
        }
        .to_modern()
        .expect("say must encode for modern")
    }

    /// The enums are renumbered, not extended. Vanilla `Say` is 0 and 1.14 reads 0 as `System`, so a
    /// value-copied type routes every line to the wrong window.
    #[test]
    fn chat_types_are_renumbered_not_copied() {
        assert_eq!(ChatMsg::Say as u8, 0, "vanilla Say");
        assert_eq!(to_modern_chat_type(ChatMsg::Say), Some(1), "1.14 Say");
        assert_eq!(ChatMsg::System as u8, 10, "vanilla System");
        assert_eq!(to_modern_chat_type(ChatMsg::System), Some(0), "1.14 System");
        // The first byte of the body is the translated type, not the vanilla one.
        assert_eq!(say("hi").contents()[0], 1);
    }

    /// Per-swing combat spam has no 1.14 chat type -- it goes through the combat log. Inventing a
    /// number would put it in an arbitrary window.
    #[test]
    fn vanilla_only_combat_types_are_declined() {
        assert_eq!(to_modern_chat_type(ChatMsg::CombatSelfHits), None);
        assert_eq!(to_modern_chat_type(ChatMsg::CombatPetMisses), None);
    }

    /// The message body's length is a 12-bit field, so the body must grow exactly with the text.
    #[test]
    fn the_body_grows_with_the_message() {
        let short = say("hi").size();
        let long = say("hello there").size();
        assert_eq!(long - short, "hello there".len() - "hi".len());
    }

    /// 1.14 writes every GUID and string slot regardless of chat type, where vanilla branches per
    /// type. A say and a whisper must therefore have the same shape.
    #[test]
    fn the_layout_does_not_branch_on_chat_type() {
        let whisper = SmsgMessageChat {
            msgtype: ChatMsg::Whisper,
            language: Language::Common,
            sender_guid: ObjectGuid::new_player(4),
            sender_name: Some("Tester"),
            target_guid: Some(ObjectGuid::new_player(5)),
            channel_name: None,
            player_rank: None,
            message: "hi",
            chat_tag: ChatTag::None,
        }
        .to_modern()
        .unwrap();

        // Only the target GUID differs in length between the two, so the whisper is longer by
        // exactly the bytes that GUID packs into.
        assert!(whisper.size() > say("hi").size());
    }
}
