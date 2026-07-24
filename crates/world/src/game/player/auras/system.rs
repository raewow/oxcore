//! AuraSystem - Stateless aura system orchestrator
//!
//! Architecture:
//! - AuraSystem has no mutable state of its own
//! - All aura data lives in player.auras (AuraState)
//! - System accesses player state via world.systems.player.manager().with_player_mut()
//! - Stat changes delegate to StatsSystem
//! - Packets sent via BroadcastManager

use crate::game::broadcast_mgr::BroadcastManager;
use crate::game::common::update_fields::*;
use crate::game::creature::movement::packet_sender::MovementFlagChange;
use crate::game::creature::movement::MoveType;
use crate::game::player::auras::aura::{Aura, AuraFlags};
use crate::game::player::auras::effects;
use crate::game::player::auras::effects::StatModifier;
use crate::game::player::auras::periodic;
use crate::game::player::auras::proc;
use crate::game::player::movement::MovementControllerSender;
use crate::World;
use oxcore_shared::messages::auras::SmsgUpdateAuraDuration;
use oxcore_shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::{ObjectGuid, Opcode, WorldPacket};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;

/// Stateless aura system - operates on player.auras via PlayerManager.
pub struct AuraSystem {
    broadcast_mgr: Arc<BroadcastManager>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuraRemoveMode {
    Default,
    Expire,
    Dispel,
}

fn removal_trigger_spell(spell_id: u32, mode: AuraRemoveMode) -> Option<u32> {
    match (spell_id, mode) {
        (26180, AuraRemoveMode::Dispel) => Some(26233),
        (24002 | 24003, AuraRemoveMode::Expire) => Some(24004),
        _ => None,
    }
}

impl AuraSystem {
    pub fn new(broadcast_mgr: Arc<BroadcastManager>) -> Self {
        Self { broadcast_mgr }
    }

    // =========================================================================
    // Apply / Remove
    // =========================================================================

    /// Apply an aura to a player.
    ///
    /// This is the main entry point for adding auras. It:
    /// 1. Creates the Aura struct from spell data
    /// 2. Checks stacking rules
    /// 3. Inserts into container (allocates slot)
    /// 4. Applies aura effects (stat modifiers, etc.)
    /// 5. Sends SMSG_AURA_UPDATE to client
    /// 6. Triggers stat recalculation if needed
    #[allow(clippy::too_many_arguments)]
    pub async fn apply_aura(
        &self,
        target_guid: ObjectGuid,
        caster_guid: ObjectGuid,
        spell_id: u32,
        effect_index: u8,
        aura_type: u32,
        misc_value: i32,
        base_value: i32,
        duration_ms: Option<u32>,
        periodic_interval_ms: u32,
        max_stacks: u8,
        max_charges: u8,
        flags: AuraFlags,
        world: &World,
    ) -> Result<Option<u8>> {
        // Creature targets: use simplified aura tracking + speed modifier
        if target_guid.is_creature() {
            self.apply_creature_aura(
                target_guid,
                spell_id,
                aura_type,
                base_value,
                duration_ms,
                world,
            );
            return Ok(None);
        }

        // Apply diminishing returns to CC auras (PvP)
        let dr_duration_ms = if target_guid.is_player() && effects::is_cc_aura(aura_type) {
            // Look up the spell's mechanic for DR group determination
            let mechanic = world
                .managers
                .spell_mgr
                .get(spell_id)
                .map(|e| e.mechanic)
                .unwrap_or(0);
            let dr_group = crate::game::player::spells::diminishing::get_dr_group_for_spell(
                mechanic, aura_type,
            );

            if dr_group != crate::game::player::spells::diminishing::DRGroup::None {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                let modifier = world
                    .systems
                    .player
                    .manager()
                    .with_player_mut(target_guid, |player| {
                        player.combat.diminishing.apply_dr(dr_group, now)
                    })
                    .unwrap_or(1.0);

                if modifier <= 0.0 {
                    tracing::debug!(
                        "[DR] Target {:?} immune to DR group {:?} for spell {}",
                        target_guid,
                        dr_group,
                        spell_id,
                    );
                    return Ok(None); // Target is immune — don't apply aura
                }

                duration_ms.map(|d| (d as f32 * modifier) as u32)
            } else {
                duration_ms
            }
        } else {
            duration_ms
        };

        let periodic_interval_ms = if effects::is_periodic_aura_type(aura_type) {
            if caster_guid.is_player() {
                world
                    .managers
                    .spell_mgr
                    .get(spell_id)
                    .and_then(|spell| {
                        world.systems.player.manager().with_player(caster_guid, |player| {
                            crate::game::player::spells::modifiers::apply_spell_modifiers_to_value(
                                &player.spells.spell_modifiers,
                                crate::game::player::spells::state::SpellModOp::ActivationTime,
                                periodic_interval_ms as i32,
                                spell.spell_family_name,
                                spell.spell_family_flags,
                            ) as u32
                        })
                    })
                    .unwrap_or(periodic_interval_ms)
            } else {
                periodic_interval_ms
            }
        } else {
            0
        };

        let mut aura = Aura::new(
            spell_id,
            caster_guid,
            effect_index,
            aura_type,
            misc_value,
            base_value,
            dr_duration_ms,
            periodic_interval_ms,
            max_stacks,
            max_charges,
            flags,
        );

        // School-absorb auras receive their caster's applicable +healing or
        // +spell-damage bonus once, when the shield is created.
        if aura_type == effects::AURA_SCHOOL_ABSORB {
            if let Some(spell_proto) = world.managers.spell_mgr.get(spell_id) {
                let bonus = crate::game::player::spells::caster::school_absorb_bonus_done(
                    caster_guid,
                    &spell_proto,
                    world,
                );
                let value = aura.current_value().saturating_add(bonus);
                if let Some(base_value) = aura.base_values.get_mut(effect_index as usize) {
                    *base_value = value;
                }
                if let Some(current_value) = aura.current_values.get_mut(effect_index as usize) {
                    *current_value = value;
                }
            }
        }

        // Store aura_type and slot for later use after lock release
        let aura_type_copy = aura_type;

        // Insert into container (handles stacking/refresh internally)
        let assigned_slot = world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                let slot = player.auras.container.add_aura(aura);

                tracing::info!(
                    "[AURA] add_aura result: spell={} aura_type={} assigned_slot={:?} base_value={} duration={:?}ms periodic={}ms",
                    spell_id, aura_type, slot, base_value, duration_ms, periodic_interval_ms,
                );

                player.auras.needs_client_update = true;

                // If this is a stat modifier aura, flag for recalc
                if effects::is_stat_modifier_aura(aura_type_copy) {
                    player.auras.needs_stat_recalc = true;
                }

                slot
            })
            .flatten();

        // Auto-sit if aura requires sitting (food/drink/sleep)
        if let Some(spell_entry) = world.managers.spell_mgr.get(spell_id) {
            if (spell_entry.aura_interrupt_flags
                & super::interrupt::AuraInterruptFlags::STANDING_CANCELS.0)
                != 0
            {
                // Send SMSG_STANDSTATE_UPDATE packet to make player sit
                self.send_stand_state_update(target_guid, 1, world);
            }
        }

        // Apply stat modifier effects outside the player lock
        if effects::is_stat_modifier_aura(aura_type_copy) {
            if let Some(slot) = assigned_slot {
                self.apply_aura_stat_modifier(target_guid, spell_id, effect_index, world)
                    .await?;
            }
        }

        // Apply spell modifier effects (talents: ADD_FLAT_MODIFIER / ADD_PCT_MODIFIER)
        if effects::is_spell_modifier_aura(aura_type_copy) {
            if let Some(_slot) = assigned_slot {
                self.apply_spell_modifier(
                    target_guid,
                    spell_id,
                    effect_index,
                    aura_type_copy,
                    misc_value,
                    base_value,
                    world,
                )?;
            }
        }

        if aura_type_copy == effects::AURA_AURA_SPELL && assigned_slot.is_some() {
            let trigger_spell_id = world
                .managers
                .spell_mgr
                .get(spell_id)
                .map(|entry| entry.effect_trigger_spell[effect_index as usize])
                .unwrap_or(0);
            if trigger_spell_id != 0 && trigger_spell_id != spell_id {
                self.apply_trigger_aura_spell(target_guid, caster_guid, trigger_spell_id, world)
                    .await?;
            }
        }

        if aura_type_copy == effects::AURA_MOD_UNATTACKABLE && assigned_slot.is_some() {
            world
                .systems
                .player
                .manager()
                .with_player_mut(target_guid, |player| {
                    player.combat.in_combat = false;
                    player.unit_flags |= crate::game::common::unit_flags::NON_ATTACKABLE_2;
                });
        }

        // Apply CC unit flags (stun, root, silence, etc.)
        if let Some(flag) = effects::cc_aura_unit_flag(aura_type_copy) {
            if assigned_slot.is_some() {
                world
                    .systems
                    .player
                    .manager()
                    .with_player_mut(target_guid, |player| {
                        player.unit_flags |= flag;
                    });
            }
        }

        // Apply movement speed modifier (snares, slows, speed boosts)
        if matches!(
            aura_type_copy,
            effects::AURA_MOD_INCREASE_SPEED
                | effects::AURA_MOD_DECREASE_SPEED
                | effects::AURA_MOD_INCREASE_MOUNTED_SPEED
        ) {
            if assigned_slot.is_some() {
                self.apply_movement_speed(target_guid, world);
            }
        }

        // Crowd-control / movement / vision / shapeshift special-case effects
        // (mount, water walk, feather fall, hover, shapeshift, transform, force
        // reaction, stealth, invisibility, tracking, scale, disarm weapon mods, ...).
        if assigned_slot.is_some() {
            self.apply_special_effect(
                target_guid,
                spell_id,
                aura_type_copy,
                misc_value,
                base_value,
                world,
            );
        }

        // Send aura update to client
        if let Some(slot) = assigned_slot {
            self.send_aura_update(target_guid, slot, world)?;
            self.send_aura_duration(target_guid, slot, world);
        }

