//! Area trigger handler
//!
//! Handles CMSG_AREATRIGGER — player enters an invisible trigger zone.
//! Runs the trigger's script, credits exploration quests, starts inn rest,
//! and sends the player through teleport triggers that let them pass.

use anyhow::Result;
use tracing::debug;

use crate::core::lua::{build_player_snapshot, execute_gossip_actions};
use crate::core::session::WorldSession;
use crate::game::area_trigger::{self, AreaTriggerEntry};
use crate::game::common::update_fields::PLAYER_FLAGS;
use crate::game::conditions::ConditionContext;
use crate::game::instance::lifecycle;
use crate::game::npc::quest::QuestStatus;
use crate::game::player::environment::RestType;
use crate::game::player::teleport::teleport_to;
use crate::World;
use oxcore_shared::messages::instance::SmsgAreaTriggerMessage;
use oxcore_shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use oxcore_shared::protocol::WorldPacket;

/// Content patch (1.4) from which instance entrances have minimum levels.
const PATCH_1_4: u8 = 2;

/// Tell the player why a trigger refused them.
fn send_area_trigger_text(session: &WorldSession, trigger_id: u32, text: String) -> Result<()> {
    session.send_msg(SmsgAreaTriggerMessage { trigger_id, text })
}

/// Handle CMSG_AREATRIGGER — player enters an area trigger zone.
///
/// Order: position check, script, exploration quest, inn, teleport.
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

    let Some((player_map_id, player_x, player_y, player_z, player_level, is_alive, is_gm)) = world
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
                player.gm_mode,
            )
        })
    else {
        return Ok(());
    };

    // Triggers flown over on a taxi do nothing.
    let in_flight = world
        .managers
        .player_mgr
        .with_player(player_guid, |player| player.taxi.is_flying())
        .unwrap_or(false);
    if in_flight {
        return Ok(());
    }

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

    // --- Exploration quests ---
    if let Some(quest_id) = area_trigger_mgr.get_quest_for_trigger(trigger_id) {
        if is_alive
            && world.systems.quest.get_quest_status(player_guid, quest_id) == QuestStatus::Incomplete
        {
            world.systems.quest.handle_area_explored(player_guid, quest_id);
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

    // --- Battleground entrance portals ---
    if crate::game::battleground::system::entrance_trigger(world, player_guid, trigger_id) {
        return Ok(());
    }

    // --- Objective scripts (battleground flag rooms, capture areas) ---
    if crate::game::pvp::objective::on_area_trigger(world, player_guid, trigger_id).await {
        return Ok(());
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
            let name = if target_map.name.is_empty() {
                "this instance"
            } else {
                target_map.name.as_str()
            };
            send_area_trigger_text(
                session,
                trigger_id,
                format!("You cannot enter {} while in ghost form.", name),
            )?;
            return Ok(());
        }
        if teleport.destination_map != corpse_map {
            if let Some(inner) = area_trigger_mgr.get_map_entrance_trigger(corpse_map) {
                teleport = inner;
            }
        }
    }

    // Level and condition (key, attunement, quest) requirements. Game
    // masters pass; minimum levels arrived with patch 1.4.
    if !is_gm {
        let level_failed = teleport.required_level > 0
            && player_level < teleport.required_level
            && world.content_patch() >= PATCH_1_4;
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
                send_area_trigger_text(session, trigger_id, teleport.message.clone())?;
            } else if level_failed {
                send_area_trigger_text(
                    session,
                    trigger_id,
                    format!(
                        "You must be at least level {} to enter.",
                        teleport.required_level
                    ),
                )?;
            }
            debug!(
                "Area trigger {}: player {} refused (level {} < {}: {}, condition {} failed: {})",
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
    }

    // Same-map triggers are near teleports; others choose the dungeon copy
    // and check the entry rules on the way.
    teleport_to(
        world,
        player_guid,
        teleport.destination_map,
        teleport.destination,
        0,
    )
    .await?;
    Ok(())
}
