//! Spell System - Main orchestrator for spell casting
//!
//! Manages the casting pipeline:
//! validate -> start -> timer -> execute -> finish

use crate::shared::messages::spells::{
    SmsgCastResult, SmsgSpellCooldown, SmsgSpellFailure, SmsgSpellGo, SmsgSpellStart,
};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::ObjectGuid;
use crate::world::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::world::game::player::spells::cooldowns;
use crate::world::game::player::spells::effects::EffectsDispatcher;
use crate::world::game::player::spells::learning;
use crate::world::game::player::spells::modifiers;
use crate::world::game::player::spells::state::{
    ActiveCast, CurrentSpellType, SpellCastError, SpellCastResult, SpellCastTargets,
    SpellEventQueue, SpellEventType, SpellModOp, SpellModType, SpellState, SpellsState,
};
use crate::world::game::player::spells::validation;
use crate::world::World;
use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Get current game time in milliseconds
fn get_game_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Stateless spell system - operates on player.spells via PlayerManager.
///
/// Architecture:
/// - SpellSystem owns no mutable state
/// - All spell data lives in player.spells (SpellsState)
/// - Accesses player state via world.systems.player.manager().with_player_mut()
/// - Sends packets via BroadcastManager
/// - Delegates to sub-modules: validation, cooldowns, learning, effects
pub struct SpellSystem {
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
    effects_dispatcher: EffectsDispatcher,
    /// Event-driven spell queue — replaces per-player polling
    event_queue: Mutex<SpellEventQueue>,
}

impl SpellSystem {
    /// Create a new spell system
    pub fn new(broadcast_mgr: Arc<dyn BroadcastManagerTrait>) -> Self {
        Self {
            broadcast_mgr,
            effects_dispatcher: EffectsDispatcher::new(),
            event_queue: Mutex::new(SpellEventQueue::new()),
        }
    }

    // =========================================================================
    // Cast Pipeline: validate -> start -> timer -> execute -> finish
    // =========================================================================

    /// Main entry point for casting a spell.
    ///
    /// Pipeline:
    /// 1. Validate (has spell, enough resources, not on CD, valid target, etc.)
    /// 2. If instant: execute immediately
    /// 3. If cast time: create ActiveCast, broadcast SMSG_SPELL_START
    /// 4. Timer runs in update_casts() until cast_time_remaining == 0
    /// 5. Execute: dispatch effects, apply results
    /// 6. Finish: broadcast SMSG_SPELL_GO, apply cooldown + GCD
    pub fn cast_spell<'a>(
        &'a self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        is_triggered: bool,
        world: &'a World,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SpellCastResult>> + Send + 'a>>
    {
        Box::pin(self.cast_spell_inner(
            caster_guid,
            spell_id,
            target_guid,
            is_triggered,
            None,
            world,
        ))
    }

    pub fn cast_spell_from_item<'a>(
        &'a self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        item_guid: ObjectGuid,
        world: &'a World,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SpellCastResult>> + Send + 'a>>
    {
        Box::pin(self.cast_spell_inner(
            caster_guid,
            spell_id,
            target_guid,
            true,
            Some(item_guid),
            world,
        ))
    }

    async fn cast_spell_inner(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        is_triggered: bool,
        cast_item_guid: Option<ObjectGuid>,
        world: &World,
    ) -> Result<SpellCastResult> {
        // Step 1: Validate
        let validate_result =
            validation::validate_cast(caster_guid, spell_id, target_guid, is_triggered, world)?;

        if validate_result != SpellCastError::None {
            // Send failure to client
            self.send_cast_failure(caster_guid, spell_id, validate_result)?;
            return Ok(SpellCastResult::Failed(validate_result));
        }

        // Step 2: Calculate cast time (modified by haste, talents, etc.)
        let cast_time_ms = self.calculate_cast_time(caster_guid, spell_id, world)?;

        // Step 3: Determine spell slot and cancel any existing spell in that slot
        let slot = self.get_spell_slot(spell_id, world);

        // Cancel existing spell in this slot (if any)
        self.cancel_spell_in_slot(caster_guid, slot, world).await?;

        // For Generic casts, also cancel Channeled (MaNGOS behavior: new generic cancels channel)
        if slot == CurrentSpellType::Generic {
            self.cancel_spell_in_slot(caster_guid, CurrentSpellType::Channeled, world)
                .await?;
        }

        // Step 4: Consume resources (mana/rage/energy) and apply GCD
        if !is_triggered {
            self.consume_resources(caster_guid, spell_id, world).await?;
            self.apply_gcd(caster_guid, spell_id, world).await?;

            // Remove auras with CASTING interrupt flag
            let _ = world
                .systems
                .auras
                .remove_auras_with_interrupt_flag(
                    caster_guid,
                    0x00400000, // AURA_INTERRUPT_FLAG_CAST (bit 22)
                    world,
                )
                .await;
        }

        // Check if this is a channeled spell
        let is_channeled = slot == CurrentSpellType::Channeled;

        if is_channeled {
            // Channeled: execute first tick immediately, then channel ticks over time
            let channel_duration = self.get_channel_duration(spell_id, world);
            let tick_count = self.get_channel_tick_count(spell_id, world);
            self.start_channel(
                caster_guid,
                spell_id,
                target_guid,
                channel_duration,
                tick_count,
                is_triggered,
                cast_item_guid,
                world,
            )
            .await?;
        } else if cast_time_ms == 0 {
            // Instant cast - execute immediately
            self.execute_spell(caster_guid, spell_id, target_guid, is_triggered, world)
                .await?;
            self.finish_cast(
                caster_guid,
                spell_id,
                target_guid,
                is_triggered,
                cast_item_guid,
                world,
            )
            .await?;
        } else {
            // Cast time spell - create ActiveCast and broadcast SPELL_START
            self.start_cast(
                caster_guid,
                spell_id,
                target_guid,
                cast_time_ms,
                is_triggered,
                slot,
                cast_item_guid,
                world,
            )
            .await?;
        }

        Ok(SpellCastResult::Success)
    }