        Ok(assigned_slot)
    }

    /// Remove an aura from a player.
    ///
    /// This:
    /// 1. Removes from container (frees slot)
    /// 2. Unapplies aura effects (stat modifiers, etc.)
    /// 3. Sends SMSG_AURA_UPDATE with empty slot to client
    /// 4. Triggers stat recalculation if needed
    pub async fn remove_aura(
        &self,
        target_guid: ObjectGuid,
        spell_id: u32,
        effect_index: u8,
        world: &World,
    ) -> Result<()> {
        self.remove_aura_with_mode(
            target_guid,
            spell_id,
            effect_index,
            AuraRemoveMode::Default,
            world,
        )
        .await
    }

    pub async fn remove_aura_with_mode(
        &self,
        target_guid: ObjectGuid,
        spell_id: u32,
        effect_index: u8,
        remove_mode: AuraRemoveMode,
        world: &World,
    ) -> Result<()> {
        // Creature targets: simplified removal
        if target_guid.is_creature() {
            self.remove_creature_aura(target_guid, spell_id, world);
            return Ok(());
        }

        let removed_aura: Option<(Aura, u8)> = world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                let removed = player.auras.container.remove_aura(spell_id, effect_index);
                if removed.is_some() {
                    player.auras.needs_client_update = true;
                }
                removed
            })
            .flatten();

        if let Some((aura, slot)) = removed_aura {
            if let Some(trigger_spell_id) = removal_trigger_spell(spell_id, remove_mode) {
                let caster_guid = if aura.caster_guid.is_empty() {
                    target_guid
                } else {
                    aura.caster_guid
                };
                let _ = world
                    .systems
                    .spells
                    .trigger_procced_spell(
                        caster_guid,
                        Some(target_guid),
                        trigger_spell_id,
                        0,
                        world,
                    )
                    .await;
            }
            if aura.aura_type == effects::AURA_AURA_SPELL {
                let trigger_spell_id = world
                    .managers
                    .spell_mgr
                    .get(spell_id)
                    .map(|entry| entry.effect_trigger_spell[aura.effect_index as usize])
                    .unwrap_or(0);
                if trigger_spell_id != 0 && trigger_spell_id != spell_id {
                    Box::pin(self.remove_spell_auras(target_guid, trigger_spell_id, world)).await?;
                }
            }

            // Remove stat modifier if applicable
            if effects::is_stat_modifier_aura(aura.aura_type) {
                self.remove_aura_stat_modifier(target_guid, &aura, world)
                    .await?;
            }

            // Remove spell modifier if applicable
            if effects::is_spell_modifier_aura(aura.aura_type) {
                self.remove_spell_modifier(target_guid, spell_id, world)?;
            }

            // Remove CC unit flags, but only if no other aura of same type remains
            if let Some(flag) = effects::cc_aura_unit_flag(aura.aura_type) {
                let aura_type = aura.aura_type;
                let has_other = world
                    .systems
                    .player
                    .manager()
                    .with_player(target_guid, |player| {
                        player.auras.container.get_auras_by_type(aura_type).len() > 0
                    })
                    .unwrap_or(false);

                if !has_other {
                    world
                        .systems
                        .player
                        .manager()
                        .with_player_mut(target_guid, |player| {
                            player.unit_flags &= !flag;
                        });
                }
            }

            if aura.aura_type == effects::AURA_MOD_UNATTACKABLE {
                let has_other = world
                    .systems
                    .player
                    .manager()
                    .with_player(target_guid, |player| {
                        player
                            .auras
                            .container
                            .has_aura_type(effects::AURA_MOD_UNATTACKABLE)
                    })
                    .unwrap_or(false);
                if !has_other {
                    world
                        .systems
                        .player
                        .manager()
                        .with_player_mut(target_guid, |player| {
                            player.unit_flags &= !crate::game::common::unit_flags::NON_ATTACKABLE_2;
                        });
                }
            }

            // Recalculate movement speed when a speed modifier aura expires
            if matches!(
                aura.aura_type,
                effects::AURA_MOD_INCREASE_SPEED
                    | effects::AURA_MOD_DECREASE_SPEED
                    | effects::AURA_MOD_INCREASE_MOUNTED_SPEED
            ) {
                self.apply_movement_speed(target_guid, world);
            }

            // Crowd-control / movement / vision / shapeshift special-case cleanup
            // (mirrors apply_special_effect on the way down).
            self.remove_special_effect(target_guid, aura.aura_type, aura.misc_value, world);

            // Send slot cleared to client
            self.send_aura_slot_cleared(target_guid, slot, world)?;

            // If removed aura had STANDING_CANCELS flag (food/drink), stand player up
            // unless another food/drink aura is still active
            if let Some(spell_entry) = world.managers.spell_mgr.get(spell_id) {
                if (spell_entry.aura_interrupt_flags
                    & super::interrupt::AuraInterruptFlags::STANDING_CANCELS.0)
                    != 0
                {
                    let has_other_sit_aura = world
                        .systems
                        .player
                        .manager()
                        .with_player(target_guid, |player| {
                            player.auras.container.all_auras().any(|a| {
                                if let Some(entry) = world.managers.spell_mgr.get(a.spell_id) {
                                    (entry.aura_interrupt_flags
                                        & super::interrupt::AuraInterruptFlags::STANDING_CANCELS.0)
                                        != 0
                                } else {
                                    false
                                }
                            })
                        })
                        .unwrap_or(false);

                    if !has_other_sit_aura {
                        self.send_stand_state_update(target_guid, 0, world);
                    }
                }
            }
        }

        Ok(())
    }

    async fn apply_trigger_aura_spell(
        &self,
        target_guid: ObjectGuid,
        caster_guid: ObjectGuid,
        trigger_spell_id: u32,
        world: &World,
    ) -> Result<()> {
        let Some(entry) = world.managers.spell_mgr.get(trigger_spell_id) else {
            tracing::warn!("[AURA] trigger aura spell {} not found", trigger_spell_id);
            return Ok(());
        };

        let duration_ms = if entry.duration_index > 0 {
            world
                .dbc
                .read()
                .get_spell_duration(entry.duration_index)
                .map(|duration| duration.duration as u32)
        } else {
            None
        };
        let is_positive = (entry.attributes_ex & 0x8000_0000) == 0;
        let flags = AuraFlags {
            is_positive,
            is_negative: !is_positive,
            is_passive: false,
            can_be_cancelled: is_positive,
            is_hidden: false,
            is_permanent: duration_ms.is_none(),
        };

        for effect_index in 0..3usize {
            let aura_type = entry.effect_apply_aura_name[effect_index];
            if aura_type == 0 {
                continue;
            }
            Box::pin(self.apply_aura(
                target_guid,
                caster_guid,
                trigger_spell_id,
                effect_index as u8,
                aura_type,
                entry.effect_misc_value[effect_index],
                entry.effect_base_points[effect_index],
                duration_ms,
                entry.effect_amplitude[effect_index],
                entry.stack_amount.max(1) as u8,
                entry.proc_charges as u8,
                flags,
                world,
            ))
            .await?;
        }

        Ok(())
    }

    /// Refresh the remaining duration of an already-active spell's aura(s), without
    /// re-triggering apply effects or touching stack count.
    ///
    /// Matches C++ `Unit::RefreshAura(spellId, duration)`: looks up the existing holder for
    /// `spell_id` and overwrites its current duration, then notifies the client via
    /// SMSG_UPDATE_AURA_DURATION. Does nothing if the spell has no active aura on this unit.
    pub async fn refresh_aura(
        &self,
        target_guid: ObjectGuid,
        spell_id: u32,
        duration_ms: i32,
        world: &World,
    ) -> Result<()> {
        let slot = world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player.auras.container.refresh_aura(spell_id, duration_ms)
            })
            .flatten();

        if let Some(slot) = slot {
            self.send_aura_duration(target_guid, slot, world);
        }

        Ok(())
    }

    /// Remove all auras from a spell (all effect indices).
    pub async fn remove_spell_auras(
        &self,
        target_guid: ObjectGuid,
        spell_id: u32,
        world: &World,
    ) -> Result<()> {
        for effect_index in 0..3u8 {
            self.remove_aura(target_guid, spell_id, effect_index, world)
                .await?;
        }
        Ok(())
    }

    /// Remove all non-passive auras (e.g., on death).
    pub async fn remove_all_auras(&self, target_guid: ObjectGuid, world: &World) -> Result<()> {
        let removed: Vec<(Aura, u8)> = world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                let removed = player.auras.container.remove_all_non_passive();
                if !removed.is_empty() {
                    player.auras.needs_client_update = true;
                    player.auras.needs_stat_recalc = true;
                }
                removed
            })
            .unwrap_or_default();

        // Unapply stat modifiers, spell modifiers, and send slot cleared for each
        for (aura, slot) in &removed {
            if effects::is_stat_modifier_aura(aura.aura_type) {
                self.remove_aura_stat_modifier(target_guid, aura, world)
                    .await?;
            }
            if effects::is_spell_modifier_aura(aura.aura_type) {
                self.remove_spell_modifier(target_guid, aura.spell_id, world)?;
            }
            self.remove_special_effect(target_guid, aura.aura_type, aura.misc_value, world);
            self.send_aura_slot_cleared(target_guid, *slot, world)?;
        }

        // Clear any CC unit flags left over from the removed auras (bulk removal skips
        // the "any other aura of same type" check done in `remove_aura` since all
        // non-passive auras are gone at once).
        let cc_flags: u32 = removed
            .iter()
            .filter_map(|(aura, _)| effects::cc_aura_unit_flag(aura.aura_type))
            .fold(0u32, |acc, f| acc | f);
        if cc_flags != 0 {
            world
                .systems
                .player
                .manager()
                .with_player_mut(target_guid, |player| {
                    player.unit_flags &= !cc_flags;
                });
        }

        // Full stat recalc after bulk removal
        if !removed.is_empty() {
            world.systems.stats.recalculate_all(target_guid);
        }

        Ok(())
    }

    /// Cancel a buff that the player right-clicked.
    /// Only works for auras with can_be_cancelled flag set.
    pub async fn cancel_aura(
        &self,
        player_guid: ObjectGuid,
        spell_id: u32,
        world: &World,
    ) -> Result<()> {
        // Check if aura exists and is cancellable
        let can_cancel = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                for effect_index in 0..3u8 {
                    if let Some(aura) = player.auras.container.get_aura(spell_id, effect_index) {
                        if aura.flags.can_be_cancelled && aura.is_positive() {
                            return true;
                        }
                    }
                }
                false
            })
            .unwrap_or(false);

        if can_cancel {
            self.remove_spell_auras(player_guid, spell_id, world)
                .await?;
        }

        Ok(())
    }

    /// Remove all auras that match the specified interrupt flags
    pub async fn remove_auras_with_interrupt_flag(
        &self,
        target_guid: ObjectGuid,
        interrupt_flags: u32,
        world: &World,
    ) -> Result<()> {
        // Collect auras to remove (spell_id, effect_index)
        let to_remove: Vec<(u32, u8)> = world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player
                    .auras
                    .container
                    .all_auras()
                    .filter_map(|aura| {
                        // Check if this spell has the interrupt flag
                        if let Some(spell_entry) = world.managers.spell_mgr.get(aura.spell_id) {
                            if (spell_entry.aura_interrupt_flags & interrupt_flags) != 0 {
                                return Some((aura.spell_id, aura.effect_index));
                            }
                        }
                        None
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Remove each aura
        for (spell_id, effect_index) in to_remove {
            self.remove_aura(target_guid, spell_id, effect_index, world)
                .await?;
        }

        Ok(())
    }

    /// Send SMSG_STANDSTATE_UPDATE packet to player
    fn send_stand_state_update(&self, player_guid: ObjectGuid, stand_state: u8, world: &World) {
        let mut packet = WorldPacket::new(Opcode::SMSG_STANDSTATE_UPDATE);
        packet.write_u8(stand_state);
        world
            .managers
            .broadcast_mgr
            .send_msg_to_player(player_guid, packet);
    }

    // =========================================================================
    // Update (called every world tick)
    // =========================================================================

    /// Update auras for all online players. Called every world tick (50ms).
    pub async fn update_all_auras(&self, diff: Duration, world: &World) -> Result<()> {
        let guids: Vec<ObjectGuid> = world.managers.player_mgr.collect_online_guids();
        for guid in guids {
            self.update_auras(guid, diff, world).await?;
        }
        Ok(())
    }

    /// Update all auras for a player. Called every world tick (50ms).
    ///
    /// This handles:
    /// 1. Decrement durations
    /// 2. Remove expired auras
    /// 3. Tick periodic effects (DoT/HoT)
    /// 4. Process charge-depleted auras
    pub async fn update_auras(
        &self,
        player_guid: ObjectGuid,
        diff: Duration,
        world: &World,
    ) -> Result<()> {
        let diff_ms = diff.as_millis() as u32;
        if diff_ms == 0 {
            return Ok(());
        }

        // Phase 1: Tick durations and collect expired auras
        let expired_keys: Vec<(u32, u8)>;
        let periodic_ticks: Vec<(u32, u8)>;

        let tick_result = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                let expired = player.auras.container.tick_durations(diff_ms);
                let periodic = player.auras.container.tick_periodic(diff_ms);
                (expired, periodic)
            });

        match tick_result {
            Some((expired, periodic)) => {
                expired_keys = expired;
                periodic_ticks = periodic;
            }
            None => return Ok(()),
        }

        // Phase 2: Process periodic ticks
        for (spell_id, effect_index) in periodic_ticks {
            self.handle_periodic_tick(player_guid, spell_id, effect_index, world)
                .await?;
        }

        // Phase 3: Remove expired auras
        for (spell_id, effect_index) in expired_keys {
            self.remove_aura_with_mode(
                player_guid,
                spell_id,
                effect_index,
                AuraRemoveMode::Expire,
                world,
            )
            .await?;
        }

        Ok(())
    }

    // =========================================================================
    // Periodic Effects
    // =========================================================================

    /// Handle a single periodic tick for an aura.
    async fn handle_periodic_tick(
        &self,
        target_guid: ObjectGuid,
        spell_id: u32,
        effect_index: u8,
        world: &World,
    ) -> Result<()> {
        // Read aura data from player (snapshot pattern - read then release lock)
        let aura_snapshot: Option<AuraTickSnapshot> = world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player
                    .auras
                    .container
                    .get_aura(spell_id, effect_index)
                    .map(|aura| AuraTickSnapshot {
                        spell_id: aura.spell_id,
                        caster_guid: aura.caster_guid,
                        aura_type: aura.aura_type,
                        current_value: aura.current_value(),
                        misc_value: aura.misc_value,
                    })
            })
            .flatten();

        if let Some(snapshot) = aura_snapshot {
            periodic::dispatch_periodic_tick(target_guid, &snapshot, world, &self.broadcast_mgr)
                .await?;
        }

        Ok(())
    }

    // =========================================================================
    // Stat Modifier Integration
    // =========================================================================

    /// Apply or remove the flat health-per-five-seconds modifier from MOD_REGEN.
    async fn modify_flat_health_regen_aura(
        &self,
        player_guid: ObjectGuid,
        aura_type: u32,
        value: i32,
        apply: bool,
        world: &World,
    ) -> Result<bool> {
        Ok(world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                apply_flat_health_regen_aura_modifier(&mut player.power, aura_type, value, apply)
            })
            .unwrap_or(false))
    }

    /// Rebuild the health-regen-percent multiplier after its aura set changes.
    async fn modify_health_regen_percent_aura(
        &self,
        player_guid: ObjectGuid,
        aura_type: u32,
        world: &World,
    ) -> Result<bool> {
        if aura_type != effects::AURA_MOD_HEALTH_REGEN_PERCENT {
            return Ok(false);
        }

        Ok(world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                let auras = player.auras.container.get_auras_by_type(aura_type);
                let values = auras.iter().map(|aura| aura.current_value());
                apply_health_regen_percent_aura_modifier(&mut player.power, aura_type, values)
            })
            .unwrap_or(false))
    }

    /// Apply a stat modifier from an aura to the StatsSystem.
    async fn apply_aura_stat_modifier(
        &self,
        target_guid: ObjectGuid,
        spell_id: u32,
        effect_index: u8,
        world: &World,
    ) -> Result<()> {
        // Read aura data
        let modifier_info: Option<(u32, i32, i32)> = world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player
                    .auras
                    .container
                    .get_aura(spell_id, effect_index)
                    .map(|aura| (aura.aura_type, aura.current_value(), aura.misc_value))
            })
            .flatten();

        if let Some((aura_type, value, misc_value)) = modifier_info {
            if self
                .modify_flat_health_regen_aura(target_guid, aura_type, value, true, world)
                .await?
            {
                return Ok(());
            }

            if self
                .modify_health_regen_percent_aura(target_guid, aura_type, world)
                .await?
            {
                return Ok(());
            }

            if self
                .modify_max_health_aura(target_guid, aura_type, value, true, world)
                .await?
            {
                return Ok(());
            }

            if self
                .modify_max_power_aura(target_guid, aura_type, misc_value, value, true, world)
                .await?
            {
                return Ok(());
            }

            if self
                .modify_resistance_aura(target_guid, aura_type, misc_value, value, true, world)
                .await?
            {
                return Ok(());
            }

            if self
                .modify_primary_stat_aura(target_guid, aura_type, misc_value, value, true, world)
                .await?
            {
                return Ok(());
            }

            if self
                .modify_damage_done_aura(target_guid, aura_type, misc_value, value, true, world)
                .await?
            {
                return Ok(());
            }

            if let Some(modifier) = create_stat_modifier(spell_id, aura_type, value, misc_value) {
                self.apply_modifier(target_guid, modifier, world).await?;
            } else {
                world.systems.stats.recalculate_all(target_guid);
            }
        }

        Ok(())
    }

    /// Remove a stat modifier using the aura data captured before container removal.
    ///
    /// Unsupported stat-classified auras retain the previous recalculation-only behavior.
    async fn remove_aura_stat_modifier(
        &self,
        target_guid: ObjectGuid,
        aura: &Aura,
        world: &World,
    ) -> Result<()> {
        if self
            .modify_flat_health_regen_aura(
                target_guid,
                aura.aura_type,
                aura.current_value(),
                false,
                world,
            )
            .await?
        {
            return Ok(());
        }

        if self
            .modify_health_regen_percent_aura(target_guid, aura.aura_type, world)
            .await?
        {
            return Ok(());
        }

        if self
            .modify_max_health_aura(
                target_guid,
                aura.aura_type,
                aura.current_value(),
                false,
                world,
            )
            .await?
        {
            return Ok(());
        }

        if self
            .modify_max_power_aura(
                target_guid,
                aura.aura_type,
                aura.misc_value,
                aura.current_value(),
                false,
                world,
            )
            .await?
        {
            return Ok(());
        }

        if self
            .modify_resistance_aura(
                target_guid,
                aura.aura_type,
                aura.misc_value,
                aura.current_value(),
                false,
                world,
            )
            .await?
        {
            return Ok(());
        }

        if self
            .modify_primary_stat_aura(
                target_guid,
                aura.aura_type,
                aura.misc_value,
                aura.current_value(),
                false,
                world,
            )
            .await?
        {
            return Ok(());
        }

        if self
            .modify_damage_done_aura(
                target_guid,
                aura.aura_type,
                aura.misc_value,
                aura.current_value(),
                false,
                world,
            )
            .await?
        {
            return Ok(());
        }

        if let Some(modifier) = create_stat_modifier(
            aura.spell_id,
            aura.aura_type,
            aura.current_value(),
            aura.misc_value,
        ) {
            self.remove_modifier(target_guid, modifier, world).await
        } else {
            world.systems.stats.recalculate_all(target_guid);
            Ok(())
        }
    }

    // =========================================================================
    // Absorb Shield Integration (SCHOOL_ABSORB)
    // =========================================================================

    /// Process absorb shields before applying damage to a player.
    ///
    /// Checks all SCHOOL_ABSORB auras on the target that match the damage school.
    /// Reduces damage by the absorb amount, depletes absorb aura values, and removes
    /// fully consumed absorb auras.
    ///
    /// Returns `(remaining_damage, total_absorbed)`.
    pub async fn absorb_damage(
        &self,
        target_guid: ObjectGuid,
        mut damage: u32,
        school: u8,
        world: &World,
    ) -> Result<(u32, u32)> {
        if damage == 0 {
            return Ok((0, 0));
        }

        let school_mask = 1u32 << school;
        let mut total_absorbed = 0u32;
        let mut depleted_auras: Vec<(u32, u8)> = Vec::new();

        // Phase 1: Apply absorbs within the player lock
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                for aura in player.auras.container.all_auras_mut() {
                    if damage == 0 {
                        break;
                    }

                    if aura.aura_type != effects::AURA_SCHOOL_ABSORB {
                        continue;
                    }

                    // Check if absorb matches the damage school (misc_value is school mask)
                    if aura.misc_value > 0 && (aura.misc_value as u32 & school_mask) == 0 {
                        continue;
                    }

                    let absorb_remaining = aura.current_value() as u32;
                    if absorb_remaining == 0 {
                        continue;
                    }

                    let absorbed = damage.min(absorb_remaining);
                    damage -= absorbed;
                    total_absorbed += absorbed;

                    // Reduce the absorb value
                    let new_value = (absorb_remaining - absorbed) as i32;
                    aura.current_values[aura.effect_index as usize] = new_value;

                    if new_value == 0 {
                        depleted_auras.push((aura.spell_id, aura.effect_index));
                    }
                }
            });

        // Phase 2: Remove depleted absorb auras outside the lock
        for (spell_id, effect_index) in depleted_auras {
            self.remove_aura(target_guid, spell_id, effect_index, world)
                .await?;
        }

        if total_absorbed > 0 {
            tracing::debug!(
                "[AURA] Absorbed {} damage (school={}) on {:?}, {} remaining",
                total_absorbed,
                school,
                target_guid,
                damage
            );
        }

        Ok((damage, total_absorbed))
    }

    // =========================================================================
    // Spell Modifier Integration (ADD_FLAT_MODIFIER / ADD_PCT_MODIFIER)
    // =========================================================================

    /// Apply a spell modifier from an aura (talent ADD_FLAT_MODIFIER / ADD_PCT_MODIFIER).
    ///
    /// These create SpellMod entries that modify spell properties (cast time, damage, cost, etc.)
    /// The `misc_value` from the aura is the SpellModOp (which property to modify).
    /// The `base_value` is the modifier amount.
    /// The spell's `spell_family_flags` and `spell_family_name` determine which spells are affected.
    fn apply_spell_modifier(
        &self,
        target_guid: ObjectGuid,
        spell_id: u32,
        effect_index: u8,
        aura_type: u32,
        misc_value: i32,
        base_value: i32,
        world: &World,
    ) -> Result<()> {
        use crate::game::player::spells::state::{SpellModOp, SpellModType};

        let mod_type = if aura_type == effects::AURA_ADD_FLAT_MODIFIER {
            SpellModType::Flat
        } else {
            SpellModType::Pct
        };

        let op = match SpellModOp::from_u32(misc_value as u32) {
            Some(op) => op,
            None => {
                tracing::warn!(
                    "[AURA] Unknown SpellModOp {} from spell {} effect {}",
                    misc_value,
                    spell_id,
                    effect_index
                );
                return Ok(());
            }
        };

        // Look up spell_family_name and spell_family_flags from the source spell
        let (family_name, family_flags) = world
            .managers
            .spell_mgr
            .get(spell_id)
            .map(|s| (s.spell_family_name, s.spell_family_flags))
            .unwrap_or((0, 0));

        // Read assigned slot from aura container
        let aura_slot = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |player| {
                player
                    .auras
                    .container
                    .get_aura(spell_id, effect_index)
                    .and_then(|a| a.slot)
            })
            .flatten();

        super::super::spells::modifiers::add_spell_modifier(
            target_guid,
            op,
            mod_type,
            base_value,
            family_flags,
            family_name,
            spell_id,
            aura_slot,
            world,
        )?;

        tracing::debug!(
            "[AURA] Applied spell modifier: spell={} op={:?} type={:?} value={} family={}:{:#x}",
            spell_id,
            op,
            mod_type,
            base_value,
            family_name,
            family_flags
        );

        Ok(())
    }

    /// Remove spell modifiers from a source spell.
    fn remove_spell_modifier(
        &self,
        target_guid: ObjectGuid,
        spell_id: u32,
        world: &World,
    ) -> Result<()> {
        super::super::spells::modifiers::remove_spell_modifier(target_guid, spell_id, world)?;

        tracing::debug!(
            "[AURA] Removed spell modifiers from source spell={}",
            spell_id
        );

        Ok(())
    }

    /// Apply a stat modifier to a player.
    async fn apply_modifier(
        &self,
        player_guid: ObjectGuid,
        modifier: StatModifier,
        world: &World,
    ) -> Result<()> {
        world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                // Apply to unit_mods based on stat type
                apply_primary_stat_modifier(&mut player.stats.unit_mods, &modifier, true);
            });

        // Trigger recalculation
        world.systems.stats.recalculate_all(player_guid);

        Ok(())
    }

    /// Remove a stat modifier from a player.
    async fn remove_modifier(
        &self,
        player_guid: ObjectGuid,
        modifier: StatModifier,
        world: &World,
    ) -> Result<()> {
        world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                apply_primary_stat_modifier(&mut player.stats.unit_mods, &modifier, false);
            });

        // Trigger recalculation
        world.systems.stats.recalculate_all(player_guid);

        Ok(())
    }

    /// Apply or remove a max-health aura directly from the health modifier group.
    async fn modify_max_health_aura(
        &self,
        player_guid: ObjectGuid,
        aura_type: u32,
        value: i32,
        apply: bool,
        world: &World,
    ) -> Result<bool> {
        let modified = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                apply_max_health_aura_modifier(&mut player.stats.unit_mods, aura_type, value, apply)
            })
            .unwrap_or(false);

        if modified {
            world.systems.stats.recalculate_all(player_guid);
        }

        Ok(modified)
    }

    /// Apply or remove a max-power aura, then grant or drain the matching max delta.
    async fn modify_max_power_aura(
        &self,
        player_guid: ObjectGuid,
        aura_type: u32,
        misc_value: i32,
        value: i32,
        apply: bool,
        world: &World,
    ) -> Result<bool> {
        let power_type = match aura_type {
            effects::AURA_MOD_INCREASE_ENERGY | effects::AURA_MOD_INCREASE_ENERGY_PERCENT => {
                crate::game::player::power::PowerType::from_u8(misc_value as u8)
                    .filter(|_| (0..5).contains(&misc_value))
            }
            _ => return Ok(false),
        };

        let Some(power_type) = power_type else {
            return Ok(true);
        };

        let old_max = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                apply_max_power_aura_modifier(
                    &mut player.stats.unit_mods,
                    aura_type,
                    misc_value,
                    value,
                    apply,
                );
                player.power.get_max(power_type)
            });

        let Some(old_max) = old_max else {
            return Ok(true);
        };

        world.systems.stats.recalculate_all(player_guid);
        world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                let delta = player.power.get_max(power_type) as i64 - old_max as i64;
                player.power.modify_power(power_type, delta as i32);
            });

        Ok(true)
    }

    /// Apply or reverse a school-resistance aura (flat/percent, base/total) across its school
    /// bitmask, then recalculate. Returns `true` if the aura was one of the resistance forms.
    async fn modify_resistance_aura(
        &self,
        player_guid: ObjectGuid,
        aura_type: u32,
        misc_value: i32,
        amount: i32,
        apply: bool,
        world: &World,
    ) -> Result<bool> {
        let handled = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                apply_resistance_aura_modifier(
                    &mut player.stats.unit_mods,
                    aura_type,
                    misc_value,
                    amount,
                    apply,
                )
            })
            .unwrap_or(false);

        if handled {
            world.systems.stats.recalculate_all(player_guid);
        }

        Ok(handled)
    }

    /// Apply or reverse a primary-stat aura (flat/percent, base/total) across the stat(s) it
    /// names, then recalculate. Returns `true` if the aura was one of the primary-stat forms.
    async fn modify_primary_stat_aura(
        &self,
        player_guid: ObjectGuid,
        aura_type: u32,
        misc_value: i32,
        amount: i32,
        apply: bool,
        world: &World,
    ) -> Result<bool> {
        let handled = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                apply_primary_stat_aura_modifier(
                    &mut player.stats.unit_mods,
                    aura_type,
                    misc_value,
                    amount,
                    apply,
                )
            })
            .unwrap_or(false);

        if handled {
            world.systems.stats.recalculate_all(player_guid);
        }

        Ok(handled)
    }

    /// Apply or reverse a physical damage-done percent aura on the weapon-damage modifiers, then
    /// recalculate. Returns `true` if the aura was one of the damage-done percent forms.
    async fn modify_damage_done_aura(
        &self,
        player_guid: ObjectGuid,
        aura_type: u32,
        misc_value: i32,
        amount: i32,
        apply: bool,
        world: &World,
    ) -> Result<bool> {
        let handled = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                apply_damage_done_aura_modifier(
                    &mut player.stats.unit_mods,
                    aura_type,
                    misc_value,
                    amount,
                    apply,
                )
            })
            .unwrap_or(false);

        if handled {
            world.systems.stats.recalculate_all(player_guid);
        }

        Ok(handled)
    }

    // =========================================================================
    // Proc System
    // =========================================================================

    /// Check all auras for proc triggers after a combat event.
    ///
    /// Called by CombatSystem when damage/healing/spell events occur.
    /// `proc_flags` indicates what happened (melee hit, spell cast, damage taken, etc.)
    /// `proc_spell_id` is the spell that caused the event (None for melee).
    pub async fn check_procs(
        &self,
        player_guid: ObjectGuid,
        event_proc_flags: u32,
        proc_ex: u32,
        proc_spell_id: Option<u32>,
        damage: u32,
        world: &World,
    ) -> Result<()> {
        // Context of the spell that caused the event (None/Normal-school for a melee swing).
        let is_melee = proc_spell_id.is_none();
        let (proc_spell_school_mask, proc_spell_family, proc_spell_is_periodic) =
            match proc_spell_id.and_then(|id| world.managers.spell_mgr.get(id)) {
                Some(e) => {
                    // A spell applies a periodic aura when any effect is an aura with a tick interval.
                    let periodic = (0..3)
                        .any(|i| e.effect_apply_aura_name[i] != 0 && e.effect_amplitude[i] > 0);
                    (
                        1u32 << (e.school as u32 & 0x07),
                        e.spell_family_name,
                        periodic,
                    )
                }
                None => (0, 0, false),
            };

        // Collect procable auras (snapshot pattern)
        let procable_auras: Vec<ProcCandidate> = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                let mut candidates = Vec::new();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                for aura in player.auras.container.all_auras() {
                    // Only consider proc-type auras
                    if aura.aura_type != effects::AURA_PROC_TRIGGER_SPELL
                        && aura.aura_type != effects::AURA_PROC_TRIGGER_DAMAGE
                        && aura.aura_type != effects::AURA_DUMMY
                    {
                        continue;
                    }

                    // Check proc_flags from the aura's spell DBC entry match the combat event
                    let spell_entry = world.managers.spell_mgr.get(aura.spell_id);
                    let (spell_proc_flags, spell_proc_chance, trigger_spell_id) = match spell_entry
                    {
                        Some(entry) => {
                            let trigger_id = entry.effect_trigger_spell[aura.effect_index as usize];
                            (entry.proc_flags, entry.proc_chance, trigger_id)
                        }
                        None => continue,
                    };

                    // Full eligibility (MaNGOS IsSpellProcEventCanTriggeredBy): proc-flag match,
                    // cast-end pairing, school/family gates, and hit-outcome requirement.
                    let proc_event = world.managers.spell_mgr.get_proc_event(aura.spell_id);
                    if !proc::is_spell_proc_event_can_triggered_by(
                        proc_event.as_ref(),
                        spell_proc_flags,
                        event_proc_flags,
                        proc_ex,
                        is_melee,
                        proc_spell_school_mask,
                        proc_spell_family,
                        proc_spell_is_periodic,
                    ) {
                        continue;
                    }

                    // Roll proc chance
                    if spell_proc_chance > 0 && spell_proc_chance < 101 {
                        if !proc::roll_proc_chance(spell_proc_chance as f32) {
                            continue;
                        }
                    }

                    // Check internal cooldown
                    let on_cd = player
                        .auras
                        .proc_cooldowns
                        .get(&aura.spell_id)
                        .map(|&cd_end| now < cd_end)
                        .unwrap_or(false);

                    if on_cd {
                        continue;
                    }

                    candidates.push(ProcCandidate {
                        spell_id: aura.spell_id,
                        effect_index: aura.effect_index,
                        aura_type: aura.aura_type,
                        current_value: aura.current_value(),
                        caster_guid: aura.caster_guid,
                        trigger_spell_id,
                        charges: aura.charges,
                    });
                }
                candidates
            })
            .unwrap_or_default();

        // Process each proc candidate, collecting triggered spell casts
        let mut triggered_casts: Vec<u32> = Vec::new();
        for candidate in &procable_auras {
            let result = proc::dispatch_proc(
                player_guid,
                candidate,
                event_proc_flags,
                proc_ex,
                proc_spell_id,
                damage,
                world,
                &self.broadcast_mgr,
            )?;
            if let Some(trigger_id) = result.trigger_spell_id {
                triggered_casts.push(trigger_id);
            }
        }

        // Consume charges for procs that fired
        if !procable_auras.is_empty() {
            world
                .systems
                .player
                .manager()
                .with_player_mut(player_guid, |player| {
                    for candidate in &procable_auras {
                        if candidate.charges > 0 {
                            // Decrement charge on the aura
                            if let Some(aura) = player
                                .auras
                                .container
                                .get_aura_mut(candidate.spell_id, candidate.effect_index)
                            {
                                if aura.charges > 0 {
                                    aura.charges -= 1;
                                }
                            }
                        }
                    }
                });

            // Remove auras with 0 charges remaining
            for candidate in &procable_auras {
                if candidate.charges == 1 {
                    // Was 1, now 0 after decrement — remove it
                    let _ = self
                        .remove_aura(
                            player_guid,
                            candidate.spell_id,
                            candidate.effect_index,
                            world,
                        )
                        .await;
                }
            }
        }

        // Cast triggered spells via TriggerProccedSpell (readiness + cooldown)
        let attack_target = world
            .systems
            .player
            .manager()
            .with_player(player_guid, |p| p.combat.attack_target)
            .flatten();
        for trigger_id in triggered_casts {
            let _ = world
                .systems
                .spells
                .trigger_procced_spell(
                    player_guid,
                    attack_target,
                    trigger_id,
                    0, // no forced cooldown
                    world,
                )
                .await;
        }

        Ok(())
    }

    // =========================================================================
    // Client Communication
    // =========================================================================

    /// Send aura update fields for a specific slot via SMSG_UPDATE_OBJECT.
    ///
    /// In vanilla 1.12.1, auras are communicated through update fields:
    /// - UNIT_FIELD_AURA (48 u32 slots, each = spell_id)
    /// - UNIT_FIELD_AURAFLAGS (6 u32s, 8 nibbles each = 48 slot flags)
    /// - UNIT_FIELD_AURALEVELS (12 u32s, 4 bytes each = 48 slot levels)
    /// - UNIT_FIELD_AURAAPPLICATIONS (12 u32s, 4 bytes each = 48 slot stacks)
    fn send_aura_update(&self, target_guid: ObjectGuid, slot: u8, world: &World) -> Result<()> {
        if slot >= 48 {
            return Ok(()); // Only 48 visible aura slots
        }

        let aura_data: Option<(u32, u8, u8, u8)> = world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player.auras.container.get_aura_at_slot(slot).map(|aura| {
                    let flags = encode_aura_flags_vanilla(aura);
                    (aura.spell_id, flags, player.level, aura.stack_count)
                })
            })
            .flatten();

        let (spell_id, flags, level, stacks) = aura_data.unwrap_or((0, 0, 0, 0));

        let mut block = ValuesUpdateBlock::new(target_guid, ObjectType::Player);

        // UNIT_FIELD_AURA[slot] = spell_id
        block = block.set_field(UNIT_FIELD_AURA + slot as u32, spell_id);

        // UNIT_FIELD_AURAFLAGS: each u32 covers 8 slots (4-bit nibbles)
        // We need to read-modify-write the full u32 for this group of 8 slots
        let flags_index = slot as u32 / 8;
        let flags_shift = (slot as u32 % 8) * 4;
        let flags_field_value = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |player| {
                self.build_aura_flags_field(&player.auras.container, flags_index as u8)
            })
            .unwrap_or(0);
        block = block.set_field(UNIT_FIELD_AURAFLAGS + flags_index, flags_field_value);

        // UNIT_FIELD_AURALEVELS: each u32 covers 4 slots (1 byte each)
        let levels_index = slot as u32 / 4;
        let levels_field_value = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |player| {
                self.build_aura_levels_field(
                    &player.auras.container,
                    levels_index as u8,
                    player.level,
                )
            })
            .unwrap_or(0);
        block = block.set_field(UNIT_FIELD_AURALEVELS + levels_index, levels_field_value);

        // UNIT_FIELD_AURAAPPLICATIONS: each u32 covers 4 slots (1 byte each)
        let apps_index = slot as u32 / 4;
        let apps_field_value = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |player| {
                self.build_aura_applications_field(&player.auras.container, apps_index as u8)
            })
            .unwrap_or(0);
        block = block.set_field(UNIT_FIELD_AURAAPPLICATIONS + apps_index, apps_field_value);

        let update_msg = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(block));
        let packet = update_msg.to_world_packet();
        tracing::info!(
            "[AURA_UPDATE] slot={} spell={} target={:?} packet_len={} bytes={:02X?}",
            slot,
            spell_id,
            target_guid,
            packet.data().len(),
            packet.data().as_ref()
        );
        self.broadcast_mgr
            .broadcast_nearby(target_guid, &packet, true);

        Ok(())
    }

    /// Send a slot-cleared update (aura removed).
    fn send_aura_slot_cleared(
        &self,
        target_guid: ObjectGuid,
        slot: u8,
        world: &World,
    ) -> Result<()> {
        // Just send the update with spell_id=0 (which is what send_aura_update does
        // when the slot is empty after removal)
        self.send_aura_update(target_guid, slot, world)
    }

    /// Send all aura slots on login via update fields.
    pub fn send_all_auras(&self, player_guid: ObjectGuid, world: &World) -> Result<()> {
        let slots: Vec<u8> = world
            .systems
            .player
            .manager()
            .with_player(player_guid, |player| {
                player
                    .auras
                    .container
                    .all_auras()
                    .filter_map(|a| if !a.flags.is_hidden { a.slot } else { None })
                    .filter(|&s| s < 48)
                    .collect()
            })
            .unwrap_or_default();

        for slot in &slots {
            self.send_aura_update(player_guid, *slot, world)?;
        }

        // Send duration packets after all update fields are sent
        for slot in &slots {
            self.send_aura_duration(player_guid, *slot, world);
        }

        Ok(())
    }

    /// Send SMSG_UPDATE_AURA_DURATION to the player for a specific aura slot.
    /// This tells the client how long the buff timer should display.
    /// Only sent to the aura owner (not nearby players).
    fn send_aura_duration(&self, player_guid: ObjectGuid, slot: u8, world: &World) {
        if slot >= 48 {
            return;
        }

        let duration_ms: Option<u32> = world
            .systems
            .player
            .manager()
            .with_player(player_guid, |player| {
                player
                    .auras
                    .container
                    .get_aura_at_slot(slot)
                    .and_then(|aura| aura.duration_ms)
            })
            .flatten();

        if let Some(ms) = duration_ms {
            if ms > 0 {
                self.broadcast_mgr.send_msg_to_player(
                    player_guid,
                    SmsgUpdateAuraDuration {
                        slot,
                        duration_ms: ms,
                    },
                );
            }
        }
    }

    /// Build the UNIT_FIELD_AURAFLAGS u32 for a group of 8 slots.
    /// Each slot gets a 4-bit nibble in the u32.
    fn build_aura_flags_field(
        &self,
        container: &super::container::AuraContainer,
        group: u8,
    ) -> u32 {
        let mut value = 0u32;
        let base_slot = group as u8 * 8;
        for i in 0..8u8 {
            let slot = base_slot + i;
            if slot >= 48 {
                break;
            }
            if let Some(aura) = container.get_aura_at_slot(slot) {
                let flags = encode_aura_flags_vanilla(aura) as u32;
                value |= flags << (i as u32 * 4);
            }
        }
        value
    }

    /// Build the UNIT_FIELD_AURALEVELS u32 for a group of 4 slots.
    /// Each slot gets 1 byte in the u32.
    fn build_aura_levels_field(
        &self,
        container: &super::container::AuraContainer,
        group: u8,
        player_level: u8,
    ) -> u32 {
        let mut value = 0u32;
        let base_slot = group as u8 * 4;
        for i in 0..4u8 {
            let slot = base_slot + i;
            if slot >= 48 {
                break;
            }
            if container.get_aura_at_slot(slot).is_some() {
                value |= (player_level as u32) << (i as u32 * 8);
            }
        }
        value
    }

    /// Build the UNIT_FIELD_AURAAPPLICATIONS u32 for a group of 4 slots.
    /// Each slot gets 1 byte: stack_count - 1 (0 = 1 stack).
    fn build_aura_applications_field(
        &self,
        container: &super::container::AuraContainer,
        group: u8,
    ) -> u32 {
        let mut value = 0u32;
        let base_slot = group as u8 * 4;
        for i in 0..4u8 {
            let slot = base_slot + i;
            if slot >= 48 {
                break;
            }
            if let Some(aura) = container.get_aura_at_slot(slot) {
                // Applications field stores count - 1 (so 0 = 1 application)
                let apps = aura.stack_count.saturating_sub(1);
                value |= (apps as u32) << (i as u32 * 8);
            }
        }
        value
    }

    // =========================================================================
    // Lifecycle Hooks
    // =========================================================================

    /// Called on login - restore saved auras and send to client.
    ///
    /// `offline_secs` is how long the character was logged out (used to debit remaining
    /// duration on auras with `HasRealTimeDuration()`; pass `0` if unknown).
    pub async fn on_login(
        &self,
        player_guid: ObjectGuid,
        offline_secs: u32,
        world: &World,
    ) -> Result<()> {
        super::persistence::load_auras(player_guid, offline_secs, world).await?;

        self.send_all_auras(player_guid, world)?;

        Ok(())
    }

    /// Called on logout - save persistent auras to database.
    pub async fn on_logout(&self, player_guid: ObjectGuid, world: &World) -> Result<()> {
        super::persistence::save_auras(player_guid, world).await
    }

    /// Called on death - remove applicable auras.
    pub async fn on_death(&self, player_guid: ObjectGuid, world: &World) -> Result<()> {
        self.remove_all_auras(player_guid, world).await
    }
}

