use crate::shared::protocol::ObjectGuid;
use crate::world::World;
use anyhow::Result;
use std::time::Duration;

use super::fall;
use super::mirror_timers;
use super::rest;
use super::state::*;

/// EnvironmentSystem manages rest XP, mirror timers, and environmental hazards
/// for all online players.
pub struct EnvironmentSystem {
    offline_rate_multiplier: f32,
}

impl EnvironmentSystem {
    pub fn new() -> Self {
        Self {
            offline_rate_multiplier: 1.0, // TODO: Read from world config
        }
    }

    /// Main update tick for the environment system.
    ///
    /// Called every world tick (50ms). Iterates over all online players and:
    /// 1. Updates rest bonus accumulation for players in rest areas
    /// 2. Refreshes environment flags from terrain data
    /// 3. Ticks mirror timers and applies damage pulses
    pub fn update(
        &self,
        diff: Duration,
        _world: &World,
        player_mgr: &crate::world::game::player::PlayerManager,
    ) -> Result<()> {
        let diff_ms = diff.as_millis() as u32;

        player_mgr.for_each_player(|_guid, player| {
            // 1. Update rest bonus
            if player.environment.rest_type != RestType::No {
                rest::update_rest_bonus(&mut player.environment, diff_ms, player.next_level_xp);
            }

            // 2. Update environment flags from terrain
            // (done in movement handler on position change, not every tick)

            // 3. Update mirror timers
            // Note: has_water_breathing and is_taxi_flying would need to be
            // determined from player state - simplified for now
            let has_water_breathing = false; // TODO: Check auras
            let is_flying = false; // TODO: Check movement flags
            let is_transport = false; // TODO: Check transport
            let is_alive = true; // TODO: Check death state
            let is_ghost = false; // TODO: Check death state

            let events = mirror_timers::update_mirror_timers(
                &mut player.environment,
                diff_ms,
                is_alive,
                is_ghost,
                is_flying,
                is_transport,
                has_water_breathing,
            );

            // 4. Process timer events
            for event in events {
                match event {
                    mirror_timers::MirrorTimerEvent::DamagePulse(timer_type) => {
                        let action = mirror_timers::on_mirror_timer_expiration_pulse(
                            timer_type,
                            player.stats.max_health,
                            player.level,
                            is_alive,
                            is_ghost,
                            player.environment.env_flags,
                        );

                        match action {
                            mirror_timers::MirrorTimerAction::Damage {
                                damage_type: _,
                                amount: _,
                            } => {
                                // TODO: Apply environmental damage
                                // This would need to be done through a damage system
                            }
                            mirror_timers::MirrorTimerAction::TeleportToGraveyard => {
                                // TODO: Teleport ghost to graveyard
                            }
                            mirror_timers::MirrorTimerAction::None => {}
                        }
                    }
                    mirror_timers::MirrorTimerEvent::Started(_) => {
                        // Network update handled by send_mirror_timers
                    }
                }
            }

            // 5. Send mirror timer network updates (only on state changes)
            self.send_mirror_timers(player, false);
        });

        Ok(())
    }

    /// Set the player's rest type (enter/exit inn or city).
    ///
    /// Called from area trigger handlers when the player enters or exits
    /// an inn area, or from zone update handlers for city rest.
    pub fn set_rest_type(
        &self,
        player_guid: ObjectGuid,
        rest_type: RestType,
        trigger_id: u32,
        player_mgr: &crate::world::game::player::PlayerManager,
    ) -> Result<()> {
        player_mgr.with_player_mut(player_guid, |player| {
            rest::set_rest_type(
                &mut player.environment,
                rest_type,
                trigger_id,
                &mut player.player_flags,
            );
        });
        Ok(())
    }

