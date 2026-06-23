use anyhow::{anyhow, Result};

use crate::core::session::WorldSession;
use crate::World;
use oxcore_shared::game::chat::Team;
use oxcore_shared::protocol::WorldPacket;

pub async fn handle_channel_kick(
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
    let target_name = packet.read_string().unwrap_or_default();

    world.systems.chat.kick_player_by_name(
        player_team,
        &channel_name,
        player_guid,
        &target_name,
    )?;

    Ok(())
}
