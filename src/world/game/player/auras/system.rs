//! AuraSystem - Stateless aura system orchestrator
//!
//! Architecture:
//! - AuraSystem has no mutable state of its own
//! - All aura data lives in player.auras (AuraState)
//! - System accesses player state via world.systems.player.manager().with_player_mut()
//! - Stat changes delegate to StatsSystem
//! - Packets sent via BroadcastManager

use crate::shared::messages::auras::SmsgUpdateAuraDuration;
use crate::shared::messages::update::{ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};
use crate::world::game::broadcast_mgr::BroadcastManager;
use crate::world::game::common::update_fields::*;
use crate::world::game::player::auras::aura::{Aura, AuraFlags};
use crate::world::game::player::auras::effects;
use crate::world::game::player::auras::effects::{ModifierSource, StatModifier};
use crate::world::game::player::auras::periodic;
use crate::world::game::player::auras::proc;
use crate::world::World;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;

/// Stateless aura system - operates on player.auras via PlayerManager.
pub struct AuraSystem {
    broadcast_mgr: Arc<BroadcastManager>,
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
            self.apply_creature_aura(target_guid, spell_id, aura_type, base_value, duration_ms, world);
            return Ok(None);
        }

        // Apply diminishing returns to CC auras (PvP)
        let dr_duration_ms = if target_guid.is_player() && effects::is_cc_aura(aura_type) {
            // Look up the spell's mechanic for DR group determination
            let mechanic = world.managers.spell_mgr.get(spell_id)
                .map(|e| e.mechanic)
                .unwrap_or(0);
            let dr_group = crate::world::game::player::spells::diminishing::get_dr_group_for_spell(mechanic, aura_type);

            if dr_group != crate::world::game::player::spells::diminishing::DRGroup::None {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                let modifier = world.systems.player.manager().with_player_mut(target_guid, |player| {
                    player.combat.diminishing.apply_dr(dr_group, now)
                }).unwrap_or(1.0);

                if modifier <= 0.0 {
                    tracing::debug!(
                        "[DR] Target {:?} immune to DR group {:?} for spell {}",
                        target_guid, dr_group, spell_id,
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

        let aura = Aura::new(
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
            if (spell_entry.aura_interrupt_flags & super::interrupt::AuraInterruptFlags::STANDING_CANCELS.0) != 0 {
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
                self.apply_spell_modifier(target_guid, spell_id, effect_index, aura_type_copy, misc_value, base_value, world)?;
            }
        }

        // Apply CC unit flags (stun, root, silence, etc.)
        if let Some(flag) = effects::cc_aura_unit_flag(aura_type_copy) {
            if assigned_slot.is_some() {
                world.systems.player.manager().with_player_mut(target_guid, |player| {
                    player.unit_flags |= flag;
                });
            }
        }

        // Apply movement speed modifier (snares, slows, speed boosts)
        if matches!(aura_type_copy,
            effects::AURA_MOD_INCREASE_SPEED |
            effects::AURA_MOD_DECREASE_SPEED |
            effects::AURA_MOD_INCREASE_MOUNTED_SPEED)
        {
            if assigned_slot.is_some() {
                self.apply_movement_speed(target_guid, world);
            }
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
            // Remove stat modifier if applicable
            if effects::is_stat_modifier_aura(aura.aura_type) {
                self.remove_aura_stat_modifier(target_guid, spell_id, effect_index, world).await?;
            }

            // Remove spell modifier if applicable
            if effects::is_spell_modifier_aura(aura.aura_type) {
                self.remove_spell_modifier(target_guid, spell_id, world)?;
            }

            // Remove CC unit flags, but only if no other aura of same type remains
            if let Some(flag) = effects::cc_aura_unit_flag(aura.aura_type) {
                let aura_type = aura.aura_type;
                let has_other = world.systems.player.manager().with_player(target_guid, |player| {
                    player.auras.container.get_auras_by_type(aura_type).len() > 0
                }).unwrap_or(false);

                if !has_other {
                    world.systems.player.manager().with_player_mut(target_guid, |player| {
                        player.unit_flags &= !flag;
                    });
                }
            }

            // Recalculate movement speed when a speed modifier aura expires
            if matches!(aura.aura_type,
                effects::AURA_MOD_INCREASE_SPEED |
                effects::AURA_MOD_DECREASE_SPEED |
                effects::AURA_MOD_INCREASE_MOUNTED_SPEED)
            {
                self.apply_movement_speed(target_guid, world);
            }

            // Send slot cleared to client
            self.send_aura_slot_cleared(target_guid, slot, world)?;

            // If removed aura had STANDING_CANCELS flag (food/drink), stand player up
            // unless another food/drink aura is still active
            if let Some(spell_entry) = world.managers.spell_mgr.get(spell_id) {
                if (spell_entry.aura_interrupt_flags & super::interrupt::AuraInterruptFlags::STANDING_CANCELS.0) != 0 {
                    let has_other_sit_aura = world
                        .systems
                        .player
                        .manager()
                        .with_player(target_guid, |player| {
                            player.auras.container.all_auras().any(|a| {
                                if let Some(entry) = world.managers.spell_mgr.get(a.spell_id) {
                                    (entry.aura_interrupt_flags & super::interrupt::AuraInterruptFlags::STANDING_CANCELS.0) != 0
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
                self.remove_aura_stat_modifier(
                    target_guid,
                    aura.spell_id,
                    aura.effect_index,
                    world,
                ).await?;
            }
            if effects::is_spell_modifier_aura(aura.aura_type) {
                self.remove_spell_modifier(target_guid, aura.spell_id, world)?;
            }
            self.send_aura_slot_cleared(target_guid, *slot, world)?;
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
            self.remove_aura(target_guid, spell_id, effect_index, world).await?;
        }

        Ok(())
    }

    /// Send SMSG_STANDSTATE_UPDATE packet to player
    fn send_stand_state_update(&self, player_guid: ObjectGuid, stand_state: u8, world: &World) {
        let mut packet = WorldPacket::new(Opcode::SMSG_STANDSTATE_UPDATE);
        packet.write_u8(stand_state);
        world.managers.broadcast_mgr.send_msg_to_player(player_guid, packet);
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
            self.remove_aura(player_guid, spell_id, effect_index, world)
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
            let stat_modifier = create_stat_modifier(spell_id, aura_type, value, misc_value);
            if let Some(modifier) = stat_modifier {
                self.apply_modifier(target_guid, modifier, world).await?;
            }
        }

        Ok(())
    }

    /// Remove a stat modifier from an aura via the StatsSystem.
    async fn remove_aura_stat_modifier(
        &self,
        target_guid: ObjectGuid,
        spell_id: u32,
        _effect_index: u8,
        world: &World,
    ) -> Result<()> {
        self.remove_modifier(target_guid, ModifierSource::Aura(spell_id), world)
            .await
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
        world.systems.player.manager().with_player_mut(target_guid, |player| {
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
            self.remove_aura(target_guid, spell_id, effect_index, world).await?;
        }

        if total_absorbed > 0 {
            tracing::debug!(
                "[AURA] Absorbed {} damage (school={}) on {:?}, {} remaining",
                total_absorbed, school, target_guid, damage
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
        use crate::world::game::player::spells::state::{SpellModOp, SpellModType};

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
                    misc_value, spell_id, effect_index
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
                player.auras.container.get_aura(spell_id, effect_index)
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
            spell_id, op, mod_type, base_value, family_name, family_flags
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
                use super::effects::{
                    STAT_AGILITY, STAT_INTELLECT, STAT_SPIRIT, STAT_STAMINA, STAT_STRENGTH,
                };
                use crate::world::game::player::stats::modifiers::{UnitModifierType, UnitMods};

                let unit_mod = match modifier.stat {
                    STAT_STRENGTH => UnitMods::StatStrength,
                    STAT_AGILITY => UnitMods::StatAgility,
                    STAT_STAMINA => UnitMods::StatStamina,
                    STAT_INTELLECT => UnitMods::StatIntellect,
                    STAT_SPIRIT => UnitMods::StatSpirit,
                    _ => return,
                };

                if modifier.flat_value != 0.0 {
                    let current = player
                        .stats
                        .unit_mods
                        .get_modifier_value(unit_mod, UnitModifierType::TotalValue);
                    player.stats.unit_mods.set_modifier_value(
                        unit_mod,
                        UnitModifierType::TotalValue,
                        current + modifier.flat_value,
                    );
                }

                if modifier.pct_value != 0.0 {
                    let current = player
                        .stats
                        .unit_mods
                        .get_modifier_value(unit_mod, UnitModifierType::TotalPct);
                    player.stats.unit_mods.set_modifier_value(
                        unit_mod,
                        UnitModifierType::TotalPct,
                        current + modifier.pct_value,
                    );
                }
            });

        // Trigger recalculation
        world.systems.stats.recalculate_all(player_guid);

        Ok(())
    }

    /// Remove a stat modifier from a player.
    async fn remove_modifier(
        &self,
        player_guid: ObjectGuid,
        source: ModifierSource,
        world: &World,
    ) -> Result<()> {
        let _ = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |_player| {
                // TODO: Track which modifiers came from which sources
                // For now, we just recalc everything
                let _ = source;
            });

        // Trigger recalculation
        world.systems.stats.recalculate_all(player_guid);

        Ok(())
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
        proc_spell_id: Option<u32>,
        damage: u32,
        world: &World,
    ) -> Result<()> {
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
                    let (spell_proc_flags, spell_proc_chance, trigger_spell_id) = match spell_entry {
                        Some(entry) => {
                            let trigger_id = entry.effect_trigger_spell[aura.effect_index as usize];
                            (entry.proc_flags, entry.proc_chance, trigger_id)
                        }
                        None => continue,
                    };

                    // If the spell has proc_flags, they must match the event
                    if spell_proc_flags != 0 && (spell_proc_flags & event_proc_flags) == 0 {
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
            world.systems.player.manager().with_player_mut(player_guid, |player| {
                for candidate in &procable_auras {
                    if candidate.charges > 0 {
                        // Decrement charge on the aura
                        if let Some(aura) = player.auras.container.get_aura_mut(candidate.spell_id, candidate.effect_index) {
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
                    let _ = self.remove_aura(player_guid, candidate.spell_id, candidate.effect_index, world).await;
                }
            }
        }

        // Cast triggered spells (must be done after proc processing to avoid re-entrancy)
        // Get the player's current target for offensive triggered spells
        let attack_target = world.systems.player.manager().with_player(player_guid, |p| {
            p.combat.attack_target
        }).flatten();
        for trigger_id in triggered_casts {
            let _ = world.systems.spells.cast_spell(
                player_guid,
                trigger_id,
                attack_target,
                true, // is_triggered = true
                world,
            ).await;
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
    fn send_aura_update(
        &self,
        target_guid: ObjectGuid,
        slot: u8,
        world: &World,
    ) -> Result<()> {
        if slot >= 48 {
            return Ok(()); // Only 48 visible aura slots
        }

        let aura_data: Option<(u32, u8, u8, u8)> = world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player
                    .auras
                    .container
                    .get_aura_at_slot(slot)
                    .map(|aura| {
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
                self.build_aura_levels_field(&player.auras.container, levels_index as u8, player.level)
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
        tracing::info!("[AURA_UPDATE] slot={} spell={} target={:?} packet_len={} bytes={:02X?}",
            slot, spell_id, target_guid, packet.data().len(), packet.data().as_ref());
        self.broadcast_mgr.broadcast_nearby(target_guid, &packet, true);

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
                    .filter_map(|a| {
                        if !a.flags.is_hidden {
                            a.slot
                        } else {
                            None
                        }
                    })
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
    fn build_aura_flags_field(&self, container: &super::container::AuraContainer, group: u8) -> u32 {
        let mut value = 0u32;
        let base_slot = group as u8 * 8;
        for i in 0..8u8 {
            let slot = base_slot + i;
            if slot >= 48 { break; }
            if let Some(aura) = container.get_aura_at_slot(slot) {
                let flags = encode_aura_flags_vanilla(aura) as u32;
                value |= flags << (i as u32 * 4);
            }
        }
        value
    }

    /// Build the UNIT_FIELD_AURALEVELS u32 for a group of 4 slots.
    /// Each slot gets 1 byte in the u32.
    fn build_aura_levels_field(&self, container: &super::container::AuraContainer, group: u8, player_level: u8) -> u32 {
        let mut value = 0u32;
        let base_slot = group as u8 * 4;
        for i in 0..4u8 {
            let slot = base_slot + i;
            if slot >= 48 { break; }
            if container.get_aura_at_slot(slot).is_some() {
                value |= (player_level as u32) << (i as u32 * 8);
            }
        }
        value
    }

    /// Build the UNIT_FIELD_AURAAPPLICATIONS u32 for a group of 4 slots.
    /// Each slot gets 1 byte: stack_count - 1 (0 = 1 stack).
    fn build_aura_applications_field(&self, container: &super::container::AuraContainer, group: u8) -> u32 {
        let mut value = 0u32;
        let base_slot = group as u8 * 4;
        for i in 0..4u8 {
            let slot = base_slot + i;
            if slot >= 48 { break; }
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
    pub async fn on_login(&self, player_guid: ObjectGuid, world: &World) -> Result<()> {
        // TODO: Load saved auras from database (character_auras table)
        // For now, just reapply passive auras from talents/racials

        self.send_all_auras(player_guid, world)?;

        Ok(())
    }

    /// Called on logout - save persistent auras to database.
    pub async fn on_logout(&self, _player_guid: ObjectGuid, _world: &World) -> Result<()> {
        // TODO: Save non-expired auras with remaining durations to database
        Ok(())
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
    /// (positive = faster, negative = slower), applies them to the base run speed (7.0
    /// yards/sec), stores the result, and sends SMSG_FORCE_RUN_SPEED_CHANGE.
    fn apply_movement_speed(&self, target_guid: ObjectGuid, world: &World) {
        const BASE_RUN_SPEED: f32 = 7.0;

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

        let new_speed = (BASE_RUN_SPEED * (1.0 + total_pct as f32 / 100.0)).max(0.1);

        // Persist updated speed on player
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player.movement.run_speed = new_speed;
            });

        // Send SMSG_FORCE_RUN_SPEED_CHANGE to the target player
        let mut packet = WorldPacket::new(Opcode::SMSG_FORCE_RUN_SPEED_CHANGE);
        packet.write_packed_guid_raw(target_guid.raw());
        packet.write_u32(0); // movement counter
        packet.write_f32(new_speed);
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
        self.apply_creature_aura_with_mgr(creature_guid, spell_id, aura_type, base_value, duration_ms, &world.managers.creature_mgr);
    }

    /// Core creature aura logic, taking CreatureManager directly for testability.
    fn apply_creature_aura_with_mgr(
        &self,
        creature_guid: ObjectGuid,
        spell_id: u32,
        aura_type: u32,
        base_value: i32,
        duration_ms: Option<u32>,
        creature_mgr: &crate::world::game::creature::CreatureManager,
    ) {
        let duration = duration_ms.unwrap_or(0);
        creature_mgr.with_creature_mut(creature_guid, |creature| {
            // Only add if not already present for this spell
            if !creature.auras.iter().any(|(id, _, _)| *id == spell_id) {
                creature.auras.push((spell_id, duration, 1));
            }

            // Apply movement speed modifier
            // base_value for MOD_DECREASE_SPEED is negative (e.g. -40 = 40% slow)
            if aura_type == effects::AURA_MOD_DECREASE_SPEED || aura_type == effects::AURA_MOD_INCREASE_SPEED {
                let new_rate = (1.0 + base_value as f32 / 100.0).max(0.1);
                creature.speed_run = new_rate;
                tracing::debug!(
                    "[AURA] Creature {:?} speed_run set to {} (base_value={})",
                    creature_guid, new_rate, base_value
                );
            }
        });

        // Broadcast SMSG_SPLINE_SET_RUN_SPEED to nearby players for creature speed change
        if aura_type == effects::AURA_MOD_DECREASE_SPEED || aura_type == effects::AURA_MOD_INCREASE_SPEED {
            if let Some(new_rate) = creature_mgr.with_creature(creature_guid, |c| c.speed_run) {
                let new_speed = new_rate * 7.0;
                let mut packet = WorldPacket::new(Opcode::SMSG_SPLINE_SET_RUN_SPEED);
                packet.write_packed_guid_raw(creature_guid.raw());
                packet.write_f32(new_speed);
                self.broadcast_mgr.broadcast_nearby(creature_guid, &packet, true);
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
        creature_mgr: &crate::world::game::creature::CreatureManager,
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
        self.broadcast_mgr.broadcast_nearby(creature_guid, &packet, true);
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
    use super::effects::{STAT_AGILITY, STAT_INTELLECT, STAT_SPIRIT, STAT_STAMINA, STAT_STRENGTH};

    match aura_type {
        effects::AURA_MOD_STAT => {
            // misc_value = stat index (0=Str, 1=Agi, 2=Sta, 3=Int, 4=Spi)
            // -1 = all stats
            let stat = if misc_value == -1 {
                // Apply to all stats - caller should call this 5 times
                // For simplicity, we apply to Strength here; full implementation
                // would loop over all stats
                STAT_STRENGTH
            } else {
                misc_value as usize
            };
            Some(StatModifier {
                source: ModifierSource::Aura(spell_id),
                stat,
                flat_value: value as f32,
                pct_value: 0.0,
            })
        }
        effects::AURA_MOD_PERCENT_STAT => {
            let stat = if misc_value == -1 {
                STAT_STRENGTH
            } else {
                misc_value as usize
            };
            Some(StatModifier {
                source: ModifierSource::Aura(spell_id),
                stat,
                flat_value: 0.0,
                pct_value: value as f32 / 100.0,
            })
        }
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
        effects::AURA_MOD_RESISTANCE => {
            // misc_value = school bitmask
            // This is handled by the stats system as a resistance modifier
            Some(StatModifier {
                source: ModifierSource::Aura(spell_id),
                stat: STAT_STAMINA, // Resistance uses a different path
                flat_value: 0.0,
                pct_value: 0.0,
            })
        }
        // Additional aura types would be mapped here
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::{HighGuid, ObjectGuid, Position};
    use crate::world::core::session::SessionManager;
    use crate::world::game::creature::manager::{CreatureManager, CreatureTemplate};
    use crate::world::game::creature::Creature;
    use crate::world::game::player::PlayerManager;
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

    // ── apply_creature_aura ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_creature_slow_reduces_speed_run() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 1);

        // Frostbolt rank 1: AURA_MOD_DECREASE_SPEED, base_value = -40 (40% slow)
        system.apply_creature_aura_with_mgr(guid, 116, effects::AURA_MOD_DECREASE_SPEED, -40, Some(10_000), &creature_mgr);

        let speed = creature_mgr.with_creature(guid, |c| c.speed_run).unwrap();
        // 1.0 + (-40 / 100.0) = 0.60
        assert!((speed - 0.60).abs() < 0.001, "Expected speed_run ~0.60, got {}", speed);
    }

    #[tokio::test]
    async fn test_creature_slow_aura_tracked_in_vec() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 2);

        system.apply_creature_aura_with_mgr(guid, 116, effects::AURA_MOD_DECREASE_SPEED, -40, Some(10_000), &creature_mgr);

        let has_aura = creature_mgr
            .with_creature(guid, |c| c.auras.iter().any(|(id, _, _)| *id == 116))
            .unwrap();
        assert!(has_aura, "Spell 116 should be tracked in creature auras vec");
    }

    #[tokio::test]
    async fn test_creature_slow_not_duplicated_on_reapply() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 3);

        system.apply_creature_aura_with_mgr(guid, 116, effects::AURA_MOD_DECREASE_SPEED, -40, Some(10_000), &creature_mgr);
        system.apply_creature_aura_with_mgr(guid, 116, effects::AURA_MOD_DECREASE_SPEED, -40, Some(10_000), &creature_mgr);

        let count = creature_mgr
            .with_creature(guid, |c| c.auras.iter().filter(|(id, _, _)| *id == 116).count())
            .unwrap();
        assert_eq!(count, 1, "Same spell should not be added twice");
    }

    #[tokio::test]
    async fn test_remove_creature_aura_restores_speed() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 4);

        system.apply_creature_aura_with_mgr(guid, 116, effects::AURA_MOD_DECREASE_SPEED, -40, Some(10_000), &creature_mgr);
        system.remove_creature_aura_with_mgr(guid, 116, &creature_mgr);

        let speed = creature_mgr.with_creature(guid, |c| c.speed_run).unwrap();
        assert!((speed - 1.14286).abs() < 0.001, "Speed should be restored to base (1.14286) after remove, got {}", speed);
    }

    #[tokio::test]
    async fn test_remove_creature_aura_clears_vec() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 5);

        system.apply_creature_aura_with_mgr(guid, 116, effects::AURA_MOD_DECREASE_SPEED, -40, Some(10_000), &creature_mgr);
        system.remove_creature_aura_with_mgr(guid, 116, &creature_mgr);

        let has_aura = creature_mgr
            .with_creature(guid, |c| c.auras.iter().any(|(id, _, _)| *id == 116))
            .unwrap();
        assert!(!has_aura, "Spell 116 should be removed from creature auras vec");
    }

    #[tokio::test]
    async fn test_speed_increase_aura_raises_speed() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 6);

        // Sprint-style buff: +30% speed
        system.apply_creature_aura_with_mgr(guid, 3, effects::AURA_MOD_INCREASE_SPEED, 30, None, &creature_mgr);

        let speed = creature_mgr.with_creature(guid, |c| c.speed_run).unwrap();
        assert!((speed - 1.30).abs() < 0.001, "Expected speed_run ~1.30, got {}", speed);
    }

    #[tokio::test]
    async fn test_non_speed_aura_does_not_change_speed() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 7);

        // AURA_MOD_STAT (29) — should not touch speed
        system.apply_creature_aura_with_mgr(guid, 999, 29, 100, None, &creature_mgr);

        let speed = creature_mgr.with_creature(guid, |c| c.speed_run).unwrap();
        assert!((speed - 1.14286).abs() < 0.001, "Non-speed aura should not modify speed_run, got {}", speed);
    }

    #[tokio::test]
    async fn test_extreme_slow_clamped_to_minimum() {
        let (system, creature_mgr) = make_aura_system();
        let guid = add_test_creature(&creature_mgr, 100, 8);

        // -200% would produce negative speed — should clamp to 0.1
        system.apply_creature_aura_with_mgr(guid, 1, effects::AURA_MOD_DECREASE_SPEED, -200, Some(5_000), &creature_mgr);

        let speed = creature_mgr.with_creature(guid, |c| c.speed_run).unwrap();
        assert!(speed >= 0.1, "Speed should not go below minimum 0.1, got {}", speed);
    }
}
