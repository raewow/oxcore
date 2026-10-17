//! Area trigger handler
//!
//! Handles CMSG_AREATRIGGER — player enters an invisible trigger zone.
//! Checks quest triggers, tavern rest, and teleport destinations.

use anyhow::Result;
use tracing::{debug, info, warn};

use crate::core::lua::{build_player_snapshot, execute_gossip_actions};
use crate::core::session::WorldSession;
use crate::game::area_trigger::{self, AreaTriggerEntry};
use crate::game::common::update_fields::PLAYER_FLAGS;
use crate::game::conditions::ConditionContext;
use crate::game::instance::lifecycle;
use crate::game::instance::manager::InstanceEntryError;
use crate::game::player::environment::RestType;
use crate::World;
use oxcore_shared::game::chat::{ChatMsg, ChatTag, Language};
use oxcore_shared::messages::chat::SmsgMessageChat;
use oxcore_shared::messages::movement::SmsgTransferAborted;
use oxcore_shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::{ObjectGuid, Position, WorldPacket};

/// Area-trigger refusals are shown to the player as a system message.
fn send_area_trigger_text(session: &WorldSession, message: &str) -> Result<()> {
    session.send_msg(SmsgMessageChat {
        msgtype: ChatMsg::System,
        language: Language::Universal,
        sender_guid: ObjectGuid::empty(),
        sender_name: None,
        target_guid: None,
        channel_name: None,
        player_rank: None,
        message,
        chat_tag: ChatTag::None,
    })
}

/// Handle CMSG_AREATRIGGER — player enters an area trigger zone.
///
/// Priority order:
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
    let (player_map_id, player_x, player_y, player_z, player_level, is_alive) = match world
        .managers
        .player_mgr
        .with_player(player_guid, |player| {
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
        let actions = world
            .managers
            .lua_mgr
            .with_lua(|lua| script.on_area_trigger(lua, &player_snap));
        if !actions.is_empty() {
            execute_gossip_actions(
                actions,
                player_guid,
                oxcore_shared::protocol::ObjectGuid::empty(),
                world,
            )
            .await?;
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
                    crate::core::common::guid::ObjectGuid::from_low(player_guid.counter());
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
    let mut teleport = match area_trigger_mgr.get_teleport(trigger_id) {
        Some(t) => t,
        None => return Ok(()), // Not a teleport trigger
    };
    let Some(target_map) = lifecycle::map_entry(world, teleport.destination_map) else {
        return Ok(());
    };

    // A ghost may only enter the dungeon holding their corpse, or one whose
    // entrance encloses it, and is landed at the corpse's own dungeon entrance.
    if !is_alive && target_map.is_dungeon() {
        let corpse_map = world
            .managers
            .player_mgr
            .with_player(player_guid, |player| player.death.corpse_map_id)
            .flatten()
            .unwrap_or(0);
        let mut map = corpse_map;
        let reachable = loop {
            if map == target_map.id {
                break true;
            }
            match lifecycle::map_entry(world, map).filter(|entry| entry.is_dungeon()) {
                Some(entry) if entry.parent != 0 && entry.parent != map => map = entry.parent,
                _ => break false,
            }
        };
        if !reachable {
            send_area_trigger_text(session, "You cannot enter this instance while in ghost form.")?;
            return Ok(());
        }
        if teleport.destination_map != corpse_map {
            if let Some(inner) = area_trigger_mgr.get_map_entrance_trigger(corpse_map) {
                teleport = inner;
            }
        }
    }

    // Level and condition (key, attunement, quest) requirements.
    let level_failed = teleport.required_level > 0 && player_level < teleport.required_level;
    let condition_failed = teleport.required_condition != 0
        && !world.managers.condition_mgr.is_satisfied(
            teleport.required_condition,
            ConditionContext {
                target: player_guid,
                source: player_guid,
            },
            world,
        );
    if level_failed || condition_failed {
        if !teleport.message.is_empty() {
            send_area_trigger_text(session, &teleport.message)?;
        } else if level_failed {
            send_area_trigger_text(
                session,
                &format!(
                    "You must be at least level {} to enter this instance.",
                    teleport.required_level
                ),
            )?;
        }
        debug!(
            "Area trigger {}: Player {} refused (level {} < {}: {}, condition {} failed: {})",
            trigger_id,
            player_guid,
            player_level,
            teleport.required_level,
            level_failed,
            teleport.required_condition,
            condition_failed
        );
        return Ok(());
    }

    // Perform teleport
    let dest_map = teleport.destination_map;
    let dest_pos = teleport.destination;

    // Dungeon destinations go through copy selection, admission and binding.
    let dest_instance_id =
        match lifecycle::resolve_transfer_instance(world, player_guid, dest_map).await {
            Ok(instance_id) => instance_id,
            Err(error) => {
                match &error {
                    InstanceEntryError::Denied(denied) => debug!(
                        "Player {} denied entry to map {}: {:?}",
                        player_guid, dest_map, denied
                    ),
                    InstanceEntryError::Internal(e) => tracing::error!(
                        "Failed to create/enter instance for map {}: {}",
                        dest_map,
                        e
                    ),
                }
                session.send_msg(SmsgTransferAborted {
                    map_id: dest_map,
                    reason: lifecycle::abort_reason(&error),
                })?;
                return Ok(());
            }
        };

    info!(
        "[AREATRIGGER] Starting teleport sequence for player {:?} to map {} instance {}",
        player_guid, dest_map, dest_instance_id
    );

    // Send SMSG_TRANSFER_PENDING (cross-map only) + SMSG_NEW_WORLD to initiate
    // teleport. Client will respond with MSG_MOVE_WORLDPORT_ACK when ready.
    let old_position = Position {
        x: player_x,
        y: player_y,
        z: player_z,
        o: 0.0,
    };
    if dest_map != player_map_id {
        session.send_msg(oxcore_shared::messages::movement::SmsgTransferPending {
            dest_map,
            old_position,
        })?;
    }
    session.send_msg(oxcore_shared::messages::movement::SmsgNewWorld { dest_map, dest_pos })?;

    // Store teleport destination for worldport ACK handler to complete
    session.set_pending_teleport(Some((dest_map, dest_instance_id, dest_pos)));

    info!(
        "[AREATRIGGER] Teleport initiated - map {} instance {}, waiting for MSG_MOVE_WORLDPORT_ACK",
        dest_map, dest_instance_id
    );

    Ok(())
}
