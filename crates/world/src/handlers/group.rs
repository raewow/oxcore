//! Group system packet handlers
//!
//! These are thin handlers that parse packets and delegate to GroupSystem.
//! All packet sending is done by GroupSystem via BroadcastManager.

use anyhow::Result;
use tracing::{debug, info, warn};

use crate::core::session::WorldSession;
use crate::game::group::{
    ERR_ALREADY_IN_GROUP_S, ERR_BAD_PLAYER_NAME_S, ERR_GROUP_FULL, ERR_IGNORING_YOU_S,
    ERR_NOT_LEADER, ERR_PARTY_RESULT_OK, ERR_PLAYER_WRONG_FACTION, ERR_TARGET_NOT_IN_GROUP_S,
    PARTY_OP_INVITE, PARTY_OP_LEAVE,
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

    let result = world
        .systems
        .group
        .uninvite_player(world, player_guid, member_name.clone())
        .await;
    send_uninvite_result(world, player_guid, &member_name, result);

    Ok(())
}

/// Tell a player why their kick failed ("X is not in your party", "You are not the leader").
fn send_uninvite_result(
    world: &World,
    player_guid: ObjectGuid,
    member_name: &str,
    result: std::result::Result<(), crate::game::group::GroupError>,
) {
    use crate::game::group::GroupError;
    use oxcore_shared::messages::group::SmsgPartyCommandResult;

    let code = match result {
        Ok(()) => return,
        Err(GroupError::NotLeader | GroupError::NotLeaderOrAssistant) => ERR_NOT_LEADER,
        Err(GroupError::NotInGroup | GroupError::MemberNotFound) => ERR_TARGET_NOT_IN_GROUP_S,
        Err(e) => {
            warn!("Uninvite failed for {:?}: {:?}", player_guid, e);
            return;
        }
    };
    world.managers.broadcast_mgr.send_msg_to_player(
        player_guid,
        SmsgPartyCommandResult {
            operation: PARTY_OP_LEAVE,
            member_name,
            result: code,
        },
    );
}

/// Handle CMSG_GROUP_UNINVITE_GUID (1.14: CMSG_PARTY_UNINVITE) - kick a member, or withdraw an
/// invite, by GUID.
/// 1.12: u64 guid. 1.14: u8 party index, guid128 target, 8-bit reason length, reason.
pub async fn handle_group_uninvite_guid(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let target_guid = match session.protocol() {
        Protocol::Vanilla => packet.read_guid_for(session.protocol()),
        Protocol::Modern => {
            let mut reader = BitReader::new(packet.contents());
            let _party_index = reader.read_u8();
            reader
                .read_packed_guid_128()
                .map(|(high, low)| ObjectGuid::from_guid128(high, low))
        }
    }
    .unwrap_or_else(ObjectGuid::empty);

    debug!(
        "CMSG_GROUP_UNINVITE_GUID: player={:?}, target={:?}",
        player_guid, target_guid
    );

    let result = world
        .systems
        .group
        .uninvite_by_guid(world, player_guid, target_guid)
        .await;
    send_uninvite_result(world, player_guid, "", result);
    Ok(())
}

/// Handle CMSG_PARTY_INVITE_RESPONSE (1.14 only) - accept or decline in one packet.
/// u8 party index, bit accept, bit has-roles, [u32 roles].
pub async fn handle_party_invite_response(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let mut reader = BitReader::new(packet.contents());
    let _party_index = reader.read_u8();
    let accept = reader.read_bit().unwrap_or(false);

    if accept {
        handle_group_accept(session, world).await
    } else {
        handle_group_decline(session, world).await
    }
}

/// Handle CMSG_DO_READY_CHECK (1.14 only; 1.12 sends an empty MSG_RAID_READY_CHECK).
/// i8 party index.
pub async fn handle_do_ready_check(session: &WorldSession, world: &World) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
    world
        .systems
        .group
        .initiate_ready_check(player_guid)
        .await?;
    Ok(())
}

/// Handle CMSG_READY_CHECK_RESPONSE (1.14 only). u8 party index, bit ready.
pub async fn handle_ready_check_response(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
    let mut reader = BitReader::new(packet.contents());
    let _party_index = reader.read_u8();
    let ready = reader.read_bit().unwrap_or(false);
    world
        .systems
        .group
        .respond_ready_check(player_guid, ready)
        .await?;
    Ok(())
}

