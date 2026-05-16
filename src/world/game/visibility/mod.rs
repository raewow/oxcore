//! Visibility System - tracks what each player can see

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use crate::shared::messages::create::SmsgOutOfRange;
use crate::shared::messages::update::SmsgUpdateObject;
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Position};
use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
use crate::world::World;

/// Visibility system - tracks what objects each player can see
pub struct VisibilitySystem {
    /// Objects visible to each player
    visible_objects: DashMap<ObjectGuid, HashSet<ObjectGuid>>,
}

impl VisibilitySystem {
    pub fn new() -> Self {
        Self {
            visible_objects: DashMap::new(),
        }
    }

    /// Get visible objects for a player
    pub fn get_visible(&self, guid: ObjectGuid) -> HashSet<ObjectGuid> {
        self.visible_objects
            .get(&guid)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    /// Check if target is visible to viewer
    pub fn can_see(&self, viewer: ObjectGuid, target: ObjectGuid) -> bool {
        self.visible_objects
            .get(&viewer)
            .map(|v| v.contains(&target))
            .unwrap_or(false)
    }

    /// Add object to player's visible set
    pub fn add_visible(&self, viewer: ObjectGuid, target: ObjectGuid) {
        self.visible_objects
            .entry(viewer)
            .or_insert_with(HashSet::new)
            .insert(target);
    }

    /// Remove object from player's visible set
    pub fn remove_visible(&self, viewer: ObjectGuid, target: ObjectGuid) {
        if let Some(mut visible) = self.visible_objects.get_mut(&viewer) {
            visible.remove(&target);
        }
    }

    /// Update visibility for a player (call after movement or on login)
    /// This function establishes bidirectional visibility and listener relationships
    /// between the player and nearby objects.
    ///
    /// CRITICAL: Must be called BEFORE session state is set to LoggedIn to avoid race conditions
    pub async fn update_visibility_for_player(
        &self,
        guid: ObjectGuid,
        world: &World,
    ) -> Result<(Vec<ObjectGuid>, Vec<ObjectGuid>)> {
        // 1. Get player and position from MovementSystem (sole authority)
        let player = world
            .managers
            .player_mgr
            .get_player(guid)
            .ok_or_else(|| anyhow!("Player not found: {:?}", guid))?;
        let map_id = player.map_id;
        let instance_id = player.instance_id;

        // Debug assertion: Verify player has a broadcaster before visibility update
        debug_assert!(
            world.managers.player_mgr.get_broadcaster(guid).is_some(),
            "Player {:?} must have a broadcaster before visibility update",
            guid
        );

        let pos = world
            .managers
            .player_mgr
            .get_position(guid)
            .ok_or_else(|| anyhow!("Player {:?} has no position", guid))?;

        tracing::debug!(
            "[VISIBILITY] Starting visibility update for {:?} at ({:.2}, {:.2}, {:.2})",
            guid,
            pos.x,
            pos.y,
            pos.z
        );

        // 2. Get map and query nearby PLAYERS only
        // NOTE: Creatures are handled separately by CreatureManager.send_nearby_creatures()
        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);
        let all_nearby = map.get_objects_in_range(pos, map.visibility_distance());

        // Filter to players only (exclude self)
        let visible_now: HashSet<ObjectGuid> = all_nearby
            .into_iter()
            .filter(|g| g.is_player() && *g != guid)
            .collect();

        // 3. Compare with previous visible set
        let previous = self.get_visible(guid);

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

        // 4. Send CREATE_OBJECT2 for appeared PLAYERS
        // NOTE: visible_now is already filtered to players only
        if !appeared.is_empty() {
            use crate::shared::messages::ToWorldPacket;
            use crate::shared::protocol::WorldPacket;

            let viewer_broadcaster = world.managers.player_mgr.get_broadcaster(guid);

            tracing::debug!(
                "[VISIBILITY] Player {:?} sees {} new players",
                guid,
                appeared.len()
            );

            for &target_guid in &appeared {
                // Send CREATE_OBJECT2 to viewer (viewer sees target player)
                if let Some(create_msg) = world
                    .managers
                    .player_mgr
                    .build_create_msg(target_guid, world)
                {
                    if let Some(ref viewer_bc) = viewer_broadcaster {
                        let packet = create_msg.to_world_packet();
                        let mut v2_packet = WorldPacket::new(packet.opcode());
                        v2_packet.write_bytes(packet.contents());
                        viewer_bc.send_direct(v2_packet);
                    }
                }

                // Send CREATE_OBJECT2 to target (target sees viewer) - bidirectional
                if let Some(viewer_create_msg) =
                    world.managers.player_mgr.build_create_msg(guid, world)
                {
                    if let Some(target_bc) = world.managers.player_mgr.get_broadcaster(target_guid)
                    {
                        let packet = viewer_create_msg.to_world_packet();
                        let mut v2_packet = WorldPacket::new(packet.opcode());
                        v2_packet.write_bytes(packet.contents());
                        target_bc.send_direct(v2_packet);
                    }
                }

                // Add bidirectional listeners
                if let (Some(viewer_bc), Some(target_bc)) = (
                    world.managers.player_mgr.get_broadcaster(guid),
                    world.managers.player_mgr.get_broadcaster(target_guid),
                ) {
                    viewer_bc.add_listener(target_guid, Arc::clone(&target_bc));
                    target_bc.add_listener(guid, Arc::clone(&viewer_bc));
                }
            }
        }

        // 5. Send OUT_OF_RANGE for disappeared players and remove listeners
        if !disappeared.is_empty() {
            use crate::shared::messages::ToWorldPacket;
            use crate::shared::protocol::WorldPacket;

            let viewer_broadcaster = world.managers.player_mgr.get_broadcaster(guid);

            tracing::debug!(
                "[VISIBILITY] Player {:?} lost sight of {} players",
                guid,
                disappeared.len()
            );

            for &target_guid in &disappeared {
                // Remove bidirectional listeners
                if let (Some(viewer_bc), Some(target_bc)) = (
                    world.managers.player_mgr.get_broadcaster(guid),
                    world.managers.player_mgr.get_broadcaster(target_guid),
                ) {
                    viewer_bc.remove_listener(target_guid);
                    target_bc.remove_listener(guid);
                }

                // Send OUT_OF_RANGE to viewer
                if let Some(ref viewer_bc) = viewer_broadcaster {
                    let world_guid = WorldObjectGuid::from_raw(target_guid.raw());
                    let msg = SmsgOutOfRange::new(vec![world_guid]);
                    let packet = msg.to_world_packet();
                    let mut v2_packet = WorldPacket::new(packet.opcode());
                    v2_packet.write_bytes(packet.contents());
                    viewer_bc.send_direct(v2_packet);
                }
            }
        }

        // 6. Update visible set
        self.visible_objects.insert(guid, visible_now.clone());

        tracing::debug!(
            "[VISIBILITY] Visibility update complete for {:?}: total_visible={}, appeared={}, disappeared={}",
            guid,
            visible_now.len(),
            appeared.len(),
            disappeared.len()
        );

        Ok((appeared, disappeared))
    }
}

impl Default for VisibilitySystem {
    fn default() -> Self {
        Self::new()
    }
}

impl VisibilitySystem {
    pub async fn init(&self) -> Result<()> {
        Ok(())
    }

    pub fn update(&self, _diff: Duration) -> Result<()> {
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.visible_objects.clear();

        Ok(())
    }

    pub fn on_player_login(
        &self,
        guid: ObjectGuid,
        position: Option<crate::shared::protocol::Position>,
    ) -> Result<()> {
        // Create visibility tracking for this player
        self.visible_objects.insert(guid, HashSet::new());

        // Log initialization with position if provided
        if let Some(pos) = position {
            tracing::debug!(
                "[VisibilitySystem] Initialized visibility tracking for player {:?} at ({:.2}, {:.2}, {:.2})",
                guid, pos.x, pos.y, pos.z
            );
        } else {
            tracing::debug!(
                "[VisibilitySystem] Initialized visibility tracking for player {:?} (no position provided)",
                guid
            );
        }

        Ok(())
    }

    pub fn on_player_logout(&self, guid: ObjectGuid) -> Result<()> {
        self.visible_objects.remove(&guid);
        Ok(())
    }
}
