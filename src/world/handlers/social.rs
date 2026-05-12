//! Social system packet handlers

use anyhow::Result;
use tracing::{debug, info};

use crate::world::game::social::types::WhoRequest;
use crate::shared::protocol::{ObjectGuid, WorldPacket};
use crate::shared::game::chat::Team;
use crate::world::core::session::WorldSession;
use crate::world::World;

pub async fn handle_add_friend(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let friend_name = packet.read_string().unwrap_or_default();

    info!(
        "CMSG_ADD_FRIEND: player={:?}, friend_name={}",
        player_guid, friend_name
    );

    world
        .systems
        .social
        .add_friend_by_name(player_guid, friend_name)
        .await?;

    Ok(())
}

pub async fn handle_del_friend(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let friend_guid_low = packet.read_u32().unwrap_or(0);
    let friend_guid = ObjectGuid::new_player(friend_guid_low);

    info!(
        "CMSG_DEL_FRIEND: player={:?}, friend={:?}",
        player_guid, friend_guid
    );

    world
        .systems
        .social
        .remove_friend(player_guid, friend_guid)
        .await?;

    Ok(())
}

pub async fn handle_friend_list(
    session: &WorldSession,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    debug!("CMSG_FRIEND_LIST: player={:?}", player_guid);

    world.systems.social.send_friend_list(player_guid);

    Ok(())
}

pub async fn handle_add_ignore(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let ignore_name = packet.read_string().unwrap_or_default();

    info!(
        "CMSG_ADD_IGNORE: player={:?}, ignore_name={}",
        player_guid, ignore_name
    );

    world
        .systems
        .social
        .add_ignore_by_name(player_guid, ignore_name)
        .await?;

    Ok(())
}

pub async fn handle_del_ignore(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let ignore_guid_low = packet.read_u32().unwrap_or(0);
    let ignore_guid = ObjectGuid::new_player(ignore_guid_low);

    info!(
        "CMSG_DEL_IGNORE: player={:?}, ignore={:?}",
        player_guid, ignore_guid
    );

    world
        .systems
        .social
        .remove_ignore(player_guid, ignore_guid)
        .await?;

    Ok(())
}

pub async fn handle_ignore_list(
    session: &WorldSession,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    debug!("CMSG_IGNORE_LIST: player={:?}", player_guid);

    world.systems.social.send_ignore_list(player_guid);

    Ok(())
}

pub async fn handle_who(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let player = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .ok_or_else(|| anyhow::anyhow!("Player not found"))?;

    let requester_race = player.race;
    let requester_team = Team::from_race(requester_race);
    let requester_security = 0; // TODO: Get actual security level

    let min_level = packet.read_u32().unwrap_or(0);
    let max_level = packet.read_u32().unwrap_or(0xFFFFFFFF);
    let player_name_filter = packet.read_string().unwrap_or_default();
    let guild_name_filter = packet.read_string().unwrap_or_default();
    let race_mask = packet.read_u32().unwrap_or(0xFFFFFFFF);
    let class_mask = packet.read_u32().unwrap_or(0xFFFFFFFF);
    let zones_count = packet.read_u32().unwrap_or(0);
    let mut zone_ids = Vec::new();
    for _ in 0..zones_count {
        zone_ids.push(packet.read_u32().unwrap_or(0));
    }
    let strings_count = packet.read_u32().unwrap_or(0);
    let mut search_strings = Vec::new();
    for _ in 0..strings_count {
        search_strings.push(packet.read_string().unwrap_or_default());
    }

    let request = WhoRequest {
        requester_guid: player_guid,
        requester_team: requester_team.as_u8(),
        requester_security,
        min_level,
        max_level,
        player_name: player_name_filter,
        guild_name: guild_name_filter,
        race_mask,
        class_mask,
        zone_ids,
        search_strings,
    };

    debug!("CMSG_WHO: player={:?}, filters={:?}", player_guid, request);

    world.systems.social.send_who_list(player_guid, request);

    Ok(())
}
