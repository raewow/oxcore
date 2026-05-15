//! Death and resurrection packet handlers
//!
//! Handles:
//! - CMSG_REPOP_REQUEST: Player clicks "Release Spirit"
//! - CMSG_RECLAIM_CORPSE: Player clicks "Resurrect" near corpse
//! - CMSG_RESURRECT_RESPONSE: Player accepts/declines resurrection offer
//! - CMSG_SPIRIT_HEALER_ACTIVATE: Player interacts with spirit healer
//! - CMSG_SELF_RES: Player uses self-resurrection (Reincarnation, Soulstone)
//! - CMSG_AREA_SPIRIT_HEALER_QUERY / QUEUE: Battleground spirit healer wave
//! - CMSG_SETDEATHBINDPOINT / CMSG_GETDEATHBINDZONE: Hearthstone bindpoint

use crate::shared::database::{CharacterRepository, Databases};
use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::core::session::WorldSession;
use crate::world::World;
use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, warn};

/// Handle CMSG_REPOP_REQUEST (0x015A)
///
/// Sent when the player clicks "Release Spirit" on the death screen.
/// No payload.
pub async fn handle_repop_request(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    world
        .systems
        .death
        .handle_release_spirit(player_guid, world)?;
    Ok(())
}

/// Handle CMSG_RECLAIM_CORPSE (0x01D2)
///
/// Sent when the player clicks "Resurrect Now" near their corpse.
///
/// Packet structure:
///   corpse_guid: u64
pub async fn handle_reclaim_corpse(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    let _corpse_guid = packet.read_guid_raw();

    // The corpse_guid is validated in the system
    world
        .systems
        .death
        .handle_reclaim_corpse(player_guid, world)?;
    Ok(())
}

/// Handle CMSG_RESURRECT_RESPONSE (0x015C)
///
/// Sent when the player accepts or declines a resurrection offer.
///
/// Packet structure:
///   resurrector_guid: u64
///   accept:           u8 (0 = decline, 1 = accept)
pub async fn handle_resurrect_response(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    let resurrector_guid = packet.read_guid_raw();

    let accept = packet.read_u8().unwrap_or(0) != 0;

    if let Some(resurrector) = resurrector_guid.map(ObjectGuid::from_raw) {
        world
            .systems
            .death
            .handle_resurrect_response(player_guid, resurrector, accept, world)?;
    }

    Ok(())
}

/// Handle CMSG_SPIRIT_HEALER_ACTIVATE (0x021C)
///
/// Sent when the player interacts with a spirit healer NPC.
///
/// Packet structure:
///   healer_guid: u64
pub async fn handle_spirit_healer_activate(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    let _healer_guid = packet.read_guid_raw();

    world
        .systems
        .death
        .handle_spirit_healer(player_guid, world)?;
    Ok(())
}

/// Handle CMSG_SELF_RES (0x02B3)
///
/// Sent when the player clicks the self-resurrection button on the death
/// screen (Shaman Reincarnation, Warlock-dropped Soulstone, Twisting Nether,
/// etc.). The spell id to cast is stored in the PLAYER_SELF_RES_SPELL update
/// field, populated at death time by `DeathSystem::on_killed`.
///
/// Packet structure: (empty)
pub async fn handle_self_res(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    // Pull the stored self-res spell id (and validate state).
    let stored = world
        .systems
        .player
        .manager()
        .with_player(player_guid, |player| {
            (player.self_res_spell, player.death.death_state)
        });
    let (spell_id, death_state) = match stored {
        Some(pair) => pair,
        None => return Ok(()),
    };

    // Must be a ghost (DeathState::Dead) with a stored self-res spell.
    use crate::world::game::player::death::state::DeathState;
    if death_state != DeathState::Dead && death_state != DeathState::Corpse {
        debug!(
            "Player {:?} pressed self-res while not dead ({:?})",
            player_guid, death_state
        );
        return Ok(());
    }
    if spell_id == 0 {
        debug!(
            "Player {:?} pressed self-res but has no stored spell id",
            player_guid
        );
        return Ok(());
    }

    // Clear the field so the button greys out immediately — prevents
    // spam-clicking or a second cast landing.
    world
        .systems
        .player
        .manager()
        .with_player_mut(player_guid, |player| {
            player.self_res_spell = 0;
        });

    // Cast the self-res spell on self. The spell's effects (EFFECT_RESURRECT
    // or EFFECT_SELF_RESURRECT) handle the actual revive + health/mana restore.
    debug!(
        "Player {:?} self-resurrecting via spell {}",
        player_guid, spell_id
    );
    world
        .systems
        .spells
        .cast_spell(
            player_guid,
            spell_id,
            Some(player_guid),
            /*is_triggered*/ true,
            world,
        )
        .await?;

    Ok(())
}

