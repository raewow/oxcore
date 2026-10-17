//! Group system packet handlers
//!
//! These are thin handlers that parse packets and delegate to GroupSystem.
//! All packet sending is done by GroupSystem via BroadcastManager.

use anyhow::Result;
use tracing::{debug, info, warn};

use crate::core::session::WorldSession;
use crate::game::group::{
    ERR_ALREADY_IN_GROUP_S, ERR_BAD_PLAYER_NAME_S, ERR_GROUP_FULL, ERR_IGNORING_YOU_S,
    ERR_NOT_LEADER, ERR_PARTY_RESULT_OK, ERR_PLAYER_WRONG_FACTION, PARTY_OP_INVITE, PARTY_OP_LEAVE,
};
use crate::core::common::packet::WorldPacketGuidExt;
use crate::World;
use oxcore_shared::protocol::bitbuf::BitReader;
use oxcore_shared::protocol::{ObjectGuid, Protocol, WorldPacket};

/// The name of a member of `player_guid`'s group, for the 1.14 bodies that name members by GUID
/// where 1.12 sends names.
fn member_name(world: &World, player_guid: ObjectGuid, member: ObjectGuid) -> Option<String> {
    world
        .systems
        .group
        .get_player_group(player_guid)?
        .get_member(member)
        .map(|m| m.name.clone())
}

/// Handle CMSG_GROUP_INVITE - player invites someone to group
/// Packet format: playerName (cstring)
pub async fn handle_group_invite(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    // 1.14: u8 party index, 9-bit name and realm lengths, u32 virtual realm, guid128 target,
    // then the two strings. The target is resolved by name on both protocols.
    let target_name = match session.protocol() {
        Protocol::Vanilla => packet.read_string().unwrap_or_default(),
        Protocol::Modern => {
            let mut reader = BitReader::new(packet.contents());
            let _party_index = reader.read_u8();
            let name_len = reader.read_bits(9).unwrap_or(0) as usize;
            let _realm_len = reader.read_bits(9);
            let _virtual_realm = reader.read_u32();
            let _target = reader.read_packed_guid_128();
            reader.read_string(name_len).unwrap_or_default()
        }
    };

    info!(
        "CMSG_GROUP_INVITE: player={:?}, target_name={}",
        player_guid, target_name
    );

    // Delegate to system - it will handle all validation and messaging
    match world
        .systems
        .group
        .invite_player(player_guid, target_name.clone())
        .await
    {
        Ok(()) => {
            // "You have invited X to join your group."
            use oxcore_shared::messages::group::SmsgPartyCommandResult;
            world.managers.broadcast_mgr.send_msg_to_player(
                player_guid,
                SmsgPartyCommandResult {
                    operation: PARTY_OP_INVITE,
                    member_name: &target_name,
                    result: ERR_PARTY_RESULT_OK,
                },
            );
        }
        Err(e) => {
            // Send error message to player
            use crate::game::group::GroupError;
            use oxcore_shared::messages::group::SmsgPartyCommandResult;

            let result_code = match e {
                GroupError::TargetNotFound => ERR_BAD_PLAYER_NAME_S,
                GroupError::TargetAlreadyInGroup | GroupError::TargetHasPendingInvite => {
                    ERR_ALREADY_IN_GROUP_S
                }
                GroupError::GroupFull => ERR_GROUP_FULL,
                GroupError::NotLeaderOrAssistant => ERR_NOT_LEADER,
                GroupError::WrongFaction => ERR_PLAYER_WRONG_FACTION,
                GroupError::TargetIgnoresPlayer => ERR_IGNORING_YOU_S,
                _ => ERR_PARTY_RESULT_OK, // Generic error
            };

            let msg = SmsgPartyCommandResult {
                operation: PARTY_OP_INVITE,
                member_name: &target_name,
                result: result_code,
            };

            world
                .managers
                .broadcast_mgr
                .send_msg_to_player(player_guid, msg);
            warn!("Group invite failed: {:?}", e);
        }
    }

    Ok(())
}

/// Handle CMSG_GROUP_ACCEPT - player accepts group invite
/// Packet format: empty
pub async fn handle_group_accept(session: &WorldSession, world: &World) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    debug!("CMSG_GROUP_ACCEPT: player={:?}", player_guid);

    world
        .systems
        .group
        .accept_invite(world, player_guid)
        .await?;

    Ok(())
}