// =============================================================================
// Internal Types
// =============================================================================

/// Snapshot of aura data for periodic tick processing.
/// Avoids holding locks while executing effects.
#[derive(Debug, Clone)]
pub struct AuraTickSnapshot {
    pub spell_id: u32,
    pub caster_guid: ObjectGuid,
    pub aura_type: u32,
    pub current_value: i32,
    pub misc_value: i32,
}

/// Snapshot of aura data for proc processing.
#[derive(Debug, Clone)]
pub struct ProcCandidate {
    pub spell_id: u32,
    pub effect_index: u8,
    pub aura_type: u32,
    pub current_value: i32,
    pub caster_guid: ObjectGuid,
    /// The spell to cast when AURA_PROC_TRIGGER_SPELL fires (from effect_trigger_spell)
    pub trigger_spell_id: u32,
    /// Charges remaining (0 = unlimited)
    pub charges: u8,
}

/// Encode AuraFlags into the 4-bit nibble for vanilla 1.12.1 UNIT_FIELD_AURAFLAGS.
///
/// Vanilla aura flag nibble bits:
/// - 0x01: EF_FLAG_0 (set for most auras)
/// - 0x02: EF_FLAG_1 (set for negative/harmful auras)
/// - 0x04: EF_FLAG_2 (unused in practice)
/// - 0x08: Cancellable (player can right-click to remove)
///
/// Typical values:
/// - Positive cancellable buff: 0x09 (0x01 | 0x08)
/// - Negative debuff: 0x02
/// - Passive/hidden: 0x00
fn encode_aura_flags_vanilla(aura: &Aura) -> u8 {
    if aura.flags.is_passive || aura.flags.is_hidden {
        return 0;
    }
    if aura.flags.is_negative {
        0x02 // Negative debuff
    } else {
        // Positive buff
        if aura.flags.can_be_cancelled {
            0x09 // EF_FLAG_0 | CANCELABLE
        } else {
            0x01 // EF_FLAG_0 only
        }
    }
}

