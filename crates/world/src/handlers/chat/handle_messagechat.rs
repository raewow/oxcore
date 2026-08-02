//! CMSG_MESSAGECHAT handler - main chat message dispatcher
//!
//! This handler is extremely thin - it only parses the packet and delegates
//! to ChatSystem. All business logic, validation, faction filtering, distance
//! checks, and packet sending happens in the system.

use anyhow::{anyhow, Result};

use crate::core::session::WorldSession;
use crate::game::chat::commands::ChatCommandContext;
use crate::World;
use oxcore_shared::common::AccountType;
use oxcore_shared::game::chat::{ChatMsg, ChatTag, Language, Team};
use oxcore_shared::messages::chat::SmsgMessageChat;
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::bitbuf::BitReader;
use oxcore_shared::protocol::{ObjectGuid, Protocol, WorldPacket};

fn read_modern_message(
    packet: &WorldPacket,
    msg_type: ChatMsg,
) -> Result<(Option<String>, Option<String>, String)> {
    let mut reader = BitReader::new(packet.contents());
    let has_language = !matches!(msg_type, ChatMsg::Emote);
    if has_language {
        reader
            .read_u32()
            .ok_or_else(|| anyhow!("Missing language in modern chat packet"))?;
    }

    if msg_type == ChatMsg::Channel {
        reader
            .read_packed_guid_128()
            .ok_or_else(|| anyhow!("Missing channel GUID in modern chat packet"))?;
    }

    let first_len = reader
        .read_bits(9)
        .ok_or_else(|| anyhow!("Missing first string length in modern chat packet"))?
        as usize;
    if matches!(msg_type, ChatMsg::Whisper | ChatMsg::Channel) {
        let second_len = reader
            .read_bits(9)
            .ok_or_else(|| anyhow!("Missing message length in modern chat packet"))?
            as usize;
        let first = reader
            .read_string(first_len)
            .ok_or_else(|| anyhow!("Invalid first string in modern chat packet"))?;
        let message = reader
            .read_string(second_len)
            .ok_or_else(|| anyhow!("Invalid message in modern chat packet"))?;
        return Ok(match msg_type {
            ChatMsg::Channel => (Some(first), None, message),
            ChatMsg::Whisper => (None, Some(first), message),
            _ => unreachable!("only channel and whisper have two strings"),
        });
    }

    let first = reader
        .read_string(first_len)
        .ok_or_else(|| anyhow!("Invalid message in modern chat packet"))?;
    Ok((None, None, first))
}

/// Handle CMSG_MESSAGECHAT - player sends a chat message
pub async fn handle_messagechat(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    handle_messagechat_type(session, packet, world, None).await
}

/// Handle one of the modern, destination-specific chat opcodes.
pub async fn handle_modern_messagechat(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
    msg_type: ChatMsg,
) -> Result<()> {
    handle_messagechat_type(session, packet, world, Some(msg_type)).await
}

async fn handle_messagechat_type(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
    modern_msg_type: Option<ChatMsg>,
) -> Result<()> {
    // Get sender context
    let sender_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let player = world
        .managers
        .player_mgr
        .get_player(sender_guid)
        .ok_or_else(|| anyhow!("Player not found"))?;
    let sender_team = Team::from_race(player.race);
    drop(player);

    let (msg_type, channel_name_opt, target_name_opt, message) = match modern_msg_type {
        Some(msg_type) => {
            let (channel, target, message) = read_modern_message(packet, msg_type)?;
            (msg_type, channel, target, message)
        }
        None => {
            if session.protocol() == Protocol::Modern {
                return Err(anyhow!(
                    "Modern chat must use a destination-specific opcode"
                ));
            }
            let msg_type =
                ChatMsg::from_u32(packet.read_u32().unwrap_or(0)).unwrap_or(ChatMsg::Say);
            let _language = packet.read_u32().unwrap_or(0);
            match msg_type {
                ChatMsg::Channel => (
                    msg_type,
                    Some(packet.read_string().unwrap_or_default()),
                    None,
                    packet.read_string().unwrap_or_default(),
                ),
                ChatMsg::Whisper => (
                    msg_type,
                    None,
                    Some(packet.read_string().unwrap_or_default()),
                    packet.read_string().unwrap_or_default(),
                ),
                _ => (
                    msg_type,
                    None,
                    None,
                    packet.read_string().unwrap_or_default(),
                ),
            }
        }
    };

    // Check for chat commands (starting with '.' or '!')
    if message.starts_with('.') || message.starts_with('!') {
        let command_str = &message[1..];
        if !command_str.is_empty() {
            // Parse command name to check if it exists
            let command_name = command_str.split_whitespace().next().unwrap_or("");

            // Only intercept if the command exists, otherwise fall through to regular chat
            if world.systems.chat.command_exists(command_name) {
                return handle_command(session, sender_guid, command_str, world).await;
            }
            // Unknown command - let it fall through to regular chat
        }
    }

    // Delegate to system - system handles EVERYTHING:
    // - Validation (flood protection, message length, etc.)
    // - Business logic (faction filtering, distance checks, ignore lists)
    // - Packet construction and sending (including error responses)
    match msg_type {
        ChatMsg::Say => {
            world
                .systems
                .chat
                .send_say(
                    sender_guid,
                    &message,
                    sender_team,
                    world.config.allow_cross_faction_chat,
                )
                .await?;
        }
        ChatMsg::Yell => {
            world
                .systems
                .chat
                .send_yell(
                    sender_guid,
                    &message,
                    sender_team,
                    world.config.allow_cross_faction_chat,
                )
                .await?;
        }
        ChatMsg::Whisper => {
            if let Some(ref target_name) = target_name_opt {
                world
                    .systems
                    .chat
                    .send_whisper(sender_guid, target_name, &message, &world.systems.social)
                    .await?;
            }
        }
        ChatMsg::Emote => {
            world.systems.chat.send_emote(sender_guid, &message).await?;
        }
        ChatMsg::Channel => {
            if let Some(ref channel_name) = channel_name_opt {
                world
                    .systems
                    .chat
                    .send_channel_message(sender_guid, channel_name, &message, sender_team)
                    .await?;
            }
        }
        ChatMsg::Party => {
            world
                .systems
                .chat
                .send_party(sender_guid, &message, &world.systems.group)
                .await?;
        }
        ChatMsg::Raid | ChatMsg::RaidLeader | ChatMsg::RaidWarning => {
            world
                .systems
                .chat
                .send_raid(sender_guid, &message, msg_type, &world.systems.group)
                .await?;
        }
        ChatMsg::Guild => {
            world
                .systems
                .chat
                .send_guild(sender_guid, &message, &world.systems.guild)?;
        }
        ChatMsg::Officer => {
            world
                .systems
                .chat
                .send_officer(sender_guid, &message, &world.systems.guild)?;
        }
        _ => {
            // Unsupported message type
        }
    }

    Ok(())
}

