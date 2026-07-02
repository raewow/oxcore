//! Spell System - Main orchestrator for spell casting
//!
//! Manages the casting pipeline:
//! validate -> start -> timer -> execute -> finish

use crate::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::game::player::spells::cooldowns;
use crate::game::player::spells::effects::EffectsDispatcher;
use crate::game::player::spells::hit;
use crate::game::player::spells::learning;
use crate::game::player::spells::modifiers;
use crate::game::player::spells::state::{
    ActiveCast, CurrentSpellType, SpellCastError, SpellCastResult, SpellCastTargets,
    SpellEventQueue, SpellEventType, SpellModOp, SpellModType, SpellState, SpellsState,
};
use crate::game::player::spells::validation;
use crate::World;
use anyhow::Result;
use oxcore_shared::messages::spells::{
    SmsgCastResult, SmsgSpellCooldown, SmsgSpellFailure, SmsgSpellGo, SmsgSpellStart,
    SPELL_RESULT_STATUS_FAIL, SPELL_RESULT_STATUS_OKAY,
};
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::ObjectGuid;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// SPELL_PREVENTION_TYPE_SILENCE — silence-type spells check school lockout.
const SPELL_PREVENTION_TYPE_SILENCE: u8 = 1;

/// Whether a spell consumes combo points on completion (MaNGOS `SpellEntry::NeedsComboPoints`).
///
/// `NeedsComboPoints() = AttributesEx & (FINISHING_MOVE_DAMAGE | FINISHING_MOVE_DURATION)`.
fn spell_needs_combo_points(attributes_ex: u32) -> bool {
    const FINISHING_MOVE_DAMAGE: u32 = 0x0010_0000;
    const FINISHING_MOVE_DURATION: u32 = 0x0040_0000;
    attributes_ex & (FINISHING_MOVE_DAMAGE | FINISHING_MOVE_DURATION) != 0
}

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
        let cast_targets = SpellCastTargets {
            unit_target_guid: target_guid,
            ..Default::default()
        };
        Box::pin(self.cast_spell_inner(
            caster_guid,
            spell_id,
            cast_targets,
            is_triggered,
            None,
            world,
        ))
    }

    /// Cast a spell with the full client-provided targets (unit/GO/item/corpse + source/dest
    /// positions). Used by the CMSG_CAST_SPELL handler so ground-targeted AoE keeps its
    /// destination position through the pipeline.
    pub fn cast_spell_with_targets<'a>(
        &'a self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        cast_targets: SpellCastTargets,
        is_triggered: bool,
        world: &'a World,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SpellCastResult>> + Send + 'a>>
    {
        Box::pin(self.cast_spell_inner(
            caster_guid,
            spell_id,
            cast_targets,
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
        let cast_targets = SpellCastTargets {
            unit_target_guid: target_guid,
            ..Default::default()
        };
        Box::pin(self.cast_spell_inner(
            caster_guid,
            spell_id,
            cast_targets,
            true,
            Some(item_guid),
            world,
        ))
    }

    /// Cast a spell with optional per-effect base point overrides.
    /// Faithful `SpellCaster::CastCustomSpell` port.
    ///
    /// Used by proc/trigger systems to cast a spell with custom damage/heal values
    /// that override the DBC base points (e.g., Ignite, Seal damage procs).
    /// Always treated as triggered (bypasses GCD and validation checks).
    ///
    /// `custom_base_points`: optional override for each effect slot (None = use DBC value).
    pub async fn cast_custom_spell(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        custom_base_points: [Option<i32>; 3],
        world: &World,
    ) -> Result<SpellCastResult> {
        // Validate spell entry exists
        if world.managers.spell_mgr.get(spell_id).is_none() {
            tracing::error!(
                "cast_custom_spell: unknown spell id {} by caster {:?}",
                spell_id,
                caster_guid
            );
            return Ok(SpellCastResult::Failed(SpellCastError::SpellNotKnown));
        }

        let cast_targets = SpellCastTargets {
            unit_target_guid: target_guid,
            ..Default::default()
        };

        // Resolve targets and dispatch effects immediately with custom base points.
        // Custom spells are always triggered and never go through the cast-time pipeline.
        use crate::game::player::spells::targets;
        let resolved = targets::resolve_spell_targets(spell_id, &cast_targets, caster_guid, world);
        self.effects_dispatcher
            .dispatch_with_targets(
                caster_guid,
                spell_id,
                target_guid,
                true, // always triggered
                Some(&resolved),
                Some(custom_base_points),
                world,
            )
            .await?;

        self.finish_cast(caster_guid, spell_id, &cast_targets, true, None, world)
            .await?;

        Ok(SpellCastResult::Success)
    }

    async fn cast_spell_inner(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        cast_targets: SpellCastTargets,
        is_triggered: bool,
        cast_item_guid: Option<ObjectGuid>,
        world: &World,
    ) -> Result<SpellCastResult> {
        let target_guid = cast_targets.unit_target();

        // Step 1: Validate
        let validate_result =
            validation::validate_cast(caster_guid, spell_id, target_guid, is_triggered, world)?;

        if validate_result != SpellCastError::None {
            // Send failure to client
            self.send_cast_failure(caster_guid, spell_id, validate_result, world)?;
            return Ok(SpellCastResult::Failed(validate_result));
        }

        // Step 2: Calculate cast time (modified by haste, talents, etc.)
        let cast_time_ms = self.calculate_cast_time(caster_guid, spell_id, world)?;
        tracing::debug!("[CAST] spell={spell_id} cast_time_ms={cast_time_ms} triggered={is_triggered}");

        // Step 3: Determine spell slot and handle all interruption logic
        // (MaNGOS SpellCaster::SetCurrentCastedSpell)
        let slot = self.get_spell_slot(spell_id, world);
        self.set_current_casted_spell(caster_guid, spell_id, slot, world).await?;

        // Step 4: Apply GCD and cast-start aura interrupts. Power/reagents/cast items
        // are NOT taken here — Spell::cast takes them at cast completion, so an
        // interrupted or cancelled cast costs nothing (see take_spell_costs).
        if !is_triggered {
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

        // Note: the success SMSG_CAST_RESULT is NOT sent at cast start. The client treats
        // it as "cast executed" and detaches its pending cast, so sending it before
        // SMSG_SPELL_START suppresses the cast bar. It belongs at cast completion, right
        // before SMSG_SPELL_GO (Spell::cast: cooldown -> take power -> cast result -> go).

        if is_channeled {
            // Channeled: Spell::cast runs at channel start, so costs are taken now
            if !is_triggered {
                self.take_spell_costs(caster_guid, spell_id, cast_item_guid, world)
                    .await?;
            }
            let channel_duration = self.get_channel_duration(spell_id, world);
            let tick_count = self.get_channel_tick_count(spell_id, world);
            self.start_channel(
                caster_guid,
                spell_id,
                channel_duration,
                tick_count,
                is_triggered,
                cast_item_guid,
                cast_targets,
                world,
            )
            .await?;
        } else if cast_time_ms == 0 {
            // Instant cast - execute immediately
            if !is_triggered {
                self.take_spell_costs(caster_guid, spell_id, cast_item_guid, world)
                    .await?;
            }
            self.execute_spell(caster_guid, spell_id, &cast_targets, is_triggered, world)
                .await?;
            self.finish_cast(
                caster_guid,
                spell_id,
                &cast_targets,
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
                cast_time_ms,
                is_triggered,
                slot,
                cast_item_guid,
                cast_targets,
                world,
            )
            .await?;
        }

        Ok(SpellCastResult::Success)
    }

    /// Take power, reagents, and the cast item. Runs at cast completion, right before
    /// effects are handled (Spell::cast: TakePower/TakeReagents), so a cancelled or
    /// interrupted cast never pays its costs.
    async fn take_spell_costs(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        cast_item_guid: Option<ObjectGuid>,
        world: &World,
    ) -> Result<()> {
        self.consume_resources(caster_guid, spell_id, cast_item_guid, world)
            .await?;
        self.take_reagents(caster_guid, spell_id, world);
        if let Some(item_guid) = cast_item_guid {
            if caster_guid.is_player() {
                world
                    .systems
                    .inventory
                    .consume_cast_item(caster_guid, item_guid)
                    .await;
            }
        }
        Ok(())
    }

    /// Start a cast-time spell. Creates ActiveCast and broadcasts SMSG_SPELL_START.
    async fn start_cast(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        cast_time_ms: u32,
        is_triggered: bool,
        slot: CurrentSpellType,
        cast_item_guid: Option<ObjectGuid>,
        cast_targets: SpellCastTargets,
        world: &World,
    ) -> Result<()> {
        let target_guid = cast_targets.unit_target();
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

                let mut active = ActiveCast::new(
                    spell_id,
                    target_guid,
                    cast_time_ms,
                    is_triggered,
                    slot,
                    x,
                    y,
                    z,
                );
                active.cast_targets = cast_targets.clone();
                player.spells.set_current_spell(slot, active);
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
                    cast_targets,
                },
            );
        }

        // Broadcast SMSG_SPELL_START to nearby players
        let msg = SmsgSpellStart {
            caster_guid,
            caster_guid_pack: caster_guid,
            spell_id,
            cast_flags: if is_triggered { 0x0000 } else { 0x0002 },
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
        duration_ms: u32,
        tick_count: u32,
        is_triggered: bool,
        cast_item_guid: Option<ObjectGuid>,
        cast_targets: SpellCastTargets,
        world: &World,
    ) -> Result<()> {
        let target_guid = cast_targets.unit_target();
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
                let mut active = ActiveCast::new_channel(
                    spell_id,
                    target_guid,
                    duration_ms,
                    tick_count,
                    is_triggered,
                    x,
                    y,
                    z,
                );
                active.cast_targets = cast_targets.clone();
                player
                    .spells
                    .set_current_spell(CurrentSpellType::Channeled, active);
            });

        // A channel executes immediately (Spell::cast runs at channel start), so the
        // success cast result is sent now, before the channel packets.
        if !is_triggered && caster_guid.is_player() {
            self.broadcast_mgr
                .send_msg_to_player(caster_guid, SmsgCastResult::success(spell_id));
        }

        // Send SMSG_CHANNEL_START
        let mut packet = oxcore_shared::protocol::WorldPacket::new(
            oxcore_shared::protocol::Opcode::MSG_CHANNEL_START,
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
                        cast_targets: cast_targets.clone(),
                    },
                );
            }
            queue.schedule(
                now + duration_ms as u64,
                SpellEventType::ChannelFinish {
                    caster_guid,
                    spell_id,
                    target_guid,
                    cast_targets,
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
        let ready_events: Vec<crate::game::player::spells::state::SpellEvent> = {
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
                    cast_targets,
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
                            self.send_cast_failure(caster_guid, spell_id, revalidate, world)?;
                            continue;
                        }

                        if !is_triggered {
                            self.take_spell_costs(caster_guid, spell_id, cast_item_guid, world)
                                .await?;
                        }

                        self.execute_spell(
                            caster_guid,
                            spell_id,
                            &cast_targets,
                            is_triggered,
                            world,
                        )
                        .await?;
                        self.finish_cast(
                            caster_guid,
                            spell_id,
                            &cast_targets,
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
                    cast_targets,
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
                        // MaNGOS Spell::update HasValidUnitPresentInTargetList: a channel whose
                        // unit target has vanished or died must be cancelled, not ticked.
                        if !Self::is_channel_target_valid(target_guid, caster_guid, world) {
                            self.cancel_spell_in_slot(
                                caster_guid,
                                CurrentSpellType::Channeled,
                                world,
                            )
                            .await?;
                        } else {
                            self.execute_channel_tick(caster_guid, spell_id, &cast_targets, world)
                                .await?;
                        }
                    }
                }
                SpellEventType::ChannelFinish {
                    caster_guid,
                    spell_id,
                    target_guid,
                    cast_targets,
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
                        self.finish_cast(caster_guid, spell_id, &cast_targets, false, None, world)
                            .await?;
                    }
                }
                SpellEventType::DelayedEffect {
                    caster_guid,
                    spell_id,
                    target_guid,
                    is_triggered,
                    cast_targets,
                } => {
                    tracing::info!(
                        "[SPELL_PROJECTILE_HIT] spell={spell_id} caster={caster_guid:?} target={target_guid:?} — executing delayed damage"
                    );
                    self.execute_spell_immediate(
                        caster_guid,
                        spell_id,
                        &cast_targets,
                        is_triggered,
                        world,
                    )
                    .await?;
                }
                SpellEventType::PendingProc {
                    caster_guid,
                    spell_id,
                    is_triggered,
                } => {
                    self.process_pending_procs(caster_guid, spell_id, is_triggered, world)
                        .await;
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
        use crate::game::player::spells::state::DelayedSpellEffect;

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
            let cast_targets = SpellCastTargets {
                unit_target_guid: effect.target_guid,
                ..Default::default()
            };
            self.execute_spell_immediate(
                effect.caster_guid,
                effect.spell_id,
                &cast_targets,
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
                for slot_idx in 0..crate::game::player::spells::state::NUM_CURRENT_SPELLS {
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
                        self.send_cast_failure(player_guid, spell_id, revalidate, world)?;
                        continue;
                    }

                    let cast_targets = SpellCastTargets {
                        unit_target_guid: target_guid,
                        ..Default::default()
                    };
                    self.execute_spell(player_guid, spell_id, &cast_targets, is_triggered, world)
                        .await?;
                    self.finish_cast(
                        player_guid,
                        spell_id,
                        &cast_targets,
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
                    let cast_targets = SpellCastTargets {
                        unit_target_guid: target_guid,
                        ..Default::default()
                    };
                    self.execute_channel_tick(player_guid, spell_id, &cast_targets, world)
                        .await?;
                }
                CastUpdateInfo::ChannelComplete {
                    spell_id,
                    target_guid,
                } => {
                    let cast_targets = SpellCastTargets {
                        unit_target_guid: target_guid,
                        ..Default::default()
                    };
                    self.finish_cast(player_guid, spell_id, &cast_targets, false, None, world)
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
        cast_targets: &SpellCastTargets,
        is_triggered: bool,
        world: &World,
    ) -> Result<()> {
        let target_guid = cast_targets.unit_target();

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
                            cast_targets: cast_targets.clone(),
                        },
                    );
                }
                return Ok(());
            }
        }

        // Immediate execution
        self.execute_spell_immediate(caster_guid, spell_id, cast_targets, is_triggered, world)
            .await
    }

    /// Execute spell effects immediately (no travel time).
    async fn execute_spell_immediate(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        cast_targets: &SpellCastTargets,
        is_triggered: bool,
        world: &World,
    ) -> Result<()> {
        use crate::game::player::spells::targets;

        let target_guid = cast_targets.unit_target();
        let resolved = targets::resolve_spell_targets(spell_id, cast_targets, caster_guid, world);

        self.effects_dispatcher
            .dispatch_with_targets(
                caster_guid,
                spell_id,
                target_guid,
                is_triggered,
                Some(&resolved),
                None,
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
                .with_creature(target_guid, |c| oxcore_shared::protocol::Position {
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
        cast_targets: &SpellCastTargets,
        world: &World,
    ) -> Result<()> {
        // Channel ticks re-execute the spell effects
        self.execute_spell(caster_guid, spell_id, cast_targets, true, world)
            .await
    }

    /// Finish a spell cast. Broadcasts SMSG_SPELL_GO, applies cooldown.
    async fn finish_cast(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        cast_targets: &SpellCastTargets,
        is_triggered: bool,
        cast_item_guid: Option<ObjectGuid>,
        world: &World,
    ) -> Result<()> {
        use crate::game::player::spells::targets;

        let target_guid = cast_targets.unit_target();

        // Resolve targets for SMSG_SPELL_GO hit list
        let resolved = targets::resolve_spell_targets(spell_id, cast_targets, caster_guid, world);

        // Collect all unique hit targets across all effects
        let mut hit_targets: Vec<ObjectGuid> = resolved
            .effect_targets
            .iter()
            .flat_map(|t| t.iter().copied())
            .collect();
        hit_targets.sort_by_key(|g| g.raw());
        hit_targets.dedup_by_key(|g| g.raw());

        // Completion packet order matches Spell::cast: SMSG_SPELL_COOLDOWN, then the
        // success SMSG_CAST_RESULT, then SMSG_SPELL_GO.
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

            if caster_guid.is_player() {
                self.broadcast_mgr
                    .send_msg_to_player(caster_guid, SmsgCastResult::success(spell_id));
            }
        }

        // Broadcast SMSG_SPELL_GO
        let msg = SmsgSpellGo {
            caster_guid,
            caster_guid_pack: caster_guid,
            spell_id,
            cast_flags: if is_triggered { 0x0000 } else { 0x0002 },
            hit_targets,
            miss_targets: Vec::new(),
            target_guid,
            cast_item_guid,
        };
        self.broadcast_mgr
            .send_msg_to_player(caster_guid, msg.to_world_packet());

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

        // Clear combo points after a finishing move completes (MaNGOS Spell::finish).
        // MaNGOS keeps combo points if a negative spell missed an enemy; we do not yet track
        // per-target miss conditions here (miss_targets is empty), so finishers always drop them.
        if caster_guid.is_player() {
            if let Some(entry) = world.managers.spell_mgr.get(spell_id) {
                if spell_needs_combo_points(entry.attributes_ex) {
                    world
                        .systems
                        .combat
                        .clear_combo_points(caster_guid, &world.systems.player.manager());
                }
            }
        }

        self.update_pending_procs(caster_guid, spell_id, is_triggered, world)
            .await;

        Ok(())
    }

    /// Queue the pending cast-end proc check for a finished spell cast.
    ///
    /// The proc work runs on the next event pass so it stays ordered after the rest of the
    /// current spell completion work.
    async fn update_pending_procs(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        is_triggered: bool,
        world: &World,
    ) {
        if is_triggered || !caster_guid.is_player() {
            return;
        }

        let Some(entry) = world.managers.spell_mgr.get(spell_id) else {
            return;
        };

        use crate::game::player::auras::proc::{proc_flags_ex, spell_cast_attacker_proc_flag};

        const SUPPRESS_CASTER_PROCS: u32 = 0x0001_0000; // SPELL_ATTR_EX3_SUPPRESS_CASTER_PROCS
        if entry.attributes_ex3 & SUPPRESS_CASTER_PROCS != 0 {
            return;
        }

        let is_auto_repeat = (entry.attributes_ex2 & 0x0000_0020) != 0;
        let is_heal = entry.effect.iter().any(|&e| e == 10); // SPELL_EFFECT_HEAL
        let proc_attacker = spell_cast_attacker_proc_flag(
            entry.dmg_class,
            entry.is_positive_spell(),
            is_heal,
            is_auto_repeat,
        );

        if proc_attacker == 0 {
            return;
        }

        if let Ok(mut queue) = self.event_queue.lock() {
            queue.schedule(
                get_game_time_ms(),
                SpellEventType::PendingProc {
                    caster_guid,
                    spell_id,
                    is_triggered,
                },
            );
        }
    }

    /// Run the pending cast-end proc check once it reaches the event queue.
    async fn process_pending_procs(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        is_triggered: bool,
        world: &World,
    ) {
        if is_triggered || !caster_guid.is_player() {
            return;
        }

        let Some(entry) = world.managers.spell_mgr.get(spell_id) else {
            return;
        };

        use crate::game::player::auras::proc::{proc_flags_ex, spell_cast_attacker_proc_flag};

        const SUPPRESS_CASTER_PROCS: u32 = 0x0001_0000; // SPELL_ATTR_EX3_SUPPRESS_CASTER_PROCS
        if entry.attributes_ex3 & SUPPRESS_CASTER_PROCS != 0 {
            return;
        }

        let is_auto_repeat = (entry.attributes_ex2 & 0x0000_0020) != 0;
        let is_heal = entry.effect.iter().any(|&e| e == 10); // SPELL_EFFECT_HEAL
        let proc_attacker = spell_cast_attacker_proc_flag(
            entry.dmg_class,
            entry.is_positive_spell(),
            is_heal,
            is_auto_repeat,
        );

        if proc_attacker == 0 {
            return;
        }

        let _ = world
            .systems
            .auras
            .check_procs(
                caster_guid,
                proc_attacker,
                proc_flags_ex::CAST_END | proc_flags_ex::NORMAL_HIT,
                Some(spell_id),
                0,
                world,
            )
            .await;
    }

    /// Whether a channeled spell's unit target is still present and alive.
    ///
    /// MaNGOS cancels a channel via `HasValidUnitPresentInTargetList` when the target has
    /// despawned or died. A self-channel or a channel with no unit target stays valid (those
    /// are area/ground channels which do not depend on a single unit).
    fn is_channel_target_valid(
        target_guid: Option<ObjectGuid>,
        caster_guid: ObjectGuid,
        world: &World,
    ) -> bool {
        let target = match target_guid {
            Some(t) if t != caster_guid => t,
            _ => return true,
        };

        if target.is_player() {
            world
                .systems
                .player
                .manager()
                .with_player(target, |p| p.stats.health > 0)
                .unwrap_or(false)
        } else if target.is_creature() {
            world
                .managers
                .creature_mgr
                .with_creature(target, |c| c.current_health > 0)
                .unwrap_or(false)
        } else {
            // GameObjects and other target types have no aliveness concept here.
            true
        }
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

    /// Cancel the current auto-repeat spell, such as Auto Shot or wand Shoot.
    pub async fn cancel_auto_repeat_spell(
        &self,
        caster_guid: ObjectGuid,
        world: &World,
    ) -> Result<()> {
        self.cancel_spell_in_slot(caster_guid, CurrentSpellType::Autorepeat, world)
            .await?;
        Ok(())
    }

    /// Interrupt all non-melee spells (MaNGOS SpellCaster::InterruptNonMeleeSpells).
    ///
    /// Interrupts Generic, Autorepeat, and Channeled slots.
    /// If `spell_id` is Some(nonzero), only interrupts slots containing that spell.
    /// Channeled spells are always interrupted (C++ ignores withDelayed for channeled).
    pub async fn interrupt_non_melee_spells(
        &self,
        caster_guid: ObjectGuid,
        with_delayed: bool,
        spell_id: Option<u32>,
        world: &World,
    ) -> Result<()> {
        // Snapshot spell IDs in each slot outside the lock
        let state: [Option<u32>; 4] = world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                let mut ids: [Option<u32>; 4] = [None, None, None, None];
                for (i, slot) in player.spells.current_spells.iter().enumerate() {
                    if let Some(active) = slot {
                        if with_delayed || active.state != SpellState::Delayed {
                            ids[i] = Some(active.spell_id);
                        }
                    }
                }
                ids
            })
            .unwrap_or([None, None, None, None]);

        let slots = [
            CurrentSpellType::Generic,
            CurrentSpellType::Autorepeat,
            CurrentSpellType::Channeled,
        ];

        for slot in slots {
            let sid = state[slot as usize];
            if let Some(sid) = sid {
                if spell_id.map_or(true, |filter| sid == filter) {
                    self.cancel_spell_in_slot(caster_guid, slot, world).await?;
                }
            }
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
        // Capture: spell_id, was_channeling, was_preparing, was_triggered
        let cancelled_info: Option<(u32, bool, bool, bool)> = world
            .systems
            .player
            .manager()
            .with_player_mut(caster_guid, |player| {
                player.spells.clear_current_spell(slot).map(|active| {
                    let was_preparing = active.state == SpellState::Preparing;
                    (
                        active.spell_id,
                        active.is_channeling,
                        was_preparing,
                        active.is_triggered,
                    )
                })
            })
            .flatten();

        // Remove any pending events for this spell
        if let Some((spell_id, _, _, _)) = cancelled_info {
            if let Ok(mut queue) = self.event_queue.lock() {
                queue.cancel_events_for(caster_guid, spell_id);
            }
        }

        if let Some((spell_id, was_channeling, was_preparing, was_triggered)) = cancelled_info {
            // MaNGOS Spell::cancel PREPARING branch: reset GCD so the player isn't locked out
            // after cancelling a cast that hasn't fired yet.
            if was_preparing && !was_triggered {
                let now = get_game_time_ms();
                world
                    .systems
                    .player
                    .manager()
                    .with_player_mut(caster_guid, |player| {
                        player.spells.gcd_end = now;
                    });
            }

            // Send SMSG_CANCEL_AUTO_REPEAT for autorepeat spells on players,
            // matching MaNGOS SpellCaster::InterruptSpell SendAutoRepeatCancel behaviour.
            if slot == CurrentSpellType::Autorepeat && caster_guid.is_player() {
                use oxcore_shared::protocol::{Opcode, WorldPacket};
                self.broadcast_mgr
                    .send_msg_to_player(caster_guid, WorldPacket::new(Opcode::SMSG_CANCEL_AUTO_REPEAT));
            }

            // Broadcast SMSG_SPELL_FAILURE
            let msg = SmsgSpellFailure {
                caster_guid,
                spell_id,
                result: SpellCastError::Interrupted as u8,
            };
            self.broadcast_mgr
                .send_msg_to_player(caster_guid, msg.to_world_packet());

            // Also send SMSG_CAST_RESULT(SPELL_FAILED_INTERRUPTED) so the client cast bar clears
            // (MaNGOS Spell::cancel sends SendCastResult on the PREPARING/DELAYED cancel path).
            self.send_cast_failure(caster_guid, spell_id, SpellCastError::Interrupted, world)?;

            // If cancelling a channel, send SMSG_CHANNEL_UPDATE with 0 remaining
            // and remove auras applied by the cancelled channel (MaNGOS RemoveAurasByCasterSpell)
            if was_channeling {
                let mut packet = oxcore_shared::protocol::WorldPacket::new(
                    oxcore_shared::protocol::Opcode::MSG_CHANNEL_UPDATE,
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

        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                // Apply pushback to Generic slot first, then Channeled
                let slot_idx =
                    if player.spells.current_spells[CurrentSpellType::Generic as usize].is_some() {
                        CurrentSpellType::Generic as usize
                    } else {
                        CurrentSpellType::Channeled as usize
                    };

                if let Some(active) = player.spells.current_spells[slot_idx].as_mut() {
                    // Check NotLoseCastTime spell modifier (reduces pushback, e.g., Concentration Aura)
                    let mut pushback_reduction_pct = 0i32;
                    for modifier in &player.spells.spell_modifiers {
                        if modifier.op
                            == crate::game::player::spells::state::SpellModOp::NotLoseCastTime
                        {
                            pushback_reduction_pct += modifier.value;
                        }
                    }

                    // Vanilla pushback values
                    let max_pushback = 1000u32; // 1 second max total for non-channeled
                    let base_pushback = 500u32; // 0.5 second per hit for non-channeled

                    // Apply pushback reduction
                    let pushback_per_hit = if pushback_reduction_pct > 0 {
                        let reduction =
                            (base_pushback as f32 * pushback_reduction_pct as f32 / 100.0) as u32;
                        base_pushback.saturating_sub(reduction)
                    } else {
                        base_pushback
                    };

                    pushback_applied = active.apply_pushback(pushback_per_hit, max_pushback);

                    if pushback_applied > 0 {
                        spell_id_for_reschedule = Some(active.spell_id);
                        tracing::debug!(
                            "Cast pushback: pushed back {}ms on spell {} (reduction={}%)",
                            pushback_applied,
                            active.spell_id,
                            pushback_reduction_pct
                        );
                    }
                }
            });

        Ok(pushback_applied)
    }

    // =========================================================================
    // Resource Consumption
    // =========================================================================

    /// Consume the power cost of a spell cast (faithful `Spell::TakePower`).
    ///
    /// `cast_item_guid` is the item the spell was cast from, if any: item casts pay
    /// no power. Triggered-by-aura and channeling-visual casts are already excluded
    /// because this is only called for non-triggered casts.
    async fn consume_resources(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        cast_item_guid: Option<ObjectGuid>,
        world: &World,
    ) -> Result<()> {
        // Item casts use no power (C++: `if (m_CastItem ...) return;`).
        if cast_item_guid.is_some() {
            return Ok(());
        }

        let spell_entry = match world.managers.spell_mgr.get(spell_id) {
            Some(entry) => (*entry).clone(),
            None => return Ok(()),
        };

        // PLAYER_CHEAT_NO_POWER bypass is deferred (no cheat-option system yet).

        let cost = self.calculate_power_cost(caster_guid, &spell_entry, world)?;

        const POWER_HEALTH: u32 = 0xFFFF_FFFE;
        const MAX_POWERS: u32 = 5;

        // Health as power: deduct health directly and return.
        if spell_entry.power_type == POWER_HEALTH {
            self.spend_health_cost(caster_guid, cost, world);
            return Ok(());
        }

        if spell_entry.power_type >= MAX_POWERS {
            tracing::error!(
                "Spell::TakePower: unknown power type {} for spell {}",
                spell_entry.power_type,
                spell_id
            );
            return Ok(());
        }

        let power_type =
            match crate::game::player::power::PowerType::from_u8(spell_entry.power_type as u8) {
                Some(pt) => pt,
                None => return Ok(()),
            };

        // Mana spells reset the five-second rule unless flagged DONT_BLOCK_MANA_REGEN.
        const SPELL_ATTR_EX2_DONT_BLOCK_MANA_REGEN: u32 = 0x0200_0000;
        let reset_mana_timer =
            cost > 0 && spell_entry.attributes_ex2 & SPELL_ATTR_EX2_DONT_BLOCK_MANA_REGEN == 0;

        world.systems.power.spend_spell_power(
            caster_guid,
            power_type,
            cost,
            reset_mana_timer,
            world,
        )?;

        Ok(())
    }

    /// Deduct a health-as-power cost and broadcast the new health value.
    fn spend_health_cost(&self, caster_guid: ObjectGuid, cost: u32, world: &World) {
        use crate::game::common::update_fields::UNIT_FIELD_HEALTH;
        use oxcore_shared::messages::update::{
            ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
        };

        let new_health = world
            .systems
            .player
            .manager()
            .with_player_mut(caster_guid, |player| {
                player.stats.modify_health(-(cost as i32));
                player.stats.dirty = true;
                player.stats.health
            });

        if let Some(new_health) = new_health {
            let block = ValuesUpdateBlock::new(caster_guid, ObjectType::Player)
                .set_field(UNIT_FIELD_HEALTH, new_health);
            let packet = SmsgUpdateObject::new()
                .add_block(UpdateBlockData::Values(block))
                .to_world_packet();
            self.broadcast_mgr
                .broadcast_nearby(caster_guid, &packet, true);
        }
    }

    /// Consume a spell's reagents on cast (faithful `Spell::TakeReagents`).
    ///
    /// Player-only. Called for non-triggered casts, where `IgnoreItemRequirements`
    /// is always false (triggered casts have their reagents removed by the master
    /// spell). The cast-item / item-target reagent-overlap adjustments are deferred
    /// until item-instance spell charges exist (see `TakeCastItem`).
    fn take_reagents(&self, caster_guid: ObjectGuid, spell_id: u32, world: &World) {
        if !caster_guid.is_player() {
            return;
        }

        let spell_entry = match world.managers.spell_mgr.get(spell_id) {
            Some(entry) => entry,
            None => return,
        };

        for x in 0..spell_entry.reagent.len() {
            let reagent = spell_entry.reagent[x];
            if reagent <= 0 {
                continue;
            }
            let item_id = reagent as u32;
            let item_count = spell_entry.reagent_count[x];
            if item_count == 0 {
                continue;
            }

            world
                .systems
                .inventory
                .destroy_item_count(caster_guid, item_id, item_count);
        }
    }

    /// Restore power to a target from a spell, broadcasting the energize combat log
    /// (faithful `SpellCaster::EnergizeBySpell` + `SpellCaster::SendEnergizeSpellLog`).
    ///
    /// The `SMSG_SPELLENERGIZELOG` packet is sent to the caster's visibility set
    /// (including the caster) *before* the power is modified — the C++ ordering, where
    /// the log must precede `ModifyPower`.
    pub fn energize_by_spell(
        &self,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        spell_id: u32,
        amount: u32,
        power_type: crate::game::player::power::PowerType,
        world: &World,
    ) -> Result<()> {
        use oxcore_shared::protocol::packet::WorldPacketGuidExt;
        use oxcore_shared::protocol::{Opcode, WorldPacket};

        let mut packet = WorldPacket::new(Opcode::SMSG_SPELLENERGIZELOG);
        packet.write_packed_guid(target_guid);
        packet.write_packed_guid(caster_guid);
        packet.write_u32(spell_id);
        packet.write_u32(power_type as u32);
        packet.write_u32(amount);
        self.broadcast_mgr
            .broadcast_nearby(caster_guid, &packet, true);

        world
            .systems
            .power
            .restore_power(target_guid, power_type, amount, world)
    }

    /// Broadcast `SMSG_SPELLLOGMISS` for a spell that missed a target
    /// (faithful `SpellCaster::SendSpellMiss`).
    pub fn send_spell_miss(
        &self,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        spell_id: u32,
        miss_info: hit::SpellMissInfo,
    ) {
        use oxcore_shared::protocol::packet::WorldPacketGuidExt;
        use oxcore_shared::protocol::{Opcode, WorldPacket};

        let mut packet = WorldPacket::new(Opcode::SMSG_SPELLLOGMISS);
        packet.write_u32(spell_id);
        packet.write_guid(caster_guid);
        packet.write_u8(0); // unk8
        packet.write_u32(1); // target count
        packet.write_guid(target_guid);
        packet.write_u8(miss_info as u8);
        self.broadcast_mgr
            .broadcast_nearby(caster_guid, &packet, true);
    }

    /// Broadcast `SMSG_PROCRESIST` for a spell resisted by a target
    /// (faithful `SpellCaster::SendSpellDamageResist`).
    pub fn send_spell_damage_resist(
        &self,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        spell_id: u32,
    ) {
        use oxcore_shared::protocol::packet::WorldPacketGuidExt;
        use oxcore_shared::protocol::{Opcode, WorldPacket};

        let mut packet = WorldPacket::new(Opcode::SMSG_PROCRESIST);
        packet.write_guid(caster_guid);
        packet.write_guid(target_guid);
        packet.write_u32(spell_id);
        packet.write_u8(0); // log format: 0-default, 1-debug
        self.broadcast_mgr
            .broadcast_nearby(caster_guid, &packet, true);
    }

    /// Broadcast `SMSG_SPELLORDAMAGE_IMMUNE` for a spell a target was immune to
    /// (faithful `SpellCaster::SendSpellOrDamageImmune`).
    pub fn send_spell_or_damage_immune(
        &self,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        spell_id: u32,
    ) {
        use oxcore_shared::protocol::packet::WorldPacketGuidExt;
        use oxcore_shared::protocol::{Opcode, WorldPacket};

        let mut packet = WorldPacket::new(Opcode::SMSG_SPELLORDAMAGE_IMMUNE);
        packet.write_guid(caster_guid);
        packet.write_guid(target_guid);
        packet.write_u32(spell_id);
        packet.write_u8(0);
        self.broadcast_mgr
            .broadcast_nearby(caster_guid, &packet, true);
    }

    /// Calculate the power cost of a spell (faithful `Spell::CalculatePowerCost`).
    fn calculate_power_cost(
        &self,
        caster_guid: ObjectGuid,
        spell_entry: &crate::dbc::structures::SpellEntry,
        world: &World,
    ) -> Result<u32> {
        // Snapshot the caster state the cost formula reads, then compute the cost.
        let cost = world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                let pt = spell_entry.power_type as usize;
                let (current_power, max_power) = if pt < 5 {
                    (player.power.current[pt], player.power.max[pt])
                } else {
                    (0, 0)
                };

                // GetSpellRank: player skill-based rank is deferred; the per-level cost
                // term (manaCostPerlevel) is 0 for all 1.12 player spells, so the
                // level-clamped Unit formula is exact where it matters (creature spells).
                let level = player.level as u32;
                let spell_rank = if spell_entry.max_level > 0 && level >= spell_entry.max_level * 5
                {
                    spell_entry.max_level * 5
                } else {
                    level
                };

                let ctx = modifiers::PowerCostContext {
                    health: player.stats.health,
                    create_health: player.stats.base_health,
                    create_mana: player.stats.base_mana,
                    current_power,
                    max_power,
                    level,
                    spell_rank,
                };

                modifiers::calculate_power_cost(
                    spell_entry,
                    &ctx,
                    false,
                    &player.spells.spell_modifiers,
                )
            })
            .unwrap_or(0);

        Ok(cost)
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

    /// Apply Global Cooldown after casting (Player::AddGCD).
    async fn apply_gcd(&self, caster_guid: ObjectGuid, spell_id: u32, world: &World) -> Result<()> {
        // Get spell entry
        let spell_entry = match world.managers.spell_mgr.get(spell_id) {
            Some(entry) => entry,
            None => return Ok(()), // No GCD if spell not found
        };

        // GCD group category for the standard 1.5s cooldown, plus the attribute
        // and damage-class values that exempt a spell from haste scaling.
        const SPELLCATEGORY_GLOBAL: u32 = 133;
        const SPELL_ATTR_USES_RANGED_SLOT: u32 = 0x0000_0002;
        const SPELL_ATTR_IS_ABILITY: u32 = 0x0000_0010;
        const SPELL_DAMAGE_CLASS_MELEE: u32 = 2;
        const SPELL_DAMAGE_CLASS_RANGED: u32 = 3;

        // Base GCD comes straight from StartRecoveryTime.
        let mut gcd_duration = spell_entry.start_recovery_time as i32;

        // No GCD at all when the spell has neither a recovery category nor time.
        if spell_entry.start_recovery_category == 0 && gcd_duration == 0 {
            return Ok(());
        }

        // Apply SPELLMOD_GLOBAL_COOLDOWN modifiers (self-only, player-only mods).
        gcd_duration = modifiers::calculate_modified_gcd(
            caster_guid,
            gcd_duration.max(0) as u32,
            spell_entry.spell_family_name,
            spell_entry.spell_family_flags,
            world,
        ) as i32;

        // Haste scaling applies only to the standard 1.5s global cooldown on
        // non-melee/non-ranged, non-ability spells: scale by UNIT_MOD_CAST_SPEED
        // then clamp to [1000, 1500]. Cast-speed haste is not modelled yet, so the
        // multiplier defaults to 1.0 (a caster with no haste), leaving 1500 intact.
        if spell_entry.start_recovery_category == SPELLCATEGORY_GLOBAL
            && gcd_duration == 1500
            && spell_entry.dmg_class != SPELL_DAMAGE_CLASS_MELEE
            && spell_entry.dmg_class != SPELL_DAMAGE_CLASS_RANGED
            && spell_entry.attributes & SPELL_ATTR_USES_RANGED_SLOT == 0
            && spell_entry.attributes & SPELL_ATTR_IS_ABILITY == 0
        {
            let cast_speed_mult = 1.0_f32; // TODO: UNIT_MOD_CAST_SPEED haste
            gcd_duration = (gcd_duration as f32 * cast_speed_mult) as i32;
            gcd_duration = gcd_duration.clamp(1000, 1500);
        }

        if gcd_duration < 1 {
            return Ok(());
        }

        // C++ subtracts CONFIG_UINT32_INTERVAL_MAPUPDATE here because GCD packets
        // flush on map update; Rust tracks GCD against wall-clock game time, so
        // there is no batching interval to subtract.

        let gcd_ms = gcd_duration as u32;
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

    /// Spell::SendCastResult — build and send SMSG_CAST_RESULT for a cast rejection.
    ///
    /// Mirrors the packet-builder overload: passive spells report DONT_REPORT
    /// instead of the real reason; spells flagged DO_NOT_REPORT_SPELL_FAILURE
    /// report OKAY; and specific errors carry extra arguments (spell focus,
    /// equipped-item class/subclass, permanent-cooldown flag).
    fn send_cast_failure(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        error: SpellCastError,
        world: &World,
    ) -> Result<()> {
        const SPELL_ATTR_COOLDOWN_ON_EVENT: u32 = 0x0200_0000;
        const SPELL_ATTR_EX2_DO_NOT_REPORT_SPELL_FAILURE: u32 = 0x0000_0080;

        let spell_entry = world.managers.spell_mgr.get(spell_id);

        // DO_NOT_REPORT_SPELL_FAILURE spells report OKAY to the client.
        let suppress = spell_entry
            .as_ref()
            .map_or(false, |e| e.attributes_ex2 & SPELL_ATTR_EX2_DO_NOT_REPORT_SPELL_FAILURE != 0);

        if error == SpellCastError::None || suppress {
            let packet = SmsgCastResult::build(spell_id, SPELL_RESULT_STATUS_OKAY, 0, None, None);
            self.broadcast_mgr.send_msg_to_player(caster_guid, packet);
            return Ok(());
        }

        // Passive spells hide the real failure reason behind DONT_REPORT.
        let is_passive = spell_entry.as_ref().map_or(false, |e| e.is_passive_spell());
        let reason_error = if is_passive {
            SpellCastError::DontReport
        } else {
            error
        };
        let failure_reason = validation::spell_cast_error_to_u8(reason_error);

        // Optional per-error arguments. Some(_) is written even when the value is
        // 0 — the C++ optional's presence, not its value, gates serialization.
        let (arg1, arg2) = if let Some(e) = spell_entry.as_ref() {
            match error {
                SpellCastError::NotReady | SpellCastError::SpellOnCooldown => {
                    if e.attributes & SPELL_ATTR_COOLDOWN_ON_EVENT != 0 {
                        // Permanent cooldowns are not modelled yet → always 0/false.
                        (Some(0u32), None)
                    } else {
                        (None, None)
                    }
                }
                SpellCastError::RequiresSpellFocus => (Some(e.requires_spell_focus), None),
                SpellCastError::EquippedItemClass
                | SpellCastError::EquippedItemClassMainhand
                | SpellCastError::EquippedItemClassOffhand => (
                    Some(e.equipped_item_class as u32),
                    Some(e.equipped_item_sub_class_mask as u32),
                ),
                _ => (None, None),
            }
        } else {
            (None, None)
        };

        let packet = SmsgCastResult::build(
            spell_id,
            SPELL_RESULT_STATUS_FAIL,
            failure_reason,
            arg1,
            arg2,
        );
        self.broadcast_mgr.send_msg_to_player(caster_guid, packet);

        Ok(())
    }

    /// Send a SMSG_CAST_RESULT failure to the caster for a pre-pipeline rejection
    /// (e.g. a handler refusing a cast before it reaches `cast_spell`).
    pub fn send_cast_result(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        error: SpellCastError,
        world: &World,
    ) {
        let _ = self.send_cast_failure(caster_guid, spell_id, error, world);
    }

    // =========================================================================
    // SpellCaster Query Methods
    // =========================================================================

    /// CheckAndIncreaseCastCounter: limit casts in chain per config.
    pub fn check_and_increase_cast_counter(
        &self,
        caster_guid: ObjectGuid,
        world: &World,
    ) -> bool {
        let max_casts = 20; // TODO: CONFIG_UINT32_MAX_SPELL_CASTS_IN_CHAIN
        world
            .systems
            .player
            .manager()
            .with_player_mut(caster_guid, |player| {
                if max_casts > 0 && player.spells.cast_counter >= max_casts {
                    false
                } else {
                    player.spells.cast_counter += 1;
                    true
                }
            })
            .unwrap_or(false)
    }

    /// IsNextSwingSpellCasted: check if the Melee slot holds a next-swing spell.
    pub fn is_next_swing_spell_casted(
        &self,
        caster_guid: ObjectGuid,
        world: &World,
    ) -> bool {
        world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                player
                    .spells
                    .get_next_swing_spell_id()
                    .and_then(|spell_id| world.managers.spell_mgr.get(spell_id))
                    .map(|entry| {
                        // IsNextMeleeSwingSpell = AttributesEx & 0x00000004
                        (entry.attributes_ex & 0x0000_0004) != 0
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    /// IsNoMovementSpellCasted: check if a currently casting spell has movement interrupt flags.
    pub fn is_no_movement_spell_casted(
        &self,
        caster_guid: ObjectGuid,
        world: &World,
    ) -> bool {
        let spell_interrupt_flag_movement: u32 = 0x00000008; // SPELL_INTERRUPT_FLAG_MOVEMENT
        let aura_interrupt_moving_cancels: u32 = 0x00000400; // AURA_INTERRUPT_MOVING_CANCELS

        world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                // Generic slot: not finished, not delayed, has movement interrupt flag
                if let Some(ref cast) =
                    player.spells.current_spells[CurrentSpellType::Generic as usize]
                {
                    if cast.state != SpellState::Finished
                        && cast.state != SpellState::Delayed
                    {
                        if let Some(entry) = world.managers.spell_mgr.get(cast.spell_id) {
                            if (entry.interrupt_flags & spell_interrupt_flag_movement) != 0 {
                                return true;
                            }
                        }
                    }
                }
                // Channeled slot: not finished, has movement or channel interrupt flag
                if let Some(ref cast) =
                    player.spells.current_spells[CurrentSpellType::Channeled as usize]
                {
                    if cast.state != SpellState::Finished {
                        if let Some(entry) = world.managers.spell_mgr.get(cast.spell_id) {
                            if (entry.interrupt_flags & spell_interrupt_flag_movement) != 0
                                || (entry.channel_interrupt_flags & aura_interrupt_moving_cancels) != 0
                            {
                                return true;
                            }
                        }
                    }
                }
                false
            })
            .unwrap_or(false)
    }

    // =========================================================================
    // SpellCaster Interrupt Methods
    // =========================================================================

    /// InterruptSpellsWithInterruptFlags: interrupt non-melee spells whose InterruptFlags match.
    pub async fn interrupt_spells_with_interrupt_flags(
        &self,
        caster_guid: ObjectGuid,
        flags: u32,
        except_spell_id: u32,
        world: &World,
    ) -> Result<()> {
        let slots_to_interrupt: Vec<CurrentSpellType> = world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                let mut result = Vec::new();
                for slot_idx in 0..4 {
                    let slot = match slot_idx {
                        0 => CurrentSpellType::Melee,
                        1 => CurrentSpellType::Autorepeat,
                        2 => CurrentSpellType::Channeled,
                        3 => CurrentSpellType::Generic,
                        _ => continue,
                    };
                    if slot == CurrentSpellType::Melee {
                        continue;
                    }
                    if let Some(ref cast) = player.spells.current_spells[slot as usize] {
                        if cast.state == SpellState::Finished {
                            continue;
                        }
                        if let Some(entry) = world.managers.spell_mgr.get(cast.spell_id) {
                            let casted_time = cast.original_cast_time_ms;
                            if casted_time == 0 {
                                continue;
                            }
                            let is_channeled = slot == CurrentSpellType::Channeled;
                            let is_preparing_or_casting = if is_channeled {
                                cast.state == SpellState::Casting
                            } else {
                                cast.state == SpellState::Preparing
                                    || cast.state == SpellState::Casting
                            };
                            if !is_preparing_or_casting {
                                continue;
                            }
                            let is_next_swing =
                                (entry.attributes_ex & 0x0000_0004) != 0;
                            let is_autorepeat = slot == CurrentSpellType::Autorepeat;
                            let is_triggered = cast.is_triggered;
                            if !is_next_swing
                                && !is_autorepeat
                                && !is_triggered
                                && (entry.interrupt_flags & flags) != 0
                                && cast.spell_id != except_spell_id
                            {
                                result.push(slot);
                            }
                        }
                    }
                }
                result
            })
            .unwrap_or_default();

        for slot in slots_to_interrupt {
            self.cancel_spell_in_slot(caster_guid, slot, world).await?;
        }

        Ok(())
    }

    /// InterruptSpellsWithChannelFlags: interrupt channeled spell whose ChannelInterruptFlags match.
    pub async fn interrupt_spells_with_channel_flags(
        &self,
        caster_guid: ObjectGuid,
        flags: u32,
        except_spell_id: u32,
        world: &World,
    ) -> Result<()> {
        let should_interrupt = world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                if let Some(ref cast) =
                    player.spells.current_spells[CurrentSpellType::Channeled as usize]
                {
                    if cast.state == SpellState::Casting {
                        if let Some(entry) = world.managers.spell_mgr.get(cast.spell_id) {
                            return (entry.channel_interrupt_flags & flags) != 0
                                && cast.spell_id != except_spell_id;
                        }
                    }
                }
                false
            })
            .unwrap_or(false);

        if should_interrupt {
            self.cancel_spell_in_slot(caster_guid, CurrentSpellType::Channeled, world)
                .await?;
        }

        Ok(())
    }

    // =========================================================================
    // SpellCaster FinishSpell
    // =========================================================================

    /// FinishSpell: public wrapper that finishes the cast in the given slot.
    /// For channeled spells, sends MSG_CHANNEL_UPDATE(0) first.
    pub async fn finish_spell(
        &self,
        caster_guid: ObjectGuid,
        slot: CurrentSpellType,
        ok: bool,
        world: &World,
    ) -> Result<()> {
        let spell_info: Option<(u32, bool)> = world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                player.spells.get_current_spell(slot).map(|cast| {
                    (cast.spell_id, cast.is_channeling)
                })
            })
            .flatten();

        if let Some((spell_id, is_channeled)) = spell_info {
            if is_channeled || slot == CurrentSpellType::Channeled {
                let mut packet = oxcore_shared::protocol::WorldPacket::new(
                    oxcore_shared::protocol::Opcode::MSG_CHANNEL_UPDATE,
                );
                packet.write_u32(0);
                self.broadcast_mgr.send_msg_to_player(caster_guid, packet);
            }

            // Delegate to finish_cast which sends SMSG_CAST_RESULT + SMSG_SPELL_GO + cooldowns
            let targets: SpellCastTargets = world
                .systems
                .player
                .manager()
                .with_player_mut(caster_guid, |player| {
                    player
                        .spells
                        .clear_current_spell(slot)
                        .map(|cast| cast.cast_targets)
                })
                .flatten()
                .unwrap_or_default();

            if ok {
                self.finish_cast(caster_guid, spell_id, &targets, false, None, world)
                    .await?;
            } else {
                self.send_cast_failure(
                    caster_guid,
                    spell_id,
                    SpellCastError::Interrupted,
                    world,
                )?;
            }
        }

        Ok(())
    }

    /// Move a channelled spell with a cast time from the Generic slot to the Channeled slot
    /// after the initial cast completes. (MaNGOS SpellCaster::MoveChannelledSpellWithCastTime)
    pub fn move_channelled_spell_with_cast_time(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        world: &World,
    ) {
        world
            .systems
            .player
            .manager()
            .with_player_mut(caster_guid, |player| {
                let state = &mut player.spells;

                let cast = state
                    .get_current_spell(CurrentSpellType::Generic)
                    .filter(|c| c.spell_id == spell_id)
                    .filter(|c| c.is_channeling)
                    .filter(|c| !c.is_triggered)
                    .filter(|c| c.state == SpellState::Casting)
                    .cloned();

                if let Some(mut cast) = cast {
                    if let Some(existing) =
                        state.get_current_spell(CurrentSpellType::Channeled)
                    {
                        if existing.spell_id == spell_id {
                            state.clear_current_spell(CurrentSpellType::Channeled);
                        }
                    }

                    state.clear_current_spell(CurrentSpellType::Generic);
                    cast.slot = CurrentSpellType::Channeled;
                    state.set_current_spell(CurrentSpellType::Channeled, cast);
                }
            });
    }

    /// IsSpellReady: check if a spell is ready to cast (no cooldown, not locked out by silence).
    pub fn is_spell_ready(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        world: &World,
    ) -> bool {
        let spell_entry = match world.managers.spell_mgr.get(spell_id) {
            Some(e) => e,
            None => return false,
        };
        let now = get_game_time_ms();

        world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                let state = &player.spells;

                if state.is_on_cooldown(spell_id, now) {
                    return false;
                }

                if spell_entry.category > 0 {
                    if let Some(&cd_end) = state.category_cooldowns.get(&spell_entry.category) {
                        if cd_end > now {
                            return false;
                        }
                    }
                }

                if spell_entry.prevention_type == SPELL_PREVENTION_TYPE_SILENCE as u32
                    && state.check_lockout_by_mask(1 << spell_entry.school, now)
                {
                    return false;
                }

                true
            })
            .unwrap_or(false)
    }

    /// IsSpellOnPermanentCooldown: stub — returns false since Rust doesn't track permanent CDs.
    pub fn is_spell_on_permanent_cooldown(
        &self,
        _caster_guid: ObjectGuid,
        _spell_id: u32,
        _world: &World,
    ) -> bool {
        false
    }

    /// TriggerProccedSpell: fire a spell triggered by a proc, with readiness check and optional
    /// forced cooldown. Returns true if the spell was cast.
    pub async fn trigger_procced_spell(
        &self,
        caster_guid: ObjectGuid,
        target_guid: Option<ObjectGuid>,
        triggered_spell_id: u32,
        forced_cooldown: u32,
        world: &World,
    ) -> Result<bool> {
        if world.managers.spell_mgr.get(triggered_spell_id).is_none() {
            return Ok(false);
        }

        if !self.is_spell_ready(caster_guid, triggered_spell_id, world) {
            return Ok(false);
        }

        self.cast_spell(caster_guid, triggered_spell_id, target_guid, true, world)
            .await?;

        if forced_cooldown > 0 {
            let now = get_game_time_ms();
            world
                .systems
                .player
                .manager()
                .with_player_mut(caster_guid, |player| {
                    player.spells.add_cooldown(triggered_spell_id, forced_cooldown, now);
                });
        }

        Ok(true)
    }

    /// SetCurrentCastedSpell: handle slot-assignment interruption logic.
    /// Called before placing a new ActiveCast into the target slot.
    /// Breaks same-type spells, handles cross-slot breakage (generic/channeled/autorepeat),
    /// and respects delayed-state protection.
    pub async fn set_current_casted_spell(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        slot: CurrentSpellType,
        world: &World,
    ) -> Result<()> {
        let (category, is_channeled) = match world.managers.spell_mgr.get(spell_id) {
            Some(e) => (
                e.category,
                (e.attributes_ex & 0x04) != 0 || (e.attributes_ex & 0x40) != 0,
            ),
            None => (0, false),
        };

        // Snapshot state across all slots
        let snap: [Option<(u32, SpellState)>; 4] = world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |player| {
                let mut s: [Option<(u32, SpellState)>; 4] = [None, None, None, None];
                for (i, cast) in player.spells.current_spells.iter().enumerate() {
                    s[i] = cast.as_ref().map(|c| (c.spell_id, c.state));
                }
                s
            })
            .unwrap_or([None, None, None, None]);

        // true = existing autorepeat should not be interrupted by a new Generic/Channeled cast
        let autorepeat_keep = match snap[CurrentSpellType::Autorepeat as usize] {
            Some((ar_id, _)) => world
                .managers
                .spell_mgr
                .get(ar_id)
                .map(|e| e.category != 351)
                .unwrap_or(true),
            None => true,
        };

        // 1. Early-return if the same spell already occupies the target slot
        let slot_idx = slot as usize;
        if let Some((sid, _)) = snap[slot_idx] {
            if sid == spell_id {
                return Ok(());
            }
        }

        // 2. Break same-type spell (skip if delayed)
        if let Some((_, state)) = snap[slot_idx] {
            if state != SpellState::Delayed {
                self.cancel_spell_in_slot(caster_guid, slot, world).await?;
            }
        }

        // 3. Slot-specific cross-breakage (matching MaNGOS SetCurrentCastedSpell logic)
        match slot {
            CurrentSpellType::Generic => {
                let should_interrupt_channel = match snap[CurrentSpellType::Channeled as usize] {
                    Some((ch_sid, ch_state)) => {
                        if ch_state == SpellState::Delayed {
                            false
                        } else if !is_channeled {
                            true
                        } else {
                            ch_sid != spell_id
                        }
                    }
                    None => false,
                };

                if should_interrupt_channel {
                    self.cancel_spell_in_slot(caster_guid, CurrentSpellType::Channeled, world)
                        .await?;
                }

                if !autorepeat_keep {
                    self.cancel_spell_in_slot(caster_guid, CurrentSpellType::Autorepeat, world)
                        .await?;
                }
            }
            CurrentSpellType::Channeled => {
                if let Some((_, state)) = snap[CurrentSpellType::Generic as usize] {
                    if state != SpellState::Delayed {
                        self.cancel_spell_in_slot(
                            caster_guid,
                            CurrentSpellType::Generic,
                            world,
                        )
                        .await?;
                    }
                }

                if snap[CurrentSpellType::Channeled as usize].is_some() {
                    self.cancel_spell_in_slot(caster_guid, CurrentSpellType::Channeled, world)
                        .await?;
                }

                if !autorepeat_keep {
                    self.cancel_spell_in_slot(caster_guid, CurrentSpellType::Autorepeat, world)
                        .await?;
                }
            }
            CurrentSpellType::Autorepeat => {
                if category == 351 {
                    if let Some((_, state)) = snap[CurrentSpellType::Generic as usize] {
                        if state != SpellState::Delayed {
                            self.cancel_spell_in_slot(
                                caster_guid,
                                CurrentSpellType::Generic,
                                world,
                            )
                            .await?;
                        }
                    }
                    if let Some((_, state)) = snap[CurrentSpellType::Channeled as usize] {
                        if state != SpellState::Delayed {
                            self.cancel_spell_in_slot(
                                caster_guid,
                                CurrentSpellType::Channeled,
                                world,
                            )
                            .await?;
                        }
                    }
                }
            }
            CurrentSpellType::Melee => {}
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finisher_attributes_need_combo_points() {
        // SPELL_ATTR_EX_FINISHING_MOVE_DAMAGE (e.g. Eviscerate, Sinister Strike finishers)
        assert!(spell_needs_combo_points(0x0010_0000));
        // SPELL_ATTR_EX_FINISHING_MOVE_DURATION (e.g. Kidney Shot, Rupture)
        assert!(spell_needs_combo_points(0x0040_0000));
        // Both bits set
        assert!(spell_needs_combo_points(0x0050_0000));
        // Combined with unrelated attribute bits
        assert!(spell_needs_combo_points(0x0010_0000 | 0x0000_0001));
    }

    #[test]
    fn non_finisher_attributes_do_not_need_combo_points() {
        assert!(!spell_needs_combo_points(0x0000_0000));
        // Neighbouring bits that are NOT the combo-point flags
        assert!(!spell_needs_combo_points(0x0020_0000)); // bit 21
        assert!(!spell_needs_combo_points(0x0080_0000)); // bit 23
        assert!(!spell_needs_combo_points(0x0000_0001 | 0x0008_0000));
    }
}
