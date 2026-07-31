//! Guild system message structs
//!
//! This module contains type-safe message structures for all guild-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.

use crate::game::guild::{CachedGuild, GuildEmblem, GuildMember, GuildRank, GRF_ONLINE};
use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::ObjectGuid;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;
use chrono::{Datelike, Timelike};
use std::collections::HashMap;

// ========== MODERN ENCODING HELPERS ==========

/// The 1.14 128-bit GUID for a guild.
///
/// 1.12 identifies a guild by a bare id and has no `ObjectGuid` form for it, so the pair is built
/// here rather than converted: guilds are high type 28, realm-qualified the same way a player GUID
/// is, with the guild id as the low half. The client keys the guild frame, the roster and the
/// invite dialog off this GUID, so a wrong high type makes them all refer to different guilds and
/// the UI silently shows nothing.
fn guild_guid_128(guild_id: u32) -> (u64, u64) {
    const GUILD_HIGH_TYPE: u64 = 28;
    let high = GUILD_HIGH_TYPE << 58 | ((DEFAULT_REALM_ID as u64 & 0x1FFF) << 42);
    (high, u64::from(guild_id))
}

/// A unix timestamp in the bit-packed calendar form 1.14 uses for dates.
///
/// Not a timestamp: the client decodes it field by field as
/// `years-since-2000 | month | day-of-month | day-of-week | hour | minute`, all zero-based except
/// the hour and minute. Sending seconds here renders as a date centuries out.
fn packed_time(unix_seconds: i64) -> u32 {
    let time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(unix_seconds.max(0) as u64);
    let datetime = chrono::DateTime::<chrono::Utc>::from(time);
    let year = (datetime.year() - 2000).max(0) as u32;
    (year << 24)
        | ((datetime.month() - 1) << 20)
        | ((datetime.day() - 1) << 14)
        | (datetime.weekday().num_days_from_sunday() << 11)
        | (datetime.hour() << 6)
        | datetime.minute()
}

