//! Guild system message structs
//!
//! This module contains type-safe message structures for all guild-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;
use crate::shared::protocol::guid::ObjectGuid;
use crate::world::game::guild::types::{
    CachedGuild, GuildEmblem, GuildMember, GuildRank, GRF_ONLINE,
};
use chrono::Datelike;
use std::collections::HashMap;

// ========== SIMPLE MESSAGES ==========

/// SMSG_GUILD_INVITE - Guild invitation notification
///
/// Sent to the invitee when someone invites them to a guild.
#[derive(Debug, Clone)]
pub struct SmsgGuildInvite<'a> {
    /// Name of the player who sent the invitation
    pub inviter_name: &'a str,
    /// Name of the guild
    pub guild_name: &'a str,
}

impl ToWorldPacket for SmsgGuildInvite<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GUILD_INVITE);
        packet.write_string(self.inviter_name);
        packet.write_string(self.guild_name);
        packet
    }
}

/// SMSG_GUILD_DECLINE - Guild invitation declined
///
/// Sent to the inviter when the invitee declines the guild invitation.
#[derive(Debug, Clone)]
pub struct SmsgGuildDecline<'a> {
    /// Name of the player who declined
    pub player_name: &'a str,
}

impl ToWorldPacket for SmsgGuildDecline<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GUILD_DECLINE);
        packet.write_string(self.player_name);
        packet
    }
}

/// SMSG_GUILD_COMMAND_RESULT - Result of a guild command
///
/// Sent after guild operations (create, invite, promote, etc.) to indicate success or error.
#[derive(Debug, Clone)]
pub struct SmsgGuildCommandResult<'a> {
    /// Command type (GUILD_CREATE_S, GUILD_INVITE_S, etc.)
    pub command: u32,
    /// Target player name (or empty string)
    pub target_name: &'a str,
    /// Error code (ERR_GUILD_SUCCESS for success, other constants for errors)
    pub error_code: u32,
}

impl ToWorldPacket for SmsgGuildCommandResult<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GUILD_COMMAND_RESULT);
        packet.write_u32(self.command);
        packet.write_string(self.target_name);
        packet.write_u32(self.error_code);
        packet
    }
}

// ========== MEDIUM COMPLEXITY MESSAGES ==========

/// SMSG_GUILD_QUERY_RESPONSE - Guild information query response
///
/// Sent in response to guild queries, provides guild name, ranks, and emblem.
#[derive(Debug, Clone)]
pub struct SmsgGuildQueryResponse<'a> {
    pub guild_id: u32,
    pub guild_name: &'a str,
    pub ranks: &'a [GuildRank],
    pub emblem: &'a GuildEmblem,
}

impl ToWorldPacket for SmsgGuildQueryResponse<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GUILD_QUERY_RESPONSE);

        packet.write_u32(self.guild_id);
        packet.write_string(self.guild_name);

        // Write ranks (up to 10, pad with empty)
        for i in 0..10 {
            if i < self.ranks.len() {
                packet.write_string(&self.ranks[i].name);
                packet.write_u32(self.ranks[i].rights);
            } else {
                packet.write_string("");
                packet.write_u32(0);
            }
        }

        // Write emblem
        packet.write_u32(self.emblem.style as u32);
        packet.write_u32(self.emblem.color as u32);
        packet.write_u32(self.emblem.border_style as u32);
        packet.write_u32(self.emblem.border_color as u32);
        packet.write_u32(self.emblem.background_color as u32);

        packet
    }
}

/// SMSG_GUILD_INFO - Guild information summary
///
/// Sent when player requests guild info, shows creation date and member counts.
#[derive(Debug, Clone)]
pub struct SmsgGuildInfo<'a> {
    pub guild_name: &'a str,
    pub create_date: i64, // Unix timestamp
    pub member_count: u32,
    pub account_count: u32,
}

impl ToWorldPacket for SmsgGuildInfo<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GUILD_INFO);

        packet.write_string(self.guild_name);

        // Parse create_date to day/month/year
        let create_time =
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(self.create_date as u64);
        let datetime = chrono::DateTime::<chrono::Utc>::from(create_time);
        let date = datetime.date_naive();

        packet.write_u32(date.day());
        packet.write_u32(date.month());
        packet.write_u32(date.year() as u32);
        packet.write_u32(self.member_count);
        packet.write_u32(self.account_count);

        packet
    }
}

