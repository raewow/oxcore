//! GM Ticket packet handlers
//!
//! Thin handlers that parse packets and delegate to TicketSystem.

use anyhow::{anyhow, Result};

use crate::shared::game::ticket::GmTicketType;
use crate::shared::protocol::WorldPacket;
use crate::world::core::session::WorldSession;
use crate::world::World;

/// CMSG_GMTICKET_GETTICKET - Player queries their active ticket
pub async fn handle_gmticket_getticket(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;
    world.systems.ticket.send_ticket(player_guid);
    Ok(())
}

/// CMSG_GMTICKET_CREATE - Player creates a new ticket
pub async fn handle_gmticket_create(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    // Parse packet
    let ticket_type_val = packet.read_u8().unwrap_or(1); // Default to Stuck
    let map_id = packet.read_u32().unwrap_or(0) as u16;
    let x = packet.read_f32().unwrap_or(0.0);
    let y = packet.read_f32().unwrap_or(0.0);
    let z = packet.read_f32().unwrap_or(0.0);
    let message = packet.read_cstring().unwrap_or_default();
    let _reserved = packet.read_cstring().unwrap_or_default(); // Usually empty

    // Get player name from PlayerManager
    let player_name = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .map(|p| p.name.clone())
        .ok_or_else(|| anyhow!("Player not found"))?;

    // Convert ticket type
    let ticket_type = match ticket_type_val {
        1 => GmTicketType::Stuck,
        2 => GmTicketType::BehaviorHarassment,
        3 => GmTicketType::Guild,
        4 => GmTicketType::Item,
        5 => GmTicketType::Environmental,
        6 => GmTicketType::NonquestCreep,
        7 => GmTicketType::QuestQuestnpc,
        8 => GmTicketType::Technical,
        9 => GmTicketType::AccountBilling,
        10 => GmTicketType::Character,
        _ => GmTicketType::Stuck, // Default
    };

    // Delegate to system
    let result = world
        .systems
        .ticket
        .create_ticket(player_guid, player_name, ticket_type, map_id, (x, y, z), message)
        .await?;

    // Send response
    world
        .systems
        .ticket
        .send_create_response(player_guid, result)
        ;
    Ok(())
}

/// CMSG_GMTICKET_UPDATETEXT - Player updates their ticket text
pub async fn handle_gmticket_updatetext(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    // Parse packet
    let _ticket_type = packet.read_u8().unwrap_or(0); // Not used for update
    let message = packet.read_cstring().unwrap_or_default();

    // Delegate to system
    let result = world
        .systems
        .ticket
        .update_ticket_text(player_guid, message)
        .await?;

    // Send response
    world
        .systems
        .ticket
        .send_update_response(player_guid, result)
        ;
    Ok(())
}

/// CMSG_GMTICKET_DELETETICKET - Player abandons/deletes their ticket
pub async fn handle_gmticket_deleteticket(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    // Delegate to system
    let result = world.systems.ticket.delete_ticket(player_guid).await?;

    // Send response
    world
        .systems
        .ticket
        .send_delete_response(player_guid, result)
        ;
    Ok(())
}

/// CMSG_GMTICKET_SYSTEMSTATUS - Player queries if ticket system is enabled
pub async fn handle_gmticket_systemstatus(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;
    world.systems.ticket.send_system_status(player_guid);
    Ok(())
}