    /// Apply environmental damage to a player.
    ///
    /// Central routing point for fall damage, drowning, fatigue, lava, fire
    /// and slime. Responsibilities:
    ///   1. Send `SMSG_ENVIRONMENTALDAMAGELOG` to the client (combat log + HUD).
    ///   2. Subtract damage from the player's health, saturating at 0.
    ///   3. If the player dies, hand off to `DeathSystem::on_killed` (no killer).
    ///
    /// Returns the actual damage applied (may be less than `amount` if the
    /// player had less health remaining).
    pub fn environmental_damage(
        &self,
        player_guid: ObjectGuid,
        dmg_type: EnvironmentalDamageType,
        amount: u32,
        world: &World,
    ) -> u32 {
        use crate::shared::protocol::{Opcode, WorldPacket};

        // Immunity: don't damage already-dead players.
        let is_alive = world
            .systems
            .player
            .manager()
            .with_player(player_guid, |p| p.stats.health > 0)
            .unwrap_or(false);
        if !is_alive || amount == 0 {
            return 0;
        }

        // Clamp to current health so we don't report a bogus amount.
        let applied = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                let applied = amount.min(player.stats.health);
                player.stats.health = player.stats.health.saturating_sub(applied);
                applied
            })
            .unwrap_or(0);

        // SMSG_ENVIRONMENTALDAMAGELOG — format:
        //   guid: u64
        //   type: u8
        //   dmg:  u32
        //   absorb: u32 (0 — absorbs are computed upstream)
        //   resist: u32 (0)
        let mut pkt = WorldPacket::new(Opcode::SMSG_ENVIRONMENTALDAMAGELOG);
        pkt.write_u64(player_guid.raw());
        pkt.write_u8(dmg_type as u8);
        pkt.write_u32(applied);
        pkt.write_u32(0);
        pkt.write_u32(0);
        if let Some(session) = world.session_mgr.get_session_by_player(player_guid) {
            let _ = session.send_packet(pkt);
        }

        // If that kill was fatal, trigger the death flow (no killer).
        let now_dead = world
            .systems
            .player
            .manager()
            .with_player(player_guid, |p| p.stats.health == 0)
            .unwrap_or(false);
        if now_dead {
            if let Err(e) = world.systems.death.on_killed(player_guid, None, None, world) {
                tracing::warn!(
                    "environmental death handling failed for {:?}: {}",
                    player_guid, e
                );
            }
        }

        applied
    }

    /// Called when a player logs in. Restores rest state and calculates
    /// offline rest XP accumulation.
    pub fn on_player_login(
        &self,
        guid: ObjectGuid,
        saved_rest_bonus: f32,
        saved_rest_type: RestType,
        logout_timestamp: u64,
        player_mgr: &crate::world::game::player::PlayerManager,
    ) -> Result<()> {
        player_mgr.with_player_mut(guid, |player| {
            rest::on_player_login(
                &mut player.environment,
                saved_rest_bonus,
                saved_rest_type,
                logout_timestamp,
                player.next_level_xp,
                self.offline_rate_multiplier,
            );

            // Send initial mirror timer state (forced full update)
            self.send_mirror_timers(player, true);
        });
        Ok(())
    }

    /// Handle an area trigger event (player entering an inn).
    ///
    /// Called from the area trigger handler after it has already checked
    /// that the trigger is a tavern via `AreaTriggerManager::is_tavern()`.
    pub fn on_area_trigger(
        &self,
        player_guid: ObjectGuid,
        trigger_id: u32,
        world: &World,
        player_mgr: &crate::world::game::player::PlayerManager,
    ) -> Result<()> {
        let is_tavern = world.managers.area_trigger_mgr.is_tavern(trigger_id);

        if is_tavern {
            self.set_rest_type(player_guid, RestType::InTavern, trigger_id, player_mgr)?;
        }

        Ok(())
    }

    /// Handle player landing after a fall.
    ///
    /// Called from the movement handler when MSG_MOVE_FALL_LAND is received.
    pub fn on_fall_landing(
        &self,
        player_guid: ObjectGuid,
        fall_distance: f32,
        player_mgr: &crate::world::game::player::PlayerManager,
    ) -> u32 {
        let mut damage = 0u32;

        player_mgr.with_player_mut(player_guid, |player| {
            // TODO: Get these values from player state
            let is_alive = true; // TODO: Check death state
            let is_taxi_flying = false; // TODO: Check movement flags
            let is_game_master = false; // TODO: Add GM check
            let has_fly_aura = false; // TODO: Check auras
            let max_health = player.stats.max_health;
            let safe_fall_bonus = 0.0f32; // TODO: Get from Safe Fall aura

            damage = fall::handle_fall_landing(
                is_alive,
                is_taxi_flying,
                is_game_master,
                has_fly_aura,
                fall_distance,
                max_health,
                safe_fall_bonus,
            );

            if damage > 0 {
                // TODO: Apply environmental damage
                // This would need to be done through a damage system
            }
        });

        damage
    }

    /// Update environment flags from terrain data.
    ///
    /// Called when player position changes to update liquid/environment state.
    pub fn update_environment_flags(
        &self,
        player_guid: ObjectGuid,
        liquid_status: &LiquidStatus,
        player_z: f32,
        player_mgr: &crate::world::game::player::PlayerManager,
    ) {
        player_mgr.with_player_mut(player_guid, |player| {
            update_environment_flags_internal(
                &mut player.environment.env_flags,
                liquid_status,
                player_z,
            );
        });
    }

    /// Send mirror timer packets for state changes.
    ///
    /// Iterates over client-visible timers and sends start/stop packets
    /// only when the timer status has changed since the last send.
    fn send_mirror_timers(&self, player: &mut crate::world::game::player::Player, forced: bool) {
        use crate::shared::messages::environment::{SmsgStartMirrorTimer, SmsgStopMirrorTimer};
        use crate::shared::messages::ToWorldPacket;

        for timer_type in [
            MirrorTimerType::Fatigue,
            MirrorTimerType::Breath,
            MirrorTimerType::FeignDeath,
        ] {
            if !timer_type.is_client_timer() {
                continue;
            }

            let timer = match timer_type {
                MirrorTimerType::Fatigue => &mut player.environment.fatigue_timer,
                MirrorTimerType::Breath => &mut player.environment.breath_timer,
                _ => continue,
            };

            let mut status = timer.fetch_status();
            if forced && timer.active {
                status = MirrorTimerStatus::FullUpdate;
            }

            match status {
                MirrorTimerStatus::FullUpdate => {
                    let msg = SmsgStartMirrorTimer {
                        timer_type: timer_type as u32,
                        current: timer.remaining(),
                        max: timer.max_ms,
                        scale: timer.scale,
                        paused: if timer.frozen { 1 } else { 0 },
                        spell_id: timer.spell_id,
                    };
                    // TODO: Send packet via broadcaster
                    // if let Some(broadcaster) = player.broadcaster() {
                    //     broadcaster.send_packet(msg.to_world_packet());
                    // }
                    let _ = msg.to_world_packet(); // Avoid unused warning for now
                }
                MirrorTimerStatus::StatusUpdate => {
                    if !timer.active {
                        let msg = SmsgStopMirrorTimer {
                            timer_type: timer_type as u32,
                        };
                        // TODO: Send packet via broadcaster
                        let _ = msg.to_world_packet(); // Avoid unused warning for now
                    } else {
                        // Client UI has a bug with pause - use full update instead
                        let msg = SmsgStartMirrorTimer {
                            timer_type: timer_type as u32,
                            current: timer.remaining(),
                            max: timer.max_ms,
                            scale: timer.scale,
                            paused: if timer.frozen { 1 } else { 0 },
                            spell_id: timer.spell_id,
                        };
                        // TODO: Send packet via broadcaster
                        let _ = msg.to_world_packet(); // Avoid unused warning for now
                    }
                }
                MirrorTimerStatus::Unchanged => {}
            }
        }
    }
}

