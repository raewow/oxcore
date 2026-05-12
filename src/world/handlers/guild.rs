//! Guild system packet handlers

use anyhow::{anyhow, Result};
use tracing::{debug, info};

use crate::shared::messages::guild::SmsgGuildDecline;
use crate::shared::protocol::{Opcode, ObjectGuid, WorldPacket};
use crate::world::core::session::WorldSession;
use crate::world::World;

pub async fn handle_guild_query(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let guild_id_low = packet.read_u32().unwrap_or(0);
    let guild_id = guild_id_low & 0x00FFFFFF; // Mask to 24 bits

    debug!("CMSG_GUILD_QUERY: player={:?}, guild_id={}", player_guid, guild_id);

    world
        .systems
        .guild
        .query_guild(player_guid, guild_id)?;

    Ok(())
}

pub async fn handle_guild_create(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let player = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .ok_or_else(|| anyhow!("Player not found"))?;
    let player_name = player.name.clone();
    drop(player);

    let guild_name = packet.read_string().unwrap_or_default();

    info!(
        "CMSG_GUILD_CREATE: player={:?} ({}), guild_name={}",
        player_guid, player_name, guild_name
    );

    world
        .systems
        .guild
        .create_guild_from_petition(player_guid, player_name, guild_name)
        .await?;

    Ok(())
}

pub async fn handle_guild_invite(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let inviter_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let inviter_state = world
        .systems
        .guild
        .get_player_guild(inviter_guid)
        .ok_or_else(|| anyhow!("Not in a guild"))?;

    let guild_id = inviter_state
        .guild_id
        .ok_or_else(|| anyhow!("Not in a guild"))?;

    let guild_data = world
        .systems
        .guild
        .get_guild(guild_id)
        .ok_or_else(|| anyhow!("Guild not found"))?;

    let guild_name = guild_data.info.name.clone();

    let invitee_name = packet.read_string().unwrap_or_default();

    info!(
        "CMSG_GUILD_INVITE: inviter={:?}, invitee_name={}, guild_name={}",
        inviter_guid, invitee_name, guild_name
    );

    world
        .systems
        .guild
        .invite_player(inviter_guid, invitee_name, guild_name)
        .await?;

    Ok(())
}

pub async fn handle_guild_accept(
    session: &WorldSession,
    world: &World,
) -> Result<()> {
    let invitee_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    // Get pending invite
    let (_inviter_guid, guild_id, guild_name) = world.systems.guild
        .remove_pending_invite(invitee_guid)
        .ok_or_else(|| anyhow!("No pending guild invite"))?;

    // Get invitee name
    let invitee_name = world.managers.player_mgr
        .get_player_name(invitee_guid)
        .ok_or_else(|| anyhow!("Player not found"))?;

    debug!("CMSG_GUILD_ACCEPT: {} accepting invite to {}", invitee_name, guild_name);

    // Join the guild
    world.systems.guild.join_guild(guild_id, invitee_guid, invitee_name).await?;

    Ok(())
}

pub async fn handle_guild_decline(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let invitee_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    // Get pending invite
    let (inviter_guid, _guild_id, _guild_name) = match world.systems.guild
        .remove_pending_invite(invitee_guid) {
        Some(invite) => invite,
        None => return Ok(()), // No invite, silently ignore
    };

    // Get invitee name
    let invitee_name = world.managers.player_mgr
        .get_player_name(invitee_guid)
        .unwrap_or_else(|| "Unknown".to_string());

    debug!("CMSG_GUILD_DECLINE: {} declined invite", invitee_name);

    // Notify inviter (need to use an empty string since SmsgGuildDecline requires 'static lifetime)
    world.managers.broadcast_mgr.send_msg_to_player(
        inviter_guid,
        SmsgGuildDecline {
            player_name: "",
        },
    );

    Ok(())
}

pub async fn handle_guild_roster(
    session: &WorldSession,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    debug!("CMSG_GUILD_ROSTER: player={:?}", player_guid);

    let guild_state = world
        .systems
        .guild
        .get_player_guild(player_guid)
        .ok_or_else(|| anyhow!("Not in a guild"))?;

    if let Some(guild_id) = guild_state.guild_id {
        world.systems.guild.send_guild_roster_to_player(player_guid, guild_id)?;
    }

    Ok(())
}

pub async fn handle_guild_leave(
    session: &WorldSession,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let player = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .ok_or_else(|| anyhow!("Player not found"))?;
    let player_name = player.name.clone();
    drop(player);

    info!(
        "CMSG_GUILD_LEAVE: player={:?} ({})",
        player_guid, player_name
    );

    world
        .systems
        .guild
        .leave_guild(player_guid, player_name)
        .await?;

    Ok(())
}

pub async fn handle_guild_remove(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let remover_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let target_name = packet.read_string().unwrap_or_default();

    // Find target
    let target_guid = world.managers.player_mgr.find_player_by_name(&target_name)
        .ok_or_else(|| anyhow!("Player '{}' not found", target_name))?;

    info!(
        "CMSG_GUILD_REMOVE: remover={:?}, target={} ({:?})",
        remover_guid, target_name, target_guid
    );

    // Delegate to system
    world.systems.guild
        .remove_member(remover_guid, target_guid, target_name)
        .await?;

    Ok(())
}

pub async fn handle_guild_promote(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let promoter_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let target_name = packet.read_string().unwrap_or_default();

    info!(
        "CMSG_GUILD_PROMOTE: promoter={:?}, target_name={}",
        promoter_guid, target_name
    );

    world
        .systems
        .guild
        .promote_member(promoter_guid, target_name)
        .await?;

    Ok(())
}

pub async fn handle_guild_demote(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let demoter_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let target_name = packet.read_string().unwrap_or_default();

    info!(
        "CMSG_GUILD_DEMOTE: demoter={:?}, target_name={}",
        demoter_guid, target_name
    );

    world
        .systems
        .guild
        .demote_member(demoter_guid, target_name)
        .await?;

    Ok(())
}

pub async fn handle_guild_disband(
    session: &WorldSession,
    world: &World,
) -> Result<()> {
    let leader_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    info!("CMSG_GUILD_DISBAND: leader={:?}", leader_guid);

    // Delegate to system
    world.systems.guild.disband_guild(leader_guid).await?;

    Ok(())
}

pub async fn handle_guild_info(
    session: &WorldSession,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    debug!("CMSG_GUILD_INFO: player={:?}", player_guid);

    // Guild info is sent via guild query response
    // For now, trigger a re-send of the query response
    let guild_state = world
        .systems
        .guild
        .get_player_guild(player_guid)
        .ok_or_else(|| anyhow!("Not in a guild"))?;

    if let Some(guild_id) = guild_state.guild_id {
        world
            .systems
            .guild
            .query_guild(player_guid, guild_id)?;
    }

    Ok(())
}
