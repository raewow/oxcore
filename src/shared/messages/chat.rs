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

use crate::shared::game::chat::{ChatMsg, ChatTag, Language};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::ObjectGuid;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;
use crate::shared::protocol::packet::WorldPacketGuidExt;

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
    fn to_world_packet(&self) -> WorldPacket {
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

impl ToWorldPacket for SmsgChatWrongFaction {
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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

impl ToWorldPacket for SmsgChatRestricted {
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_EMOTE);
        packet.write_u32(self.emote_id);
        packet.write_guid(self.guid);
        packet
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
    pub target_name: Option<&'a str>,
}

impl ToWorldPacket for SmsgTextEmote<'_> {
    fn to_world_packet(&self) -> WorldPacket {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smsg_message_chat_whisper() {
        let sender_guid = ObjectGuid::from_low(100);
        let target_guid = ObjectGuid::from_low(200);
        let msg = SmsgMessageChat::whisper(sender_guid, "Sender", target_guid, "Hello!");
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_MESSAGECHAT);
    }

    #[test]
    fn test_smsg_message_chat_whisper_inform() {
        let target_guid = ObjectGuid::from_low(200);
        let msg = SmsgMessageChat::whisper_inform(target_guid, "Target", "Hello!");
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_MESSAGECHAT);
    }

    #[test]
    fn test_smsg_message_chat_ignored() {
        let target_guid = ObjectGuid::from_low(200);
        let msg = SmsgMessageChat::ignored(target_guid, "Target");
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_MESSAGECHAT);
    }

    #[test]
    fn test_smsg_chat_wrong_faction() {
        let msg = SmsgChatWrongFaction;
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_CHAT_WRONG_FACTION);
    }

    #[test]
    fn test_smsg_chat_player_not_found() {
        let msg = SmsgChatPlayerNotFound { name: "Unknown" };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_CHAT_PLAYER_NOT_FOUND);
    }

    #[test]
    fn test_smsg_chat_restricted() {
        let msg = SmsgChatRestricted;
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_CHAT_RESTRICTED);
    }

    #[test]
    fn test_smsg_chat_player_ambiguous() {
        let msg = SmsgChatPlayerAmbiguous { name: "Player" };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_CHAT_PLAYER_AMBIGUOUS);
    }

    #[test]
    fn test_smsg_emote() {
        let guid = ObjectGuid::from_low(100);
        let msg = SmsgEmote { emote_id: 1, guid };
        let packet = msg.to_world_packet();
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
        };
        let packet = msg.to_world_packet();
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
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_TEXT_EMOTE);
    }

    #[test]
    fn test_monster_say_packet_structure() {
        use crate::shared::game::chat::{ChatMsg, ChatTag, Language};

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
        let packet = msg.to_world_packet();
        let data = packet.contents();

        // Byte 0: chat type = MonsterSay = 0x0B
        assert_eq!(data[0], 0x0B, "chat type should be MonsterSay (0x0B)");

        // Bytes 1-4: language = 0 (Universal, u32 LE)
        assert_eq!(&data[1..5], &[0, 0, 0, 0], "language should be Universal (0)");

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
        use crate::shared::game::chat::{ChatMsg, ChatTag, Language};

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
        let packet = msg.to_world_packet();
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
        use crate::shared::game::chat::{ChatMsg, ChatTag, Language};

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
        let packet = msg.to_world_packet();
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
        assert_eq!(data.len(), 34, "packet should be exactly 34 bytes (no extra flags field)");

        // Verify name_len is at offset 13 (immediately after GUID), not offset 17
        let name_len = u32::from_le_bytes(data[13..17].try_into().unwrap());
        assert_eq!(name_len, 2, "name_len should be at offset 13 with value 2");
    }
}