/// Seconds since the epoch, right now.
fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GUILD_INVITE);
        packet.write_string(self.inviter_name);
        packet.write_string(self.guild_name);
        packet
    }

    /// `GuildInvite` in 1.14.
    ///
    /// Both string lengths move into a bit run at the *front* of the body — 6 bits for the player
    /// name, 7 each for the new and previous guild name — and the strings themselves go last, after
    /// two GUIDs and eleven fixed fields. Writing the names where vanilla put them shifts the whole
    /// body.
    ///
    /// The struct carries names only, so the guild GUID goes out empty: 1.12's invite packet has no
    /// guild id in it, and the invite dialog is driven by the names. The tabard fields are zeroed
    /// for the same reason (the dialog previews the emblem; a zeroed one just draws blank), and the
    /// achievement count is the -1 "unknown" the client expects on a realm with no achievements.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let inviter = self.inviter_name.as_bytes();
        let guild = self.guild_name.as_bytes();
        writer.write_bits(inviter.len() as u32, 6);
        writer.write_bits(guild.len() as u32, 7);
        writer.write_bits(0, 7); // OldGuildName -- 1.12 never names a previous guild
        writer.flush_bits();

        writer.write_u32(0); // InviterVirtualRealmAddress
        writer.write_u32(0); // GuildVirtualRealmAddress
        writer.write_packed_guid_128(0, 0); // GuildGUID -- see above
        writer.write_u32(0); // OldGuildVirtualRealmAddress
        writer.write_packed_guid_128(0, 0); // OldGuildGUID
        writer.write_u32(0); // EmblemStyle
        writer.write_u32(0); // EmblemColor
        writer.write_u32(0); // BorderStyle
        writer.write_u32(0); // BorderColor
        writer.write_u32(0); // BackgroundColor
        writer.write_i32(-1); // AchievementPoints -- "not tracked"

        writer.write_bytes(inviter);
        writer.write_bytes(guild);
        Some(writer.finish(Opcode::SMSG_GUILD_INVITE))
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GUILD_DECLINE);
        packet.write_string(self.player_name);
        packet
    }

    /// `GuildInviteDeclined` in 1.14: a 6-bit name length and an auto-decline bit lead, the realm
    /// follows, and the name itself is written last. Vanilla sends the bare name.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let name = self.player_name.as_bytes();
        writer.write_bits(name.len() as u32, 6);
        // Vanilla cannot distinguish a manual decline from the client's auto-decline setting, and
        // the client only uses the bit to pick the chat wording.
        writer.write_bit(false); // AutoDecline
        writer.flush_bits();
        writer.write_u32(0); // InviterVirtualRealmAddress
        writer.write_bytes(name);
        Some(writer.finish(Opcode::SMSG_GUILD_DECLINE))
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GUILD_COMMAND_RESULT);
        packet.write_u32(self.command);
        packet.write_string(self.target_name);
        packet.write_u32(self.error_code);
        packet
    }

    /// `GuildCommandResult` in 1.14.
    ///
    /// The two u32s **swap**: 1.14 writes the result first and the command second, where vanilla
    /// writes command, name, result. Passing vanilla's order straight through reports every success
    /// as whichever error shares the command's number, which for `GUILD_CREATE_S` (0) means every
    /// failure reads as success. The name moves to the end behind an 8-bit packed length.
    ///
    /// The command and error numbers themselves are unchanged between the two protocols, so they
    /// pass through as-is.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.error_code); // Result -- first, unlike vanilla
        writer.write_u32(self.command); // Command
        let name = self.target_name.as_bytes();
        writer.write_bits(name.len() as u32, 8);
        writer.flush_bits();
        writer.write_bytes(name);
        Some(writer.finish(Opcode::SMSG_GUILD_COMMAND_RESULT))
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
    fn to_vanilla(&self) -> WorldPacket {
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

    /// `QueryGuildInfoResponse` in 1.14.
    ///
    /// Two reshapes matter. Ranks are no longer a fixed ten-slot array padded with empty names —
    /// 1.14 sends a real count and only the ranks that exist, each carrying its id and sort order
    /// alongside the name, so padding to ten here would leave eight blank ranks in the guild
    /// control UI. And the rank rights are gone from this packet entirely; 1.14 delivers them in
    /// `SMSG_GUILD_RANKS`, which has no vanilla trigger, so a modern client sees rank names here
    /// and no permissions until it asks for them.
    ///
    /// Guild id becomes a 128-bit GUID, and the whole body hangs off a `HasGuildInfo` bit — clear
    /// it and the client treats the guild as unknown.
    ///
    /// 1.14.2 dropped the querying player's GUID that the earlier 1.14 builds wrote after the guild
    /// GUID; build 42597 must not send it.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = guild_guid_128(self.guild_id);
        writer.write_packed_guid_128(high, low); // GuildGUID
        writer.write_bit(true); // HasGuildInfo
        writer.flush_bits();

        // 1.12 pads its rank array out to ten with empty names; only the real ones belong here.
        let ranks: Vec<&GuildRank> = self.ranks.iter().filter(|r| !r.name.is_empty()).collect();

        writer.write_packed_guid_128(high, low); // Info.GuildGuid
        writer.write_u32(0); // VirtualRealmAddress
        writer.write_i32(ranks.len() as i32);
        writer.write_u32(self.emblem.style as u32);
        writer.write_u32(self.emblem.color as u32);
        writer.write_u32(self.emblem.border_style as u32);
        writer.write_u32(self.emblem.border_color as u32);
        writer.write_u32(self.emblem.background_color as u32);

        let guild_name = self.guild_name.as_bytes();
        writer.write_bits(guild_name.len() as u32, 7);
        writer.flush_bits();

        for rank in &ranks {
            writer.write_u32(u32::from(rank.id)); // RankID
            // 1.12 rank ids are already the display order, highest authority first.
            writer.write_u32(u32::from(rank.id)); // RankOrder
            let name = rank.name.as_bytes();
            writer.write_bits(name.len() as u32, 7);
            writer.flush_bits();
            writer.write_bytes(name);
        }

        writer.write_bytes(guild_name);
        Some(writer.finish(Opcode::SMSG_GUILD_QUERY_RESPONSE))
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

/// No 1.14 counterpart, so no `to_modern`.
///
/// 1.14 has no standalone guild-info packet: the creation date and account count it carries were
/// folded into the header of `SMSG_GUILD_ROSTER`, and the client reads them only from there. There
/// is nowhere to send this body — the roster encoder supplies both values instead.
impl ToWorldPacket for SmsgGuildInfo<'_> {
    fn to_vanilla(&self) -> WorldPacket {
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

/// Left unported deliberately — this one message is *sixteen* messages in 1.14.
///
/// Vanilla multiplexes every guild event through one opcode carrying an event type and up to three
/// names. 1.14 gave each event its own opcode (`SMSG_GUILD_EVENT_PLAYER_JOINED`,
/// `..._PLAYER_LEFT`, `..._NEW_LEADER`, `..._PRESENCE_CHANGE`, `..._MOTD`, `..._DISBANDED`, and so
/// on) with its own body, and every one of the player-facing bodies wants the subject's **GUID**
/// and realm alongside the name. This struct carries names only, so those GUIDs cannot be
/// recovered here.
///
/// Two further blockers, both of which would produce plausible-looking wrong packets:
///
/// - The event type would have to select the opcode, and the callers in the world crate pass
///   values that do not agree with each other (the same numeric type is commented `GE_REMOVED` in
///   one call site and `GE_DISBANDED` in another). Dispatching on that would send the wrong
///   message, not merely a malformed one.
/// - `SmsgGuildEvent` erases which event it is once built: the variant records only how many name
///   parameters there were, and the rank-change and presence events need parameters this struct
///   never receives.
///
/// Porting this needs the event type to become a real enum and the affected GUIDs to reach the
/// message, both of which are changes to the struct and its callers.
impl ToWorldPacket for SmsgGuildEvent {
    fn to_vanilla(&self) -> WorldPacket {
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
    fn to_vanilla(&self) -> WorldPacket {
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

    /// `GuildRoster` with `GuildRosterMemberData` in 1.14.
    ///
    /// The header gained the account count and creation date that vanilla sent separately in
    /// `SMSG_GUILD_INFO`, and lost the rank-rights array entirely (1.14 carries rights in
    /// `SMSG_GUILD_RANKS`, which has no vanilla trigger). Both text blocks move to the very end
    /// behind 11-bit lengths in the header's bit run, so a client that finds them inline reads the
    /// member list out of the message text.
    ///
    /// Per member the reordering is total: rank, area, achievement points, reputation and the
    /// last-seen float come *before* the fixed bytes, and all three strings move behind a bit run
    /// carrying their lengths at 6/8/8 bits. The offline float that vanilla writes only for logged
    /// out members is unconditional here — omitting it for online members shifts every later field.
    ///
    /// Values 1.12 has no source for are sent as the client's own "unknown" markers rather than
    /// guessed: achievement points and guild reputation as -1, the two profession slots as zeroed
    /// records (their ids come from a client database this server has no equivalent of), and gender
    /// as 0 because the 1.12 roster does not carry it.
    ///
    /// The creation date is the one real loss. 1.12 puts it in `SMSG_GUILD_INFO`, which this
    /// message has no access to, so the roster reports "now" — the guild-info tab will show today's
    /// date. A wrong date is cosmetic; an absent roster is not.
    fn to_modern(&self) -> Option<WorldPacket> {
        let now = now_unix();
        let num_accounts = self
            .members
            .iter()
            .map(|m| m.account_id)
            .collect::<std::collections::HashSet<_>>()
            .len() as u32;

        let mut writer = BitWriter::new();
        writer.write_u32(num_accounts);
        writer.write_u32(packed_time(now)); // CreateDate -- see above
        writer.write_i32(2); // GuildFlags
        writer.write_i32(self.members.len() as i32);

        let motd = self.motd.as_bytes();
        let info = self.info.as_bytes();
        writer.write_bits(motd.len() as u32, 11);
        writer.write_bits(info.len() as u32, 11);
        writer.flush_bits();

        for member in self.members.iter() {
            let is_online = self
                .online_players
                .get(&member.guid)
                .copied()
                .unwrap_or(false);

            let (high, low) = member.guid.to_guid128(DEFAULT_REALM_ID);
            writer.write_packed_guid_128(high, low);
            writer.write_i32(i32::from(member.rank)); // RankID
            writer.write_i32(member.zone as i32); // AreaID
            writer.write_i32(-1); // PersonalAchievementPoints -- not tracked
            writer.write_i32(-1); // GuildReputation -- not tracked

            // Days since this member was last seen. Always present in 1.14; vanilla writes it only
            // for offline members, and an online member is zero days stale by definition.
            let last_save = if is_online || member.logout_time <= 0 {
                0.0
            } else {
                (now - member.logout_time) as f32 / 86400.0
            };
            writer.write_f32(last_save);

            // Two profession slots. 1.12 has no source for the profession database ids, so both
            // are sent empty rather than invented.
            for _ in 0..2 {
                writer.write_i32(0); // DbID
                writer.write_i32(0); // Rank
                writer.write_i32(0); // Step
            }

            writer.write_u32(0); // VirtualRealmAddress
            writer.write_u8(if is_online { GRF_ONLINE } else { 0 }); // Status
            writer.write_u8(member.level);
            writer.write_u8(member.class);
            writer.write_u8(0); // SexID -- absent from the 1.12 roster

            let name = member.name.as_bytes();
            let note = member.public_note.as_bytes();
            let officer_note = member.officer_note.as_bytes();
            writer.write_bits(name.len() as u32, 6);
            writer.write_bits(note.len() as u32, 8);
            writer.write_bits(officer_note.len() as u32, 8);
            writer.write_bit(is_online); // Authenticated
            writer.write_bit(false); // SorEligible
            writer.flush_bits();

            writer.write_bytes(name);
            writer.write_bytes(note);
            writer.write_bytes(officer_note);
        }

        writer.write_bytes(motd);
        writer.write_bytes(info);
        Some(writer.finish(Opcode::SMSG_GUILD_ROSTER))
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
    use crate::game::guild::ERR_GUILD_SUCCESS;
    use crate::protocol::Opcode;
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
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_GUILD_INVITE);
    }

    #[test]
    fn test_smsg_guild_decline() {
        let msg = SmsgGuildDecline { player_name: "Bob" };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_GUILD_DECLINE);
    }

    #[test]
    fn test_smsg_guild_command_result() {
        let msg = SmsgGuildCommandResult {
            command: GUILD_CREATE_S,
            target_name: "TestPlayer",
            error_code: ERR_GUILD_SUCCESS,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_GUILD_COMMAND_RESULT);
    }

    #[test]
    fn test_smsg_guild_query_response() {
        use crate::game::guild::{CachedGuild, Guild, GuildEmblem, GuildRank};

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
        let _packet = msg.to_vanilla();
        // Verify it serializes without panicking
    }

    #[test]
    fn test_smsg_guild_event_from_params() {
        let msg = smsg_guild_event_from_params(GE_JOINED, &["PlayerName"]);
        let _packet = msg.to_vanilla();
        // Verify it serializes without panicking

        let msg2 = smsg_guild_event_from_params(GE_PROMOTION, &["Player", "Promoter"]);
        let _packet2 = msg2.to_vanilla();
        // Verify it serializes without panicking

        let msg3 = smsg_guild_event_from_params(GE_LEADER_CHANGED, &["Old", "New", "Reason"]);
        let _packet3 = msg3.to_vanilla();
        // Verify it serializes without panicking
    }
}
