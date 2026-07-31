//! Social system message structs
//!
//! This module contains type-safe message structures for all social-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgFriendList`] - Complete friend list with status information
//! - [`SmsgFriendStatus`] - Friend status updates (add, remove, online/offline)
//! - [`SmsgIgnoreList`] - Complete ignore list
//! - [`SmsgWho`] - WHO command response with matching players
//! - [`SmsgStandstateUpdate`] - Stand state animation update (sit, stand, kneel, etc.)

use crate::game::social::{FriendInfo, FriendStatus, FriendsResult};
use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::{HighGuid, ObjectGuid, Opcode, WorldPacket};

/// `region << 24 | site << 16 | realm id`, the same scheme the bnet realm list advertises.
const VIRTUAL_REALM_ADDRESS: u32 = 0x0101_0000 | DEFAULT_REALM_ID as u32;

// `SocialFlag`, per the 1.14 wire format. Same bit values as this server's own `SocialFlag`, but
// 1.14 needs them *on the wire*: one contact list serves both relationships, and this word is the
// only thing that says which one a body is.
const SOCIAL_FLAG_FRIEND: u32 = 0x01;
const SOCIAL_FLAG_IGNORED: u32 = 0x02;

/// `ContactInfo::Write`, per the 1.14 wire format.
///
/// Shared by the friend list and the ignore list, which 1.14 serves from one message.
///
/// Every field is unconditional here, where vanilla omits area, level and class for anyone offline.
/// Skipping them for an offline contact would leave the client reading the next contact's GUID out
/// of the middle of this one's numbers.
///
/// The account GUID and the note are sent empty: vanilla has no account identity on the wire, and
/// friend notes arrive two expansions later.
fn write_modern_contact(
    writer: &mut BitWriter,
    guid: ObjectGuid,
    type_flags: u32,
    status: u8,
    area: u32,
    level: u32,
    class: u32,
) {
    let (high, low) = guid.to_guid128(DEFAULT_REALM_ID);
    writer.write_packed_guid_128(high, low);
    writer.write_packed_guid_128(0, 0); // WowAccountGuid
    writer.write_u32(VIRTUAL_REALM_ADDRESS);
    writer.write_u32(VIRTUAL_REALM_ADDRESS); // NativeRealmAddr -- single realm, so the same address
    writer.write_u32(type_flags);
    writer.write_u8(status);
    writer.write_u32(area);
    writer.write_u32(level);
    writer.write_u32(class);
    writer.write_bits(0, 10); // Note length
    writer.write_bit(false); // Mobile -- the companion app postdates this client
    writer.flush_bits();
}

/// SMSG_FRIEND_LIST - Complete friend list with status information
///
/// Sent when player opens their friends list UI or when friends come online/offline.
/// Contains all friends with their current status and location information.
///
/// Note: Names are NOT included in this packet. The client uses its name cache
/// (populated via SMSG_NAME_QUERY_RESPONSE) to display friend names.
#[derive(Debug, Clone)]
pub struct SmsgFriendList<'a> {
    /// Reference to array of friend GUIDs (low 32-bit)
    pub friend_guids: &'a [u32],
    /// Reference to array of friend information (status, area, level, class)
    pub friend_infos: &'a [FriendInfo],
}

impl ToWorldPacket for SmsgFriendList<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_FRIEND_LIST);
        packet.write_u8(self.friend_guids.len() as u8);

        for (i, &friend_guid_low) in self.friend_guids.iter().enumerate() {
            let friend_obj_guid = ObjectGuid::new_without_entry(HighGuid::Player, friend_guid_low);
            packet.write_guid_raw(friend_obj_guid.raw());

            // Get friend info for this friend
            let friend_info = &self.friend_infos[i];
            packet.write_u8(friend_info.status as u8);

            // Only include area/level/class for online friends
            if friend_info.status != FriendStatus::Offline {
                packet.write_u32(friend_info.area);
                packet.write_u32(friend_info.level);
                packet.write_u32(friend_info.class);
            }
        }

        packet
    }

    /// `ContactList::Write`, under a different opcode: 1.14 has **no friend-list message**. Friends
    /// and ignores are one contact list distinguished by a leading flags word, which is why this
    /// finishes as `SMSG_CONTACT_LIST` rather than the opcode the struct is named for.
    ///
    /// `FriendStatus` keeps vanilla's values (offline 0, online 1, AFK 2, DND 4), so the status byte
    /// carries over unchanged.
    ///
    /// The GUIDs and the infos are zipped rather than indexed in parallel: the count is written
    /// before the entries, so a length mismatch between the two slices has to shorten the count too
    /// or the client reads one contact past the end of the body.
    fn to_modern(&self) -> Option<WorldPacket> {
        let count = self.friend_guids.len().min(self.friend_infos.len());

        let mut writer = BitWriter::new();
        writer.write_u32(SOCIAL_FLAG_FRIEND);
        writer.write_bits(count as u32, 8);
        writer.flush_bits();

        for (&guid_low, info) in self
            .friend_guids
            .iter()
            .zip(self.friend_infos.iter())
            .take(count)
        {
            let guid = ObjectGuid::new_without_entry(HighGuid::Player, guid_low);
            write_modern_contact(
                &mut writer,
                guid,
                SOCIAL_FLAG_FRIEND,
                info.status as u8,
                info.area,
                info.level,
                info.class,
            );
        }

        Some(writer.finish(Opcode::SMSG_CONTACT_LIST))
    }
}