/// SMSG_GUILD_EVENT - Guild event notification
///
/// Sent to guild members when events occur (member joined, promoted, etc.).
/// Different event types require different parameters.
#[derive(Debug, Clone)]
pub enum SmsgGuildEvent {
    /// Single parameter event (e.g., member joined)
    SingleParam { event_type: u8, param1: String },
    /// Two parameter event (e.g., member promoted by someone)
    TwoParam {
        event_type: u8,
        param1: String,
        param2: String,
    },
    /// Three parameter event
    ThreeParam {
        event_type: u8,
        param1: String,
        param2: String,
        param3: String,
    },
}

impl ToWorldPacket for SmsgGuildEvent {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GUILD_EVENT);

        match self {
            Self::SingleParam { event_type, param1 } => {
                packet.write_u8(*event_type);
                packet.write_u8(1); // param count
                packet.write_string(param1);
            }
            Self::TwoParam {
                event_type,
                param1,
                param2,
            } => {
                packet.write_u8(*event_type);
                packet.write_u8(2); // param count
                packet.write_string(param1);
                packet.write_string(param2);
            }
            Self::ThreeParam {
                event_type,
                param1,
                param2,
                param3,
            } => {
                packet.write_u8(*event_type);
                packet.write_u8(3); // param count
                packet.write_string(param1);
                packet.write_string(param2);
                packet.write_string(param3);
            }
        }

        packet
    }
}

// ========== COMPLEX MESSAGES ==========

/// SMSG_GUILD_ROSTER - Complete guild roster
///
/// Sent when player opens guild roster, contains all members with their status.
#[derive(Debug)]
pub struct SmsgGuildRoster<'a> {
    pub motd: &'a str,
    pub info: &'a str,
    pub ranks: &'a [GuildRank],
    pub members: &'a [GuildMember],
    pub online_players: &'a HashMap<ObjectGuid, bool>,
}

impl ToWorldPacket for SmsgGuildRoster<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GUILD_ROSTER);

        packet.write_u32(self.members.len() as u32);
        packet.write_string(self.motd);
        packet.write_string(self.info);

        // Write rank rights
        packet.write_u32(self.ranks.len() as u32);
        for rank in self.ranks.iter() {
            packet.write_u32(rank.rights);
        }

        // Write members
        for member in self.members.iter() {
            packet.write_guid_raw(member.guid.raw());

            // Online status flags
            let is_online = self
                .online_players
                .get(&member.guid)
                .copied()
                .unwrap_or(false);
            let status = if is_online { GRF_ONLINE } else { 0 };
            packet.write_u8(status);

            packet.write_string(&member.name);
            packet.write_u32(member.rank as u32);
            packet.write_u8(member.level);
            packet.write_u8(member.class);
            packet.write_u32(member.zone);

            // Only send logout time if offline
            if !is_online {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                let days_since_logout = if member.logout_time > 0 {
                    (now - member.logout_time) as f32 / 86400.0
                } else {
                    0.0
                };
                packet.write_f32(days_since_logout);
            }

            packet.write_string(&member.public_note);
            packet.write_string(&member.officer_note);
        }

        packet
    }
}

// ========== HELPER FUNCTIONS ==========

/// Helper to create SmsgGuildEvent from a slice of parameters
///
/// This mirrors the interface of the old `build_guild_event` function for easy migration.
pub fn smsg_guild_event_from_params(event_type: u8, params: &[&str]) -> SmsgGuildEvent {
    match params.len() {
        1 => SmsgGuildEvent::SingleParam {
            event_type,
            param1: params[0].to_string(),
        },
        2 => SmsgGuildEvent::TwoParam {
            event_type,
            param1: params[0].to_string(),
            param2: params[1].to_string(),
        },
        3 => SmsgGuildEvent::ThreeParam {
            event_type,
            param1: params[0].to_string(),
            param2: params[1].to_string(),
            param3: params[2].to_string(),
        },
        _ => SmsgGuildEvent::SingleParam {
            event_type,
            param1: String::new(),
        },
    }
}

