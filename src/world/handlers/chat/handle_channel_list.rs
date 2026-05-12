//! CMSG_CHANNEL_LIST handler

use anyhow::{anyhow, Result};

use crate::shared::game::chat::Team;
use crate::shared::protocol::WorldPacket;
use crate::world::core::session::WorldSession;
use crate::world::World;

/// Handle CMSG_CHANNEL_LIST - player requests channel member list
pub async fn handle_channel_list(
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
    let player_team = Team::from_race(player.race);
    drop(player);

    let channel_name = packet.read_string().unwrap_or_default();

    // System handles sending the channel list
    world
        .systems
        .chat
        .send_channel_list(player_guid, player_team, &channel_name).await?;

    Ok(())
}