impl AuraSystem {
    // =========================================================================
    // Movement Speed
    // =========================================================================

    /// Recalculate and broadcast the player's run speed based on active speed auras.
    ///
    /// Sums all AURA_MOD_INCREASE_SPEED / AURA_MOD_DECREASE_SPEED percentage modifiers
    /// (positive = faster, negative = slower) and forces the resulting rate on the
    /// controlling client. The authoritative speed is stored when the client acks.
    fn apply_movement_speed(&self, target_guid: ObjectGuid, world: &World) {
        // Keeps the old floor of 0.1 yards/sec, expressed as a rate off the 7.0 base.
        const MIN_SPEED_RATE: f32 = 0.1 / 7.0;

        // Sum percentage modifiers from all active speed auras
        let total_pct: i32 = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |player| {
                let mut sum = 0i32;
                for aura in player.auras.container.all_auras() {
                    match aura.aura_type {
                        t if t == effects::AURA_MOD_DECREASE_SPEED
                            || t == effects::AURA_MOD_INCREASE_SPEED =>
                        {
                            sum += aura.current_value() as i32;
                        }
                        _ => {}
                    }
                }
                sum
            })
            .unwrap_or(0);

        let new_rate = (1.0 + total_pct as f32 / 100.0).max(MIN_SPEED_RATE);

