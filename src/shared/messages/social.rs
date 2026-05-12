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

use crate::shared::game::social::{FriendInfo, FriendStatus, FriendsResult};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{HighGuid, ObjectGuid, Opcode, WorldPacket};

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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_IGNORE_LIST);
        packet.write_u8(self.ignore_guids.len() as u8);

        for &ignore_guid_low in self.ignore_guids {
            let ignore_obj_guid = ObjectGuid::new_without_entry(HighGuid::Player, ignore_guid_low);
            packet.write_guid_raw(ignore_obj_guid.raw());
        }

        packet
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

impl ToWorldPacket for SmsgWho<'_> {
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_STANDSTATE_UPDATE);
        packet.write_u8(self.stand_state);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::Opcode;
    // Use shared types instead of world types
    use crate::shared::game::social::{FriendInfo, FriendStatus, FriendsResult};

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
        let packet = msg.to_world_packet();
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
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_FRIEND_STATUS);
    }

    #[test]
    fn test_smsg_friend_status_offline() {
        let msg = SmsgFriendStatus {
            result: FriendsResult::Offline,
            friend_guid: ObjectGuid::from_low(123),
            friend_info: None,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_FRIEND_STATUS);
    }

    #[test]
    fn test_smsg_ignore_list() {
        let ignore_guids = vec![123, 456, 789];

        let msg = SmsgIgnoreList {
            ignore_guids: &ignore_guids,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_IGNORE_LIST);
    }
}