    /// Start a cast-time spell. Creates ActiveCast and broadcasts SMSG_SPELL_START.
    async fn start_cast(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        cast_time_ms: u32,
        is_triggered: bool,
        slot: CurrentSpellType,
        cast_item_guid: Option<ObjectGuid>,
        world: &World,
    ) -> Result<()> {
        world
            .systems
            .player
            .manager()
            .with_player_mut(caster_guid, |player| {
                let (x, y, z) = (
                    player.movement.position.x,
                    player.movement.position.y,
                    player.movement.position.z,
                );

                player.spells.set_current_spell(
                    slot,
                    ActiveCast::new(
                        spell_id,
                        target_guid,
                        cast_time_ms,
                        is_triggered,
                        slot,
                        x,
                        y,
                        z,
                    ),
                );
            });

        // Schedule CastFinish event
        let now = get_game_time_ms();
        if let Ok(mut queue) = self.event_queue.lock() {
            queue.schedule(
                now + cast_time_ms as u64,
                SpellEventType::CastFinish {
                    caster_guid,
                    spell_id,
                    target_guid,
                    is_triggered,
                    slot,
                    cast_item_guid,
                },
            );
        }

        // Broadcast SMSG_SPELL_START to nearby players
        let msg = SmsgSpellStart {
            caster_guid,
            caster_guid_pack: caster_guid,
            spell_id,
            cast_flags: if is_triggered { 0x0002 } else { 0x0000 },
            cast_time_ms,
            target_guid,
            cast_item_guid,
        };
        self.broadcast_mgr
            .send_msg_to_player(caster_guid, msg.to_world_packet());

        Ok(())
    }

    /// Start a channeled spell. Creates ActiveCast in channel mode.
    async fn start_channel(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        duration_ms: u32,
        tick_count: u32,
        is_triggered: bool,
        cast_item_guid: Option<ObjectGuid>,
        world: &World,
    ) -> Result<()> {
        world
            .systems
            .player
            .manager()
            .with_player_mut(caster_guid, |player| {
                let (x, y, z) = (
                    player.movement.position.x,
                    player.movement.position.y,
                    player.movement.position.z,
                );
                player.spells.set_current_spell(
                    CurrentSpellType::Channeled,
                    ActiveCast::new_channel(
                        spell_id,
                        target_guid,
                        duration_ms,
                        tick_count,
                        is_triggered,
                        x,
                        y,
                        z,
                    ),
                );
            });

        // Send SMSG_CHANNEL_START
        let mut packet = crate::shared::protocol::WorldPacket::new(
            crate::shared::protocol::Opcode::MSG_CHANNEL_START,
        );
        packet.write_u32(spell_id);
        packet.write_u32(duration_ms);
        self.broadcast_mgr.send_msg_to_player(caster_guid, packet);

        // Schedule channel tick events and channel finish
        let now = get_game_time_ms();
        let tick_interval = if tick_count > 0 {
            duration_ms / tick_count
        } else {
            duration_ms
        };
        if let Ok(mut queue) = self.event_queue.lock() {
            for tick in 0..tick_count {
                queue.schedule(
                    now + (tick_interval as u64 * (tick as u64 + 1)),
                    SpellEventType::ChannelTick {
                        caster_guid,
                        spell_id,
                        target_guid,
                        tick_number: tick,
                    },
                );
            }
            queue.schedule(
                now + duration_ms as u64,
                SpellEventType::ChannelFinish {
                    caster_guid,
                    spell_id,
                    target_guid,
                },
            );
        }

        // Broadcast SMSG_SPELL_GO to show the channel began
        let msg = SmsgSpellGo {
            caster_guid,
            caster_guid_pack: caster_guid,
            spell_id,
            cast_flags: 0x0000,
            hit_targets: target_guid.into_iter().collect(),
            miss_targets: Vec::new(),
            target_guid,
            cast_item_guid,
        };
        self.broadcast_mgr
            .send_msg_to_player(caster_guid, msg.to_world_packet());

        Ok(())
    }

    /// Determine which spell slot a spell belongs in (matches MaNGOS GetCurrentContainer).
    fn get_spell_slot(&self, spell_id: u32, world: &World) -> CurrentSpellType {
        let spell_entry = match world.managers.spell_mgr.get(spell_id) {
            Some(entry) => entry,
            None => return CurrentSpellType::Generic,
        };

        // Channeled spells go to the Channeled slot
        // SPELL_ATTR_EX_CHANNELED_1 = 0x04, SPELL_ATTR_EX_CHANNELED_2 = 0x40
        if (spell_entry.attributes_ex & 0x04) != 0 || (spell_entry.attributes_ex & 0x40) != 0 {
            return CurrentSpellType::Channeled;
        }

        // Melee spells (on-next-melee): SPELL_ATTR_ON_NEXT_SWING_1 = 0x01, SPELL_ATTR_ON_NEXT_SWING_2 = 0x80000000
        if (spell_entry.attributes & 0x01) != 0 || (spell_entry.attributes & 0x80000000) != 0 {
            return CurrentSpellType::Melee;
        }

        // Auto-repeat spells (Auto-Shot, Wand): SPELL_ATTR_EX2_AUTOREPEAT_FLAG = 0x00000020
        if (spell_entry.attributes_ex2 & 0x00000020) != 0 {
            return CurrentSpellType::Autorepeat;
        }

        CurrentSpellType::Generic
    }

