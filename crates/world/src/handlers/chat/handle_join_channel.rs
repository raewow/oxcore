//! CMSG_JOIN_CHANNEL handler

use anyhow::{Result, anyhow};

use crate::World;
use crate::core::session::WorldSession;
use oxcore_shared::game::chat::Team;
use oxcore_shared::protocol::WorldPacket;

/// Handle CMSG_JOIN_CHANNEL - player joins a chat channel
pub async fn handle_join_channel(
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
    let password = packet.read_string().unwrap_or_default();
    let password_opt = if password.is_empty() {
        None
    } else {
        Some(password.as_str())
    };

    // System handles everything including error notifications
    world
        .systems
        .chat
        .join_channel(player_guid, &channel_name, password_opt, player_team)
        .await?;

    Ok(())
}
