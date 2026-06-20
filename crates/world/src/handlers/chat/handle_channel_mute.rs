use anyhow::{Result, anyhow};

use crate::World;
use crate::core::session::WorldSession;
use oxcore_shared::game::chat::Team;
use oxcore_shared::protocol::WorldPacket;

pub async fn handle_channel_mute(
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

    let target_guid = world
        .managers
        .player_mgr
        .find_player_by_name(&target_name)
        .ok_or_else(|| anyhow!("Target not found"))?;

    world
        .systems
        .chat
        .set_muted(player_team, &channel_name, target_guid, true)?;

    Ok(())
}