    /// Check if a spell is channeled
    fn is_channeled_spell(&self, spell_id: u32, world: &World) -> bool {
        let spell_entry = match world.managers.spell_mgr.get(spell_id) {
            Some(entry) => entry,
            None => return false,
        };
        // SPELL_ATTR_EX_CHANNELED_1 = 0x04
        // SPELL_ATTR_EX_CHANNELED_2 = 0x40
        (spell_entry.attributes_ex & 0x04) != 0 || (spell_entry.attributes_ex & 0x40) != 0
    }

    /// Get the channel duration for a channeled spell (from duration DBC)
    fn get_channel_duration(&self, spell_id: u32, world: &World) -> u32 {
        let spell_entry = match world.managers.spell_mgr.get(spell_id) {
            Some(entry) => entry,
            None => return 0,
        };

        if spell_entry.duration_index > 0 {
            let dbc = world.dbc.read();
            if let Some(dur) = dbc.get_spell_duration(spell_entry.duration_index) {
                return dur.duration.max(0) as u32;
            }
        }
        0
    }

    /// Get the number of channel ticks (from effect amplitude)
    fn get_channel_tick_count(&self, spell_id: u32, world: &World) -> u32 {
        let spell_entry = match world.managers.spell_mgr.get(spell_id) {
            Some(entry) => entry,
            None => return 1,
        };

        let duration = self.get_channel_duration(spell_id, world);
        if duration == 0 {
            return 1;
        }

        // Use the first non-zero effect amplitude for tick interval
        for i in 0..3 {
            if spell_entry.effect_amplitude[i] > 0 {
                return (duration / spell_entry.effect_amplitude[i]).max(1);
            }
        }

        // Default: 1 tick per second
        (duration / 1000).max(1)
    }

    /// Process all ready spell events. Called every world tick (50ms).
    /// Event-driven: only processes events that are due, not all players.
    pub async fn update_all_casts(&self, _diff: Duration, world: &World) -> Result<()> {
        let now = get_game_time_ms();

        // Drain ready events from the queue
        let ready_events: Vec<crate::world::game::player::spells::state::SpellEvent> = {
            match self.event_queue.lock() {
                Ok(mut queue) => queue.drain_ready(now),
                Err(_) => return Ok(()),
            }
        };

        // Process each event
        for event in ready_events {
            match event.event_type {
                SpellEventType::CastFinish {
                    caster_guid,
                    spell_id,
                    target_guid,
                    is_triggered,
                    slot,
                    cast_item_guid,
                } => {
                    // Verify the spell is still in the slot (wasn't cancelled)
                    let still_active = world
                        .systems
                        .player
                        .manager()
                        .with_player_mut(caster_guid, |player| {
                            player
                                .spells
                                .get_current_spell(slot)
                                .map_or(false, |cast| cast.spell_id == spell_id)
                        })
                        .unwrap_or(false);

                    if still_active {
                        // Clear the slot
                        world
                            .systems
                            .player
                            .manager()
                            .with_player_mut(caster_guid, |player| {
                                player.spells.clear_current_spell(slot);
                            });

                        // Re-validate (MaNGOS CheckCast(false))
                        let revalidate = validation::validate_cast(
                            caster_guid,
                            spell_id,
                            target_guid,
                            true,
                            world,
                        )
                        .unwrap_or(SpellCastError::InvalidTarget);

                        if revalidate != SpellCastError::None {
                            self.send_cast_failure(caster_guid, spell_id, revalidate)?;
                            continue;
                        }

                        self.execute_spell(caster_guid, spell_id, target_guid, is_triggered, world)
                            .await?;
                        self.finish_cast(
                            caster_guid,
                            spell_id,
                            target_guid,
                            is_triggered,
                            cast_item_guid,
                            world,
                        )
                        .await?;
                    }
                }
                SpellEventType::ChannelTick {
                    caster_guid,
                    spell_id,
                    target_guid,
                    ..
                } => {
                    // Verify channel is still active
                    let still_active = world
                        .systems
                        .player
                        .manager()
                        .with_player_mut(caster_guid, |player| {
                            player
                                .spells
                                .get_current_spell(CurrentSpellType::Channeled)
                                .map_or(false, |cast| cast.spell_id == spell_id)
                        })
                        .unwrap_or(false);

                    if still_active {
                        self.execute_channel_tick(caster_guid, spell_id, target_guid, world)
                            .await?;
                    }
                }
                SpellEventType::ChannelFinish {
                    caster_guid,
                    spell_id,
                    target_guid,
                } => {
                    // Verify channel is still active
                    let still_active = world
                        .systems
                        .player
                        .manager()
                        .with_player_mut(caster_guid, |player| {
                            player
                                .spells
                                .get_current_spell(CurrentSpellType::Channeled)
                                .map_or(false, |cast| cast.spell_id == spell_id)
                        })
                        .unwrap_or(false);

                    if still_active {
                        world
                            .systems
                            .player
                            .manager()
                            .with_player_mut(caster_guid, |player| {
                                player
                                    .spells
                                    .clear_current_spell(CurrentSpellType::Channeled);
                            });
                        self.finish_cast(caster_guid, spell_id, target_guid, false, None, world)
                            .await?;
                    }
                }
                SpellEventType::DelayedEffect {
                    caster_guid,
                    spell_id,
                    target_guid,
                    is_triggered,
                } => {
                    tracing::info!(
                        "[SPELL_PROJECTILE_HIT] spell={spell_id} caster={caster_guid:?} target={target_guid:?} — executing delayed damage"
                    );
                    self.execute_spell_immediate(
                        caster_guid,
                        spell_id,
                        target_guid,
                        is_triggered,
                        world,
                    )
                    .await?;
                }
            }
        }

        Ok(())
    }