/// SMSG_FRIEND_STATUS - Friend status updates
///
/// Sent when friends are added, removed, or change online/offline status.
/// Different result types include different amounts of additional information.
#[derive(Debug, Clone)]
pub struct SmsgFriendStatus {
    /// Result type (AddedOnline, Online, Offline, Removed, etc.)
    pub result: FriendsResult,
    /// GUID of the friend this status update is about
    pub friend_guid: ObjectGuid,
    /// Friend information (only included for online status updates)
    pub friend_info: Option<FriendInfo>,
}

impl ToWorldPacket for SmsgFriendStatus {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_FRIEND_STATUS);
        packet.write_u8(self.result as u8);
        packet.write_guid_raw(self.friend_guid.raw());

        // Add friend info for online status results
        match self.result {
            FriendsResult::AddedOnline | FriendsResult::Online => {
                if let Some(friend_info) = &self.friend_info {
                    packet.write_u8(friend_info.status as u8);
                    packet.write_u32(friend_info.area);
                    packet.write_u32(friend_info.level);
                    packet.write_u32(friend_info.class);
                }
            }
            _ => {}
        }

        packet
    }

    /// `FriendStatusPkt::Write`.
    ///
    /// The result code is **not** renumbered — 1.14 reads the same `FriendsResult` values vanilla
    /// sends (added-online 6, removed 5, and so on) — so it is cast rather than translated.
    ///
    /// The status block is unconditional, where vanilla appends it only for the two online results.
    /// A removal or an offline notification therefore still has to carry a status, area, level and
    /// class; omitting them for those results truncates the body and the client drops the update, so
    /// the friend stays listed as online forever.
    fn to_modern(&self) -> Option<WorldPacket> {
        let info = self.friend_info.as_ref();

        let mut writer = BitWriter::new();
        writer.write_u8(self.result.as_u8());

        let (high, low) = self.friend_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_packed_guid_128(0, 0); // WowAccountGuid -- no account identity in vanilla
        writer.write_u32(VIRTUAL_REALM_ADDRESS);

        writer.write_u8(info.map_or(0, |info| info.status as u8));
        writer.write_u32(info.map_or(0, |info| info.area));
        writer.write_u32(info.map_or(0, |info| info.level));
        writer.write_u32(info.map_or(0, |info| info.class));

        writer.write_bits(0, 10); // Notes length -- friend notes postdate this client
        writer.write_bit(false); // Mobile
        writer.flush_bits();

        Some(writer.finish(Opcode::SMSG_FRIEND_STATUS))
    }
}

/// SMSG_IGNORE_LIST - Complete ignore list
///
/// Sent when player opens their ignore list UI.
/// Contains all players currently being ignored.
#[derive(Debug, Clone)]
pub struct SmsgIgnoreList<'a> {
    /// Reference to array of ignored player GUIDs (low 32-bit)
    pub ignore_guids: &'a [u32],
}

impl ToWorldPacket for SmsgIgnoreList<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_IGNORE_LIST);
        packet.write_u8(self.ignore_guids.len() as u8);

        for &ignore_guid_low in self.ignore_guids {
            let ignore_obj_guid = ObjectGuid::new_without_entry(HighGuid::Player, ignore_guid_low);
            packet.write_guid_raw(ignore_obj_guid.raw());
        }

        packet
    }

    /// `ContactList::Write` again, tagged `Ignored` instead of `Friend` — see
    /// [`SmsgFriendList::to_modern`]. 1.14 has no separate ignore-list message; the flags word is
    /// the *only* thing that stops the client filing these players as friends.
    ///
    /// Each contact still carries the full status/area/level/class block even though an ignore entry
    /// has none of it. Those are the zeros the client expects for someone it is not tracking.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(SOCIAL_FLAG_IGNORED);
        writer.write_bits(self.ignore_guids.len() as u32, 8);
        writer.flush_bits();

        for &ignore_guid_low in self.ignore_guids {
            let guid = ObjectGuid::new_without_entry(HighGuid::Player, ignore_guid_low);
            write_modern_contact(&mut writer, guid, SOCIAL_FLAG_IGNORED, 0, 0, 0, 0);
        }

        Some(writer.finish(Opcode::SMSG_CONTACT_LIST))
    }
}

