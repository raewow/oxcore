//! Area trigger handler
//!
//! Handles CMSG_AREATRIGGER — player enters an invisible trigger zone.
//! Checks quest triggers, tavern rest, and teleport destinations.

use anyhow::Result;
use tracing::{debug, warn, info};

use crate::shared::game::chat::{ChatMsg, ChatTag, Language};
use crate::shared::messages::chat::SmsgMessageChat;
use crate::shared::messages::update::{ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};
use crate::world::game::common::update_fields::PLAYER_FLAGS;
use crate::world::core::lua::{build_player_snapshot, execute_gossip_actions};
use crate::world::core::session::WorldSession;
use crate::world::game::area_trigger::{self, AreaTriggerEntry};
use crate::world::game::player::environment::RestType;
use crate::world::World;

/// Handle CMSG_AREATRIGGER — player enters an area trigger zone.
///
/// Priority order (matching MaNGOS):
/// 1. Position validation (anti-cheat)
/// 2. Quest triggers
/// 3. Tavern triggers (set rest state)
/// 4. Teleport triggers (level check, teleport)
pub async fn handle_area_trigger(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let trigger_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read trigger ID"))?;

    debug!(
        "Player {} triggered area trigger {}",
        player_guid, trigger_id
    );

    // Get player info
    let (player_map_id, player_x, player_y, player_z, player_level, is_alive) =
        match world.managers.player_mgr.with_player(player_guid, |player| {
            (
                player.map_id,
                player.movement.position.x,
                player.movement.position.y,
                player.movement.position.z,
                player.level,
                player.is_alive(),
            )
        }) {
            Some(info) => info,
            None => return Ok(()),
        };

    let area_trigger_mgr = &world.managers.area_trigger_mgr;

    // Get trigger geometry: template table first, DBC fallback
    let trigger: AreaTriggerEntry = if let Some(t) = area_trigger_mgr.get_template(trigger_id) {
        t
    } else {
        let dbc = world.dbc.read();
        if let Some(dbc_entry) = dbc.get_area_trigger(trigger_id) {
            area_trigger::from_dbc_entry(dbc_entry)
        } else {
            debug!(
                "Player {} triggered unknown area trigger {} (not in DB or DBC)",
                player_guid, trigger_id
            );
            return Ok(());
        }
    };

    // Anti-cheat: validate player is near the trigger zone
    const POSITION_TOLERANCE: f32 = 5.0;
    if !area_trigger::is_point_in_area_trigger_zone(
        &trigger,
        player_map_id,
        player_x,
        player_y,
        player_z,
        POSITION_TOLERANCE,
    ) {
        debug!(
            "Player {} too far from area trigger {}, ignoring (anti-cheat)",
            player_guid, trigger_id
        );
        return Ok(());
    }

    // --- Lua area trigger script ---
    if let Some(script) = world.managers.lua_mgr.get_area_trigger_script(trigger_id) {
        let player_snap = build_player_snapshot(player_guid, world);
        let actions = world.managers.lua_mgr.with_lua(|lua| {
            script.on_area_trigger(lua, &player_snap)
        });
        if !actions.is_empty() {
            execute_gossip_actions(actions, player_guid, crate::shared::protocol::ObjectGuid::empty(), world).await?;
        }
        // Do not return — allow normal tavern/teleport logic to still run.
    }

    // --- Quest triggers ---
    if let Some(_quest_id) = area_trigger_mgr.get_quest_for_trigger(trigger_id) {
        if is_alive {
            // TODO: Check if player has this quest active and update exploration objective
            // Requires quest system integration: world.systems.quest.update_exploration_objective(...)
            debug!(
                "Area trigger {} is a quest trigger (quest {}), TODO: update objective",
                trigger_id, _quest_id
            );
        }
    }

    // --- Tavern triggers (set rest state) ---
    if area_trigger_mgr.is_tavern(trigger_id) {
        let player_mgr = &world.managers.player_mgr;

        // Only set tavern rest if not already resting in a city (city rest takes priority)
        let current_rest = player_mgr
            .with_player(player_guid, |p| p.environment.rest_type)
            .unwrap_or(RestType::No);

        if current_rest != RestType::InCity {
            world.systems.environment.set_rest_type(
                player_guid,
                RestType::InTavern,
                trigger_id,
                player_mgr,
            )?;

            // Send updated PLAYER_FLAGS to client so it shows the Zzz resting icon
            if let Some(new_flags) = player_mgr.with_player(player_guid, |p| p.player_flags) {
                let world_guid =
                    crate::world::core::common::guid::ObjectGuid::from_low(player_guid.counter());
                let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
                    ValuesUpdateBlock::new(world_guid, ObjectType::Player)
                        .set_field(PLAYER_FLAGS, new_flags),
                ));
                world
                    .managers
                    .broadcast_mgr
                    .send_msg_to_player(player_guid, update);
            }
        }

        return Ok(()); // Tavern triggers don't teleport
    }

    // --- Teleport triggers ---
    let teleport = match area_trigger_mgr.get_teleport(trigger_id) {
        Some(t) => t,
        None => return Ok(()), // Not a teleport trigger
    };

    // Check level requirement
    if teleport.required_level > 0 && player_level < teleport.required_level {
        let msg = if !teleport.message.is_empty() {
            teleport.message.clone()
        } else {
            format!(
                "You must be at least level {} to enter this instance.",
                teleport.required_level
            )
        };

        let chat = SmsgMessageChat {
            msgtype: ChatMsg::System,
            language: Language::Universal,
            sender_guid: ObjectGuid::empty(),
            sender_name: None,
            target_guid: None,
            channel_name: None,
            player_rank: None,
            message: &msg,
            chat_tag: ChatTag::None,
        };
        session.send_msg(chat)?;

        debug!(
            "Area trigger {}: Player {} level {} < required {}",
            trigger_id, player_guid, player_level, teleport.required_level
        );
        return Ok(());
    }

    // Perform teleport
    let dest_map = teleport.destination_map;
    let dest_pos = teleport.destination;

    // Check if destination is an instance map and get/create instance
    let dest_instance_id = {
        let dbc = world.dbc.read();
        let is_instance_map = dbc.get_map(dest_map)
            .map(|m| m.map_type == 1 || m.map_type == 2)  // 1=Instance, 2=Raid
            .unwrap_or(false);
        let is_raid = dbc.get_map(dest_map)
            .map(|m| m.map_type == 2)
            .unwrap_or(false);
        drop(dbc);

        if is_instance_map {
            // Get or create instance for this player
            // TODO: Get group_leader_guid from player's group when group system is ready
            let group_leader_guid = None;

            match world.managers.instance_mgr.enter_instance(
                &world.databases,
                dest_map,
                player_guid,
                group_leader_guid,
                is_raid,
            ).await {
                Ok(instance_id) => {
                    debug!(
                        "Player {} entering instance {} (map {}, raid={})",
                        player_guid, instance_id, dest_map, is_raid
                    );
                    instance_id
                }
                Err(e) => {
                    tracing::error!("Failed to create/enter instance for map {}: {}", dest_map, e);
                    // Fall back to continent instance (should not happen for instance maps)
                    0
                }
            }
        } else {
            0  // Continents use instance_id = 0
        }
    };

    info!(
        "[AREATRIGGER] Starting teleport sequence for player {:?} to map {} instance {}",
        player_guid, dest_map, dest_instance_id
    );

    // Send SMSG_TRANSFER_PENDING + SMSG_NEW_WORLD to initiate teleport
    // Client will respond with MSG_MOVE_WORLDPORT_ACK when ready
    if dest_map != player_map_id {
        info!("[AREATRIGGER] Sending SMSG_TRANSFER_PENDING (different map)");
        let mut transfer_packet = WorldPacket::new(Opcode::SMSG_TRANSFER_PENDING);
        transfer_packet.write_u32(dest_map);
        session.send_packet(transfer_packet)?;
        info!("[AREATRIGGER] SMSG_TRANSFER_PENDING sent successfully");
    } else {
        info!("[AREATRIGGER] Skipping SMSG_TRANSFER_PENDING (same map)");
    }

    info!(
        "[AREATRIGGER] Sending SMSG_NEW_WORLD (map={}, pos={},{},{},{})",
        dest_map, dest_pos.x, dest_pos.y, dest_pos.z, dest_pos.o
    );
    let mut new_world_packet = WorldPacket::new(Opcode::SMSG_NEW_WORLD);
    new_world_packet.write_u32(dest_map);
    new_world_packet.write_f32(dest_pos.x);
    new_world_packet.write_f32(dest_pos.y);
    new_world_packet.write_f32(dest_pos.z);
    new_world_packet.write_f32(dest_pos.o);
    session.send_packet(new_world_packet)?;
    info!("[AREATRIGGER] SMSG_NEW_WORLD sent successfully");

    // Store teleport destination for worldport ACK handler to complete
    info!("[AREATRIGGER] Storing pending teleport in session");
    session.set_pending_teleport(Some((dest_map, dest_instance_id, dest_pos)));

    info!(
        "[AREATRIGGER] ✓ Teleport initiated successfully - map {} instance {}, waiting for MSG_MOVE_WORLDPORT_ACK from client",
        dest_map, dest_instance_id
    );

    Ok(())
}