        MovementControllerSender::add_speed_change_to_controller(
            world,
            target_guid,
            MoveType::Run,
            new_rate,
        );
    }

    // =========================================================================
    // Crowd-Control / Movement / Vision / Shapeshift special-case effects
    //
    // Mirrors the per-aura-type `Aura::Handle*` methods in SpellAuras.cpp for
    // effects that aren't covered by the generic stat-modifier / CC-unit-flag /
    // movement-speed paths above. Only player targets are handled (creatures use
    // the simplified `apply_creature_aura` path in AuraSystem::apply_aura).
    // =========================================================================

    /// Dispatch on aura apply. Mirrors the "AT APPLY" side of each `Aura::Handle*`.
    #[allow(clippy::too_many_arguments)]
    fn apply_special_effect(
        &self,
        target_guid: ObjectGuid,
        spell_id: u32,
        aura_type: u32,
        misc_value: i32,
        base_value: i32,
        world: &World,
    ) {
        if !target_guid.is_player() {
            return;
        }

        match aura_type {
            // --- Aura::HandleAuraMounted ---
            effects::AURA_MOUNTED => {
                let display_id = misc_value.max(0) as u32;
                world
                    .systems
                    .player
                    .manager()
                    .with_player_mut(target_guid, |player| {
                        player.mount_display_id = display_id;
                    });
                self.send_mount_field_update(target_guid, display_id, world);
            }

            // --- Aura::HandleAuraWaterWalk ---
            effects::AURA_WATER_WALK => {
                world
                    .systems
                    .player
                    .manager()
                    .with_player_mut(target_guid, |player| {
                        player.movement.water_walking = true;
                    });
                self.send_water_walk(target_guid, true, world);
            }

            // --- Aura::HandleAuraFeatherFall ---
            effects::AURA_FEATHER_FALL => {
                world
                    .systems
                    .player
                    .manager()
                    .with_player_mut(target_guid, |player| {
                        player.movement.feather_fall = true;
                    });
                self.send_feather_fall(target_guid, true, world);
            }

            // --- Aura::HandleAuraHover ---
            effects::AURA_HOVER => {
                world
                    .systems
                    .player
                    .manager()
                    .with_player_mut(target_guid, |player| {
                        player.movement.hover = true;
                    });
                self.send_hover(target_guid, true, world);
            }

            // --- Aura::HandleAuraModShapeshift ---
            effects::AURA_MOD_SHAPESHIFT => {
                self.apply_shapeshift(target_guid, misc_value as u8, world);
            }

            // --- Aura::HandleAuraTransform ---
            effects::AURA_TRANSFORM => {
                self.apply_transform(target_guid, spell_id, misc_value, world);
            }

            // --- Aura::HandleForceReaction ---
            effects::AURA_FORCE_REACTION => {
                self.apply_force_reaction(target_guid, misc_value, base_value, world);
            }

            // --- Aura::HandleAuraModScale ---
            effects::AURA_MOD_SCALE => {
                let pct = base_value as f32 / 100.0;
                world
                    .systems
                    .player
                    .manager()
                    .with_player_mut(target_guid, |player| {
                        player.scale *= 1.0 + pct;
                    });
            }

            // --- Aura::HandleModStealth ---
            effects::AURA_MOD_STEALTH => {
                self.apply_stealth(target_guid, world);
            }

            // --- Aura::HandleInvisibility ---
            effects::AURA_MOD_INVISIBILITY => {
                self.apply_invisibility(target_guid, misc_value, world);
            }

            // --- Aura::HandleInvisibilityDetect ---
            effects::AURA_MOD_INVISIBILITY_DETECTION => {
                self.apply_invisibility_detect(target_guid, misc_value, world);
            }

            // --- Aura::HandleAuraTrackCreatures ---
            effects::AURA_TRACK_CREATURES => {
                if misc_value >= 1 {
                    let bit = 1u32 << (misc_value - 1);
                    world
                        .systems
                        .player
                        .manager()
                        .with_player_mut(target_guid, |player| {
                            player.track_creatures_mask |= bit;
                        });
                }
            }

            // --- Aura::HandleAuraTrackResources ---
            effects::AURA_TRACK_RESOURCES => {
                if misc_value >= 1 {
                    let bit = 1u32 << (misc_value - 1);
                    world
                        .systems
                        .player
                        .manager()
                        .with_player_mut(target_guid, |player| {
                            player.track_resources_mask |= bit;
                        });
                }
            }

            // --- Aura::HandleAuraTrackStealthed ---
            effects::AURA_TRACK_STEALTHED => {
                self.set_player_bytes2_flag(target_guid, 0x04, true, world);
            }

            effects::AURA_UNTRACKABLE => {
                self.set_visibility_flag(
                    target_guid,
                    effects::UNIT_VIS_FLAGS_UNTRACKABLE,
                    true,
                    world,
                );
            }

            // --- Aura::HandleDetectAmore ---
            effects::AURA_DETECT_AMORE => {
                self.set_player_bytes2_flag(
                    target_guid,
                    effects::PLAYER_FIELD_BYTE2_DETECT_AMORE,
                    true,
                    world,
                );
            }

            // --- Aura::HandleAuraModDisarm ---
            effects::AURA_MOD_DISARM => {
                // Unit flag itself is applied generically via cc_aura_unit_flag (system.rs
                // apply_aura). C++ also resets the swing timer to BASE_ATTACK_TIME and
                // unapplies weapon-dependent mods (_ApplyWeaponDependentAuraMods) — the
                // latter has no equivalent hook in the current combat/stats system, so
                // only the swing-timer reset is mirrored here.
                world
                    .systems
                    .player
                    .manager()
                    .with_player_mut(target_guid, |player| {
                        player.combat.main_hand_timer = player.combat.main_hand_speed;
                    });
            }

            _ => {}
        }
    }

    /// Dispatch on aura remove. Mirrors the "AT REMOVE" side of each `Aura::Handle*`.
    /// `misc_value` is read from the just-removed `Aura` (container already popped it).
    fn remove_special_effect(
        &self,
        target_guid: ObjectGuid,
        aura_type: u32,
        misc_value: i32,
        world: &World,
    ) {
        if !target_guid.is_player() {
            return;
        }

        match aura_type {
            effects::AURA_MOUNTED => {
                world
                    .systems
                    .player
                    .manager()
                    .with_player_mut(target_guid, |player| {
                        player.mount_display_id = 0;
                    });
                self.send_mount_field_update(target_guid, 0, world);
            }

            effects::AURA_WATER_WALK => {
                let has_other =
                    self.player_has_aura_type(target_guid, effects::AURA_WATER_WALK, world);
                if !has_other {
                    world
                        .systems
                        .player
                        .manager()
                        .with_player_mut(target_guid, |player| {
                            player.movement.water_walking = false;
                        });
                    self.send_water_walk(target_guid, false, world);
                }
            }

            effects::AURA_FEATHER_FALL => {
                let has_other =
                    self.player_has_aura_type(target_guid, effects::AURA_FEATHER_FALL, world);
                if !has_other {
                    world
                        .systems
                        .player
                        .manager()
                        .with_player_mut(target_guid, |player| {
                            player.movement.feather_fall = false;
                        });
                    self.send_feather_fall(target_guid, false, world);
                }
            }

            effects::AURA_HOVER => {
                let has_other = self.player_has_aura_type(target_guid, effects::AURA_HOVER, world);
                if !has_other {
                    world
                        .systems
                        .player
                        .manager()
                        .with_player_mut(target_guid, |player| {
                            player.movement.hover = false;
                        });
                    self.send_hover(target_guid, false, world);
                }
            }

            effects::AURA_MOD_SHAPESHIFT => {
                self.remove_shapeshift(target_guid, world);
            }

            effects::AURA_TRANSFORM => {
                self.remove_transform(target_guid, world);
            }

            effects::AURA_FORCE_REACTION => {
                self.remove_force_reaction(target_guid, misc_value, world);
            }

            effects::AURA_MOD_SCALE => {
                // Recompute from remaining SPELL_AURA_MOD_SCALE auras rather than trying to
                // invert a single multiplicative step (matches intent, not literal C++ code
                // which uses an additive ApplyPercentModFloatValue on the raw field).
                let remaining_pct: i32 = world
                    .systems
                    .player
                    .manager()
                    .with_player(target_guid, |player| {
                        player
                            .auras
                            .container
                            .get_auras_by_type(effects::AURA_MOD_SCALE)
                            .iter()
                            .map(|a| a.current_value())
                            .sum()
                    })
                    .unwrap_or(0);
                let scale = 1.0 + remaining_pct as f32 / 100.0;
                world
                    .systems
                    .player
                    .manager()
                    .with_player_mut(target_guid, |player| {
                        player.scale = scale.max(0.01);
                    });
            }

            effects::AURA_MOD_STEALTH => {
                self.remove_stealth(target_guid, world);
            }

            effects::AURA_MOD_INVISIBILITY => {
                self.remove_invisibility(target_guid, world);
            }

            effects::AURA_MOD_INVISIBILITY_DETECTION => {
                self.remove_invisibility_detect(target_guid, world);
            }

            effects::AURA_TRACK_CREATURES => {
                if misc_value >= 1 {
                    let bit = 1u32 << (misc_value - 1);
                    world
                        .systems
                        .player
                        .manager()
                        .with_player_mut(target_guid, |player| {
                            player.track_creatures_mask &= !bit;
                        });
                }
            }

            effects::AURA_TRACK_RESOURCES => {
                if misc_value >= 1 {
                    let bit = 1u32 << (misc_value - 1);
                    world
                        .systems
                        .player
                        .manager()
                        .with_player_mut(target_guid, |player| {
                            player.track_resources_mask &= !bit;
                        });
                }
            }

            effects::AURA_TRACK_STEALTHED => {
                self.set_player_bytes2_flag(target_guid, 0x04, false, world);
            }

            effects::AURA_UNTRACKABLE => {
                if !self.player_has_aura_type(target_guid, effects::AURA_UNTRACKABLE, world) {
                    self.set_visibility_flag(
                        target_guid,
                        effects::UNIT_VIS_FLAGS_UNTRACKABLE,
                        false,
                        world,
                    );
                }
            }

            effects::AURA_DETECT_AMORE => {
                self.set_player_bytes2_flag(
                    target_guid,
                    effects::PLAYER_FIELD_BYTE2_DETECT_AMORE,
                    false,
                    world,
                );
            }

            effects::AURA_MOD_DISARM => {
                // `remove_aura` clears UNIT_FLAG_DISARMED just before calling us if no
                // other SPELL_AURA_MOD_DISARM aura remains, so re-reading it here tells
                // us whether this was the last disarm effect.
                let still_disarmed = (world
                    .systems
                    .player
                    .manager()
                    .with_player(target_guid, |player| player.unit_flags)
                    .unwrap_or(0)
                    & effects::UNIT_FLAG_DISARMED)
                    != 0;
                if !still_disarmed {
                    world
                        .systems
                        .player
                        .manager()
                        .with_player_mut(target_guid, |player| {
                            player.combat.main_hand_timer = player.combat.main_hand_speed;
                        });
                }
            }

            _ => {}
        }
    }

    /// True if the player has at least one active aura of `aura_type` (used to gate
    /// "remove effect only if last aura of this type" like the C++ `HasAuraType` checks).
    fn player_has_aura_type(&self, target_guid: ObjectGuid, aura_type: u32, world: &World) -> bool {
        world
            .systems
            .player
            .manager()
            .with_player(target_guid, |player| {
                !player
                    .auras
                    .container
                    .get_auras_by_type(aura_type)
                    .is_empty()
            })
            .unwrap_or(false)
    }

    // ---- Mount ----

    fn send_mount_field_update(&self, target_guid: ObjectGuid, display_id: u32, world: &World) {
        let mut block = ValuesUpdateBlock::new(target_guid, ObjectType::Player);
        block = block.set_field(
            oxcore_shared::protocol::update_fields::UNIT_FIELD_MOUNTDISPLAYID,
            display_id,
        );
        let packet = SmsgUpdateObject::new()
            .add_block(UpdateBlockData::Values(block))
            .to_world_packet();
        self.broadcast_mgr
            .broadcast_nearby(target_guid, &packet, true);
    }

    // ---- Water walk / feather fall / hover ----

    fn send_water_walk(&self, target_guid: ObjectGuid, enable: bool, world: &World) {
        MovementControllerSender::add_movement_flag_change_to_controller(
            world,
            target_guid,
            MovementFlagChange::WaterWalking,
            enable,
        );
    }

    fn send_feather_fall(&self, target_guid: ObjectGuid, enable: bool, world: &World) {
        MovementControllerSender::add_movement_flag_change_to_controller(
            world,
            target_guid,
            MovementFlagChange::SafeFall,
            enable,
        );
    }

    fn send_hover(&self, target_guid: ObjectGuid, enable: bool, world: &World) {
        MovementControllerSender::add_movement_flag_change_to_controller(
            world,
            target_guid,
            MovementFlagChange::Hover,
            enable,
        );
    }

    // ---- Shapeshift ----

    /// Mirrors `Aura::HandleAuraModShapeshift` apply side: sets the shapeshift form,
    /// swaps power type for forms that use rage/energy, and updates the display id
    /// for forms with a hardcoded model. Does not yet implement `RemoveSpellsCausingAura`
    /// (removing other shapeshift auras) or `CastSpell(9033)` (root/slow cleanse on
    /// entering a travel-type form) — those need cross-aura removal / spell-cast hooks
    /// that aren't wired up from within AuraSystem yet.
    fn apply_shapeshift(&self, target_guid: ObjectGuid, form: u8, world: &World) {
        let is_alliance = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |p| {
                matches!(p.race, 1 | 3 | 4 | 7 | 11) // Human/Dwarf/NightElf/Gnome/Draenei-ish alliance set
            })
            .unwrap_or(true);

        let (display_id, _scale) = effects::shapeshift_display_info(form, is_alliance);

        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player.shapeshift_form = form;
            });

        if let Some(power_type) = effects::shapeshift_power_type(form) {
            world
                .systems
                .player
                .manager()
                .with_player_mut(target_guid, |player| {
                    player.power.set_power_type(power_type);
                });
        }

        // Push form/power-type bytes and UNIT_FIELD_DISPLAYID if changed.
        self.send_shapeshift_field_update(target_guid, display_id, world);
    }

    fn remove_shapeshift(&self, target_guid: ObjectGuid, world: &World) {
        let (class, native_display_id) = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |p| (p.class, 0u32))
            .unwrap_or((0, 0));

        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player.shapeshift_form = 0;
                if class == 11 {
                    // CLASS_DRUID
                    player
                        .power
                        .set_power_type(crate::game::player::power::PowerType::Mana);
                    player.power.current[crate::game::player::power::PowerType::Rage as usize] = 0;
                }
            });

        self.send_shapeshift_field_update(target_guid, native_display_id, world);
    }

    fn send_shapeshift_field_update(
        &self,
        target_guid: ObjectGuid,
        display_id: u32,
        world: &World,
    ) {
        let (race, class, gender, power_type, shapeshift_form, stand_state) = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |p| {
                (
                    p.race,
                    p.class,
                    p.gender,
                    p.power.power_type.as_u8(),
                    p.shapeshift_form,
                    p.stand_state,
                )
            })
            .unwrap_or((0, 0, 0, 0, 0, 0));

        let bytes_0 = u32::from_le_bytes([race, class, gender, power_type]);
        let bytes_1 = u32::from_le_bytes([stand_state, 0, shapeshift_form, 0]);
        let mut block = ValuesUpdateBlock::new(target_guid, ObjectType::Player);
        block = block.set_field(UNIT_FIELD_BYTES_0, bytes_0).set_field(
            oxcore_shared::protocol::update_fields::UNIT_FIELD_BYTES_1,
            bytes_1,
        );
        if display_id != 0 {
            block = block.set_field(
                oxcore_shared::protocol::update_fields::UNIT_FIELD_DISPLAYID,
                display_id,
            );
        }
        let packet = SmsgUpdateObject::new()
            .add_block(UpdateBlockData::Values(block))
            .to_world_packet();
        self.broadcast_mgr
            .broadcast_nearby(target_guid, &packet, true);
    }

    // ---- Transform ----

    /// Mirrors `Aura::HandleAuraTransform`. Only the `misc_value != 0` (explicit
    /// creature-template display id) branch is implemented faithfully; the
    /// `misc_value == 0` special-cased-by-spell-id branch (e.g. Orb of Deception)
    /// is not covered (would need a creature-template display-id lookup for the
    /// generic case and per-race hardcoded ids for the special case).
    fn apply_transform(
        &self,
        target_guid: ObjectGuid,
        spell_id: u32,
        misc_value: i32,
        world: &World,
    ) {
        if misc_value == 0 {
            tracing::debug!(
                "[AURA] HandleAuraTransform: spell {} has no creature template id (misc_value=0); \
                 special-cased transforms (e.g. Orb of Deception) are not implemented",
                spell_id
            );
            return;
        }

        // misc_value is a creature_template entry. C++ uses `Creature::ChooseDisplayId`
        // (randomizes gender-variant models); we don't have that helper here, so we
        // just use the template's primary model id.
        let display_id = world
            .managers
            .creature_mgr
            .get_template(misc_value as u32)
            .map(|t| t.model_id_1)
            .unwrap_or(0);

        if display_id == 0 {
            return;
        }

        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player.transform_spell_id = spell_id;
                player.transform_display_id = display_id;
            });

        let mut block = ValuesUpdateBlock::new(target_guid, ObjectType::Player);
        block = block.set_field(
            oxcore_shared::protocol::update_fields::UNIT_FIELD_DISPLAYID,
            display_id,
        );
        let packet = SmsgUpdateObject::new()
            .add_block(UpdateBlockData::Values(block))
            .to_world_packet();
        self.broadcast_mgr
            .broadcast_nearby(target_guid, &packet, true);
    }

    fn remove_transform(&self, target_guid: ObjectGuid, world: &World) {
        let native_display_id = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |p| {
                crate::game::common::player_constants::get_player_display_id(p.race, p.gender)
            })
            .unwrap_or(0);

        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player.transform_spell_id = 0;
                player.transform_display_id = 0;
            });

        if native_display_id != 0 {
            let mut block = ValuesUpdateBlock::new(target_guid, ObjectType::Player);
            block = block.set_field(
                oxcore_shared::protocol::update_fields::UNIT_FIELD_DISPLAYID,
                native_display_id,
            );
            let packet = SmsgUpdateObject::new()
                .add_block(UpdateBlockData::Values(block))
                .to_world_packet();
            self.broadcast_mgr
                .broadcast_nearby(target_guid, &packet, true);
        }
    }

    // ---- Force Reaction ----

    /// Mirrors `Aura::HandleForceReaction`: overrides the caster's perceived
    /// reputation rank vs `faction_id` and pushes SMSG_SET_FORCED_REACTIONS.
    /// Does not implement `StopAttackFaction` (no combat-vs-faction tracking hook
    /// available from AuraSystem yet).
    fn apply_force_reaction(
        &self,
        target_guid: ObjectGuid,
        misc_value: i32,
        base_value: i32,
        world: &World,
    ) {
        use oxcore_shared::game::reputation::ReputationRank;
        let faction_id = misc_value.max(0) as u32;
        // C++ reads m_modifier.m_amount directly as `ReputationRank(uint32(m_amount))` — it's
        // the 0..7 rank enum value itself, not a reputation point total.
        let Some(rank) = ReputationRank::from_i32(base_value) else {
            tracing::warn!(
                "[AURA] HandleForceReaction: invalid rank value {} for faction {}",
                base_value,
                faction_id
            );
            return;
        };

        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player.reputation.forced_reactions.insert(faction_id, rank);
            });

        let _ = world
            .systems
            .reputation
            .send_forced_reactions(target_guid, world);
    }

    fn remove_force_reaction(&self, target_guid: ObjectGuid, misc_value: i32, world: &World) {
        let faction_id = misc_value.max(0) as u32;
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player.reputation.forced_reactions.remove(&faction_id);
            });

        let _ = world
            .systems
            .reputation
            .send_forced_reactions(target_guid, world);
    }

    // ---- Stealth ----

    /// Mirrors `Aura::HandleModStealth` apply side: sets the stealth visual flags.
    /// Does not implement `RemoveAurasWithInterruptFlags(AURA_INTERRUPT_STEALTH_INVIS_CANCELS)`
    /// or `InterruptSpellsCastedOnMe` (need aura-interrupt-flag cross removal + spell
    /// tracking not available from this call site), nor the real distance/level-based
    /// stealth detection in the visibility system (VisibilitySubsystem only does
    /// distance+phase checks today — see crates/world/src/game/player/visibility/system.rs).
    fn apply_stealth(&self, target_guid: ObjectGuid, world: &World) {
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player.vis_flags_byte |= effects::UNIT_VIS_FLAGS_CREEP;
            });
        self.set_player_bytes2_flag(
            target_guid,
            effects::PLAYER_FIELD_BYTE2_STEALTH,
            true,
            world,
        );
        self.send_vis_flag_update(target_guid, world);
    }

    fn set_visibility_flag(&self, target_guid: ObjectGuid, flag: u8, apply: bool, world: &World) {
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                if apply {
                    player.vis_flags_byte |= flag;
                } else {
                    player.vis_flags_byte &= !flag;
                }
            });
        self.send_vis_flag_update(target_guid, world);
    }

    fn remove_stealth(&self, target_guid: ObjectGuid, world: &World) {
        if self.player_has_aura_type(target_guid, effects::AURA_MOD_STEALTH, world) {
            return; // another stealth aura still active
        }
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player.vis_flags_byte &= !effects::UNIT_VIS_FLAGS_CREEP;
            });
        self.set_player_bytes2_flag(
            target_guid,
            effects::PLAYER_FIELD_BYTE2_STEALTH,
            false,
            world,
        );
        self.send_vis_flag_update(target_guid, world);
    }

    fn send_vis_flag_update(&self, target_guid: ObjectGuid, world: &World) {
        let vis_flags = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |p| p.vis_flags_byte)
            .unwrap_or(0);

        let (stand_state, shapeshift_form) = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |p| (p.stand_state, p.shapeshift_form))
            .unwrap_or((0, 0));

        let bytes_1 = u32::from_le_bytes([stand_state, 0, shapeshift_form, vis_flags]);
        let mut block = ValuesUpdateBlock::new(target_guid, ObjectType::Player);
        block = block.set_field(
            oxcore_shared::protocol::update_fields::UNIT_FIELD_BYTES_1,
            bytes_1,
        );
        let packet = SmsgUpdateObject::new()
            .add_block(UpdateBlockData::Values(block))
            .to_world_packet();
        self.broadcast_mgr
            .broadcast_nearby(target_guid, &packet, true);
    }

    // ---- Invisibility ----

    /// Mirrors `Aura::HandleInvisibility` apply side: sets the invisibility bitmask
    /// and glow flag. Does not implement the visibility-system integration (client
    /// still needs a real "invisible unless detected" occlusion rule; see
    /// VisibilitySubsystem TODO above) — the bitmask is tracked so a future
    /// visibility pass can consult it.
    fn apply_invisibility(&self, target_guid: ObjectGuid, misc_value: i32, world: &World) {
        if (0..32).contains(&misc_value) {
            world
                .systems
                .player
                .manager()
                .with_player_mut(target_guid, |player| {
                    player.invisibility_mask |= 1 << misc_value;
                });
        }
        self.set_player_bytes2_flag(
            target_guid,
            effects::PLAYER_FIELD_BYTE2_INVISIBILITY_GLOW,
            true,
            world,
        );
    }

    fn remove_invisibility(&self, target_guid: ObjectGuid, world: &World) {
        let remaining_misc: Vec<i32> = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |player| {
                player
                    .auras
                    .container
                    .get_auras_by_type(effects::AURA_MOD_INVISIBILITY)
                    .iter()
                    .map(|a| a.misc_value)
                    .collect()
            })
            .unwrap_or_default();

        let mask = effects::recompute_invisibility_mask(remaining_misc.into_iter());
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player.invisibility_mask = mask;
            });

        if mask == 0 {
            self.set_player_bytes2_flag(
                target_guid,
                effects::PLAYER_FIELD_BYTE2_INVISIBILITY_GLOW,
                false,
                world,
            );
        }
    }

    fn apply_invisibility_detect(&self, target_guid: ObjectGuid, misc_value: i32, world: &World) {
        if (0..32).contains(&misc_value) {
            world
                .systems
                .player
                .manager()
                .with_player_mut(target_guid, |player| {
                    player.detect_invisibility_mask |= 1 << misc_value;
                });
        }
    }

    fn remove_invisibility_detect(&self, target_guid: ObjectGuid, world: &World) {
        let remaining_misc: Vec<i32> = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |player| {
                player
                    .auras
                    .container
                    .get_auras_by_type(effects::AURA_MOD_INVISIBILITY_DETECTION)
                    .iter()
                    .map(|a| a.misc_value)
                    .collect()
            })
            .unwrap_or_default();

        let mask = effects::recompute_detect_invisibility_mask(remaining_misc.into_iter());
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player.detect_invisibility_mask = mask;
            });
    }

    // ---- PLAYER_FIELD_BYTES2 helper (stealth/invis-glow/detect-amore/track-stealthed) ----

    fn set_player_bytes2_flag(&self, target_guid: ObjectGuid, bit: u8, set: bool, world: &World) {
        let new_value = world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                if set {
                    player.player_bytes2_flags |= bit;
                } else {
                    player.player_bytes2_flags &= !bit;
                }
                player.player_bytes2_flags
            });

        let Some(byte_val) = new_value else { return };
        let bytes2 = u32::from_le_bytes([0, byte_val, 0, 0]);
        let mut block = ValuesUpdateBlock::new(target_guid, ObjectType::Player);
        block = block.set_field(
            oxcore_shared::protocol::update_fields::PLAYER_FIELD_BYTES2,
            bytes2,
        );
        let packet = SmsgUpdateObject::new()
            .add_block(UpdateBlockData::Values(block))
            .to_world_packet();
        self.broadcast_mgr.send_to_player(target_guid, packet);
    }

    // =========================================================================
    // Creature Aura Helpers
    // =========================================================================

    /// Apply a simplified aura to a creature target.
    ///
    /// Tracks the aura in the creature's aura vec and applies movement speed
    /// modifiers (snares/slows) immediately via SMSG_SPLINE_SET_RUN_SPEED.
    fn apply_creature_aura(
        &self,
        creature_guid: ObjectGuid,
        spell_id: u32,
        aura_type: u32,
        base_value: i32,
        duration_ms: Option<u32>,
        world: &World,
    ) {
        self.apply_creature_aura_with_mgr(
            creature_guid,
            spell_id,
            aura_type,
            base_value,
            duration_ms,
            &world.managers.creature_mgr,
        );
    }

    /// Core creature aura logic, taking CreatureManager directly for testability.
    fn apply_creature_aura_with_mgr(
        &self,
        creature_guid: ObjectGuid,
        spell_id: u32,
        aura_type: u32,
        base_value: i32,
        duration_ms: Option<u32>,
        creature_mgr: &crate::game::creature::CreatureManager,
    ) {
        let duration = duration_ms.unwrap_or(0);
        creature_mgr.with_creature_mut(creature_guid, |creature| {
            // Only add if not already present for this spell
            if !creature.auras.iter().any(|(id, _, _)| *id == spell_id) {
                creature.auras.push((spell_id, duration, 1));
            }

            // Apply movement speed modifier
            // base_value for MOD_DECREASE_SPEED is negative (e.g. -40 = 40% slow)
            if aura_type == effects::AURA_MOD_DECREASE_SPEED
                || aura_type == effects::AURA_MOD_INCREASE_SPEED
            {
                let new_rate = (1.0 + base_value as f32 / 100.0).max(0.1);
                creature.speed_run = new_rate;
                tracing::debug!(
                    "[AURA] Creature {:?} speed_run set to {} (base_value={})",
                    creature_guid,
                    new_rate,
                    base_value
                );
            }
        });

        // Broadcast SMSG_SPLINE_SET_RUN_SPEED to nearby players for creature speed change
        if aura_type == effects::AURA_MOD_DECREASE_SPEED
            || aura_type == effects::AURA_MOD_INCREASE_SPEED
        {
            if let Some(new_rate) = creature_mgr.with_creature(creature_guid, |c| c.speed_run) {
                let new_speed = new_rate * 7.0;
                let mut packet = WorldPacket::new(Opcode::SMSG_SPLINE_SET_RUN_SPEED);
                packet.write_packed_guid_raw(creature_guid.raw());
                packet.write_f32(new_speed);
                self.broadcast_mgr
                    .broadcast_nearby(creature_guid, &packet, true);
            }
        }
    }

    /// Remove a simplified aura from a creature target.
    ///
    /// Removes the spell from the creature's aura vec and restores movement speed
    /// if it was a speed modifier aura.
    fn remove_creature_aura(&self, creature_guid: ObjectGuid, spell_id: u32, world: &World) {
        self.remove_creature_aura_with_mgr(creature_guid, spell_id, &world.managers.creature_mgr);
    }

    /// Core creature aura removal logic, taking CreatureManager directly for testability.
    fn remove_creature_aura_with_mgr(
        &self,
        creature_guid: ObjectGuid,
        spell_id: u32,
        creature_mgr: &crate::game::creature::CreatureManager,
    ) {
        creature_mgr.with_creature_mut(creature_guid, |creature| {
            creature.auras.retain(|(id, _, _)| *id != spell_id);
            // Restore base run speed (VMaNGOS DEFAULT_NPC_RUN_SPEED_RATE)
            // TODO: re-sum remaining speed auras if multiple snares can stack
            creature.speed_run = 1.14286;
        });

        // Broadcast restored speed (1.14286 * 7.0 = ~8.0 yds/sec, VMaNGOS default NPC run)
        let mut packet = WorldPacket::new(Opcode::SMSG_SPLINE_SET_RUN_SPEED);
        packet.write_packed_guid_raw(creature_guid.raw());
        packet.write_f32(1.14286 * 7.0);
        self.broadcast_mgr
            .broadcast_nearby(creature_guid, &packet, true);
    }
}

