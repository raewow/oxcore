//! Player System - manages player lifecycle

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use super::manager::PlayerManager;
use super::movement::MovementSystem;
use super::visibility::VisibilitySubsystem;
use crate::shared::protocol::{ObjectGuid, Position};
use crate::world::World;

/// Player System - wraps PlayerManager with system lifecycle
pub struct PlayerSystem {
    manager: Arc<PlayerManager>,
    movement: MovementSystem,
    visibility: VisibilitySubsystem,
}

impl PlayerSystem {
    pub fn new(manager: Arc<PlayerManager>) -> Self {
        Self {
            manager,
            movement: MovementSystem::new(),
            visibility: VisibilitySubsystem::new(),
        }
    }

    /// Get the underlying PlayerManager
    pub fn manager(&self) -> Arc<PlayerManager> {
        Arc::clone(&self.manager)
    }

    /// Get movement subsystem
    pub fn movement(&self) -> &MovementSystem {
        &self.movement
    }

    /// Get visibility subsystem
    pub fn visibility(&self) -> &VisibilitySubsystem {
        &self.visibility
    }

    /// Initialize system
    pub async fn init(&self) -> Result<()> {
        Ok(())
    }

    /// Update system (global player updates)
    pub fn update(&self, _diff: Duration) -> Result<()> {
        // Global player updates (if any) - currently none
        Ok(())
    }

    /// Update visibility for a specific player (sync - just calculates)
    /// Called from map update loop Phase 1
    pub fn update_player_visibility(
        &self,
        player_guid: ObjectGuid,
        current_tick: u32,
        world: &World,
    ) -> Result<bool> {
        self.visibility
            .update_player(player_guid, current_tick, world)
    }

    /// Update a specific player and flush visibility notifications (async)
    /// Called from map update loop Phase 2
    pub async fn update_player_async(
        &self,
        player_guid: ObjectGuid,
        diff: Duration,
        world: &World,
    ) -> Result<()> {
        // Flush pending visibility notifications (sends packets)
        self.visibility
            .flush_pending_notifications(player_guid, world)
            .await?;

        // Update movement subsystem for this player
        self.movement.update_player(player_guid, diff, world)?;

        // Update combat (auto-attack swing timers and pending attacks)
        self.update_player_combat(player_guid, diff, world).await?;

        // Future: update other subsystems
        // self.auras.update_player(player_guid, diff, world)?;

        Ok(())
    }

    /// Update combat for a specific player - processes swing timers and executes attacks
    async fn update_player_combat(
        &self,
        player_guid: ObjectGuid,
        diff: Duration,
        world: &World,
    ) -> Result<()> {
        use crate::shared::protocol::{Opcode, WorldPacket};
        use crate::world::game::combat::melee_range::{self, DEFAULT_COMBAT_REACH};

        let diff_ms = diff.as_millis() as u32;

        // Update auto-attack timers, get any pending attacks
        let pending_attacks = world.systems.combat.update_player_auto_attack(
            player_guid,
            diff_ms,
            &world.managers.player_mgr,
        );

        if pending_attacks.is_empty() {
            return Ok(());
        }

        // Get player position for range checks
        let player_pos = world
            .managers
            .player_mgr
            .get_position(player_guid)
            .unwrap_or_default();

        // Check if player is moving (for leeway calculation)
        // Movement flags bits 0-3: forward, backward, strafe left, strafe right
        let player_moving = world
            .managers
            .player_mgr
            .with_player(player_guid, |p| p.movement.movement_flags & 0x0F != 0)
            .unwrap_or(false);

        // Execute each pending attack
        for attack in &pending_attacks {
            if attack.target.is_unit() && !attack.target.is_player() {
                let target_pos = world
                    .managers
                    .creature_mgr
                    .get_position(attack.target)
                    .unwrap_or_default();
                let target_reach = world.managers.creature_mgr.get_combat_reach(attack.target);

                // Check if target is moving (for leeway)
                let target_moving = world
                    .managers
                    .creature_mgr
                    .with_creature(attack.target, |c| c.motion_master.is_moving())
                    .unwrap_or(false);

                let both_moving = player_moving && target_moving;

                if !melee_range::is_within_melee_range(
                    &player_pos,
                    DEFAULT_COMBAT_REACH,
                    &target_pos,
                    target_reach,
                    both_moving,
                ) {
                    // Out of range: check and update error state, determine if packet should be sent
                    // DO NOT send packet while holding player lock to avoid deadlock!
                    let should_send_notinrange = world
                        .managers
                        .player_mgr
                        .with_player_mut(player_guid, |p| {
                            let should_send = p.combat.last_swing_error != 1;
                            if should_send {
                                p.combat.last_swing_error = 1;
                            }
                            // Delay auto-attacks by 100ms (vmangos DelayAutoAttacks)
                            p.combat.main_hand_timer = 100;
                            should_send
                        })
                        .unwrap_or(false);

                    // Send packet OUTSIDE the player lock to avoid deadlock
                    if should_send_notinrange {
                        let notinrange_packet =
                            WorldPacket::new(Opcode::SMSG_ATTACKSWING_NOTINRANGE);
                        world
                            .managers
                            .broadcast_mgr
                            .send_to_player(player_guid, notinrange_packet);
                    }
                    continue;
                }

                // In range - clear last error and execute attack
                // Keep this quick - just update state, no packet sending while holding lock
                world.managers.player_mgr.with_player_mut(player_guid, |p| {
                    p.combat.last_swing_error = 0;
                });

                let target_died =
                    crate::world::handlers::creature_combat::execute_pending_attack_vs_creature(
                        world,
                        attack.attacker,
                        attack.target,
                    )
                    .await?;

                if target_died {
                    // Stop auto-attack
                    world
                        .systems
                        .combat
                        .stop_attack(player_guid, &world.managers.player_mgr)
                        .await?;
                    break;
                }
            }
        }

        Ok(())
    }

    /// Shutdown system
    pub async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    /// Player login notification
    pub async fn on_player_login(&self, _guid: ObjectGuid) -> Result<()> {
        // No action needed (PlayerManager handles this)
        Ok(())
    }

    /// Player logout notification
    pub async fn on_player_logout(&self, guid: ObjectGuid, world: &World) -> Result<()> {
        // Notify visibility subsystem to handle observer cleanup
        self.visibility.on_player_logout(guid, world).await?;
        Ok(())
    }
}