/// Handle CMSG_GROUP_DECLINE - player declines group invite
/// Packet format: empty
pub async fn handle_group_decline(session: &WorldSession, world: &World) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    debug!("CMSG_GROUP_DECLINE: player={:?}", player_guid);

    world.systems.group.decline_invite(player_guid).await?;

    Ok(())
}

/// Handle CMSG_GROUP_UNINVITE - leader/assistant removes member
/// Packet format: memberName (cstring)
pub async fn handle_group_uninvite(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let member_name = packet.read_string().unwrap_or_default();

    info!(
        "CMSG_GROUP_UNINVITE: player={:?}, member_name={}",
        player_guid, member_name
    );

    world
        .systems
        .group
        .uninvite_player(world, player_guid, member_name)
        .await?;

    Ok(())
}

/// Handle MSG_PARTY_LEAVE - player leaves group
/// Packet format: empty
pub async fn handle_party_leave(session: &WorldSession, world: &World) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    debug!("MSG_PARTY_LEAVE: player={:?}", player_guid);

    // Handle errors gracefully - if not in group, just ignore
    match world.systems.group.leave_group(world, player_guid).await {
        Ok(()) => {
            info!("Player {:?} left group successfully", player_guid);
        }
        Err(e) => {
            warn!("Leave group failed for player {:?}: {:?}", player_guid, e);
            // Don't return error - just log it
            // Player might not be in a group, which is fine
        }
    }

    Ok(())
}

/// Handle CMSG_GROUP_SET_LEADER - leader changes group leader
/// Packet format: newLeaderGuid (u64)
pub async fn handle_group_set_leader(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    // 1.14 leads with a party index byte, then a guid128.
    if session.protocol() == Protocol::Modern {
        let _party_index = packet.read_i8();
    }
    let new_leader_guid = packet
        .read_guid_for(session.protocol())
        .unwrap_or_else(ObjectGuid::empty);

    info!(
        "CMSG_GROUP_SET_LEADER: player={:?}, new_leader={:?}",
        player_guid, new_leader_guid
    );

    world
        .systems
        .group
        .set_leader(world, player_guid, new_leader_guid)
        .await?;

    Ok(())
}

/// Handle CMSG_GROUP_DISBAND - leader disbands group
/// Packet format: empty
pub async fn handle_group_disband(session: &WorldSession, world: &World) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    info!("CMSG_GROUP_DISBAND: player={:?}", player_guid);

    // For disband, we just have the leader leave - this will trigger disbanding if they're the last member
    world.systems.group.leave_group(world, player_guid).await?;

    Ok(())
}

/// Handle CMSG_GROUP_RAID_CONVERT - convert party to raid
/// Packet format: empty
pub async fn handle_group_raid_convert(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    info!("CMSG_GROUP_RAID_CONVERT: player={:?}", player_guid);

    // 1.14 carries one bit: raid (convert) or party (convert back, which is not supported).
    if session.protocol() == Protocol::Modern {
        let to_raid = BitReader::new(packet.contents()).read_bit().unwrap_or(true);
        if !to_raid {
            return Ok(());
        }
    }

    world.systems.group.convert_to_raid(player_guid).await?;

    Ok(())
}

/// Handle CMSG_GROUP_CHANGE_SUB_GROUP - change member's subgroup
/// Packet format: memberName (cstring), subgroup (u8)
pub async fn handle_group_change_sub_group(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    // 1.14 names the member by guid128, then a party index and the subgroup.
    let (member_name, subgroup) = match session.protocol() {
        Protocol::Vanilla => (
            packet.read_string().unwrap_or_default(),
            packet.read_u8().unwrap_or(0),
        ),
        Protocol::Modern => {
            let member = packet
                .read_guid_for(session.protocol())
                .unwrap_or_else(ObjectGuid::empty);
            let _party_index = packet.read_i8();
            let subgroup = packet.read_u8().unwrap_or(0);
            (
                member_name(world, player_guid, member).unwrap_or_default(),
                subgroup,
            )
        }
    };

    info!(
        "CMSG_GROUP_CHANGE_SUB_GROUP: player={:?}, member={}, subgroup={}",
        player_guid, member_name, subgroup
    );

    world
        .systems
        .group
        .change_subgroup(player_guid, member_name, subgroup)
        .await?;

    Ok(())
}