/// Create a StatModifier from aura data.
/// Returns None if the aura type doesn't map to a stat modifier.
fn create_stat_modifier(
    spell_id: u32,
    aura_type: u32,
    value: i32,
    misc_value: i32,
) -> Option<StatModifier> {
    use super::effects::{ModifierSource, STAT_STRENGTH};

    // Primary-stat auras (AURA_MOD_STAT / AURA_MOD_PERCENT_STAT /
    // AURA_MOD_TOTAL_STAT_PERCENTAGE) are handled per-stat by
    // apply_primary_stat_aura_modifier before this function is reached.
    match aura_type {
        effects::AURA_MOD_ATTACK_POWER => {
            // Map to melee AP - stored as stat modifier with a custom stat index
            // In practice, AP is a derived stat, so we store it as a flat modifier
            // and the stats system handles the rest
            Some(StatModifier {
                source: ModifierSource::Aura(spell_id),
                stat: STAT_STRENGTH, // AP maps through strength for melee
                flat_value: 0.0,     // AP is applied separately in derived stat calc
                pct_value: 0.0,
            })
        }
        // Resistance auras (flat/percent, base/total) are handled per-school by
        // apply_resistance_aura_modifier before this function is reached.
        // Additional aura types would be mapped here
        _ => None,
    }
}

/// Apply or reverse the primary-stat modifier forms currently supported by `create_stat_modifier`.
fn apply_primary_stat_modifier(
    unit_mods: &mut crate::game::player::stats::modifiers::UnitModifierGroup,
    modifier: &StatModifier,
    apply: bool,
) {
    use super::effects::{STAT_AGILITY, STAT_INTELLECT, STAT_SPIRIT, STAT_STAMINA, STAT_STRENGTH};
    use crate::game::player::stats::modifiers::{UnitModifierType, UnitMods};

    let unit_mod = match modifier.stat {
        STAT_STRENGTH => UnitMods::StatStrength,
        STAT_AGILITY => UnitMods::StatAgility,
        STAT_STAMINA => UnitMods::StatStamina,
        STAT_INTELLECT => UnitMods::StatIntellect,
        STAT_SPIRIT => UnitMods::StatSpirit,
        _ => return,
    };

    if modifier.flat_value != 0.0 {
        unit_mods.handle_stat_modifier(
            unit_mod,
            UnitModifierType::TotalValue,
            modifier.flat_value,
            apply,
        );
    }

    if modifier.pct_value != 0.0 {
        unit_mods.handle_stat_modifier(
            unit_mod,
            UnitModifierType::TotalPct,
            modifier.pct_value * 100.0,
            apply,
        );
    }
}

