use crate::World;
use anyhow::Result;
use oxcore_map::terrain::{LiquidData, LiquidStatusFlags};
use oxcore_shared::protocol::ObjectGuid;
use std::time::Duration;

use super::fall;
use super::mirror_timers;
use super::rest;
use super::state::*;

/// Fallback model collision height, used when the player has no model data.
/// Matches `Unit::m_modelCollisionHeight`'s initial value.
pub const DEFAULT_COLLISION_HEIGHT: f32 = 2.0;

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
        world: &World,
        player_mgr: &crate::game::player::PlayerManager,
    ) -> Result<()> {
        use crate::game::player::auras::effects::AURA_WATER_BREATHING;
        use oxcore_shared::protocol::MoveFlags;

        let diff_ms = diff.as_millis() as u32;

        // Damage is applied after the player loop: `environmental_damage` needs
        // the manager, which is borrowed for the duration of `for_each_player`.
        let mut pending_damage: Vec<(ObjectGuid, EnvironmentalDamageType, u32)> = Vec::new();
        let mut pending_graveyard: Vec<ObjectGuid> = Vec::new();
        let mut pending_packets: Vec<(ObjectGuid, oxcore_shared::protocol::WorldPacket)> =
            Vec::new();

        player_mgr.for_each_player(|guid, player| {
            // 1. Update rest bonus
            if player.environment.rest_type != RestType::No {
                rest::update_rest_bonus(&mut player.environment, diff_ms, player.next_level_xp);
            }

            // 2. Environment flags are refreshed from terrain on position change
            // (see `environment::liquid::update_player_liquid_status`), not here.

            // 3. Update mirror timers
            let has_water_breathing = player.auras.container.has_aura_type(AURA_WATER_BREATHING);
            let move_flags = MoveFlags::new(player.movement.movement_flags);
            let is_flying = move_flags.has_flag(MoveFlags::FLYING);
            let is_transport = player.movement.transport_guid.is_some();
            let is_alive = player.stats.health > 0;
            let is_ghost =
                !is_alive && player.player_flags & crate::game::player::PLAYER_FLAGS_GHOST != 0;

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
                                damage_type,
                                amount,
                            } => pending_damage.push((guid, damage_type, amount)),
                            mirror_timers::MirrorTimerAction::TeleportToGraveyard => {
                                pending_graveyard.push(guid)
                            }
                            mirror_timers::MirrorTimerAction::None => {}
                        }
                    }
                    mirror_timers::MirrorTimerEvent::Started(_) => {
                        // Network update handled by collect_mirror_timer_packets
                    }
                }
            }

            // 5. Collect mirror timer network updates (only on state changes)
            self.collect_mirror_timer_packets(guid, player, false, &mut pending_packets);
        });

        for (guid, packet) in pending_packets {
            if let Some(session) = world.session_mgr.get_session_by_player(guid) {
                let _ = session.send_packet(packet);
            }
        }

        for (guid, damage_type, amount) in pending_damage {
            self.environmental_damage(guid, damage_type, amount, world);
        }

        for guid in pending_graveyard {
            // A ghost that drifts into the fatigue zone is pulled back to land.
            if let Err(e) = world.systems.death.repop_at_graveyard(guid, world) {
                tracing::warn!("fatigue graveyard teleport failed for {:?}: {}", guid, e);
            }
        }

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
        player_mgr: &crate::game::player::PlayerManager,
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
        use oxcore_shared::protocol::{Opcode, WorldPacket};

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

        // Apply damage via modify_health which clamps to [0, max_health].
        let applied = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                let loss = player.stats.modify_health(-(amount as i32));
                (-loss) as u32
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
            if let Err(e) = world
                .systems
                .death
                .on_killed(player_guid, None, None, world)
            {
                tracing::warn!(
                    "environmental death handling failed for {:?}: {}",
                    player_guid,
                    e
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
        world: &World,
        player_mgr: &crate::game::player::PlayerManager,
    ) -> Result<()> {
        let mut packets = Vec::new();

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
            self.collect_mirror_timer_packets(guid, player, true, &mut packets);
        });

        for (guid, packet) in packets {
            if let Some(session) = world.session_mgr.get_session_by_player(guid) {
                let _ = session.send_packet(packet);
            }
        }

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
        player_mgr: &crate::game::player::PlayerManager,
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
        player_mgr: &crate::game::player::PlayerManager,
    ) -> u32 {
        let mut damage = 0u32;

        player_mgr.with_player_mut(player_guid, |player| {
            // TODO: Get these values from player state
            let is_alive = true; // TODO: Check death state
            let is_taxi_flying = false; // TODO: Check movement flags
            let is_game_master = false; // TODO: Add GM check
            let has_fly_aura = false; // TODO: Check auras
            let max_health = player.stats.max_health;
            let safe_fall_bonus = player
                .auras
                .container
                .get_total_aura_modifier(crate::game::player::auras::effects::AURA_SAFE_FALL)
                .max(0) as f32;

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

    /// Update environment flags from the liquid at the player's position.
    ///
    /// Called when the player's position changes. Also pushes any resulting
    /// mirror timer packets (the breath and fatigue bars) to the client.
    pub fn update_environment_flags(
        &self,
        player_guid: ObjectGuid,
        liquid_status: &LiquidStatus,
        player_z: f32,
        world: &World,
        player_mgr: &crate::game::player::PlayerManager,
    ) {
        use crate::game::player::auras::effects::AURA_WATER_BREATHING;

        let mut packets = Vec::new();

        player_mgr.with_player_mut(player_guid, |player| {
            let has_water_breathing = player.auras.container.has_aura_type(AURA_WATER_BREATHING);

            update_environment_flags_internal(
                &mut player.environment,
                liquid_status,
                player_z,
                DEFAULT_COLLISION_HEIGHT,
                has_water_breathing,
            );

            self.collect_mirror_timer_packets(player_guid, player, false, &mut packets);
        });

        for (guid, packet) in packets {
            if let Some(session) = world.session_mgr.get_session_by_player(guid) {
                let _ = session.send_packet(packet);
            }
        }
    }

    /// Collect mirror timer packets for state changes.
    ///
    /// Iterates over client-visible timers and emits start/stop packets only
    /// when the timer status has changed since the last send. Packets are
    /// collected rather than sent directly so callers can drop the player borrow
    /// before touching the session manager.
    fn collect_mirror_timer_packets(
        &self,
        player_guid: ObjectGuid,
        player: &mut crate::game::player::Player,
        forced: bool,
        out: &mut Vec<(ObjectGuid, oxcore_shared::protocol::WorldPacket)>,
    ) {
        use oxcore_shared::messages::environment::{SmsgStartMirrorTimer, SmsgStopMirrorTimer};
        use oxcore_shared::messages::ToWorldPacket;

        for timer_type in [MirrorTimerType::Fatigue, MirrorTimerType::Breath] {
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

            let start_packet = || {
                SmsgStartMirrorTimer {
                    timer_type: timer_type as u32,
                    current: timer.remaining(),
                    max: timer.max_ms,
                    scale: timer.scale,
                    paused: if timer.frozen { 1 } else { 0 },
                    spell_id: timer.spell_id,
                }
                .to_vanilla()
            };

            match status {
                MirrorTimerStatus::FullUpdate => out.push((player_guid, start_packet())),
                MirrorTimerStatus::StatusUpdate => {
                    if timer.active {
                        // The client's pause handling is unreliable, so resend a
                        // full update instead of a pause notification.
                        out.push((player_guid, start_packet()));
                    } else {
                        out.push((
                            player_guid,
                            SmsgStopMirrorTimer {
                                timer_type: timer_type as u32,
                            }
                            .to_vanilla(),
                        ));
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

/// The liquid at a position: where it sits relative to the surface, plus the
/// liquid's kind and heights.
///
/// Produced by `environment::liquid::query_liquid_status` from terrain (ADT) and
/// VMap (WMO) data.
#[derive(Debug, Clone, Copy, Default)]
pub struct LiquidStatus {
    /// Where the position sits relative to the surface.
    pub status: LiquidStatusFlags,
    /// The liquid's kind, surface height, and floor height.
    pub data: LiquidData,
}

impl LiquidStatus {
    /// No liquid at this position.
    pub fn none() -> Self {
        Self {
            status: LiquidStatusFlags::NO_WATER,
            data: LiquidData::default(),
        }
    }

    pub fn has_liquid(&self) -> bool {
        !self.status.is_empty()
    }
}

/// Update a player's environment flags from the liquid at their position.
///
/// Ported from `Player::UpdateTerainEnvironmentFlags`. Each flag has its own
/// condition rather than being derived from a single liquid kind, because the
/// thresholds differ: you burn in lava while merely standing on it, but only
/// drown once your head is under the surface.
///
/// `collision_height` is the player's model collision height; the swim threshold
/// is derived from it the way the client does.
pub fn update_environment_flags_internal(
    env: &mut EnvironmentState,
    liquid: &LiquidStatus,
    player_z: f32,
    collision_height: f32,
    has_water_breathing: bool,
) {
    use oxcore_map::terrain::{
        MAP_LIQUID_TYPE_DEEP_WATER, MAP_LIQUID_TYPE_MAGMA, MAP_LIQUID_TYPE_OCEAN,
        MAP_LIQUID_TYPE_SLIME, MAP_LIQUID_TYPE_WATER,
    };

    if !liquid.has_liquid() {
        set_environment_flags(
            env,
            EnvironmentFlags::MASK_LIQUID_FLAGS,
            false,
            has_water_breathing,
        );
        return;
    }

    let type_flags = liquid.data.type_flags;
    let level = liquid.data.level;
    let status = liquid.status;

    // Inside an area with liquid at all.
    set_environment_flags(env, EnvironmentFlags::LIQUID, true, has_water_breathing);

    // Each flag's condition combines "is this liquid kind present here" with the
    // depth test for that kind. Unlike the reference, an absent kind clears its
    // flag rather than leaving the previous value: swimming straight from slime
    // into water would otherwise keep IN_SLIME set and keep dealing damage.
    let any_liquid = type_flags
        & (MAP_LIQUID_TYPE_WATER
            | MAP_LIQUID_TYPE_OCEAN
            | MAP_LIQUID_TYPE_MAGMA
            | MAP_LIQUID_TYPE_SLIME)
        != 0;

    // Any liquid kind: submerged once the surface is above head height.
    set_environment_flags(
        env,
        EnvironmentFlags::UNDERWATER,
        any_liquid
            && status.intersects(LiquidStatusFlags::UNDER_WATER)
            && level > (player_z + collision_height),
        has_water_breathing,
    );

    // Water and ocean: on or under the surface.
    set_environment_flags(
        env,
        EnvironmentFlags::IN_WATER,
        type_flags & (MAP_LIQUID_TYPE_WATER | MAP_LIQUID_TYPE_OCEAN) != 0
            && status.intersects(LiquidStatusFlags::MASK_SWIMMING),
        has_water_breathing,
    );

    // Magma and slime also burn when standing just above the surface.
    set_environment_flags(
        env,
        EnvironmentFlags::IN_MAGMA,
        type_flags & MAP_LIQUID_TYPE_MAGMA != 0
            && status.intersects(LiquidStatusFlags::MASK_TOUCHING),
        has_water_breathing,
    );
    set_environment_flags(
        env,
        EnvironmentFlags::IN_SLIME,
        type_flags & MAP_LIQUID_TYPE_SLIME != 0
            && status.intersects(LiquidStatusFlags::MASK_TOUCHING),
        has_water_breathing,
    );

    // Deep sea applies anywhere in the area, above or below the surface.
    set_environment_flags(
        env,
        EnvironmentFlags::HIGH_SEA,
        type_flags & MAP_LIQUID_TYPE_DEEP_WATER != 0,
        has_water_breathing,
    );

    // Deep enough to swim rather than wade.
    let min_swim_depth = collision_height * 0.75;
    set_environment_flags(
        env,
        EnvironmentFlags::HIGH_LIQUID,
        status.intersects(LiquidStatusFlags::MASK_SWIMMING) && level > (player_z + min_swim_depth),
        has_water_breathing,
    );
}

/// Apply or clear environment flags, running the mirror-timer side effects that
/// each transition triggers.
///
/// Ported from `Player::SetEnvironmentFlags`. Entering deep sea starts draining
/// the fatigue timer; submerging drains breath; touching magma or slime drains
/// the environmental timer. Leaving flips the timer to fast recovery rather than
/// resetting it, which is what makes the breath bar refill when you surface.
///
/// The reference also refreshes threat tables and cancels water-dependent auras
/// here; those live in other systems and are not driven from this call.
fn set_environment_flags(
    env: &mut EnvironmentState,
    flags: EnvironmentFlags,
    apply: bool,
    has_water_breathing: bool,
) {
    // Nothing to do if every requested flag already has the target state.
    if env.env_flags.intersects(flags) == apply {
        return;
    }

    if apply {
        env.env_flags |= flags;
    } else {
        env.env_flags &= !flags;
    }

    // Recovery runs faster than depletion, matching the client's expectation.
    const RECOVER_SCALE: i32 = 10;
    const DEPLETE_SCALE: i32 = -1;

    if flags.contains(EnvironmentFlags::HIGH_SEA) {
        env.fatigue_timer
            .set_scale(if apply { DEPLETE_SCALE } else { RECOVER_SCALE });
    }

    if flags.contains(EnvironmentFlags::UNDERWATER) {
        // A water breathing aura means the breath bar never drains.
        env.breath_timer
            .set_scale(if apply && !has_water_breathing {
                DEPLETE_SCALE
            } else {
                RECOVER_SCALE
            });
    }

    if flags.intersects(EnvironmentFlags::MASK_LIQUID_HAZARD) {
        // Keyed off the resulting state, not `apply`: leaving lava for slime
        // must keep the timer draining.
        env.environmental_timer.set_scale(
            if env
                .env_flags
                .intersects(EnvironmentFlags::MASK_LIQUID_HAZARD)
            {
                DEPLETE_SCALE
            } else {
                RECOVER_SCALE
            },
        );
    }
}