/// Handle CMSG_GROUP_SWAP_SUB_GROUP - swap two members' subgroups
/// Packet format: memberName1 (cstring), memberName2 (cstring)
pub async fn handle_group_swap_sub_group(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    // 1.14: party index, then both members as guid128.
    let (member1_name, member2_name) = match session.protocol() {
        Protocol::Vanilla => (
            packet.read_string().unwrap_or_default(),
            packet.read_string().unwrap_or_default(),
        ),
        Protocol::Modern => {
            let _party_index = packet.read_i8();
            let first = packet
                .read_guid_for(session.protocol())
                .unwrap_or_else(ObjectGuid::empty);
            let second = packet
                .read_guid_for(session.protocol())
                .unwrap_or_else(ObjectGuid::empty);
            (
                member_name(world, player_guid, first).unwrap_or_default(),
                member_name(world, player_guid, second).unwrap_or_default(),
            )
        }
    };

    info!(
        "CMSG_GROUP_SWAP_SUB_GROUP: player={:?}, member1={}, member2={}",
        player_guid, member1_name, member2_name
    );

    world
        .systems
        .group
        .swap_subgroups(player_guid, member1_name, member2_name)
        .await?;

    Ok(())
}

/// Handle CMSG_GROUP_ASSISTANT_LEADER - set/unset assistant
/// Packet format: memberGuid (u64), enable (u8)
pub async fn handle_group_assistant_leader(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    // 1.14: party index, guid128, then the flag as a single bit.
    let (member_guid, enable) = match session.protocol() {
        Protocol::Vanilla => (
            ObjectGuid::from_raw(packet.read_u64().unwrap_or(0)),
            packet.read_u8().unwrap_or(0) != 0,
        ),
        Protocol::Modern => {
            let mut reader = BitReader::new(packet.contents());
            let _party_index = reader.read_u8();
            let member = reader
                .read_packed_guid_128()
                .map(|(high, low)| ObjectGuid::from_guid128(high, low))
                .unwrap_or_else(ObjectGuid::empty);
            (member, reader.read_bit().unwrap_or(false))
        }
    };

    info!(
        "CMSG_GROUP_ASSISTANT_LEADER: player={:?}, member={:?}, enable={}",
        player_guid, member_guid, enable
    );

    world
        .systems
        .group
        .set_assistant(player_guid, member_guid, enable)
        .await?;

    Ok(())
}

/// Handle CMSG_SET_LOOT_METHOD - change loot method
/// Packet format: lootMethod (u32), masterLooterGuid (u64), lootThreshold (u32)
pub async fn handle_set_loot_method(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    // 1.12: u32 lootMethod, u64 masterLooterGuid, u32 lootThreshold.
    // 1.14: i8 party index, u8 lootMethod, guid128 master looter, u32 lootThreshold.
    let (loot_method_raw, master_looter_guid, loot_threshold_u32) = match session.protocol() {
        Protocol::Vanilla => (
            packet.read_u32().unwrap_or(0),
            ObjectGuid::from_raw(packet.read_u64().unwrap_or(0)),
            packet.read_u32().unwrap_or(2),
        ),
        Protocol::Modern => {
            let _party_index = packet.read_i8();
            let method = u32::from(packet.read_u8().unwrap_or(0));
            let master = packet
                .read_guid_for(session.protocol())
                .unwrap_or_else(ObjectGuid::empty);
            (method, master, packet.read_u32().unwrap_or(2))
        }
    };
    // Methods run 0-4; anything else is not a loot method.
    if loot_method_raw > 4 {
        return Ok(());
    }
    let loot_method = crate::game::group::LootMethod::from(loot_method_raw as u8);
    let loot_threshold = loot_threshold_u32 as u8;

    info!(
        "CMSG_SET_LOOT_METHOD: player={:?}, method={:?}, master={:?}, threshold={}",
        player_guid, loot_method, master_looter_guid, loot_threshold
    );

    world
        .systems
        .group
        .set_loot_method(player_guid, loot_method, loot_threshold, master_looter_guid)
        .await?;

    Ok(())
}