/// Apply or reverse the max-health aura forms without treating health as a primary stat.
fn apply_max_health_aura_modifier(
    unit_mods: &mut crate::game::player::stats::modifiers::UnitModifierGroup,
    aura_type: u32,
    value: i32,
    apply: bool,
) -> bool {
    use crate::game::player::stats::modifiers::{UnitModifierType, UnitMods};

    let modifier_type = match aura_type {
        effects::AURA_MOD_INCREASE_HEALTH => UnitModifierType::TotalValue,
        effects::AURA_MOD_INCREASE_HEALTH_PERCENT => UnitModifierType::TotalPct,
        _ => return false,
    };

    unit_mods.handle_stat_modifier(UnitMods::Health, modifier_type, value as f32, apply)
}

/// Apply or reverse a flat health-regeneration aura.
///
/// `AURA_MOD_REGEN` stores health restored per five seconds, matching
/// `Aura::HandleModRegen` and `Player::RegenerateHealth` in C++.
fn apply_flat_health_regen_aura_modifier(
    power: &mut crate::game::player::power::PowerState,
    aura_type: u32,
    value: i32,
    apply: bool,
) -> bool {
    if aura_type != effects::AURA_MOD_REGEN {
        return false;
    }

    let value = value as f32;
    power.health_regen_per_5 += if apply { value } else { -value };
    true
}

/// Rebuild the multiplier used for spirit-based health regeneration.
///
/// C++ applies each `AURA_MOD_HEALTH_REGEN_PERCENT` aura successively, rather
/// than summing their percentages, so retain that multiplicative behavior.
fn apply_health_regen_percent_aura_modifier(
    power: &mut crate::game::player::power::PowerState,
    aura_type: u32,
    values: impl Iterator<Item = i32>,
) -> bool {
    if aura_type != effects::AURA_MOD_HEALTH_REGEN_PERCENT {
        return false;
    }

    power.health_regen_multiplier = values.fold(1.0, |multiplier, value| {
        multiplier * (100.0 + value as f32) / 100.0
    });
    true
}

/// Apply or reverse a max-power aura for the power type in `misc_value`.
fn apply_max_power_aura_modifier(
    unit_mods: &mut crate::game::player::stats::modifiers::UnitModifierGroup,
    aura_type: u32,
    misc_value: i32,
    value: i32,
    apply: bool,
) -> bool {
    use crate::game::player::stats::modifiers::{UnitModifierType, UnitMods};

    let modifier_type = match aura_type {
        effects::AURA_MOD_INCREASE_ENERGY => UnitModifierType::TotalValue,
        effects::AURA_MOD_INCREASE_ENERGY_PERCENT => UnitModifierType::TotalPct,
        _ => return false,
    };

    if !(0..5).contains(&misc_value) {
        return true;
    }

    unit_mods.handle_stat_modifier(
        UnitMods::from_power(misc_value as u8).unwrap(),
        modifier_type,
        value as f32,
        apply,
    )
}

fn modify_current_power_for_max_delta(
    power: &mut crate::game::player::power::PowerState,
    power_type: crate::game::player::power::PowerType,
    old_max: u32,
) {
    let delta = power.get_max(power_type) as i64 - old_max as i64;
    power.modify_power(power_type, delta as i32);
}

/// Apply or reverse a school-resistance aura across every school in its bitmask.
///
/// Mirrors MaNGOS `Aura::HandleAuraModResistance` / `HandleModResistancePercent` /
/// `HandleModBaseResistance` / `HandleAuraModBaseResistancePercent`: `misc_value` is a spell
/// school bitmask (bit `i` → school `i`, where 0 = physical/armor), and each set school gets a
/// `HandleStatModifier(UNIT_MOD_RESISTANCE_START + i, <type>, amount, apply)` call. The four aura
/// types differ only in which `UnitModifierType` they target (base vs total, flat vs percent).
///
/// Returns `true` if `aura_type` is one of the four resistance forms (and was handled), `false`
/// otherwise. Like the C++ handlers, a zero amount is a no-op. The player-only
/// `ApplyResistanceBuffModsMod` UI hook and the Faerie Fire dispel-immunity side effect on
/// `HandleAuraModResistance` are client-facing / dispel-system concerns and are not modeled here.
fn apply_resistance_aura_modifier(
    unit_mods: &mut crate::game::player::stats::modifiers::UnitModifierGroup,
    aura_type: u32,
    misc_value: i32,
    amount: i32,
    apply: bool,
) -> bool {
    use crate::game::player::stats::modifiers::{UnitModifierType, UnitMods};

    let modifier_type = match aura_type {
        effects::AURA_MOD_RESISTANCE => UnitModifierType::TotalValue,
        effects::AURA_MOD_RESISTANCE_PCT => UnitModifierType::TotalPct,
        effects::AURA_MOD_BASE_RESISTANCE => UnitModifierType::BaseValue,
        effects::AURA_MOD_BASE_RESISTANCE_PCT => UnitModifierType::BasePct,
        _ => return false,
    };

    // Zero-amount resistance auras are a no-op in C++ (`if (!m_modifier.m_amount) return;`) but
    // still "belong" to the resistance path, so report handled to skip the stat fallback.
    if amount == 0 {
        return true;
    }

    let school_mask = misc_value as u32;
    // Schools 0..=6 map to Armor..ResistanceArcane (see UnitMods::from_resistance).
    for school in 0u8..7 {
        if school_mask & (1 << school) != 0 {
            if let Some(unit_mod) = UnitMods::from_resistance(school) {
                unit_mods.handle_stat_modifier(unit_mod, modifier_type, amount as f32, apply);
            }
        }
    }

    true
}

/// Apply or reverse a primary-stat aura across the stat(s) named by `misc_value`.
///
/// Mirrors MaNGOS `Aura::HandleAuraModStat` / `HandleModPercentStat` /
/// `HandleModTotalPercentStat`. `misc_value` is a stat index (0=STR, 1=AGI, 2=STA, 3=INT, 4=SPI);
/// a negative value means "all stats" (C++ accepts -1, and -2 for AURA_MOD_STAT), so each of the
/// five stats gets a `HandleStatModifier(UNIT_MOD_STAT_START + i, <type>, amount, apply)` call. The
/// three aura types differ only in which `UnitModifierType` they target:
/// - `AURA_MOD_STAT`             → `TotalValue` (flat)
/// - `AURA_MOD_PERCENT_STAT`     → `BasePct`   (C++ uses BASE_PCT, not total)
/// - `AURA_MOD_TOTAL_STAT_PERCENTAGE` → `TotalPct`
///
/// Returns `true` if `aura_type` is one of the three primary-stat forms (and was handled). Out-of
/// range `misc_value` (matching the C++ validity guards) is reported handled but applies nothing.
/// The player-only `ApplyStatBuffMod` / `ApplyStatPercentBuffMod` UI hooks and the Stamina-driven
/// current-HP rescale on `HandleModTotalPercentStat` are client-facing / derived-stat concerns and
/// are handled by the stats recalculation, not modeled here.
fn apply_primary_stat_aura_modifier(
    unit_mods: &mut crate::game::player::stats::modifiers::UnitModifierGroup,
    aura_type: u32,
    misc_value: i32,
    amount: i32,
    apply: bool,
) -> bool {
    use crate::game::player::stats::modifiers::{UnitModifierType, UnitMods};

    // Lowest misc_value that still means "all stats" — AURA_MOD_STAT also accepts -2.
    let (modifier_type, min_misc) = match aura_type {
        effects::AURA_MOD_STAT => (UnitModifierType::TotalValue, -2),
        effects::AURA_MOD_PERCENT_STAT => (UnitModifierType::BasePct, -1),
        effects::AURA_MOD_TOTAL_STAT_PERCENTAGE => (UnitModifierType::TotalPct, -1),
        _ => return false,
    };

    // C++ guards: reject misc values below the all-stats sentinel or above the last stat (SPI=4).
    if misc_value < min_misc || misc_value > 4 {
        return true;
    }

    for stat in 0u8..5 {
        // Negative misc_value = all stats; otherwise only the matching stat index.
        if misc_value < 0 || misc_value == stat as i32 {
            if let Some(unit_mod) = UnitMods::from_stat(stat) {
                unit_mods.handle_stat_modifier(unit_mod, modifier_type, amount as f32, apply);
            }
        }
    }

    true
}

