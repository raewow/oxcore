//! CMSG_LEAVE_CHANNEL handler

use anyhow::{anyhow, Result};

use crate::shared::game::chat::Team;
use crate::shared::protocol::WorldPacket;
use crate::world::core::session::WorldSession;
use crate::world::World;

/// Handle CMSG_LEAVE_CHANNEL - player leaves a chat channel
pub async fn handle_leave_channel(
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

    world
        .systems
        .chat
        .leave_channel(player_guid, &channel_name, player_team)
        .await?;

    Ok(())
}
