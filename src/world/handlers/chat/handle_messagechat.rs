//! CMSG_MESSAGECHAT handler - main chat message dispatcher
//!
//! This handler is extremely thin - it only parses the packet and delegates
//! to ChatSystem. All business logic, validation, faction filtering, distance
//! checks, and packet sending happens in the system.

use anyhow::{anyhow, Result};

use crate::shared::common::AccountType;
use crate::shared::game::chat::{ChatMsg, ChatTag, Language, Team};
use crate::shared::messages::chat::SmsgMessageChat;
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, WorldPacket};
use crate::world::core::session::WorldSession;
use crate::world::game::chat::commands::ChatCommandContext;
use crate::world::World;

/// Handle CMSG_MESSAGECHAT - player sends a chat message
pub async fn handle_messagechat(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
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

    // Parse packet - CMSG_MESSAGECHAT format:
    // - type: u32 (NOT u8!)
    // - language: u32 (ignored - system determines language)
    // - For Whisper: target_name (cstring), message (cstring)
    // - For Channel: channel_name (cstring), message (cstring)
    // - For Say/Yell/Party/etc: message (cstring)
    let msg_type_u32 = packet.read_u32().unwrap_or(0);
    let _language = packet.read_u32().unwrap_or(0); // Ignored - system determines language

    let msg_type = ChatMsg::from_u32(msg_type_u32).unwrap_or(ChatMsg::Say);

    // Read type-specific fields
    let (channel_name_opt, target_name_opt, message) = match msg_type {
        ChatMsg::Channel => {
            let channel = packet.read_string().unwrap_or_default();
            let msg = packet.read_string().unwrap_or_default();
            (Some(channel), None, msg)
        }
        ChatMsg::Whisper => {
            let target = packet.read_string().unwrap_or_default();
            let msg = packet.read_string().unwrap_or_default();
            (None, Some(target), msg)
        }
        _ => {
            // Say, Yell, Party, Guild, etc - just message
            let msg = packet.read_string().unwrap_or_default();
            (None, None, msg)
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
                .send_to_player(player_guid, packet.to_world_packet())
                ;
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
                .send_to_player(player_guid, packet.to_world_packet())
                ;
        }
    }

    Ok(())
}
