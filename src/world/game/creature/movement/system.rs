//! MovementSystem - handles all movement business logic and packet sending

use crate::shared::messages::movement::SmsgMonsterMove;
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Position};
use crate::world::game::broadcast_mgr::{BroadcastManagerTrait, BroadcastManagerExt};
use crate::world::World;
use super::generator::MovementUpdate;
use super::spline::MoveSpline;
use super::types::MovementGeneratorType;
use super::generators::{ChaseMovementGenerator, FleeMovementGenerator};
use std::sync::Arc;

/// MovementSystem - handles all movement business logic and packet sending
pub struct MovementSystem {
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,  // OWNS broadcast_mgr
}

impl MovementSystem {
    pub fn new(broadcast_mgr: Arc<dyn BroadcastManagerTrait>) -> Self {
        Self { broadcast_mgr }
    }

    /// Update movement for all creatures
    pub fn update_creatures(&self,
        diff_ms: u32,
        world: &World
    ) -> anyhow::Result<()> {
        // Get all creatures that might need movement updates
        let moving_creatures: Vec<(ObjectGuid, Position)> = world.managers.creature_mgr
            .iter_creatures()
            .filter(|e| e.value().death_state.is_alive())
            .map(|e| (*e.key(), e.value().position))
            .collect();

        for (guid, current_pos) in moving_creatures {
            self.update_single_creature(guid, current_pos, diff_ms, world);
        }

        Ok(())
    }

