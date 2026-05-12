//! Group system packet handlers
//!
//! These are thin handlers that parse packets and delegate to GroupSystem.
//! All packet sending is done by GroupSystem via BroadcastManager.

use anyhow::Result;
use tracing::{debug, info, warn};

use crate::shared::protocol::{ObjectGuid, WorldPacket};
use crate::world::core::session::WorldSession;
use crate::world::game::group::{
    ERR_ALREADY_IN_GROUP_S, ERR_BAD_PLAYER_NAME_S, ERR_GROUP_FULL, ERR_IGNORING_YOU_S,
    ERR_NOT_LEADER, ERR_PARTY_RESULT_OK, ERR_PLAYER_WRONG_FACTION, PARTY_OP_INVITE, PARTY_OP_LEAVE,
};
use crate::world::World;

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

    let target_name = packet.read_string().unwrap_or_default();

    info!(
        "CMSG_GROUP_INVITE: player={:?}, target_name={}",
        player_guid, target_name
    );

    // Delegate to system - it will handle all validation and messaging
    match world
        .systems
        .group
        .invite_player(player_guid, target_name.clone()).await

    {
        Ok(()) => {},
        Err(e) => {
            // Send error message to player
            use crate::shared::messages::group::SmsgPartyCommandResult;
            use crate::world::game::group::GroupError;

            let result_code = match e {
                GroupError::TargetNotFound => ERR_BAD_PLAYER_NAME_S,
                GroupError::TargetAlreadyInGroup => ERR_ALREADY_IN_GROUP_S,
                GroupError::GroupFull => ERR_GROUP_FULL,
                GroupError::NotLeaderOrAssistant => ERR_NOT_LEADER,
                _ => ERR_PARTY_RESULT_OK, // Generic error
            };

            let msg = SmsgPartyCommandResult {
                operation: PARTY_OP_INVITE,
                member_name: &target_name,
                result: result_code,
            };

            world.managers.broadcast_mgr.send_msg_to_player(player_guid, msg);
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

    world.systems.group.accept_invite(player_guid).await?;

    Ok(())
}

/// Handle CMSG_GROUP_DECLINE - player declines group invite
/// Packet format: empty
pub async fn handle_group_decline(session: &WorldSession, world: &World) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    debug!("CMSG_GROUP_DECLINE: player={:?}", player_guid);

    world.systems.group.decline_invite(player_guid);

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
        .uninvite_player(player_guid, member_name)
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
    match world.systems.group.leave_group(player_guid).await {
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

    let new_leader_guid = ObjectGuid::from_raw(packet.read_u64().unwrap_or(0));

    info!(
        "CMSG_GROUP_SET_LEADER: player={:?}, new_leader={:?}",
        player_guid, new_leader_guid
    );

    world
        .systems
        .group
        .set_leader(player_guid, new_leader_guid)
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
    world.systems.group.leave_group(player_guid).await?;

    Ok(())
}

/// Handle CMSG_GROUP_RAID_CONVERT - convert party to raid
/// Packet format: empty
pub async fn handle_group_raid_convert(session: &WorldSession, world: &World) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    info!("CMSG_GROUP_RAID_CONVERT: player={:?}", player_guid);

    world
        .systems
        .group
        .convert_to_raid(player_guid)
        .await?;

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

    let member_name = packet.read_string().unwrap_or_default();
    let subgroup = packet.read_u8().unwrap_or(0);

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

    let member1_name = packet.read_string().unwrap_or_default();
    let member2_name = packet.read_string().unwrap_or_default();

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

    let member_guid = ObjectGuid::from_raw(packet.read_u64().unwrap_or(0));
    let enable = packet.read_u8().unwrap_or(0) != 0;

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

    // Client sends: u32 lootMethod, u64 masterLooterGuid, u32 lootThreshold
    let loot_method_u32 = packet.read_u32().unwrap_or(0);
    let loot_method = crate::world::game::group::LootMethod::from(loot_method_u32 as u8);
    let master_looter_guid = ObjectGuid::from_raw(packet.read_u64().unwrap_or(0));
    let loot_threshold_u32 = packet.read_u32().unwrap_or(2);
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
        debug!(
            "MSG_RAID_READY_CHECK (initiate): player={:?}",
            player_guid
        );

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

    let mode = packet.read_u8().unwrap_or(0);

    if mode == 0 {
        // Set target icon
        let target_guid = ObjectGuid::from_raw(packet.read_u64().unwrap_or(0));
        let icon = packet.read_u8().unwrap_or(0);

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
        debug!(
            "MSG_RAID_TARGET_UPDATE (request): player={:?}",
            player_guid
        );

        world.systems.group.send_target_icons(player_guid);
    }

    Ok(())
}

