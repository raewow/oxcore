//! Reputation packet handlers for world
//!
//! These handlers are slim wrappers that parse packets and delegate
//! to the ReputationSystem. No business logic lives here.

use crate::shared::protocol::{Opcode, WorldPacket};
use crate::world::core::session::WorldSession;
use crate::world::World;
use anyhow::Result;
use tracing::{debug, warn};

/// Handle CMSG_SET_FACTION_ATWAR (0x0125)
///
/// Sent when the player right-clicks a faction in the reputation panel
/// to toggle at-war status.
///
/// Wire format:
///   u32 reputation_list_id
///   u8  at_war (0 or 1)
pub async fn handle_set_faction_atwar(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let reputation_list_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read reputation_list_id"))?;
    let at_war = packet
        .read_u8()
        .ok_or_else(|| anyhow::anyhow!("Failed to read at_war"))?
        != 0;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_SET_FACTION_ATWAR received but player not logged in");
            return Ok(());
        }
    };

    debug!(
        "CMSG_SET_FACTION_ATWAR: rep_list_id={}, at_war={}",
        reputation_list_id, at_war
    );

    world
        .systems
        .reputation
        .set_at_war(player_guid, reputation_list_id, at_war, world)?;

    Ok(())
}

/// Handle CMSG_SET_FACTION_INACTIVE (0x0317)
///
/// Sent when the player marks a faction as inactive (collapsed) in
/// the reputation panel.
///
/// Wire format:
///   u32 reputation_list_id
///   u8  inactive (0 or 1)
pub async fn handle_set_faction_inactive(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let reputation_list_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read reputation_list_id"))?;
    let inactive = packet
        .read_u8()
        .ok_or_else(|| anyhow::anyhow!("Failed to read inactive"))?
        != 0;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_SET_FACTION_INACTIVE received but player not logged in");
            return Ok(());
        }
    };

    debug!(
        "CMSG_SET_FACTION_INACTIVE: rep_list_id={}, inactive={}",
        reputation_list_id, inactive
    );

    world
        .systems
        .reputation
        .set_inactive(player_guid, reputation_list_id, inactive, world)?;

    Ok(())
}
