//! Battleground and PvP handlers.
//!
//! Vanilla names a battleground by its map; 1.14 by its `BattlemasterList`
//! id, and answers invitations with the ride ticket (queue slot + 1) the
//! status packets gave it.

use anyhow::Result;

use crate::core::common::packet::WorldPacketGuidExt;
use crate::core::session::WorldSession;
use crate::game::battleground::system;
use crate::World;
use oxcore_shared::game::battleground::BattleGroundTypeId;
use oxcore_shared::protocol::{Protocol, WorldPacket};

/// Mask removing the queue-type bits from a 1.14 queue id.
const MODERN_QUEUE_ID_MASK: u64 = !0x1F10_0000_0000_0000;

/// The first bit of the next byte (1.14 bit fields start at the high bit).
fn read_bit(packet: &mut WorldPacket) -> Option<bool> {
    packet.read_u8().map(|b| b & 0x80 != 0)
}

fn bg_from_map(packet: &mut WorldPacket) -> Option<BattleGroundTypeId> {
    packet
        .read_u32()
        .map(BattleGroundTypeId::from_map_id)
        .filter(|bg| *bg != BattleGroundTypeId::None)
}

/// CMSG_BATTLEMASTER_HELLO: talk to a battlemaster.
pub fn handle_battlemaster_hello(session: &WorldSession, packet: &mut WorldPacket, world: &World) -> Result<()> {
    let (Some(player), Some(npc)) = (session.player_guid(), packet.read_guid_for(session.protocol())) else {
        return Ok(());
    };
    system::battlemaster_hello(world, player, npc);
    Ok(())
}

/// CMSG_BATTLEFIELD_LIST: the copies of a battleground (asked from the PvP tab).
pub fn handle_battlefield_list(session: &WorldSession, packet: &mut WorldPacket, world: &World) -> Result<()> {
    let Some(player) = session.player_guid() else {
        return Ok(());
    };
    let bg_type = match session.protocol() {
        Protocol::Vanilla => bg_from_map(packet),
        Protocol::Modern => packet.read_u32().map(BattleGroundTypeId::from_u32),
    };
    if let Some(bg_type) = bg_type.filter(|bg| *bg != BattleGroundTypeId::None) {
        system::send_list(world, player, player, bg_type);
    }
    Ok(())
}

/// CMSG_BATTLEMASTER_JOIN: join a queue from a battlemaster or a portal.
pub fn handle_battlemaster_join(session: &WorldSession, packet: &mut WorldPacket, world: &World) -> Result<()> {
    let Some(player) = session.player_guid() else {
        return Ok(());
    };
    let parsed = match session.protocol() {
        Protocol::Vanilla => (|| {
            let battlemaster = packet.read_guid()?;
            let bg = BattleGroundTypeId::from_map_id(packet.read_u32()?);
            let instance = packet.read_u32()?;
            let as_group = packet.read_u8()? != 0;
            Some((battlemaster, bg, instance, as_group))
        })(),
        Protocol::Modern => (|| {
            let queue_id = packet.read_u64()?;
            let _roles = packet.read_u8()?;
            let _blacklist = (packet.read_u32()?, packet.read_u32()?);
            let battlemaster = packet.read_guid_for(Protocol::Modern)?;
            let _verification = packet.read_u32()?;
            let instance = packet.read_u32()?;
            let as_group = read_bit(packet).unwrap_or(false);
            let bg = BattleGroundTypeId::from_u32((queue_id & MODERN_QUEUE_ID_MASK) as u32);
            Some((battlemaster, bg, instance, as_group))
        })(),
    };
    if let Some((battlemaster, bg_type, instance, as_group)) = parsed {
        system::request_join(world, player, battlemaster, bg_type, instance, as_group);
    }
    Ok(())
}

/// CMSG_BATTLEFIELD_JOIN (vanilla, from before battlemasters had a list): join
/// at an entrance portal.
pub fn handle_battlefield_join(session: &WorldSession, packet: &mut WorldPacket, world: &World) -> Result<()> {
    let (Some(player), Some(bg_type)) = (session.player_guid(), bg_from_map(packet)) else {
        return Ok(());
    };
    system::request_join(world, player, player, bg_type, 0, false);
    Ok(())
}