/// Handle CMSG_UPDATE_RAID_TARGET (1.14 only). i8 party index, guid128 target, i8 symbol.
///
/// A negative symbol clears whatever icon the target carries.
pub async fn handle_update_raid_target(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
    let _party_index = packet.read_i8();
    let target_guid = packet
        .read_guid_for(session.protocol())
        .unwrap_or_else(ObjectGuid::empty);
    let symbol = packet.read_i8().unwrap_or(-1);

    let group = &world.systems.group;
    let icon = if symbol >= 0 {
        Some((symbol as u8, target_guid))
    } else {
        // Clear: find the icon this target carries and empty it.
        group
            .get_player_group(player_guid)
            .and_then(|g| g.target_icons.iter().position(|t| *t == target_guid))
            .map(|index| (index as u8, ObjectGuid::empty()))
    };
    if let Some((icon, target)) = icon {
        group.set_target_icon(player_guid, icon, target).await?;
    }
    Ok(())
}

/// Handle CMSG_MINIMAP_PING (1.14 only). f32 x, f32 y, i8 party index.
pub async fn handle_modern_minimap_ping(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
    let x = packet.read_f32().unwrap_or(0.0);
    let y = packet.read_f32().unwrap_or(0.0);
    let _ = world.systems.group.send_minimap_ping(player_guid, x, y);
    Ok(())
}

/// Handle CMSG_RANDOM_ROLL (1.14 only). i32 min, i32 max, u8 party index.
pub async fn handle_modern_random_roll(
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

/// Handle CMSG_SET_EVERYONE_IS_ASSISTANT (1.14 only). u8 party index, bit apply.
pub async fn handle_set_everyone_is_assistant(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
    let mut reader = BitReader::new(packet.contents());
    let _party_index = reader.read_u8();
    let apply = reader.read_bit().unwrap_or(false);
    world
        .systems
        .group
        .set_everyone_assistant(player_guid, apply)
        .await?;
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
pub async fn handle_request_raid_info(session: &WorldSession, world: &World) -> Result<()> {
    use oxcore_shared::messages::instance::{RaidLock, SmsgRaidInstanceInfo};

    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    debug!("CMSG_REQUEST_RAID_INFO: player={:?}", player_guid);

    let instance_mgr = &world.managers.instance_mgr;
    instance_mgr
        .ensure_player_bindings(&world.databases, player_guid)
        .await?;
    let locks = instance_mgr
        .raid_info(player_guid)
        .into_iter()
        .map(|(map_id, time_left, instance_id)| RaidLock {
            map_id,
            instance_id,
            time_left: time_left.min(u64::from(u32::MAX)) as u32,
        })
        .collect();
    session.send_msg(SmsgRaidInstanceInfo { locks })?;

    Ok(())
}

/// Handle CMSG_RESET_INSTANCES - the "Reset all instances" button.
///
/// A group leader resets the group's normal dungeon saves; a player outside a
/// group resets their own. Raids and permanently saved copies are never reset,
/// and a copy with players inside reports a failure.
pub async fn handle_reset_instances(session: &WorldSession, world: &World) -> Result<()> {
    use crate::game::instance::lifecycle;
    use crate::game::instance::manager::ResetScope;

    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
    debug!("CMSG_RESET_INSTANCES: player={:?}", player_guid);

    let players_in_instance = |map_id: u32, instance_id: u32| {
        world
            .managers
            .map_mgr
            .get_map(map_id, instance_id)
            .map(|map| map.player_guids())
            .unwrap_or_default()
    };
    let is_raid_map =
        |map_id: u32| lifecycle::map_entry(world, map_id).is_some_and(|entry| entry.is_raid());
    let instance_mgr = &world.managers.instance_mgr;

    let outcome = match world.systems.group.get_player_group(player_guid) {
        Some(group) => {
            if group.leader_guid != player_guid {
                return Ok(());
            }
            let members_offline = group.members.iter().any(|member| {
                world
                    .session_mgr
                    .get_session_by_player(member.guid)
                    .is_none()
            });
            instance_mgr
                .reset_group_instances(
                    &world.databases,
                    group.leader_guid,
                    ResetScope::All,
                    members_offline,
                    is_raid_map,
                    players_in_instance,
                )
                .await
        }
        None => {
            let current_map = world
                .managers
                .player_mgr
                .with_player(player_guid, |player| player.map_id);
            instance_mgr
                .reset_player_instances(
                    &world.databases,
                    player_guid,
                    current_map,
                    ResetScope::All,
                    is_raid_map,
                    players_in_instance,
                )
                .await
        }
    };

    lifecycle::apply_reset_outcome(world, &outcome);
    lifecycle::send_reset_replies(world, player_guid, &outcome);
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