/// Handle chat command execution
async fn handle_command(
    session: &WorldSession,
    player_guid: ObjectGuid,
    command_str: &str,
    world: &World,
) -> Result<()> {
    // Get target if player has one selected
    let target = world.managers.player_mgr.get_selection(player_guid);

    // Build command context
    let ctx = ChatCommandContext {
        session,
        player_guid,
        target,
        world,
        security: AccountType::from_u8(session.security()),
    };

    // Execute command
    match world.systems.chat.execute_command(command_str, &ctx).await {
        Ok(msg) if !msg.is_empty() => {
            // Send result as system message to player
            let packet = SmsgMessageChat {
                msgtype: ChatMsg::System,
                language: Language::Universal,
                sender_guid: ObjectGuid::empty(),
                sender_name: None,
                target_guid: None,
                channel_name: None,
                player_rank: None,
                message: &msg,
                chat_tag: ChatTag::None,
            };
            world
                .managers
                .broadcast_mgr
                .send_msg_to_player(player_guid, packet);
        }
        Ok(_) => {
            // Empty response - command handled but no feedback
        }
        Err(e) => {
            // Command failed - send error message
            let error_msg = format!("Command error: {}", e);
            let packet = SmsgMessageChat {
                msgtype: ChatMsg::System,
                language: Language::Universal,
                sender_guid: ObjectGuid::empty(),
                sender_name: None,
                target_guid: None,
                channel_name: None,
                player_rank: None,
                message: &error_msg,
                chat_tag: ChatTag::None,
            };
            world
                .managers
                .broadcast_mgr
                .send_msg_to_player(player_guid, packet);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcore_shared::protocol::bitbuf::BitWriter;
    use oxcore_shared::protocol::Opcode;

    fn packet(writer: BitWriter) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::CMSG_CHAT_MESSAGE_SAY);
        packet.write_bytes(&writer.into_bytes());
        packet
    }

    #[test]
    fn parses_modern_say() {
        let mut writer = BitWriter::new();
        writer.write_u32(Language::Common as u32);
        writer.write_bits(5, 9);
        writer.write_string_raw("hello");

        assert_eq!(
            read_modern_message(&packet(writer), ChatMsg::Say).unwrap(),
            (None, None, "hello".to_string())
        );
    }

    #[test]
    fn parses_modern_whisper_lengths_before_strings() {
        let mut writer = BitWriter::new();
        writer.write_u32(Language::Common as u32);
        writer.write_bits(4, 9);
        writer.write_bits(2, 9);
        writer.write_string_raw("Mary");
        writer.write_string_raw("hi");

        assert_eq!(
            read_modern_message(&packet(writer), ChatMsg::Whisper).unwrap(),
            (None, Some("Mary".to_string()), "hi".to_string())
        );
    }

    #[test]
    fn parses_modern_channel_after_packed_guid() {
        let channel_guid = ObjectGuid::new_player(42);
        let (high, low) = channel_guid.to_guid128(1);
        let mut writer = BitWriter::new();
        writer.write_u32(Language::Common as u32);
        writer.write_packed_guid_128(high, low);
        writer.write_bits(7, 9);
        writer.write_bits(2, 9);
        writer.write_string_raw("General");
        writer.write_string_raw("hi");

        assert_eq!(
            read_modern_message(&packet(writer), ChatMsg::Channel).unwrap(),
            (Some("General".to_string()), None, "hi".to_string())
        );
    }

    #[test]
    fn parses_modern_emote_without_language() {
        let mut writer = BitWriter::new();
        writer.write_bits(5, 9);
        writer.write_string_raw("waves");

        assert_eq!(
            read_modern_message(&packet(writer), ChatMsg::Emote).unwrap(),
            (None, None, "waves".to_string())
        );
    }
}
