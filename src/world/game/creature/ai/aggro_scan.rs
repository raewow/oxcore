//! Aggro Scan System
//!
//! Periodically scans for players in aggro range and triggers aggro.

use super::aggro::{
    calculate_aggro_range, distance_squared_2d, is_npc, is_valid_aggro_target,
    should_aggro_creature, MAX_AGGRO_RANGE,
};
use super::system::queue_event;
use super::types::{AIEvent, AIState};
use crate::shared::protocol::{ObjectGuid, Position};
use crate::world::World;

/// Scan for players in aggro range
/// Called periodically for idle creatures
pub async fn scan_for_aggro(world: &World) -> anyhow::Result<()> {
    // Get all idle creatures that can aggro
    let idle_creatures: Vec<(
        ObjectGuid,
        u32,
        u32,
        Position,
        u8,
        u32,
        u32,
        u32,
        AIState,
        bool,
    )> = world
        .managers
        .creature_mgr
        .iter_creatures()
        .filter(|entry| {
            let creature = entry.value();
            // Only idle creatures that are alive and not NPCs
            creature.ai_state == AIState::Idle
                && creature.death_state.is_alive()
                && !is_npc(creature.npc_flags)
        })
        .map(|entry| {
            let creature = entry.value();
            (
                *entry.key(),
                creature.map_id,
                creature.instance_id,
                creature.position,
                creature.level,
                creature.faction,
                creature.unit_flags,
                creature.npc_flags,
                creature.ai_state,
                creature.death_state.is_alive(),
            )
        })
        .collect();

    for (creature_guid, map_id, instance_id, pos, level, faction, unit_flags, npc_flags, _, _) in
        idle_creatures
    {
        check_creature_aggro(
            world,
            creature_guid,
            map_id,
            instance_id,
            pos,
            level,
            faction,
            unit_flags,
            npc_flags,
        )
        .await?;
    }

    Ok(())
}

async fn check_creature_aggro(
    world: &World,
    creature_guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
    creature_pos: Position,
    creature_level: u8,
    creature_faction: u32,
    creature_flags: u32,
    _npc_flags: u32,
) -> anyhow::Result<()> {
    // Get nearby players from map
    let map = world
        .managers
        .map_mgr
        .get_or_create_map(map_id, instance_id);
    let nearby = map.get_objects_in_range(creature_pos, MAX_AGGRO_RANGE);

    for target_guid in nearby {
        // Only check players
        if !target_guid.is_player() {
            continue;
        }

        // Get player info
        let player_info = world
            .managers
            .player_mgr
            .with_player(target_guid, |player| {
                // Player faction is determined by race (Alliance vs Horde)
                // For aggro purposes, we use a simplified check based on race
                // Alliance races: 1, 3, 4, 7, 11 (Human, Dwarf, Night Elf, Gnome, Draenei)
                // Horde races: 2, 5, 6, 8, 10 (Orc, Undead, Tauren, Troll, Blood Elf)
                let player_faction = if player.race == 1
                    || player.race == 3
                    || player.race == 4
                    || player.race == 7
                    || player.race == 11
                {
                    1 // Alliance
                } else {
                    2 // Horde
                };
                (player.movement.position, player.level as u8, player_faction)
            });

        let Some((player_pos, player_level, player_faction)) = player_info else {
            continue;
        };

        // Check if player is alive and attackable
        let is_alive = world.managers.player_mgr.is_player_alive(target_guid);
        if !is_alive {
            continue;
        }

        let is_hostile =
            should_aggro_creature(creature_faction, creature_flags, player_faction, true);

        // Check distance
        let aggro_range = calculate_aggro_range(creature_level, player_level);
        let dist_sq = distance_squared_2d(&creature_pos, &player_pos);

        if dist_sq <= aggro_range * aggro_range {
            // Check line of sight via VMap (walls, buildings, terrain block aggro)
            if !world
                .managers
                .vmap_mgr
                .is_in_line_of_sight(map_id, creature_pos, player_pos)
            {
                continue;
            }

            // Fire MoveInLineOfSight for Lua-scripted creatures regardless of hostility.
            // This allows passive proximity scripts (cauldron sensors, tower detectors) to work.
            if world.managers.lua_mgr.has_creature_ai(
                world
                    .managers
                    .creature_mgr
                    .with_creature(creature_guid, |c| c.entry)
                    .unwrap_or(0),
            ) {
                queue_event(
                    world,
                    creature_guid,
                    AIEvent::UnitInLineOfSight {
                        unit_guid: target_guid,
                        is_hostile,
                    },
                );
            }

            if !is_hostile {
                continue;
            }

            tracing::debug!(
                "[AGGRO] Creature {:?} aggroing player {:?} at distance {:.1}",
                creature_guid,
                target_guid,
                dist_sq.sqrt()
            );

            // Queue aggro event
            queue_event(
                world,
                creature_guid,
                AIEvent::UnitInRange {
                    unit_guid: target_guid,
                    distance: dist_sq.sqrt(),
                    is_hostile: true,
                    is_player: true,
                },
            );

            break; // Only aggro one target at a time
        }
    }

    Ok(())
}