/// Player information for WHO command response
#[derive(Debug, Clone)]
pub struct WhoPlayerInfo {
    pub name: String,
    pub guild_name: String,
    pub level: u32,
    pub class: u32,
    pub race: u32,
    pub zone: u32,
}

/// SMSG_WHO - WHO command response
///
/// Sent in response to CMSG_WHO with list of online players matching search criteria.
/// Contains matching player information and total online count.
#[derive(Debug)]
pub struct SmsgWho<'a> {
    /// Reference to array of matching players
    pub players: &'a [WhoPlayerInfo],
    /// Total number of players online
    pub total_online: usize,
}

/// Not ported to 1.14: this struct is missing the two things the 1.14 body is built around.
///
/// * **The request id.** 1.14's who request carries an id that the response must echo, and the
///   client matches the answer to the outstanding query by that id alone — a response carrying the
///   wrong one is discarded in full, so there is no partial-credit version of this message. The id
///   arrives on the request and nothing on this struct remembers it.
/// * **A GUID per player.** Each entry embeds the same player-lookup block a name query answers,
///   which is keyed on the player's GUID; [`WhoPlayerInfo`] has a name but no GUID, and the client
///   needs the GUID for the right-click actions (whisper, invite, add friend) that are the point of
///   the who list. Gender is likewise absent.
///
/// Unblocking it needs the request id threaded onto this struct and a GUID (and gender) on
/// [`WhoPlayerInfo`]. Both are available where the message is built; neither is reachable from here.
impl ToWorldPacket for SmsgWho<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_WHO);
        packet.write_u32(self.players.len() as u32);
        packet.write_u32(self.total_online as u32);

        for player in self.players {
            packet.write_cstring(&player.name);
            packet.write_cstring(&player.guild_name);
            packet.write_u32(player.level);
            packet.write_u32(player.class);
            packet.write_u32(player.race);
            packet.write_u32(player.zone);
        }

        packet
    }
}

/// SMSG_STANDSTATE_UPDATE - Stand state animation update
///
/// Sent when player's stand state changes (sit, stand, kneel, sleep, etc.).
/// Updates the player's visual animation state.
///
/// Stand state values:
/// - 0 = Stand
/// - 1 = Sit
/// - 2 = Sit in chair
/// - 3 = Sleep
/// - 4 = Sit in low chair
/// - 5 = Sit in medium chair
/// - 6 = Sit in high chair
/// - 7 = Dead
/// - 8 = Kneel
#[derive(Debug, Clone)]
pub struct SmsgStandstateUpdate {
    /// Stand state value (0-8)
    pub stand_state: u8,
}

impl ToWorldPacket for SmsgStandstateUpdate {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_STANDSTATE_UPDATE);
        packet.write_u8(self.stand_state);
        packet
    }

    /// 1.14 prefixes an AnimKitID. We have no source for one, and 0 means "no override".
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut packet = WorldPacket::new(Opcode::SMSG_STANDSTATE_UPDATE);
        packet.write_u32(0); // AnimKitID
        packet.write_u8(self.stand_state);
        Some(packet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Opcode;
    // Use shared types instead of world types
    use crate::game::social::{FriendInfo, FriendStatus, FriendsResult};

    #[test]
    fn test_smsg_friend_list() {
        let friend_guids = vec![123, 456];
        let friend_infos = vec![
            FriendInfo {
                status: FriendStatus::Online,
                flags: 1,
                area: 1,
                level: 60,
                class: 1,
            },
            FriendInfo {
                status: FriendStatus::Offline,
                flags: 1,
                area: 0,
                level: 0,
                class: 0,
            },
        ];

        let msg = SmsgFriendList {
            friend_guids: &friend_guids,
            friend_infos: &friend_infos,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_FRIEND_LIST);
    }

    #[test]
    fn test_smsg_friend_status_added_online() {
        let friend_info = FriendInfo {
            status: FriendStatus::Online,
            flags: 1,
            area: 1,
            level: 60,
            class: 1,
        };

        let msg = SmsgFriendStatus {
            result: FriendsResult::AddedOnline,
            friend_guid: ObjectGuid::from_low(123),
            friend_info: Some(friend_info),
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_FRIEND_STATUS);
    }

    #[test]
    fn test_smsg_friend_status_offline() {
        let msg = SmsgFriendStatus {
            result: FriendsResult::Offline,
            friend_guid: ObjectGuid::from_low(123),
            friend_info: None,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_FRIEND_STATUS);
    }

    #[test]
    fn test_smsg_ignore_list() {
        let ignore_guids = vec![123, 456, 789];

        let msg = SmsgIgnoreList {
            ignore_guids: &ignore_guids,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_IGNORE_LIST);
    }
}