/// Helper to create SmsgGuildQueryResponse from CachedGuild
///
/// Simplifies construction when you have a CachedGuild reference.
pub fn smsg_guild_query_response_from_cached<'a>(
    guild: &'a CachedGuild,
) -> SmsgGuildQueryResponse<'a> {
    SmsgGuildQueryResponse {
        guild_id: guild.guild.id,
        guild_name: &guild.guild.name,
        ranks: &guild.ranks,
        emblem: &guild.guild.emblem,
    }
}

/// Helper to create SmsgGuildRoster from CachedGuild
///
/// Simplifies construction when you have a CachedGuild reference.
pub fn smsg_guild_roster_from_cached<'a>(
    guild: &'a CachedGuild,
    online_players: &'a HashMap<ObjectGuid, bool>,
) -> SmsgGuildRoster<'a> {
    SmsgGuildRoster {
        motd: &guild.guild.motd,
        info: &guild.guild.info,
        ranks: &guild.ranks,
        members: &guild.members,
        online_players,
    }
}

/// Helper to create SmsgGuildInfo from CachedGuild
///
/// Simplifies construction when you have a CachedGuild reference.
pub fn smsg_guild_info_from_cached<'a>(guild: &'a CachedGuild) -> SmsgGuildInfo<'a> {
    SmsgGuildInfo {
        guild_name: &guild.guild.name,
        create_date: guild.guild.create_date,
        member_count: guild.members.len() as u32,
        account_count: guild
            .members
            .iter()
            .map(|m| m.account_id)
            .collect::<std::collections::HashSet<_>>()
            .len() as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::Opcode;
    use crate::world::game::guild::types::ERR_GUILD_SUCCESS;
    const GUILD_CREATE_S: u32 = 0;
    const GE_JOINED: u8 = 3;
    const GE_PROMOTION: u8 = 6;
    const GE_LEADER_CHANGED: u8 = 9;

    #[test]
    fn test_smsg_guild_invite() {
        let msg = SmsgGuildInvite {
            inviter_name: "Alice",
            guild_name: "TestGuild",
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_GUILD_INVITE);
    }

    #[test]
    fn test_smsg_guild_decline() {
        let msg = SmsgGuildDecline { player_name: "Bob" };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_GUILD_DECLINE);
    }

    #[test]
    fn test_smsg_guild_command_result() {
        let msg = SmsgGuildCommandResult {
            command: GUILD_CREATE_S,
            target_name: "TestPlayer",
            error_code: ERR_GUILD_SUCCESS,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_GUILD_COMMAND_RESULT);
    }

    #[test]
    fn test_smsg_guild_query_response() {
        use crate::world::game::guild::types::{CachedGuild, Guild, GuildEmblem, GuildRank};

        let guild = Guild {
            id: 1,
            name: "TestGuild".to_string(),
            leader_guid: ObjectGuid::empty(),
            leader_name: "Leader".to_string(),
            emblem: GuildEmblem::default(),
            info: String::new(),
            motd: String::new(),
            create_date: 0,
        };

        let ranks = vec![GuildRank {
            id: 0,
            name: "Guild Master".to_string(),
            rights: 0x000FF1FF,
        }];

        let cached = CachedGuild {
            guild,
            ranks,
            members: vec![],
        };

        let msg = smsg_guild_query_response_from_cached(&cached);
        let _packet = msg.to_world_packet();
        // Verify it serializes without panicking
    }

    #[test]
    fn test_smsg_guild_event_from_params() {
        let msg = smsg_guild_event_from_params(GE_JOINED, &["PlayerName"]);
        let _packet = msg.to_world_packet();
        // Verify it serializes without panicking

        let msg2 = smsg_guild_event_from_params(GE_PROMOTION, &["Player", "Promoter"]);
        let _packet2 = msg2.to_world_packet();
        // Verify it serializes without panicking

        let msg3 = smsg_guild_event_from_params(GE_LEADER_CHANGED, &["Old", "New", "Reason"]);
        let _packet3 = msg3.to_world_packet();
        // Verify it serializes without panicking
    }
}