    /// Tick delayed spell effects (projectile travel) for a player.
    async fn update_delayed_effects(
        &self,
        player_guid: ObjectGuid,
        diff: Duration,
        world: &World,
    ) -> Result<()> {
        use crate::world::game::player::spells::state::DelayedSpellEffect;

        let diff_ms = diff.as_millis() as u32;
        if diff_ms == 0 {
            return Ok(());
        }

        // Tick timers and collect ready effects
        let mut ready_effects: Vec<DelayedSpellEffect> = Vec::new();
        world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                let mut i = 0;
                while i < player.spells.delayed_effects.len() {
                    if player.spells.delayed_effects[i].delivery_time_ms <= diff_ms {
                        ready_effects.push(player.spells.delayed_effects.remove(i));
                    } else {
                        player.spells.delayed_effects[i].delivery_time_ms -= diff_ms;
                        i += 1;
                    }
                }
            });

        // Execute ready effects
        for effect in ready_effects {
            self.execute_spell_immediate(
                effect.caster_guid,
                effect.spell_id,
                effect.target_guid,
                effect.is_triggered,
                world,
            )
            .await?;
        }

        Ok(())
    }

    /// Update active casts for a single player. Called every world tick (50ms).
    ///
    /// Iterates all 4 spell slots, decrements cast timers, and fires spells when complete.
    pub async fn update_casts(
        &self,
        player_guid: ObjectGuid,
        diff: Duration,
        world: &World,
    ) -> Result<()> {
        let diff_ms = diff.as_millis() as u32;
        if diff_ms == 0 {
            return Ok(());
        }

        // Collect update results from all slots (snapshot pattern)
        let mut updates: Vec<CastUpdateInfo> = Vec::new();

        world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                for slot_idx in 0..crate::world::game::player::spells::state::NUM_CURRENT_SPELLS {
                    if let Some(ref mut active) = player.spells.current_spells[slot_idx] {
                        if active.is_channeling {
                            // Channel: tick the channel timer
                            match active.tick_channel(diff_ms) {
                                None => {
                                    // Channel complete
                                    updates.push(CastUpdateInfo::ChannelComplete {
                                        spell_id: active.spell_id,
                                        target_guid: active.target_guid,
                                    });
                                    player.spells.current_spells[slot_idx] = None;
                                }
                                Some(true) => {
                                    // Channel tick fired
                                    updates.push(CastUpdateInfo::ChannelTick {
                                        spell_id: active.spell_id,
                                        target_guid: active.target_guid,
                                        ticks_remaining: active.channel_ticks_remaining,
                                    });
                                }
                                Some(false) => {
                                    // Just decrementing timer
                                }
                            }
                        } else if active.state == SpellState::Preparing {
                            // Non-channeled: decrement cast timer
                            if active.tick(diff_ms) {
                                // Cast complete
                                updates.push(CastUpdateInfo::CastComplete {
                                    spell_id: active.spell_id,
                                    target_guid: active.target_guid,
                                    is_triggered: active.is_triggered,
                                });
                                player.spells.current_spells[slot_idx] = None;
                            }
                        }
                    }
                }
            });

        // Execute based on update results (outside player lock)
        for info in updates {
            match info {
                CastUpdateInfo::CastComplete {
                    spell_id,
                    target_guid,
                    is_triggered,
                } => {
                    // MaNGOS re-validates when cast timer expires (CheckCast(false)).
                    // Use is_triggered=true to skip GCD/cooldown/resource checks (already consumed).
                    // This re-check validates: target alive, caster alive, in range, not CC'd.
                    let revalidate = validation::validate_cast(
                        player_guid,
                        spell_id,
                        target_guid,
                        true, // skip GCD/cooldown/resource/already-casting checks
                        world,
                    )
                    .unwrap_or(SpellCastError::InvalidTarget);

                    if revalidate != SpellCastError::None {
                        // Cast failed on completion — send failure and skip execution
                        self.send_cast_failure(player_guid, spell_id, revalidate)?;
                        continue;
                    }

                    self.execute_spell(player_guid, spell_id, target_guid, is_triggered, world)
                        .await?;
                    self.finish_cast(
                        player_guid,
                        spell_id,
                        target_guid,
                        is_triggered,
                        None,
                        world,
                    )
                    .await?;
                }
                CastUpdateInfo::ChannelTick {
                    spell_id,
                    target_guid,
                    ..
                } => {
                    self.execute_channel_tick(player_guid, spell_id, target_guid, world)
                        .await?;
                }
                CastUpdateInfo::ChannelComplete {
                    spell_id,
                    target_guid,
                } => {
                    self.finish_cast(player_guid, spell_id, target_guid, false, None, world)
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// Execute spell effects. Called when cast time completes (or instantly for instant casts).
    ///
    /// Uses the target resolution system to determine per-effect targets,
    /// then dispatches effects with hit/miss rolls applied.
    /// If the spell has a projectile speed, effects are deferred for travel time.
    async fn execute_spell(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        is_triggered: bool,
        world: &World,
    ) -> Result<()> {
        use crate::world::game::player::spells::state::DelayedSpellEffect;
        use crate::world::game::player::spells::targets;

        // Check if spell has projectile travel time
        let speed = world
            .managers
            .spell_mgr
            .get(spell_id)
            .map(|s| s.speed)
            .unwrap_or(0.0);

        if speed > 0.0 && target_guid.is_some() {
            // Calculate travel time based on distance
            let travel_time_ms =
                self.calculate_travel_time(caster_guid, target_guid.unwrap(), speed, world);
            tracing::info!(
                "[SPELL_PROJECTILE] spell={spell_id} speed={speed} travel_time={travel_time_ms}ms target={:?}",
                target_guid
            );
            if travel_time_ms > 0 {
                // Schedule delayed effect via event queue
                let now = get_game_time_ms();
                if let Ok(mut queue) = self.event_queue.lock() {
                    queue.schedule(
                        now + travel_time_ms as u64,
                        SpellEventType::DelayedEffect {
                            caster_guid,
                            spell_id,
                            target_guid,
                            is_triggered,
                        },
                    );
                }
                return Ok(());
            }
        }

        // Immediate execution
        self.execute_spell_immediate(caster_guid, spell_id, target_guid, is_triggered, world)
            .await
    }

    /// Execute spell effects immediately (no travel time).
    async fn execute_spell_immediate(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        is_triggered: bool,
        world: &World,
    ) -> Result<()> {
        use crate::world::game::player::spells::targets;

        let cast_targets = SpellCastTargets {
            unit_target_guid: target_guid,
            ..Default::default()
        };

        let resolved = targets::resolve_spell_targets(spell_id, &cast_targets, caster_guid, world);

        self.effects_dispatcher
            .dispatch_with_targets(
                caster_guid,
                spell_id,
                target_guid,
                is_triggered,
                Some(&resolved),
                world,
            )
            .await?;

        Ok(())
    }

    /// Calculate projectile travel time in milliseconds.
    fn calculate_travel_time(
        &self,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        speed: f32,
        world: &World,
    ) -> u32 {
        let caster_pos = world
            .managers
            .player_mgr
            .with_player(caster_guid, |p| p.movement.position)
            .unwrap_or_default();

        let target_pos = if target_guid.is_player() {
            world
                .managers
                .player_mgr
                .with_player(target_guid, |p| p.movement.position)
                .unwrap_or_default()
        } else if target_guid.is_creature() {
            world
                .managers
                .creature_mgr
                .with_creature(target_guid, |c| crate::shared::protocol::Position {
                    x: c.position.x,
                    y: c.position.y,
                    z: c.position.z,
                    o: 0.0,
                })
                .unwrap_or_default()
        } else {
            return 0;
        };

        let dx = caster_pos.x - target_pos.x;
        let dy = caster_pos.y - target_pos.y;
        let dz = caster_pos.z - target_pos.z;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // speed is in yards per second
        if speed > 0.0 {
            ((distance / speed) * 1000.0) as u32
        } else {
            0
        }
    }

    /// Execute a single channel tick.
    async fn execute_channel_tick(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        world: &World,
    ) -> Result<()> {
        // Channel ticks re-execute the spell effects
        self.execute_spell(caster_guid, spell_id, target_guid, true, world)
            .await
    }

    /// Finish a spell cast. Broadcasts SMSG_SPELL_GO, applies cooldown.
    async fn finish_cast(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        is_triggered: bool,
        cast_item_guid: Option<ObjectGuid>,
        world: &World,
    ) -> Result<()> {
        use crate::world::game::player::spells::targets;

        // Resolve targets for SMSG_SPELL_GO hit list
        let cast_targets = SpellCastTargets {
            unit_target_guid: target_guid,
            ..Default::default()
        };
        let resolved = targets::resolve_spell_targets(spell_id, &cast_targets, caster_guid, world);

        // Collect all unique hit targets across all effects
        let mut hit_targets: Vec<ObjectGuid> = resolved
            .effect_targets
            .iter()
            .flat_map(|t| t.iter().copied())
            .collect();
        hit_targets.sort_by_key(|g| g.raw());
        hit_targets.dedup_by_key(|g| g.raw());

        // Send SMSG_CAST_RESULT success to unlock client cast bar
        self.broadcast_mgr
            .send_msg_to_player(caster_guid, SmsgCastResult::success(spell_id));

        // Broadcast SMSG_SPELL_GO
        let msg = SmsgSpellGo {
            caster_guid,
            caster_guid_pack: caster_guid,
            spell_id,
            cast_flags: if is_triggered { 0x0002 } else { 0x0000 },
            hit_targets,
            miss_targets: Vec::new(),
            target_guid,
            cast_item_guid,
        };
        self.broadcast_mgr
            .send_msg_to_player(caster_guid, msg.to_world_packet());

        // Apply cooldown (if not triggered)
        if !is_triggered {
            cooldowns::apply_cooldown(caster_guid, spell_id, world)?;

            // Send SMSG_SPELL_COOLDOWN to client with actual cooldown duration
            if let Some(entry) = world.managers.spell_mgr.get(spell_id) {
                let cd_ms = entry.recovery_time.max(entry.category_recovery_time);
                if cd_ms > 0 {
                    let msg = SmsgSpellCooldown {
                        caster_guid,
                        cooldowns: vec![(spell_id, cd_ms)],
                    };
                    self.broadcast_mgr
                        .send_msg_to_player(caster_guid, msg.to_world_packet());
                }
            }
        }

        // Reset main-hand attack timer after cast-time spells (MaNGOS behavior).
        // Prevents players from getting a free swing immediately after a cast.
        if !is_triggered {
            if let Some(entry) = world.managers.spell_mgr.get(spell_id) {
                // Only reset for spells with cast time, not autorepeat or channeled
                let has_cast_time = entry.casting_time_index > 0;
                let is_autorepeat = (entry.attributes_ex2 & 0x00000020) != 0;
                let is_channeled =
                    (entry.attributes_ex & 0x04) != 0 || (entry.attributes_ex & 0x40) != 0;
                if has_cast_time && !is_autorepeat && !is_channeled {
                    world
                        .systems
                        .player
                        .manager()
                        .with_player_mut(caster_guid, |player| {
                            player.combat.main_hand_timer = player.combat.main_hand_speed;
                        });
                }
            }
        }

        Ok(())
    }

    // =========================================================================
    // Cancel / Interrupt
    // =========================================================================

    /// Cancel the current cast (player-initiated, e.g., pressing Escape or moving).
    /// Cancels the Generic slot first, then Channeled if no generic cast active.
    pub async fn cancel_cast(&self, caster_guid: ObjectGuid, world: &World) -> Result<()> {
        // Try Generic first, then Channeled
        let cancelled = self
            .cancel_spell_in_slot(caster_guid, CurrentSpellType::Generic, world)
            .await?;
        if !cancelled {
            self.cancel_spell_in_slot(caster_guid, CurrentSpellType::Channeled, world)
                .await?;
        }
        Ok(())
    }

    /// Cancel a spell by spell_id (for CMSG_CANCEL_CAST which sends specific spell_id).
    pub async fn cancel_cast_by_spell_id(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        world: &World,
    ) -> Result<()> {
        let slot = world
            .systems
            .player
            .manager()
            .with_player_mut(caster_guid, |player| {
                player.spells.find_spell_slot(spell_id)
            })
            .flatten();

        if let Some(slot) = slot {
            self.cancel_spell_in_slot(caster_guid, slot, world).await?;
        }
        Ok(())
    }

    /// Cancel the spell in a specific slot. Returns true if a spell was cancelled.
    async fn cancel_spell_in_slot(
        &self,
        caster_guid: ObjectGuid,
        slot: CurrentSpellType,
        world: &World,
    ) -> Result<bool> {
        let cancelled_info: Option<(u32, bool)> = world
            .systems
            .player
            .manager()
            .with_player_mut(caster_guid, |player| {
                player
                    .spells
                    .clear_current_spell(slot)
                    .map(|active| (active.spell_id, active.is_channeling))
            })
            .flatten();

        // Remove any pending events for this spell
        if let Some((spell_id, _)) = cancelled_info {
            if let Ok(mut queue) = self.event_queue.lock() {
                queue.cancel_events_for(caster_guid, spell_id);
            }
        }

        if let Some((spell_id, was_channeling)) = cancelled_info {
            // Broadcast SMSG_SPELL_FAILURE
            let msg = SmsgSpellFailure {
                caster_guid,
                spell_id,
                result: SpellCastError::Interrupted as u8,
            };
            self.broadcast_mgr
                .send_msg_to_player(caster_guid, msg.to_world_packet());

            // If cancelling a channel, send SMSG_CHANNEL_UPDATE with 0 remaining
            // and remove auras applied by the cancelled channel (MaNGOS RemoveAurasByCasterSpell)
            if was_channeling {
                let mut packet = crate::shared::protocol::WorldPacket::new(
                    crate::shared::protocol::Opcode::MSG_CHANNEL_UPDATE,
                );
                packet.write_u32(0); // 0 = channel interrupted
                self.broadcast_mgr.send_msg_to_player(caster_guid, packet);

                // Remove auras applied by this channeled spell on all targets
                world
                    .systems
                    .auras
                    .remove_spell_auras(caster_guid, spell_id, world)
                    .await?;
            }
            return Ok(true);
        }

        Ok(false)
    }

    /// Interrupt a cast (from damage, CC, Counterspell, etc.).
    ///
    /// Unlike cancel, interrupt can also lock the spell's school.
    /// `lockout_duration_ms` is how long the school is locked (0 = no lockout).
    pub async fn interrupt_cast(
        &self,
        target_guid: ObjectGuid,
        interrupter_guid: ObjectGuid,
        lockout_duration_ms: u32,
        world: &World,
    ) -> Result<()> {
        // Interrupt Generic first, then Channeled
        let interrupted_info: Option<(u32, u32)> = world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                // Try generic slot first
                let cast = player
                    .spells
                    .clear_current_spell(CurrentSpellType::Generic)
                    .or_else(|| {
                        player
                            .spells
                            .clear_current_spell(CurrentSpellType::Channeled)
                    });
                cast.map(|active| (active.spell_id, active.spell_id))
            })
            .flatten();

        if let Some((spell_id, interrupted_spell_id)) = interrupted_info {
            // Apply school lockout if specified
            if lockout_duration_ms > 0 {
                // Get spell school from spell entry
                if let Some(spell_entry) = world.managers.spell_mgr.get(interrupted_spell_id) {
                    let school = spell_entry.school as u8;
                    if school > 0 {
                        // Don't lock Physical school
                        let now = get_game_time_ms();
                        world
                            .systems
                            .player
                            .manager()
                            .with_player_mut(target_guid, |player| {
                                player.spells.apply_school_lockout(
                                    school,
                                    lockout_duration_ms,
                                    now,
                                );
                            });
                    }
                }
            }

            // Broadcast SMSG_SPELL_FAILURE
            let msg = SmsgSpellFailure {
                caster_guid: target_guid,
                spell_id,
                result: SpellCastError::Interrupted as u8,
            };
            self.broadcast_mgr
                .send_msg_to_player(target_guid, msg.to_world_packet());

            tracing::debug!(
                "Cast interrupted: target={}, interrupter={}, spell={}",
                target_guid,
                interrupter_guid,
                spell_id
            );
        }

        Ok(())
    }

    /// Apply cast pushback from taking damage while casting.
    ///
    /// Vanilla rules:
    /// - Non-channeled: +0.5s per hit, capped at +1.0s total pushback
    /// - Channeled: lose 25% of remaining channel time per hit
    /// - ResistPushback aura (e.g., Concentration Aura) reduces pushback %
    pub fn apply_cast_pushback(&self, target_guid: ObjectGuid, world: &World) -> Result<u32> {
        let mut pushback_applied = 0u32;
        let mut spell_id_for_reschedule: Option<u32> = None;

        world.systems.player.manager().with_player_mut(target_guid, |player| {
            // Apply pushback to Generic slot first, then Channeled
            let slot_idx = if player.spells.current_spells[CurrentSpellType::Generic as usize].is_some() {
                CurrentSpellType::Generic as usize
            } else {
                CurrentSpellType::Channeled as usize
            };

            if let Some(active) = player.spells.current_spells[slot_idx].as_mut() {
                // Check NotLoseCastTime spell modifier (reduces pushback, e.g., Concentration Aura)
                let mut pushback_reduction_pct = 0i32;
                for modifier in &player.spells.spell_modifiers {
                    if modifier.op == crate::world::game::player::spells::state::SpellModOp::NotLoseCastTime {
                        pushback_reduction_pct += modifier.value;
                    }
                }

                // Vanilla pushback values
                let max_pushback = 1000u32; // 1 second max total for non-channeled
                let base_pushback = 500u32; // 0.5 second per hit for non-channeled

                // Apply pushback reduction
                let pushback_per_hit = if pushback_reduction_pct > 0 {
                    let reduction = (base_pushback as f32 * pushback_reduction_pct as f32 / 100.0) as u32;
                    base_pushback.saturating_sub(reduction)
                } else {
                    base_pushback
                };

                pushback_applied = active.apply_pushback(pushback_per_hit, max_pushback);

                if pushback_applied > 0 {
                    spell_id_for_reschedule = Some(active.spell_id);
                    tracing::debug!(
                        "Cast pushback: pushed back {}ms on spell {} (reduction={}%)",
                        pushback_applied, active.spell_id, pushback_reduction_pct
                    );
                }
            }
        });

        Ok(pushback_applied)
    }

    // =========================================================================
    // Resource Consumption
    // =========================================================================

    /// Consume power (mana/rage/energy) for a spell cast.
    async fn consume_resources(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        world: &World,
    ) -> Result<bool> {
        // Get spell entry
        let spell_entry = match world.managers.spell_mgr.get(spell_id) {
            Some(entry) => (*entry).clone(),
            None => return Ok(true), // No cost if spell not found
        };

        // Calculate cost (base cost + modifiers from talents/auras)
        let cost = self.calculate_power_cost(caster_guid, &spell_entry, world)?;

        if cost > 0 {
            // Get power type from spell entry
            let power_type = match spell_entry.power_type {
                0 => crate::world::game::player::power::PowerType::Mana,
                1 => crate::world::game::player::power::PowerType::Rage,
                3 => crate::world::game::player::power::PowerType::Energy,
                _ => crate::world::game::player::power::PowerType::Mana,
            };
            let success =
                world
                    .systems
                    .power
                    .consume_power(caster_guid, power_type, cost, world)?;

            if !success {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Calculate power cost after spell modifiers.
    fn calculate_power_cost(
        &self,
        caster_guid: ObjectGuid,
        spell_entry: &crate::world::dbc::structures::SpellEntry,
        world: &World,
    ) -> Result<u32> {
        // Base cost from spell DBC entry
        let base_cost = spell_entry.mana_cost;

        // Apply cost modifiers from talents/auras (SpellModOp::Cost)
        let modified_cost = modifiers::calculate_modified_power_cost(
            caster_guid,
            base_cost,
            spell_entry.spell_family_name,
            spell_entry.spell_family_flags,
            world,
        );

        Ok(modified_cost)
    }

    /// Calculate cast time after haste and talent modifiers.
    fn calculate_cast_time(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        world: &World,
    ) -> Result<u32> {
        // Get spell entry
        let spell_entry = match world.managers.spell_mgr.get(spell_id) {
            Some(entry) => entry,
            None => return Ok(0), // Instant cast if spell not found
        };
        let casting_time_index = spell_entry.casting_time_index;

        // Get base cast time from SpellCastTimes.dbc
        let base_cast_time = if casting_time_index > 0 {
            world
                .dbc
                .read()
                .get_spell_cast_time(casting_time_index)
                .map(|ct| ct.cast_time.max(0) as u32)
                .unwrap_or(0)
        } else {
            0
        };

        // Apply cast time modifiers from talents/auras (SpellModOp::CastTime)
        let modified = modifiers::calculate_modified_cast_time(
            caster_guid,
            base_cast_time,
            spell_entry.spell_family_name,
            spell_entry.spell_family_flags,
            world,
        );

        Ok(modified)
    }

    // =========================================================================
    // GCD
    // =========================================================================

    /// Apply Global Cooldown after casting.
    async fn apply_gcd(&self, caster_guid: ObjectGuid, spell_id: u32, world: &World) -> Result<()> {
        // Get spell entry
        let spell_entry = match world.managers.spell_mgr.get(spell_id) {
            Some(entry) => entry,
            None => return Ok(()), // No GCD if spell not found
        };

        // Use start_recovery_time from spell entry if set, otherwise default 1500ms.
        // start_recovery_category determines which GCD group this spell belongs to.
        // Spells with SPELL_ATTR_RANGED (0x00000002) have 0ms GCD.
        // Spells with start_recovery_time = 0 and start_recovery_category = 0 have no GCD.
        let base_gcd_ms = if spell_entry.attributes & 0x00000002 != 0 {
            0 // No GCD for ranged auto-attack spells
        } else if spell_entry.start_recovery_time > 0 {
            spell_entry.start_recovery_time // Use spell-specific GCD duration
        } else if spell_entry.start_recovery_category > 0 {
            1500 // Has a GCD category but no override duration — use default
        } else {
            0 // No GCD category and no recovery time — no GCD
        };

        // Apply GCD modifiers
        let gcd_ms = modifiers::calculate_modified_gcd(
            caster_guid,
            base_gcd_ms,
            spell_entry.spell_family_name,
            spell_entry.spell_family_flags,
            world,
        );

        let now = get_game_time_ms();
        world
            .systems
            .player
            .manager()
            .with_player_mut(caster_guid, |player| {
                player.spells.apply_gcd(gcd_ms, now);
            });

        // Send GCD to client
        let msg = SmsgSpellCooldown {
            caster_guid,
            cooldowns: vec![(spell_id, gcd_ms)],
        };
        self.broadcast_mgr
            .send_msg_to_player(caster_guid, msg.to_world_packet());

        Ok(())
    }

    // =========================================================================
    // Spell Learning (delegates to learning module)
    // =========================================================================

    /// Learn a new spell.
    pub async fn learn_spell(
        &self,
        player_guid: ObjectGuid,
        spell_id: u32,
        world: &World,
    ) -> Result<bool> {
        learning::learn_spell(player_guid, spell_id, world, &self.broadcast_mgr).await
    }

    /// Unlearn a spell.
    pub async fn unlearn_spell(
        &self,
        player_guid: ObjectGuid,
        spell_id: u32,
        world: &World,
    ) -> Result<()> {
        learning::unlearn_spell(player_guid, spell_id, world, &self.broadcast_mgr).await
    }

    /// Send initial spellbook on login.
    pub fn send_initial_spells(&self, player_guid: ObjectGuid, world: &World) -> Result<()> {
        learning::send_initial_spells(player_guid, world, &self.broadcast_mgr)
    }

    /// Auto-learn spells for a level up.
    pub async fn auto_learn_spells_for_level(
        &self,
        player_guid: ObjectGuid,
        new_level: u8,
        world: &World,
    ) -> Result<()> {
        learning::auto_learn_for_level(player_guid, new_level, world, &self.broadcast_mgr).await
    }

    // =========================================================================
    // Login / Logout
    // =========================================================================

    /// Called on login: load spells, send spellbook, send cooldowns.
    pub async fn on_login(&self, player_guid: ObjectGuid, world: &World) -> Result<()> {
        // Load spells from database
        learning::load_from_db(player_guid, world)?;

        // Send spellbook to client
        self.send_initial_spells(player_guid, world)?;

        // Send active cooldowns to client
        cooldowns::send_cooldowns_on_login(player_guid, world, &self.broadcast_mgr)?;

        Ok(())
    }

    /// Called on logout: save spells and cooldowns.
    pub async fn on_logout(&self, player_guid: ObjectGuid, world: &World) -> Result<()> {
        // Cancel any active cast
        self.cancel_cast(player_guid, world).await?;

        // Save spells to database
        learning::save_to_db(player_guid, world)?;

        // Save cooldowns to database
        cooldowns::save_cooldowns(player_guid, world)?;

        Ok(())
    }

    // =========================================================================
    // Talent Integration
    // =========================================================================

    /// Apply a spell from a talent rank.
    ///
    /// Called by the talent system when a player learns a talent rank.
    /// This spell may be:
    /// - A passive aura (most common)
    /// - A spell modifier
    /// - A learned ability (e.g., Mortal Strike)
    pub async fn apply_talent_spell(
        &self,
        player_guid: ObjectGuid,
        spell_id: u32,
        world: &World,
    ) -> Result<()> {
        // TODO: Implement based on spell effects
        // For now, just learn the spell if it's not already known
        let already_known = world
            .systems
            .player
            .manager()
            .with_player(player_guid, |player| player.spells.knows_spell(spell_id))
            .unwrap_or(false);

        if !already_known {
            self.learn_spell(player_guid, spell_id, world).await?;
        }

        // TODO: Apply passive aura if the spell has SPELL_AURA_PASSIVE
        // TODO: Add spell modifiers if the spell has SPELL_AURA_ADD_FLAT_MODIFIER

        Ok(())
    }

    /// Unlearn a spell granted by a talent.
    ///
    /// Called by the talent system during talent reset.
    pub async fn unlearn_talent_spell(
        &self,
        player_guid: ObjectGuid,
        spell_id: u32,
        world: &World,
    ) -> Result<()> {
        // Check if this spell was learned from a talent
        // In a full implementation, we'd track which spells came from talents
        // For now, we just unlearn it
        self.unlearn_spell(player_guid, spell_id, world).await?;

        Ok(())
    }

    // =========================================================================
    // Client Communication
    // =========================================================================

    fn send_cast_failure(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        error: SpellCastError,
    ) -> Result<()> {
        let error_code = validation::spell_cast_error_to_u8(error);
        let packet = SmsgCastResult::failure(spell_id, error_code);
        self.broadcast_mgr.send_msg_to_player(caster_guid, packet);

        Ok(())
    }
}

// =============================================================================
// Internal Types
// =============================================================================

/// Result from updating an active cast timer.
enum CastUpdateInfo {
    CastComplete {
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        is_triggered: bool,
    },
    ChannelTick {
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        ticks_remaining: u32,
    },
    ChannelComplete {
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
    },
}