/// CMSG_BATTLEFIELD_PORT: accept an invitation or leave a queue.
pub async fn handle_battlefield_port(session: &WorldSession, packet: &mut WorldPacket, world: &World) -> Result<()> {
    let Some(player) = session.player_guid() else {
        return Ok(());
    };
    let parsed = match session.protocol() {
        Protocol::Vanilla => (|| {
            let bg = bg_from_map(packet)?;
            let accept = packet.read_u8()? == 1;
            Some((bg, accept))
        })(),
        Protocol::Modern => (|| {
            let _requester = packet.read_guid_for(Protocol::Modern)?;
            let ticket_id = packet.read_u32()?;
            let _ticket_type = packet.read_u32()?;
            let _ticket_time = packet.read_u64()?;
            let accept = read_bit(packet)?;
            let bg = system::bg_type_of_ticket(world, player, ticket_id)?;
            Some((bg, accept))
        })(),
    };
    if let Some((bg_type, accept)) = parsed {
        system::port(world, player, bg_type, accept).await;
    }
    Ok(())
}

/// CMSG_LEAVE_BATTLEFIELD: leave the battleground (vanilla names its map).
pub async fn handle_leave_battlefield(session: &WorldSession, packet: &mut WorldPacket, world: &World) -> Result<()> {
    let Some(player) = session.player_guid() else {
        return Ok(());
    };
    if session.protocol() == Protocol::Vanilla {
        let current = world.managers.player_mgr.with_player(player, |p| p.map_id);
        if packet.read_u32() != current {
            return Ok(());
        }
    }
    system::request_leave(world, player).await;
    Ok(())
}

/// CMSG_BATTLEFIELD_STATUS / CMSG_REQUEST_BATTLEFIELD_STATUS: every queue slot.
pub fn handle_battlefield_status(session: &WorldSession, world: &World) -> Result<()> {
    if let Some(player) = session.player_guid() {
        system::send_all_statuses(world, player);
    }
    Ok(())
}

/// MSG_PVP_LOG_DATA / CMSG_PVP_LOG_DATA: the scoreboard.
pub fn handle_pvp_log_data(session: &WorldSession, world: &World) -> Result<()> {
    if let Some(player) = session.player_guid() {
        system::send_scoreboard(world, player);
    }
    Ok(())
}

/// Inspect range for the honour tab.
const INSPECT_DISTANCE: f32 = 10.0;

/// MSG_INSPECT_HONOR_STATS / CMSG_INSPECT_HONOR_STATS: a nearby player's honour tab.
/// Players who could fight each other right now see nothing.
pub fn handle_inspect_honor_stats(session: &WorldSession, packet: &mut WorldPacket, world: &World) -> Result<()> {
    let (Some(player), Some(target)) = (session.player_guid(), packet.read_guid_for(session.protocol())) else {
        return Ok(());
    };
    let players = &world.managers.player_mgr;
    let (Some((map, position, team)), Some((target_map, target_position, target_team))) = (
        players.with_player(player, |p| (p.map_id, p.movement.position, p.get_team())),
        players.with_player(target, |p| (p.map_id, p.movement.position, p.get_team())),
    ) else {
        return Ok(());
    };
    let dx = position.x - target_position.x;
    let dy = position.y - target_position.y;
    let dz = position.z - target_position.z;
    if map != target_map || (dx * dx + dy * dy + dz * dz).sqrt() > INSPECT_DISTANCE {
        return Ok(());
    }
    if team != target_team && crate::game::pvp::flag::is_pvp(world, target) {
        return Ok(());
    }
    if let Some(stats) = crate::game::player::honor::system::inspect_stats(world, target) {
        world.managers.broadcast_mgr.send_msg_to_player(player, stats);
    }
    Ok(())
}

/// MSG_BATTLEGROUND_PLAYER_POSITIONS: teammates and the flag carrier.
pub fn handle_player_positions(session: &WorldSession, world: &World) -> Result<()> {
    if let Some(player) = session.player_guid() {
        system::send_positions(world, player);
    }
    Ok(())
}

/// CMSG_TOGGLE_PVP (vanilla: an optional byte sets the state) / CMSG_SET_PVP (1.14).
pub fn handle_toggle_pvp(session: &WorldSession, packet: &mut WorldPacket, world: &World) -> Result<()> {
    let Some(player) = session.player_guid() else {
        return Ok(());
    };
    let explicit = match session.protocol() {
        Protocol::Vanilla => packet.read_u8().map(|b| b != 0),
        Protocol::Modern => None,
    };
    crate::game::pvp::flag::toggle(world, player, explicit);
    Ok(())
}

/// CMSG_SET_PVP (1.14): set the PvP preference.
pub fn handle_set_pvp(session: &WorldSession, packet: &mut WorldPacket, world: &World) -> Result<()> {
    let Some(player) = session.player_guid() else {
        return Ok(());
    };
    let enable = read_bit(packet).unwrap_or(false);
    crate::game::pvp::flag::toggle(world, player, Some(enable));
    Ok(())
}
