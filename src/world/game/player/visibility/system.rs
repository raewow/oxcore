//! Visibility subsystem - processes visibility updates for players
//!
//! - Only update players who crossed cell boundaries (or forced)
//! - Batch appeared/disappeared notifications
//! - Throttle updates to once per 200ms
//! - Process visibility calculation synchronously, send packets async

use anyhow::Result;
use std::collections::HashSet;

use crate::shared::messages::create::SmsgOutOfRange;
use crate::shared::messages::update::{SmsgUpdateObject, UpdateBlockData};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Position, WorldPacket};
use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
use crate::world::core::common::compress_update_packet_if_needed;
use crate::world::map::grid_coords::CellPair;
use crate::world::World;

/// Minimum ticks between visibility updates (throttle)
/// At 50ms per tick, 4 ticks = 200ms minimum between updates
const UPDATE_THROTTLE_TICKS: u32 = 4;

/// Visibility subsystem - processes visibility updates for players
pub struct VisibilitySubsystem;

impl VisibilitySubsystem {
    pub fn new() -> Self {
        Self
    }

    /// Check if player crossed a cell boundary and mark dirty if so
    /// Called from MovementSystem after position update - O(1) operation
    pub fn check_cell_crossing(
        &self,
        player_guid: ObjectGuid,
        old_pos: Position,
        new_pos: Position,
        world: &World,
    ) {
        let old_cell = CellPair::from_world_coords(old_pos.x, old_pos.y);
        let new_cell = CellPair::from_world_coords(new_pos.x, new_pos.y);

        if old_cell != new_cell {
            world
                .managers
                .player_mgr
                .with_player_mut(player_guid, |player| {
                    player.visibility.update_cell(new_cell);
                });
        }
    }