/// Handle CMSG_AREA_SPIRIT_HEALER_QUERY (0x02E2)
///
/// Sent by a ghost in a battleground when clicking a spirit healer. Server
/// replies with SMSG_AREA_SPIRIT_HEALER_TIME containing the countdown (ms)
/// until the next resurrection wave. Vanilla BG waves are every 30 seconds.
///
/// Packet structure:
///   healer_guid: u64
pub async fn handle_area_spirit_healer_query(
    session: &WorldSession,
    packet: &mut WorldPacket,
    _world: &World,
) -> Result<()> {
    let healer_guid = match packet.read_guid_raw() {
        Some(g) => g,
        None => return Ok(()),
    };

    // Reply with time until next wave. Real BG system (Phase 7+) will compute
    // this from the battleground's wave timer. For now, return a fixed 30s.
    const WAVE_INTERVAL_MS: u32 = 30_000;

    let mut reply = WorldPacket::new(Opcode::SMSG_AREA_SPIRIT_HEALER_TIME);
    reply.write_u64(healer_guid);
    reply.write_u32(WAVE_INTERVAL_MS);
    session.send_packet(reply)?;
    Ok(())
}

/// Handle CMSG_AREA_SPIRIT_HEALER_QUEUE (0x02E3)
///
/// Sent when a ghost queues themselves for the next resurrection wave at a
/// spirit healer in a battleground. In vanilla this flags the player for
/// batch resurrection on the wave tick.
///
/// Packet structure:
///   healer_guid: u64
pub async fn handle_area_spirit_healer_queue(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(g) => g,
        None => return Ok(()),
    };
    let _healer_guid = packet.read_guid_raw();

    // Delegate to DeathSystem. The real BG wave system (Phase 8) will consume
    // this queue on its 30s timer. For now we just mark the player queued.
    world.systems.death.queue_for_spirit_healer(player_guid);
    Ok(())
}

/// Handle CMSG_SETDEATHBINDPOINT (0x0154)
///
/// Sent by an innkeeper's gossip flow when binding a hearthstone. The client
/// asks the server to persist the player's homebind to the current location.
///
/// Packet structure: (empty — position/zone/map taken from the player's
/// current world position)
pub async fn handle_setdeathbindpoint(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    databases: &Databases,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(g) => g,
        None => return Ok(()),
    };

    // Pull current position + zone + map from the live player.
    let bind = world
        .systems
        .player
        .manager()
        .with_player_mut(player_guid, |player| {
            let pos = player.movement.position;
            player.homebind_map = player.map_id;
            player.homebind_zone = player.zone_id;
            player.homebind_x = pos.x;
            player.homebind_y = pos.y;
            player.homebind_z = pos.z;
            (
                player.guid.counter(),
                player.map_id,
                player.zone_id,
                pos.x,
                pos.y,
                pos.z,
            )
        });

    let (char_guid, map, zone, x, y, z) = match bind {
        Some(v) => v,
        None => return Ok(()),
    };

    // Persist to DB.
    let repo = CharacterRepository::new(Arc::new(databases.character.clone()));
    if let Err(e) = repo.save_homebind(char_guid, map, zone, x, y, z).await {
        warn!("Failed to save homebind for {:?}: {}", player_guid, e);
    }

    // Send confirmation packet so client updates the hearthstone UI.
    send_bindpoint_update(session, map, zone, x, y, z)?;

    debug!(
        "Player {:?} bound home at map={} zone={}",
        player_guid, map, zone
    );
    Ok(())
}

/// Handle CMSG_GETDEATHBINDZONE (0x0156)
///
/// Sent by the client when it wants to refresh its cached bindpoint (eg at
/// login). We reply with SMSG_BINDPOINTUPDATE containing the stored homebind.
///
/// Packet structure: (empty)
pub async fn handle_getdeathbindzone(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(g) => g,
        None => return Ok(()),
    };

    let bind = world
        .systems
        .player
        .manager()
        .with_player(player_guid, |player| {
            (
                player.homebind_map,
                player.homebind_zone,
                player.homebind_x,
                player.homebind_y,
                player.homebind_z,
            )
        });

    if let Some((map, zone, x, y, z)) = bind {
        send_bindpoint_update(session, map, zone, x, y, z)?;
    }
    Ok(())
}

/// Send SMSG_BINDPOINTUPDATE to the client with the player's hearthstone
/// destination. Format: x(f32), y(f32), z(f32), map(u32), zone(u32).
fn send_bindpoint_update(
    session: &WorldSession,
    map: u32,
    zone: u32,
    x: f32,
    y: f32,
    z: f32,
) -> Result<()> {
    let mut packet = WorldPacket::new(Opcode::SMSG_BINDPOINTUPDATE);
    packet.write_f32(x);
    packet.write_f32(y);
    packet.write_f32(z);
    packet.write_u32(map);
    packet.write_u32(zone);
    session.send_packet(packet)?;
    Ok(())
}