/// Handle MSG_RAID_READY_CHECK - initiate or respond to ready check
/// Packet format: (initiate) empty, (respond) isReady (u8)
pub async fn handle_raid_ready_check(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    // If packet has data, it's a response
    if let Some(ready_byte) = packet.read_u8() {
        let is_ready = ready_byte != 0;
        debug!(
            "MSG_RAID_READY_CHECK (response): player={:?}, ready={}",
            player_guid, is_ready
        );

        world
            .systems
            .group
            .respond_ready_check(player_guid, is_ready)
            .await?;
    } else {
        // Initiate ready check
        debug!("MSG_RAID_READY_CHECK (initiate): player={:?}", player_guid);

        world
            .systems
            .group
            .initiate_ready_check(player_guid)
            .await?;
    }

    Ok(())
}

/// Handle MSG_RAID_TARGET_UPDATE - set or clear target icon
/// Packet format: mode (u8), targetGuid (u64), icon (u8)
pub async fn handle_raid_target_update(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    // u8 icon, then the target guid — unless the icon is 0xFF, which asks for the full list.
    let icon = packet.read_u8().unwrap_or(0xFF);

    if icon != 0xFF {
        // Set target icon
        let target_guid = ObjectGuid::from_raw(packet.read_u64().unwrap_or(0));

        debug!(
            "MSG_RAID_TARGET_UPDATE (set): player={:?}, target={:?}, icon={}",
            player_guid, target_guid, icon
        );

        world
            .systems
            .group
            .set_target_icon(player_guid, icon, target_guid)
            .await?;
    } else {
        // Request current icons (mode == 1)
        debug!("MSG_RAID_TARGET_UPDATE (request): player={:?}", player_guid);

        world.systems.group.send_target_icons(player_guid);
    }

    Ok(())
}

/// Handle CMSG_REQUEST_RAID_INFO - client requests raid lockout info
/// Sent when player opens character screen or joins a raid group
/// Packet format: empty
pub async fn handle_request_raid_info(session: &WorldSession, _world: &World) -> Result<()> {
    use oxcore_shared::protocol::Opcode;

    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    debug!("CMSG_REQUEST_RAID_INFO: player={:?}", player_guid);

    // Send empty raid instance info (no lockouts)
    // Packet format: u32 count, then for each: map_id (u32), reset_time (u32), instance_id (u32)
    let mut packet = oxcore_shared::protocol::WorldPacket::new(Opcode::SMSG_RAID_INSTANCE_INFO);
    packet.write_u32(0); // No raid lockouts

    session.send_packet(packet);

    Ok(())
}

/// Handle CMSG_REQUEST_PARTY_MEMBER_STATS - client requests stats for a group member
/// Sent when viewing raid frames or party UI
/// Packet format: memberGuid (u64)
pub async fn handle_request_party_member_stats(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    // 1.14 leads with a party index byte, then a guid128.
    if session.protocol() == Protocol::Modern {
        let _party_index = packet.read_u8();
    }
    let target_guid = packet
        .read_guid_for(session.protocol())
        .unwrap_or_else(ObjectGuid::empty);

    debug!(
        "CMSG_REQUEST_PARTY_MEMBER_STATS: player={:?}, target={:?}",
        player_guid, target_guid
    );

    // The system validates the group membership and sends the full stats reply.
    world
        .systems
        .group
        .request_member_stats(player_guid, target_guid)
        .await?;

    Ok(())
}

/// Handle MSG_RANDOM_ROLL - `/random`
/// Packet format: min (u32), max (u32)
pub async fn handle_random_roll(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
    let min = packet.read_u32().unwrap_or(0);
    let max = packet.read_u32().unwrap_or(0);

    world
        .systems
        .group
        .random_roll(player_guid, min, max)
        .await?;
    Ok(())
}

/// Handle MSG_MINIMAP_PING - ping the minimap for the rest of the group
/// Packet format: x (f32), y (f32)
pub async fn handle_minimap_ping(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
    let x = packet.read_f32().unwrap_or(0.0);
    let y = packet.read_f32().unwrap_or(0.0);

    // Not being in a group is not an error for a ping; there is simply nobody to tell.
    let _ = world.systems.group.send_minimap_ping(player_guid, x, y);
    Ok(())
}