    /// Mark player for forced immediate visibility update (login/teleport)
    pub fn mark_force_immediate(&self, player_guid: ObjectGuid, world: &World) {
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.visibility.mark_force_immediate();
            });
    }

    /// Process visibility for a single player (called from map update loop)
    /// Only processes if player is marked dirty, force_immediate, or throttle expired
    /// Returns true if visibility was updated
    pub fn update_player(
        &self,
        player_guid: ObjectGuid,
        current_tick: u32,
        world: &World,
    ) -> Result<bool> {
        // Get player state atomically to decide if we should update
        let update_info = world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                // Check if another update is already in progress
                if player.visibility.update_in_progress {
                    return None;
                }

                let dirty = player.visibility.dirty;
                let force = player.visibility.force_immediate;
                let last_tick = player.visibility.last_update_tick;
                let throttle_expired =
                    current_tick.saturating_sub(last_tick) >= UPDATE_THROTTLE_TICKS;

                let should_update = dirty || force || throttle_expired;

                if should_update {
                    // Clear flags and mark update in progress
                    player.visibility.dirty = false;
                    player.visibility.force_immediate = false;
                    player.visibility.last_update_tick = current_tick;
                    player.visibility.update_in_progress = true;
                }

                Some((
                    should_update,
                    player.map_id,
                    player.instance_id,
                    player.movement.position,
                    player.visibility.visible_objects.clone(),
                ))
            });

        let Some(Some((should_update, map_id, instance_id, pos, previous_visible))) = update_info
        else {
            return Ok(false);
        };

        if !should_update {
            return Ok(false);
        }

        // Check if grids around player are fully loaded (async grid loading)
        // If grids are still loading, defer visibility update to next tick to prevent
        // sending creature CREATE_OBJECT2 packets before the grid is ready (causes client crash)
        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);
        if !world.systems.grid.are_grids_loaded(&map, pos) {
            tracing::trace!(
                "[VISIBILITY] Grids not yet loaded for player {:?} at ({:.1}, {:.1}), deferring visibility update",
                player_guid,
                pos.x,
                pos.y
            );

            // Re-mark as dirty so we retry next tick
            world
                .managers
                .player_mgr
                .with_player_mut(player_guid, |player| {
                    player.visibility.dirty = true;
                    player.visibility.update_in_progress = false;
                });

            return Ok(false);
        }

        // Perform visibility calculation
        let (appeared, disappeared, visible_now) = self.calculate_visibility_delta(
            player_guid,
            map_id,
            instance_id,
            pos,
            &previous_visible,
            world,
        )?;

        // Update this player's visible set and queue pending notifications
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.visibility.visible_objects = visible_now.clone();
                player.visibility.pending_appeared.extend(appeared.iter());
                player
                    .visibility
                    .pending_disappeared
                    .extend(disappeared.iter());
                // Clear update_in_progress flag
                player.visibility.update_in_progress = false;
            });

        // Update bidirectional visibility state for other players
        // This prevents duplicate CREATE_OBJECT2 when the other player moves
        for &target_guid in &appeared {
            world
                .managers
                .player_mgr
                .with_player_mut(target_guid, |target| {
                    target.visibility.visible_objects.insert(player_guid);
                });
        }

        for &target_guid in &disappeared {
            world
                .managers
                .player_mgr
                .with_player_mut(target_guid, |target| {
                    target.visibility.visible_objects.remove(&player_guid);
                });
        }

        if !appeared.is_empty() || !disappeared.is_empty() {
            tracing::debug!(
                "[VISIBILITY] Player {:?} visibility update: appeared={}, disappeared={}",
                player_guid,
                appeared.len(),
                disappeared.len()
            );
        }

        Ok(true)
    }

    /// Flush pending visibility notifications (batched)
    /// Called during player update phase - sends packets asynchronously
    pub async fn flush_pending_notifications(
        &self,
        player_guid: ObjectGuid,
        world: &World,
    ) -> Result<()> {
        // Extract pending notifications atomically
        let (appeared, disappeared) = world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                (
                    std::mem::take(&mut player.visibility.pending_appeared),
                    std::mem::take(&mut player.visibility.pending_disappeared),
                )
            })
            .unwrap_or((Vec::new(), Vec::new()));

        if appeared.is_empty() && disappeared.is_empty() {
            return Ok(());
        }

        // Send batched CREATE_OBJECT2 packets for appeared objects
        self.send_batched_create_objects(player_guid, &appeared, world)?;

        // Send reverse CREATE_OBJECT2 (so appeared players can see us too)
        self.send_reverse_create_objects(player_guid, &appeared, world)?;

        // Send batched OUT_OF_RANGE packets for disappeared objects
        self.send_batched_out_of_range(player_guid, &disappeared, world)?;

        // Update bidirectional listener relationships
        self.update_bidirectional_listeners(player_guid, &appeared, &disappeared, world);

        Ok(())
    }

    /// Calculate visibility delta efficiently
    fn calculate_visibility_delta(
        &self,
        player_guid: ObjectGuid,
        map_id: u32,
        instance_id: u32,
        pos: Position,
        previous: &HashSet<ObjectGuid>,
        world: &World,
    ) -> Result<(Vec<ObjectGuid>, Vec<ObjectGuid>, HashSet<ObjectGuid>)> {
        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);
        let visibility_distance = map.visibility_distance();
        let visibility_distance_sq = visibility_distance * visibility_distance;

        // Get player's phase mask and alive state
        let (player_phase, player_is_alive) = world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |p| (p.phase_mask, p.is_alive()))
            .unwrap_or((1, true));

        // Get objects in grid range (coarse filter)
        let candidates = map.get_objects_in_range(pos, visibility_distance);

        // Fine-grained distance filter
        let mut visible_now = HashSet::with_capacity(candidates.len());
        for &candidate_guid in &candidates {
            if candidate_guid == player_guid {
                continue; // Skip self
            }

            // Get candidate position and phase based on type
            let (candidate_pos, candidate_phase) = if candidate_guid.is_player() {
                // Ghost visibility: living players and dead players cannot see each other
                let candidate_alive = world
                    .managers
                    .player_mgr
                    .with_player_mut(candidate_guid, |p| p.is_alive())
                    .unwrap_or(true);
                if player_is_alive != candidate_alive {
                    continue; // Living/dead mismatch — invisible to each other
                }

                let pos = world.managers.player_mgr.get_position(candidate_guid);
                let phase = world
                    .managers
                    .player_mgr
                    .with_player_mut(candidate_guid, |p| p.phase_mask)
                    .unwrap_or(1);
                (pos, phase)
            } else if candidate_guid.is_unit() {
                // Ghost visibility filter for creatures
                let static_flags = world
                    .managers
                    .creature_mgr
                    .get_static_flags1(candidate_guid)
                    .unwrap_or(0);
                let is_ghost_visible = (static_flags & crate::world::game::common::creature_flags::CREATURE_STATIC_FLAG_VISIBLE_TO_GHOSTS) != 0;

                if is_ghost_visible && player_is_alive {
                    continue; // Spirit healer visible only to dead players
                }
                if !player_is_alive && !is_ghost_visible {
                    continue; // Dead player can't see regular creatures
                }

                let pos = world.managers.creature_mgr.get_position(candidate_guid);
                let phase = world
                    .managers
                    .creature_mgr
                    .get_phase_mask(candidate_guid)
                    .unwrap_or(1);
                (pos, phase)
            } else if candidate_guid.is_game_object() {
                let pos = world.managers.gameobject_mgr.get_position(candidate_guid);
                let phase = world
                    .managers
                    .gameobject_mgr
                    .get_phase_mask(candidate_guid)
                    .unwrap_or(1);
                (pos, phase)
            } else if candidate_guid.is_corpse() {
                // Corpses are visible to everyone (alive and ghost viewers).
                // Pull the corpse's position from the manager; phase 1 always.
                let corpse = world.managers.corpse_mgr.get(candidate_guid);
                let pos = match corpse {
                    Some(c) => c.position,
                    None => continue, // stale GUID — skip
                };
                (Some(pos), 1u32)
            } else {
                continue;
            };

            // Check phase compatibility (must share at least one phase bit)
            if (player_phase & candidate_phase) == 0 {
                continue; // Different phase, not visible
            }

            // Check distance
            if let Some(candidate_pos) = candidate_pos {
                let dx = pos.x - candidate_pos.x;
                let dy = pos.y - candidate_pos.y;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq <= visibility_distance_sq {
                    visible_now.insert(candidate_guid);
                }
            }
        }

        // Calculate deltas
        let appeared: Vec<_> = visible_now
            .iter()
            .filter(|g| !previous.contains(g))
            .copied()
            .collect();

        let disappeared: Vec<_> = previous
            .iter()
            .filter(|g| !visible_now.contains(g))
            .copied()
            .collect();

        Ok((appeared, disappeared, visible_now))
    }

    /// Send batched CREATE_OBJECT2 for multiple objects to the viewer
    /// Implements chunking and compression like the old world to prevent crashes
    fn send_batched_create_objects(
        &self,
        viewer_guid: ObjectGuid,
        targets: &[ObjectGuid],
        world: &World,
    ) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }

        let viewer_broadcaster = world.managers.player_mgr.get_broadcaster(viewer_guid);
        let Some(broadcaster) = viewer_broadcaster else {
            tracing::warn!(
                "[VISIBILITY] Cannot send CREATE_OBJECT2: no broadcaster for viewer {:?}",
                viewer_guid
            );
            return Ok(());
        };

        // Filter out objects that have already had CREATE_OBJECT2 sent (deduplication guard)
        let targets_to_send: Vec<ObjectGuid> = targets
            .iter()
            .filter(|&&target_guid| {
                let should_send = world
                    .managers
                    .player_mgr
                    .with_player_mut(viewer_guid, |player| {
                        if player.visibility.objects_created.contains(&target_guid) {
                            false // Already sent, skip
                        } else {
                            player.visibility.objects_created.insert(target_guid);
                            true
                        }
                    })
                    .unwrap_or(false);

                if !should_send {
                    tracing::warn!(
                        "[VISIBILITY] Prevented duplicate CREATE_OBJECT2: {:?} -> {:?}",
                        target_guid,
                        viewer_guid
                    );
                }
                should_send
            })
            .copied()
            .collect();

        if targets_to_send.is_empty() {
            return Ok(());
        }

        // Maximum uncompressed packet size before chunking (32KB like old world)
        const MAX_UNCOMPRESSED_PACKET_SIZE: usize = 0x8000; // 32KB

        // Collect all update blocks first
        let mut all_blocks = Vec::new();
        for &target_guid in &targets_to_send {
            // Build create message based on object type
            let single_msg = if target_guid.is_player() {
                world
                    .managers
                    .player_mgr
                    .build_create_msg(target_guid, world)
            } else if target_guid.is_unit() {
                world
                    .managers
                    .creature_mgr
                    .build_create_msg(target_guid, world)
            } else if target_guid.is_game_object() {
                world
                    .managers
                    .gameobject_mgr
                    .build_create_msg(target_guid, world)
            } else if target_guid.is_corpse() {
                world
                    .managers
                    .corpse_mgr
                    .build_create_msg(target_guid, world)
            } else {
                None
            };

            if let Some(single_msg) = single_msg {
                // Extract blocks from the single message
                all_blocks.extend(single_msg.blocks);
            }
        }

        if all_blocks.is_empty() {
            return Ok(());
        }

        // Chunk blocks into multiple packets if needed
        let mut packets_to_send = Vec::new();
        let mut current_msg = SmsgUpdateObject::new();
        let mut current_size_estimate = 0;

        for block in all_blocks {
            // Estimate block size (rough estimate based on type)
            let block_size_estimate = match &block {
                UpdateBlockData::CreateObject2(_) => 200, // ~200 bytes per creature
                UpdateBlockData::CreateObject(_) => 200,
                UpdateBlockData::Values(_) => 50,
                UpdateBlockData::Movement(_) => 50,
                UpdateBlockData::OutOfRange(_) => 10,
            };

            // Check if adding this block would exceed the limit
            if current_size_estimate + block_size_estimate > MAX_UNCOMPRESSED_PACKET_SIZE
                && !current_msg.blocks.is_empty()
            {
                // Send current packet and start a new one
                packets_to_send.push(current_msg);
                current_msg = SmsgUpdateObject::new();
                current_size_estimate = 0;
            }

            current_msg = current_msg.add_block(block);
            current_size_estimate += block_size_estimate;
        }

        // Add final packet if it has blocks
        if !current_msg.blocks.is_empty() {
            packets_to_send.push(current_msg);
        }

        tracing::debug!(
            "[VISIBILITY] Sending batched CREATE_OBJECT2 to {:?}: {} targets in {} packet(s)",
            viewer_guid,
            targets.len(),
            packets_to_send.len()
        );

        // Send all packets with compression
        for update_msg in packets_to_send {
            let packet = update_msg.to_world_packet();

            // Apply compression if needed (>128 bytes threshold)
            let compressed_packet = compress_update_packet_if_needed(packet)?;

            let mut v2_packet = WorldPacket::new(compressed_packet.opcode());
            v2_packet.write_bytes(compressed_packet.contents());

            broadcaster.send_direct(v2_packet);
        }

        // Send movement sync packets for creatures that are currently mid-movement.
        // CREATE_OBJECT2 includes position but no spline data, so without this the
        // client shows moving creatures as standing still until their next movement.
        for &target_guid in &targets_to_send {
            if target_guid.is_unit() && !target_guid.is_player() {
                if let Some(move_packet) = world
                    .managers
                    .creature_mgr
                    .build_movement_sync_packet(target_guid)
                {
                    broadcaster.send_direct(move_packet);
                }
            }
        }

        Ok(())
    }

    /// Send reverse CREATE_OBJECT2 (so appeared players can see us)
    fn send_reverse_create_objects(
        &self,
        viewer_guid: ObjectGuid,
        targets: &[ObjectGuid],
        world: &World,
    ) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }

        // Filter to only player targets (creatures don't need reverse creates)
        // and check deduplication: skip if target already has viewer in objects_created
        let targets_to_send: Vec<ObjectGuid> = targets
            .iter()
            .filter(|&&target_guid| {
                // Only send to players
                if !target_guid.is_player() {
                    return false;
                }

                // Check deduplication on target's objects_created
                let should_send = world
                    .managers
                    .player_mgr
                    .with_player_mut(target_guid, |target| {
                        if target.visibility.objects_created.contains(&viewer_guid) {
                            false // Target already has viewer, skip
                        } else {
                            target.visibility.objects_created.insert(viewer_guid);
                            true
                        }
                    })
                    .unwrap_or(false);

                if !should_send {
                    tracing::warn!(
                        "[VISIBILITY] Prevented duplicate reverse CREATE_OBJECT2: {:?} -> {:?}",
                        viewer_guid,
                        target_guid
                    );
                }
                should_send
            })
            .copied()
            .collect();

        if targets_to_send.is_empty() {
            return Ok(());
        }

        // Build our create message once
        let Some(our_create_msg) = world
            .managers
            .player_mgr
            .build_create_msg(viewer_guid, world)
        else {
            return Ok(());
        };

        let packet = our_create_msg.to_world_packet();

        for &target_guid in &targets_to_send {
            if let Some(target_broadcaster) = world.managers.player_mgr.get_broadcaster(target_guid)
            {
                let mut v2_packet = WorldPacket::new(packet.opcode());
                v2_packet.write_bytes(packet.contents());

                tracing::debug!(
                    "[VISIBILITY] Sending reverse CREATE_OBJECT2: {:?} -> {:?}",
                    viewer_guid,
                    target_guid
                );

                target_broadcaster.send_direct(v2_packet);
            }
        }

        Ok(())
    }

    /// Send batched OUT_OF_RANGE for multiple objects
    fn send_batched_out_of_range(
        &self,
        viewer_guid: ObjectGuid,
        targets: &[ObjectGuid],
        world: &World,
    ) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }

        // Remove disappeared objects from objects_created (so they can be re-created later)
        world
            .managers
            .player_mgr
            .with_player_mut(viewer_guid, |player| {
                for &target in targets {
                    player.visibility.objects_created.remove(&target);
                }
            });

        let viewer_broadcaster = world.managers.player_mgr.get_broadcaster(viewer_guid);
        let Some(broadcaster) = viewer_broadcaster else {
            return Ok(());
        };

        // Build single SMSG_OUT_OF_RANGE with all GUIDs
        let world_guids: Vec<WorldObjectGuid> = targets
            .iter()
            .map(|g| WorldObjectGuid::from_raw(g.raw()))
            .collect();

        let msg = SmsgOutOfRange::new(world_guids);
        let packet = msg.to_world_packet();
        let mut v2_packet = WorldPacket::new(packet.opcode());
        v2_packet.write_bytes(packet.contents());

        // Diagnostic: log when creatures in combat are sent out of range
        for &target in targets {
            if !target.is_player() {
                if let Some(in_combat) = world
                    .managers
                    .creature_mgr
                    .with_creature(target, |c| c.combat.in_combat)
                {
                    if in_combat {
                        tracing::debug!(
                            "[VISIBILITY] Sending OUT_OF_RANGE for creature {:?} that is IN COMBAT! viewer={:?}",
                            target, viewer_guid
                        );
                    }
                }
            }
        }

        tracing::debug!(
            "[VISIBILITY] Sending batched OUT_OF_RANGE to {:?}: {} targets",
            viewer_guid,
            targets.len()
        );

        broadcaster.send_direct(v2_packet);

        Ok(())
    }

    /// Update bidirectional listener relationships
    fn update_bidirectional_listeners(
        &self,
        viewer_guid: ObjectGuid,
        appeared: &[ObjectGuid],
        disappeared: &[ObjectGuid],
        world: &World,
    ) {
        let viewer_broadcaster = world.managers.player_mgr.get_broadcaster(viewer_guid);

        // Add listeners for appeared objects
        for &target_guid in appeared {
            if let (Some(viewer_bc), Some(target_bc)) = (
                viewer_broadcaster.clone(),
                world.managers.player_mgr.get_broadcaster(target_guid),
            ) {
                viewer_bc.add_listener(target_guid, target_bc.clone());
                target_bc.add_listener(viewer_guid, viewer_bc);
            }
        }

        // Remove listeners for disappeared objects
        for &target_guid in disappeared {
            if let (Some(viewer_bc), Some(target_bc)) = (
                viewer_broadcaster.clone(),
                world.managers.player_mgr.get_broadcaster(target_guid),
            ) {
                viewer_bc.remove_listener(target_guid);
                target_bc.remove_listener(viewer_guid);
            }
        }
    }

    /// Handle player logout - immediately send OUT_OF_RANGE to all observers
    /// This must be called BEFORE the player is removed from map/PlayerManager
    pub async fn on_player_logout(&self, guid: ObjectGuid, world: &World) -> Result<()> {
        // 1. Get observers from the logging-out player's visible_objects set
        let observers: Vec<ObjectGuid> = world
            .managers
            .player_mgr
            .with_player_mut(guid, |player| {
                player.visibility.visible_objects.iter().copied().collect()
            })
            .unwrap_or_default();

        if observers.is_empty() {
            // Still clean up broadcaster even with no observers
            if let Some(broadcaster) = world.managers.player_mgr.get_broadcaster(guid) {
                broadcaster.free_at_logout();
            }
            return Ok(());
        }

        tracing::debug!(
            "[VISIBILITY] Player {:?} logout: notifying {} observers",
            guid,
            observers.len()
        );

        // 2. For each observer: queue disappear and clean up references
        for observer_guid in &observers {
            // Queue the logged-out player for removal on next visibility flush
            // (avoids sending packet immediately - they'll disappear on next update)
            world
                .managers
                .player_mgr
                .with_player_mut(*observer_guid, |observer| {
                    observer.visibility.visible_objects.remove(&guid);
                    observer.visibility.pending_disappeared.push(guid);
                });

            // Remove from observer's broadcaster listeners
            if let Some(observer_bc) = world.managers.player_mgr.get_broadcaster(*observer_guid) {
                observer_bc.remove_listener(guid);
            }
        }

        // 3. Clean up logging-out player's broadcaster
        if let Some(broadcaster) = world.managers.player_mgr.get_broadcaster(guid) {
            broadcaster.free_at_logout();
        }

        Ok(())
    }
}

impl Default for VisibilitySubsystem {
    fn default() -> Self {
        Self::new()
    }
}