    /// Update a single creature's movement
    fn update_single_creature(
        &self,
        guid: ObjectGuid,
        current_pos: Position,
        diff_ms: u32,
        world: &World,
    ) {
        // Update spline position if active
        let spline_finished = world.managers.creature_mgr
            .with_creature_mut(guid, |creature| {
                if creature.move_spline.is_active() {
                    let still_active = creature.move_spline.update(diff_ms);
                    if still_active {
                        // Update creature position from spline
                        creature.position = creature.move_spline.get_position();
                    } else {
                        // Spline finished - snap to final position
                        creature.position = creature.move_spline.final_position();
                        return Some(true);
                    }
                }
                None
            })
            .flatten();

        // If spline just finished, notify motion master
        if spline_finished == Some(true) {
            world.managers.creature_mgr.with_creature_mut(guid, |creature| {
                creature.motion_master.movement_complete(guid);
            });
        }

        // Snap Z to terrain height during spline movement to prevent floating/sinking
        if let Some((map_id, pos)) = world.managers.creature_mgr
            .with_creature_mut(guid, |c| {
                if c.move_spline.is_active() {
                    Some((c.map_id, c.position))
                } else {
                    None
                }
            })
            .flatten()
        {
            if let Some(ground_z) = world.managers.vmap_mgr.get_height(map_id, pos.x, pos.y, pos.z + 5.0) {
                if (ground_z - pos.z).abs() < 3.0 {
                    world.managers.creature_mgr.with_creature_mut(guid, |c| {
                        c.position.z = ground_z;
                    });
                }
            }
        }

        // Relocate creature in grid if position changed from spline movement
        if let Some(new_pos) = world.managers.creature_mgr
            .with_creature_mut(guid, |c| c.position)
        {
            if new_pos.x != current_pos.x || new_pos.y != current_pos.y {
                let (map_id, instance_id) = world.managers.creature_mgr
                    .with_creature_mut(guid, |c| (c.map_id, c.instance_id))
                    .unwrap_or((0, 0));
                let map = world.managers.map_mgr.get_or_create_map(map_id, instance_id);
                map.relocate_creature(guid, current_pos, new_pos);
            }
        }

        // Update target position for chase generators
        let target = world.managers.creature_mgr
            .with_creature_mut(guid, |c| c.combat.attacking)
            .flatten();

        if let Some(target_guid) = target {
            if let Some(target_pos) = world.managers.player_mgr.get_position(target_guid) {
                world.managers.creature_mgr.with_creature_mut(guid, |creature| {
                    let creature_pos = creature.position;

                    // Update chase generator target + creature position
                    if let Some(gen) = creature.motion_master.get_generator_mut(MovementGeneratorType::Chase) {
                        if let Some(chase) = gen.as_any_mut().downcast_mut::<ChaseMovementGenerator>() {
                            chase.update_target_position(target_pos);
                            chase.set_creature_position(creature_pos);
                        }
                    }

                    // Update flee generator target position
                    if let Some(gen) = creature.motion_master.get_generator_mut(MovementGeneratorType::Fleeing) {
                        if let Some(flee) = gen.as_any_mut().downcast_mut::<FleeMovementGenerator>() {
                            flee.update_target_position(target_pos);
                        }
                    }
                });
            }
        }

        // Get the CURRENT position (after spline update), not the stale snapshot
        // from the start of the tick. This ensures motion_master decisions and
        // SMSG_MONSTER_MOVE packets use the correct creature position.
        let current_pos = world.managers.creature_mgr
            .with_creature_mut(guid, |c| c.position)
            .unwrap_or(current_pos);

        // Run movement update
        let update = world.managers.creature_mgr
            .with_creature_mut(guid, |creature| {
                creature.motion_master.update(guid, current_pos, diff_ms)
            })
            .flatten();

        let Some(update) = update else {
            return;
        };

        // Check if this is chase movement (for facing)
        let chase_target = world.managers.creature_mgr
            .with_creature_mut(guid, |creature| {
                if creature.motion_master.active_generator() == MovementGeneratorType::Chase {
                    if let Some(gen) = creature.motion_master.get_generator_mut(MovementGeneratorType::Chase) {
                        if let Some(chase) = gen.as_any_mut().downcast_mut::<ChaseMovementGenerator>() {
                            return Some(chase.target);
                        }
                    }
                }
                None
            })
            .flatten();

        // Handle movement update results
        match update {
            MovementUpdate::NewDestination { destination, speed, is_walking } => {
                // Get creature map_id and compute real start position from active spline
                // (MaNGOS: MoveSplineInit::Launch computes position from running spline)
                let (map_id, real_start) = world.managers.creature_mgr
                    .with_creature_mut(guid, |c| {
                        let start = if c.move_spline.is_active() {
                            let pos = c.move_spline.get_position();
                            c.position = pos; // Update creature position to spline position
                            pos
                        } else {
                            c.position
                        };
                        (c.map_id, start)
                    })
                    .unwrap_or((0, current_pos));

                // Query pathfinder for a path (VMap LOS -> NavMesh A* -> obstacle avoidance)
                let path_result = world.managers.pathfinder.calculate_path(map_id, real_start, destination);
                let path_waypoints = path_result.waypoints();

                let final_dest = path_waypoints.last().copied().unwrap_or(destination);

                if path_waypoints.len() > 2 {
                    // Multi-waypoint path from NavMesh/obstacle avoidance
                    // Build full spline path: start + all path waypoints
                    let mut spline_path = Vec::with_capacity(path_waypoints.len() + 1);
                    spline_path.push(real_start);
                    spline_path.extend_from_slice(&path_waypoints);
                    let spline = MoveSpline::new(spline_path, speed);

                    let duration = spline.total_duration();
                    // Packet waypoints: intermediate + destination (excludes start)
                    let packet_waypoints: Vec<Position> = path_waypoints.to_vec();

                    world.managers.creature_mgr.with_creature_mut(guid, |creature| {
                        creature.move_spline = spline;
                    });

                    self.send_path_movement_packet(guid, real_start, packet_waypoints, duration, is_walking, chase_target, world);
                } else {
                    // Straight line or simple 2-point path
                    let spline = MoveSpline::new(vec![real_start, final_dest], speed);

                    world.managers.creature_mgr.with_creature_mut(guid, |creature| {
                        creature.move_spline = spline;
                    });

                    self.send_movement_packet(guid, real_start, final_dest, speed, is_walking, chase_target, world);
                }
            }
            MovementUpdate::Arrived => {
                // Stop the active spline and notify motion master
                world.managers.creature_mgr.with_creature_mut(guid, |creature| {
                    creature.move_spline.stop();
                    creature.motion_master.movement_complete(guid);
                });

                // Send stop packet to client so creature visually stops
                self.send_stop_packet(guid, current_pos, world);
            }
            MovementUpdate::Finished | MovementUpdate::Continue => {}
        }
    }