impl Default for EnvironmentSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Liquid status from terrain/map system
#[derive(Debug, Clone)]
pub struct LiquidStatus {
    pub has_liquid: bool,
    pub liquid_type: LiquidType,
    pub depth: f32,
    pub surface_z: f32,
    pub swim_threshold: f32,
    pub is_deep_water: bool,
}

/// Types of liquid in the game world
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LiquidType {
    Water = 0,
    Magma = 1,
    Slime = 2,
}

/// Update environment flags from terrain data.
///
/// Called every player update tick. Reads the liquid info at the player's
/// current position from the map and translates it into EnvironmentFlags.
pub fn update_environment_flags_internal(
    env_flags: &mut EnvironmentFlags,
    liquid_status: &LiquidStatus,
    player_z: f32,
) {
    // Reset all liquid flags
    *env_flags &= !EnvironmentFlags::MASK_LIQUID_FLAGS;

    if !liquid_status.has_liquid {
        return;
    }

    // Set base liquid type
    *env_flags |= EnvironmentFlags::LIQUID;

    match liquid_status.liquid_type {
        LiquidType::Water => *env_flags |= EnvironmentFlags::IN_WATER,
        LiquidType::Magma => *env_flags |= EnvironmentFlags::IN_MAGMA,
        LiquidType::Slime => *env_flags |= EnvironmentFlags::IN_SLIME,
    }

    // Check depth for swimming and submersion
    if liquid_status.depth >= liquid_status.swim_threshold {
        *env_flags |= EnvironmentFlags::HIGH_LIQUID;
    }

    if player_z < liquid_status.surface_z - 1.6 {
        // Head is below surface (1.6 yards = approximate head height)
        *env_flags |= EnvironmentFlags::UNDERWATER;
    }

    // Deep water zone flag (from area table)
    if liquid_status.is_deep_water {
        *env_flags |= EnvironmentFlags::HIGH_SEA;
    }
}