/// Apply or reverse a physical damage-done percent aura on the weapon-damage modifiers.
///
/// Mirrors MaNGOS `Aura::HandleModDamagePercentDone` and `HandleModOffhandDamagePercent`:
/// - `AURA_MOD_DAMAGE_PERCENT_DONE` with the physical school bit set (`SPELL_SCHOOL_MASK_NORMAL`,
///   bit 0) applies a `TOTAL_PCT` modifier to main-hand, off-hand and ranged weapon damage.
/// - `AURA_MOD_OFFHAND_DAMAGE_PCT` applies a `TOTAL_PCT` modifier to off-hand damage only.
///
/// Returns `true` if `aura_type` is one of those forms. The magic-school portion of
/// `HandleModDamagePercentDone` is client-display only in C++ (the real magic bonus lives in
/// `SpellDamageBonusDone`), so only the physical/weapon side is modeled here. The
/// `_ApplyWeaponDependentAuraDamageMod` per-equipped-weapon path (spells that restrict to an item
/// class) is not modeled — the percent applies to all weapon slots regardless of the spell's
/// equipped-item requirement.
fn apply_damage_done_aura_modifier(
    unit_mods: &mut crate::game::player::stats::modifiers::UnitModifierGroup,
    aura_type: u32,
    misc_value: i32,
    amount: i32,
    apply: bool,
) -> bool {
    use crate::game::player::stats::modifiers::{UnitModifierType, UnitMods};

    // SPELL_SCHOOL_MASK_NORMAL — the physical damage school bit.
    const SCHOOL_MASK_NORMAL: i32 = 1;

    match aura_type {
        effects::AURA_MOD_DAMAGE_PERCENT_DONE => {
            if misc_value & SCHOOL_MASK_NORMAL != 0 {
                for unit_mod in [
                    UnitMods::DamageMainhand,
                    UnitMods::DamageOffhand,
                    UnitMods::DamageRanged,
                ] {
                    unit_mods.handle_stat_modifier(
                        unit_mod,
                        UnitModifierType::TotalPct,
                        amount as f32,
                        apply,
                    );
                }
            }
            true
        }
        effects::AURA_MOD_OFFHAND_DAMAGE_PCT => {
            unit_mods.handle_stat_modifier(
                UnitMods::DamageOffhand,
                UnitModifierType::TotalPct,
                amount as f32,
                apply,
            );
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::SessionManager;
    use crate::game::creature::manager::{CreatureManager, CreatureTemplate};
    use crate::game::creature::Creature;
    use crate::game::player::stats::modifiers::{UnitModifierGroup, UnitModifierType, UnitMods};
    use crate::game::player::PlayerManager;
    use oxcore_shared::protocol::{HighGuid, ObjectGuid, Position};
    use std::sync::Arc;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn test_creature_guid(entry: u32, low: u32) -> ObjectGuid {
        ObjectGuid::new_with_entry(HighGuid::Unit, entry, low)
    }

    fn minimal_template(entry: u32) -> CreatureTemplate {
        CreatureTemplate {
            entry,
            name: format!("TestCreature{}", entry),
            subname: None,
            min_level: 1,
            max_level: 1,
            faction: 1,
            model_id_1: 1,
            model_id_2: 0,
            model_id_3: 0,
            model_id_4: 0,
            scale: 1.0,
            npc_flags: 0,
            unit_flags: 0,
            static_flags1: 0,
            flags_extra: 0,
            creature_type: 1,
            unit_class: 1,
            health_multiplier: 1.0,
            power_multiplier: 1.0,
            armor_multiplier: 1.0,
            damage_multiplier: 1.0,
            damage_variance: 0.1,
            attack_time: 2000,
            rank: 0,
            gossip_menu_id: 0,
            vendor_id: 0,
            trainer_id: 0,
            trainer_type: 0,
            spells: [0; 4],
        }
    }

    fn add_test_creature(creature_mgr: &CreatureManager, entry: u32, low: u32) -> ObjectGuid {
        let guid = test_creature_guid(entry, low);
        let template = minimal_template(entry);
        let creature = Creature::new(
            guid,
            entry,
            0,
            Position::default(),
            0,
            0,
            &template,
            1,
            None,
        );
        creature_mgr.add_creature(creature);
        guid
    }

    fn make_aura_system() -> (AuraSystem, Arc<CreatureManager>) {
        let session_mgr = Arc::new(SessionManager::new());
        let player_mgr = Arc::new(PlayerManager::new());
        let broadcast_mgr = Arc::new(BroadcastManager::new(session_mgr, player_mgr));

        // connect_lazy builds a pool object without actually connecting — safe for unit tests
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool");
        let creature_mgr = Arc::new(CreatureManager::new(Arc::new(pool)));

        let system = AuraSystem::new(broadcast_mgr);
        (system, creature_mgr)
    }

    // ── apply_primary_stat_aura_modifier (Aura::HandleAuraModStat family) ─────

    #[test]
    fn flat_health_regen_aura_applies_and_reverses_hp5() {
        let mut power = crate::game::player::power::PowerState::default();

        assert!(apply_flat_health_regen_aura_modifier(
            &mut power,
            effects::AURA_MOD_REGEN,
            12,
            true,
        ));
        assert_eq!(power.health_regen_per_5, 12.0);

        assert!(apply_flat_health_regen_aura_modifier(
            &mut power,
            effects::AURA_MOD_REGEN,
            12,
            false,
        ));
        assert_eq!(power.health_regen_per_5, 0.0);
    }

    #[test]
    fn percent_health_regen_aura_applies_and_removes_multiplier() {
        let mut power = crate::game::player::power::PowerState::default();

        assert!(apply_health_regen_percent_aura_modifier(
            &mut power,
            effects::AURA_MOD_HEALTH_REGEN_PERCENT,
            [50, 20].into_iter(),
        ));
        assert_eq!(power.health_regen_multiplier, 1.8);

        assert!(apply_health_regen_percent_aura_modifier(
            &mut power,
            effects::AURA_MOD_HEALTH_REGEN_PERCENT,
            std::iter::empty(),
        ));
        assert_eq!(power.health_regen_multiplier, 1.0);
    }

    #[test]
    fn flat_primary_stat_aura_applies_single_stat_and_reverses() {
        let mut unit_mods = UnitModifierGroup::new();

        assert!(apply_primary_stat_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_STAT,
            effects::STAT_STRENGTH as i32,
            12,
            true,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::StatStrength, UnitModifierType::TotalValue),
            12.0
        );
        // Other stats untouched.
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::StatAgility, UnitModifierType::TotalValue),
            0.0
        );

        assert!(apply_primary_stat_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_STAT,
            effects::STAT_STRENGTH as i32,
            12,
            false,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::StatStrength, UnitModifierType::TotalValue),
            0.0
        );
    }

    #[test]
    fn flat_primary_stat_aura_all_stats_applies_to_every_stat() {
        // misc_value = -1 → all five stats (the old create_stat_modifier only did Strength).
        let mut unit_mods = UnitModifierGroup::new();

        assert!(apply_primary_stat_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_STAT,
            -1,
            8,
            true,
        ));

        for stat in 0u8..5 {
            let unit_mod = UnitMods::from_stat(stat).unwrap();
            assert_eq!(
                unit_mods.get_modifier_value(unit_mod, UnitModifierType::TotalValue),
                8.0,
                "stat {stat} not modified",
            );
        }
    }

    #[test]
    fn percent_primary_stat_aura_targets_base_pct() {
        // C++ HandleModPercentStat uses BASE_PCT (not total) — regression guard.
        let mut unit_mods = UnitModifierGroup::new();

        assert!(apply_primary_stat_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_PERCENT_STAT,
            effects::STAT_AGILITY as i32,
            10,
            true,
        ));
        assert!(
            (unit_mods.get_modifier_value(UnitMods::StatAgility, UnitModifierType::BasePct) - 1.1)
                .abs()
                < f32::EPSILON
        );
        // Total pct must be left alone.
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::StatAgility, UnitModifierType::TotalPct),
            1.0
        );
    }

    #[test]
    fn total_percent_stat_aura_targets_total_pct() {
        let mut unit_mods = UnitModifierGroup::new();

        assert!(apply_primary_stat_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_TOTAL_STAT_PERCENTAGE,
            effects::STAT_STAMINA as i32,
            10,
            true,
        ));
        assert!(
            (unit_mods.get_modifier_value(UnitMods::StatStamina, UnitModifierType::TotalPct) - 1.1)
                .abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn primary_stat_aura_out_of_range_misc_is_handled_noop() {
        let mut unit_mods = UnitModifierGroup::new();
        // misc_value 5 is past SPI (4); C++ logs an error and returns without applying.
        assert!(apply_primary_stat_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_STAT,
            5,
            12,
            true,
        ));
        for stat in 0u8..5 {
            let unit_mod = UnitMods::from_stat(stat).unwrap();
            assert_eq!(
                unit_mods.get_modifier_value(unit_mod, UnitModifierType::TotalValue),
                0.0
            );
        }
    }

    #[test]
    fn non_primary_stat_aura_is_not_handled() {
        let mut unit_mods = UnitModifierGroup::new();
        assert!(!apply_primary_stat_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_RESISTANCE,
            0,
            12,
            true,
        ));
    }

    // ── apply_damage_done_aura_modifier (HandleModDamagePercentDone family) ───

    #[test]
    fn physical_damage_percent_done_applies_to_all_weapon_slots() {
        let mut unit_mods = UnitModifierGroup::new();
        // +10% physical damage done (school mask has the physical bit).
        assert!(apply_damage_done_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_DAMAGE_PERCENT_DONE,
            1,
            10,
            true,
        ));

        // Main-hand and ranged start at a 1.0 multiplier → 1.1 after +10%.
        for unit_mod in [UnitMods::DamageMainhand, UnitMods::DamageRanged] {
            assert!(
                (unit_mods.get_modifier_value(unit_mod, UnitModifierType::TotalPct) - 1.1).abs()
                    < f32::EPSILON,
                "{unit_mod:?} not scaled",
            );
        }
        // Off-hand carries the inherent 0.5 penalty multiplier → 0.5 * 1.1 = 0.55.
        assert!(
            (unit_mods.get_modifier_value(UnitMods::DamageOffhand, UnitModifierType::TotalPct)
                - 0.55)
                .abs()
                < f32::EPSILON
        );

        // Reverse restores the main-hand multiplier to 1.0.
        assert!(apply_damage_done_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_DAMAGE_PERCENT_DONE,
            1,
            10,
            false,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::DamageMainhand, UnitModifierType::TotalPct),
            1.0
        );
    }

    #[test]
    fn magic_only_damage_percent_done_does_not_touch_weapon_damage() {
        let mut unit_mods = UnitModifierGroup::new();
        // Fire-only mask (bit 2), no physical bit → weapon damage untouched but still "handled".
        assert!(apply_damage_done_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_DAMAGE_PERCENT_DONE,
            1 << 2,
            10,
            true,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::DamageMainhand, UnitModifierType::TotalPct),
            1.0
        );
    }

    #[test]
    fn offhand_damage_percent_applies_to_offhand_only() {
        let mut unit_mods = UnitModifierGroup::new();
        // Off-hand penalty: -50% on top of the inherent 0.5 penalty → 0.5 * 0.5 = 0.25.
        assert!(apply_damage_done_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_OFFHAND_DAMAGE_PCT,
            0,
            -50,
            true,
        ));
        assert!(
            (unit_mods.get_modifier_value(UnitMods::DamageOffhand, UnitModifierType::TotalPct)
                - 0.25)
                .abs()
                < f32::EPSILON
        );
        // Main-hand and ranged untouched.
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::DamageMainhand, UnitModifierType::TotalPct),
            1.0
        );
    }

    #[test]
    fn non_damage_done_aura_is_not_handled() {
        let mut unit_mods = UnitModifierGroup::new();
        assert!(!apply_damage_done_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_STAT,
            1,
            10,
            true,
        ));
    }

    #[test]
    fn flat_max_health_aura_applies_and_removes() {
        let mut unit_mods = UnitModifierGroup::new();

        assert!(apply_max_health_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_INCREASE_HEALTH,
            120,
            true,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::Health, UnitModifierType::TotalValue),
            120.0
        );

        assert!(apply_max_health_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_INCREASE_HEALTH,
            120,
            false,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::Health, UnitModifierType::TotalValue),
            0.0
        );
    }

    #[test]
    fn flat_max_power_aura_applies_to_misc_power_and_removes() {
        let mut unit_mods = UnitModifierGroup::new();

        assert!(apply_max_power_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_INCREASE_ENERGY,
            3,
            20,
            true,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::Energy, UnitModifierType::TotalValue),
            20.0
        );

        assert!(apply_max_power_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_INCREASE_ENERGY,
            3,
            20,
            false,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::Energy, UnitModifierType::TotalValue),
            0.0
        );
    }

    #[test]
    fn percent_max_power_aura_applies_to_misc_power_and_removes() {
        let mut unit_mods = UnitModifierGroup::new();

        assert!(apply_max_power_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_INCREASE_ENERGY_PERCENT,
            1,
            10,
            true,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::Rage, UnitModifierType::TotalPct),
            1.1
        );

        assert!(apply_max_power_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_INCREASE_ENERGY_PERCENT,
            1,
            10,
            false,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::Rage, UnitModifierType::TotalPct),
            1.0
        );
    }

    #[test]
    fn max_power_change_adjusts_current_by_max_delta() {
        let mut power = crate::game::player::power::PowerState::default();
        power.max[3] = 120;
        power.current[3] = 50;

        modify_current_power_for_max_delta(
            &mut power,
            crate::game::player::power::PowerType::Energy,
            100,
        );
        assert_eq!(power.current[3], 70);

        power.max[3] = 100;
        modify_current_power_for_max_delta(
            &mut power,
            crate::game::player::power::PowerType::Energy,
            120,
        );
        assert_eq!(power.current[3], 50);
    }

    // ── apply_resistance_aura_modifier (Aura::HandleAuraModResistance family) ──

    #[test]
    fn flat_resistance_aura_applies_per_school_and_reverses() {
        let mut unit_mods = UnitModifierGroup::new();
        // Fire (school 2) resistance, +50 flat.
        let mask = 1 << 2;

        assert!(apply_resistance_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_RESISTANCE,
            mask,
            50,
            true,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::ResistanceFire, UnitModifierType::TotalValue),
            50.0
        );
        // Only the masked school is touched.
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::ResistanceFrost, UnitModifierType::TotalValue),
            0.0
        );

        assert!(apply_resistance_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_RESISTANCE,
            mask,
            50,
            false,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::ResistanceFire, UnitModifierType::TotalValue),
            0.0
        );
    }

    #[test]
    fn flat_resistance_aura_applies_to_every_masked_school() {
        let mut unit_mods = UnitModifierGroup::new();
        // All-school mask (physical..arcane): bits 0..=6.
        let mask = 0x7F;

        assert!(apply_resistance_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_RESISTANCE,
            mask,
            25,
            true,
        ));

        for school in 0u8..7 {
            let unit_mod = UnitMods::from_resistance(school).unwrap();
            assert_eq!(
                unit_mods.get_modifier_value(unit_mod, UnitModifierType::TotalValue),
                25.0,
                "school {school} not modified",
            );
        }
    }

    #[test]
    fn base_resistance_aura_targets_base_value() {
        let mut unit_mods = UnitModifierGroup::new();
        let mask = 1 << 5; // Shadow

        assert!(apply_resistance_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_BASE_RESISTANCE,
            mask,
            30,
            true,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::ResistanceShadow, UnitModifierType::BaseValue),
            30.0
        );
    }

    #[test]
    fn percent_resistance_aura_multiplies_total_pct() {
        let mut unit_mods = UnitModifierGroup::new();
        let mask = 1 << 4; // Frost, +10%

        assert!(apply_resistance_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_RESISTANCE_PCT,
            mask,
            10,
            true,
        ));
        assert!(
            (unit_mods.get_modifier_value(UnitMods::ResistanceFrost, UnitModifierType::TotalPct)
                - 1.1)
                .abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn zero_amount_resistance_aura_is_handled_noop() {
        let mut unit_mods = UnitModifierGroup::new();
        // Handled (returns true, skips stat fallback) but changes nothing.
        assert!(apply_resistance_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_RESISTANCE,
            1 << 2,
            0,
            true,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::ResistanceFire, UnitModifierType::TotalValue),
            0.0
        );
    }

    #[test]
    fn non_resistance_aura_is_not_handled() {
        let mut unit_mods = UnitModifierGroup::new();
        assert!(!apply_resistance_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_STAT,
            1 << 2,
            50,
            true,
        ));
    }

    #[test]
    fn percent_max_health_aura_applies_and_removes() {
        let mut unit_mods = UnitModifierGroup::new();

        assert!(apply_max_health_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_INCREASE_HEALTH_PERCENT,
            10,
            true,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::Health, UnitModifierType::TotalPct),
            1.1
        );

        assert!(apply_max_health_aura_modifier(
            &mut unit_mods,
            effects::AURA_MOD_INCREASE_HEALTH_PERCENT,
            10,
            false,
        ));
        assert_eq!(
            unit_mods.get_modifier_value(UnitMods::Health, UnitModifierType::TotalPct),
            1.0
        );
    }

    // ── apply_creature_aura ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_creature_slow_reduces_speed_run() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 1);

        // Frostbolt rank 1: AURA_MOD_DECREASE_SPEED, base_value = -40 (40% slow)
        system.apply_creature_aura_with_mgr(
            guid,
            116,
            effects::AURA_MOD_DECREASE_SPEED,
            -40,
            Some(10_000),
            &creature_mgr,
        );

        let speed = creature_mgr.with_creature(guid, |c| c.speed_run).unwrap();
        // 1.0 + (-40 / 100.0) = 0.60
        assert!(
            (speed - 0.60).abs() < 0.001,
            "Expected speed_run ~0.60, got {}",
            speed
        );
    }

    #[tokio::test]
    async fn test_creature_slow_aura_tracked_in_vec() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 2);

        system.apply_creature_aura_with_mgr(
            guid,
            116,
            effects::AURA_MOD_DECREASE_SPEED,
            -40,
            Some(10_000),
            &creature_mgr,
        );

        let has_aura = creature_mgr
            .with_creature(guid, |c| c.auras.iter().any(|(id, _, _)| *id == 116))
            .unwrap();
        assert!(
            has_aura,
            "Spell 116 should be tracked in creature auras vec"
        );
    }

    #[tokio::test]
    async fn test_creature_slow_not_duplicated_on_reapply() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 3);

        system.apply_creature_aura_with_mgr(
            guid,
            116,
            effects::AURA_MOD_DECREASE_SPEED,
            -40,
            Some(10_000),
            &creature_mgr,
        );
        system.apply_creature_aura_with_mgr(
            guid,
            116,
            effects::AURA_MOD_DECREASE_SPEED,
            -40,
            Some(10_000),
            &creature_mgr,
        );

        let count = creature_mgr
            .with_creature(guid, |c| {
                c.auras.iter().filter(|(id, _, _)| *id == 116).count()
            })
            .unwrap();
        assert_eq!(count, 1, "Same spell should not be added twice");
    }

    #[tokio::test]
    async fn test_remove_creature_aura_restores_speed() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 4);

        system.apply_creature_aura_with_mgr(
            guid,
            116,
            effects::AURA_MOD_DECREASE_SPEED,
            -40,
            Some(10_000),
            &creature_mgr,
        );
        system.remove_creature_aura_with_mgr(guid, 116, &creature_mgr);

        let speed = creature_mgr.with_creature(guid, |c| c.speed_run).unwrap();
        assert!(
            (speed - 1.14286).abs() < 0.001,
            "Speed should be restored to base (1.14286) after remove, got {}",
            speed
        );
    }

    #[tokio::test]
    async fn test_remove_creature_aura_clears_vec() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 5);

        system.apply_creature_aura_with_mgr(
            guid,
            116,
            effects::AURA_MOD_DECREASE_SPEED,
            -40,
            Some(10_000),
            &creature_mgr,
        );
        system.remove_creature_aura_with_mgr(guid, 116, &creature_mgr);

        let has_aura = creature_mgr
            .with_creature(guid, |c| c.auras.iter().any(|(id, _, _)| *id == 116))
            .unwrap();
        assert!(
            !has_aura,
            "Spell 116 should be removed from creature auras vec"
        );
    }

    #[tokio::test]
    async fn test_speed_increase_aura_raises_speed() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 6);

        // Sprint-style buff: +30% speed
        system.apply_creature_aura_with_mgr(
            guid,
            3,
            effects::AURA_MOD_INCREASE_SPEED,
            30,
            None,
            &creature_mgr,
        );

        let speed = creature_mgr.with_creature(guid, |c| c.speed_run).unwrap();
        assert!(
            (speed - 1.30).abs() < 0.001,
            "Expected speed_run ~1.30, got {}",
            speed
        );
    }

    #[tokio::test]
    async fn test_non_speed_aura_does_not_change_speed() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 7);

        // AURA_MOD_STAT (29) — should not touch speed
        system.apply_creature_aura_with_mgr(guid, 999, 29, 100, None, &creature_mgr);

        let speed = creature_mgr.with_creature(guid, |c| c.speed_run).unwrap();
        assert!(
            (speed - 1.14286).abs() < 0.001,
            "Non-speed aura should not modify speed_run, got {}",
            speed
        );
    }

    #[tokio::test]
    async fn test_extreme_slow_clamped_to_minimum() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 8);

        // -200% would produce negative speed — should clamp to 0.1
        system.apply_creature_aura_with_mgr(
            guid,
            1,
            effects::AURA_MOD_DECREASE_SPEED,
            -200,
            Some(5_000),
            &creature_mgr,
        );

        let speed = creature_mgr.with_creature(guid, |c| c.speed_run).unwrap();
        assert!(
            speed >= 0.1,
            "Speed should not go below minimum 0.1, got {}",
            speed
        );
    }
}