    /// Get nearby players for creature packets.
    /// Sends to ALL nearby players - the client safely ignores packets for unknown GUIDs.
    fn get_visible_nearby_players(
        &self,
        creature_guid: ObjectGuid,
        position: Position,
        world: &World,
    ) -> Vec<ObjectGuid> {
        let (map_id, instance_id) = world.managers.creature_mgr
            .with_creature_mut(creature_guid, |c| (c.map_id, c.instance_id))
            .unwrap_or((0, 0));
        let map = world.managers.map_mgr.get_or_create_map(map_id, instance_id);
        map.get_players_in_range(position, map.visibility_distance())
    }

    /// Send multi-waypoint path movement packet to nearby players
    fn send_path_movement_packet(
        &self,
        guid: ObjectGuid,
        from: Position,
        path: Vec<Position>,
        duration: u32,
        is_walking: bool,
        facing_target: Option<ObjectGuid>,
        world: &World,
    ) {
        let msg = if let Some(target) = facing_target {
            SmsgMonsterMove::new_chase_path_move(guid, from, path, duration, target)
        } else {
            SmsgMonsterMove::new_path_move(guid, from, path, duration, is_walking)
        };
        let packet = msg.to_world_packet();

        let visible_players = self.get_visible_nearby_players(guid, from, world);
        self.broadcast_mgr.broadcast_to_players(&visible_players, &packet);
    }

    /// Send movement packet to nearby players
    fn send_movement_packet(
        &self,
        guid: ObjectGuid,
        from: Position,
        to: Position,
        speed: f32,
        is_walking: bool,
        facing_target: Option<ObjectGuid>,
        world: &World,
    ) {
        let msg = if let Some(target) = facing_target {
            SmsgMonsterMove::new_chase_move(guid, from, to, speed, target)
        } else {
            SmsgMonsterMove::new_point_move(guid, from, to, speed, is_walking)
        };
        let packet = msg.to_world_packet();

        let visible_players = self.get_visible_nearby_players(guid, from, world);
        self.broadcast_mgr.broadcast_to_players(&visible_players, &packet);

        tracing::trace!(
            "[MOVEMENT] Creature {:?} moving from ({:.1}, {:.1}) to ({:.1}, {:.1}), sent to {} players",
            guid,
            from.x, from.y,
            to.x, to.y,
            visible_players.len()
        );
    }

    /// Send facing angle packet (creature rotates to face a direction)
    pub fn send_facing_packet(&self,
        guid: ObjectGuid,
        position: Position,
        angle: f32,
        world: &World,
    ) {
        let msg = SmsgMonsterMove::new_facing_angle(guid, position, angle);
        let packet = msg.to_world_packet();

        let visible_players = self.get_visible_nearby_players(guid, position, world);
        self.broadcast_mgr.broadcast_to_players(&visible_players, &packet);
    }

    /// Send stop movement packet
    pub fn send_stop_packet(&self,
        guid: ObjectGuid,
        position: Position,
        world: &World,
    ) {
        let msg = SmsgMonsterMove::new_stop(guid, position);
        let packet = msg.to_world_packet();

        let visible_players = self.get_visible_nearby_players(guid, position, world);
        self.broadcast_mgr.broadcast_to_players(&visible_players, &packet);
    }
}
