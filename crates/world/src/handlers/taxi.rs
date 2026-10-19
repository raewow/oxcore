//! Flight master handlers: node status, the flight map, and taking off.

use anyhow::Result;

use crate::core::common::packet::WorldPacketGuidExt;
use crate::core::session::WorldSession;
use crate::game::taxi::flight;
use crate::World;
use oxcore_shared::protocol::{Protocol, WorldPacket};

/// CMSG_TAXINODE_STATUS_QUERY: is the flight point at this NPC known?
pub fn handle_taxinode_status_query(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let (Some(player_guid), Some(npc)) = (
        session.player_guid(),
        packet.read_guid_for(session.protocol()),
    ) else {
        return Ok(());
    };
    flight::send_taxi_status(world, player_guid, npc);
    Ok(())
}

/// CMSG_TAXIQUERYAVAILABLENODES: talk to a flight master.
pub fn handle_taxi_query_available_nodes(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let (Some(player_guid), Some(npc)) = (
        session.player_guid(),
        packet.read_guid_for(session.protocol()),
    ) else {
        return Ok(());
    };
    flight::talk_to_flight_master(world, player_guid, npc);
    Ok(())
}

/// CMSG_ACTIVATETAXI. 1.12 names the source and destination nodes; 1.14
/// names only the destination and leaves the route to the server.
pub async fn handle_activate_taxi(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let (Some(player_guid), Some(npc)) = (
        session.player_guid(),
        packet.read_guid_for(session.protocol()),
    ) else {
        return Ok(());
    };
    match session.protocol() {
        Protocol::Vanilla => {
            let (Some(from), Some(to)) = (packet.read_u32(), packet.read_u32()) else {
                return Ok(());
            };
            flight::activate(world, player_guid, &[from, to], Some(npc), 0).await?;
        }
        Protocol::Modern => {
            let Some(to) = packet.read_u32() else {
                return Ok(());
            };
            flight::activate_to(world, player_guid, npc, to).await?;
        }
    }
    Ok(())
}

/// CMSG_ACTIVATETAXIEXPRESS (1.12): a multi-hop route the client worked out.
pub async fn handle_activate_taxi_express(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    /// More nodes than any journey crosses: a malformed packet.
    const MAX_NODES: u32 = 64;

    let (Some(player_guid), Some(npc)) = (
        session.player_guid(),
        packet.read_guid_for(session.protocol()),
    ) else {
        return Ok(());
    };
    let (Some(_total_cost), Some(count)) = (packet.read_u32(), packet.read_u32()) else {
        return Ok(());
    };
    if count == 0 || count > MAX_NODES {
        return Ok(());
    }
    let mut nodes = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let Some(node) = packet.read_u32() else {
            return Ok(());
        };
        nodes.push(node);
    }
    flight::activate(world, player_guid, &nodes, Some(npc), 0).await?;
    Ok(())
}