/// Handle CMSG_REQUEST_RAID_INFO - client requests raid lockout info
/// Sent when player opens character screen or joins a raid group
/// Packet format: empty
pub async fn handle_request_raid_info(
    session: &WorldSession,
    _world: &World,
) -> Result<()> {
    use crate::shared::protocol::Opcode;

    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    debug!(
        "CMSG_REQUEST_RAID_INFO: player={:?}",
        player_guid
    );

    // Send empty raid instance info (no lockouts)
    // Packet format: u32 count, then for each: map_id (u32), reset_time (u32), instance_id (u32)
    let mut packet = crate::shared::protocol::WorldPacket::new(Opcode::SMSG_RAID_INSTANCE_INFO);
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
    use crate::shared::protocol::Opcode;
    use crate::world::game::common::group_update_flags;

    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let target_guid = ObjectGuid::from_raw(packet.read_u64().unwrap_or(0));

    debug!(
        "CMSG_REQUEST_PARTY_MEMBER_STATS: player={:?}, target={:?}",
        player_guid, target_guid
    );

    // Check if target is in the same group and online
    let target_player = world.managers.player_mgr.get_player(target_guid);

    let mut response = crate::shared::protocol::WorldPacket::new(Opcode::SMSG_PARTY_MEMBER_STATS_FULL);

    // Write packed GUID (for 1.12.1)
    response.write_packed_guid_raw(target_guid.raw());

    if let Some(player) = target_player {
        // Player is online - send full stats
        // For now, use hardcoded values since stats system isn't integrated
        let update_flags = group_update_flags::STATUS
            | group_update_flags::CUR_HP
            | group_update_flags::MAX_HP
            | group_update_flags::POWER_TYPE
            | group_update_flags::CUR_POWER
            | group_update_flags::MAX_POWER
            | group_update_flags::LEVEL
            | group_update_flags::ZONE;

        response.write_u32(update_flags);

        // STATUS - online
        response.write_u8(0x01); // MEMBER_STATUS_ONLINE

        // Use placeholder health values (level * 50 as rough estimate)
        let max_health: u16 = (player.level as u16) * 50;
        let cur_health: u16 = max_health; // Full health

        // CUR_HP
        response.write_u16(cur_health);

        // MAX_HP
        response.write_u16(max_health);

        // POWER_TYPE based on class (0=mana, 1=rage, 3=energy)
        // Vanilla class IDs: 1=Warrior, 2=Paladin, 3=Hunter, 4=Rogue, 5=Priest,
        //                    7=Shaman, 8=Mage, 9=Warlock, 11=Druid
        let power_type: u8 = match player.class {
            1 => 1,  // Warrior - Rage
            4 => 3,  // Rogue - Energy
            _ => 0,  // Everyone else - Mana
        };
        response.write_u8(power_type);

        // Use placeholder power values
        let (max_power, cur_power): (u16, u16) = match power_type {
            1 => (100, 0),    // Rage: max 100, starts at 0
            3 => (100, 100),  // Energy: max 100, full
            _ => ((player.level as u16) * 50, (player.level as u16) * 50), // Mana: scales with level
        };

        // CUR_POWER
        response.write_u16(cur_power);

        // MAX_POWER
        response.write_u16(max_power);

        // LEVEL
        response.write_u16(player.level as u16);

        // ZONE
        response.write_u16(player.zone_id.min(65535) as u16);
    } else {
        // Player is offline - send just status flag
        response.write_u32(group_update_flags::STATUS);
        response.write_u8(0x00); // MEMBER_STATUS_OFFLINE
    }

    session.send_packet(response);

    Ok(())
}
