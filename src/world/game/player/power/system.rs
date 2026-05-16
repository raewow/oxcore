//! Power System - handles regeneration and power consumption
//!
//! Stateless system that operates on PowerState embedded in Player.

use crate::shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::ObjectGuid;
use crate::world::game::broadcast_mgr::BroadcastManagerTrait;
use crate::world::game::common::update_fields::*;
use crate::world::game::player::manager::PlayerManager;
use crate::world::game::player::Player;
use crate::world::World;
use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::regen;
use super::state::{PowerState, PowerType};

/// Get current time in milliseconds
fn get_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Regen tick interval (2 seconds)
const REGEN_TICK_MS: u32 = 2000;

/// 5-second rule duration
const FIVE_SECOND_RULE_MS: u64 = 5000;

/// Stateless power system
pub struct PowerSystem {
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
    regen_accumulator: std::sync::atomic::AtomicU32, // Tracks time since last regen tick
}

impl PowerSystem {
    pub fn new(broadcast_mgr: Arc<dyn BroadcastManagerTrait>) -> Self {
        Self {
            broadcast_mgr,
            regen_accumulator: std::sync::atomic::AtomicU32::new(0),
        }
    }

    /// Called every world tick (50ms default)
    /// Handles regeneration for all online players
    pub fn update(&self, diff: Duration, world: &World) -> Result<()> {
        let diff_ms = diff.as_millis() as u32;
        let accumulated = self
            .regen_accumulator
            .fetch_add(diff_ms, std::sync::atomic::Ordering::Relaxed)
            + diff_ms;

        // Only process regen every 2 seconds
        if accumulated < REGEN_TICK_MS {
            return Ok(());
        }
        self.regen_accumulator.store(
            accumulated - REGEN_TICK_MS,
            std::sync::atomic::Ordering::Relaxed,
        );

        let now = get_time_ms();

        // Process regen for all online players, collecting power changes
        let mut power_updates: Vec<(ObjectGuid, PowerType, u32)> = Vec::new();
        let player_mgr = world.managers.player_mgr.clone();
        player_mgr.for_each_player(|guid, player| {
            let power_type = player.power.power_type;
            let idx = power_type as usize;
            let old_value = player.power.current[idx];
            self.regen_tick(guid, player, now);
            let new_value = player.power.current[idx];
            if new_value != old_value {
                tracing::info!(
                    "[POWER] {:?} regen tick: {:?} {} -> {} (max={})",
                    guid,
                    power_type,
                    old_value,
                    new_value,
                    player.power.max[idx]
                );
                power_updates.push((guid, power_type, new_value));
            }
        });

        // Broadcast power updates outside the player lock
        for (guid, power_type, value) in power_updates {
            self.broadcast_power_value(guid, power_type, value, world);
        }

        Ok(())
    }

    /// Process one regen tick for a player
    fn regen_tick(&self, guid: ObjectGuid, player: &mut Player, now: u64) {
        // Sum flat mana-per-tick from AURA_MOD_POWER_REGEN (85) auras (drinks, Blessing of Wisdom)
        // misc_value 0 = Mana. Value is flat mana restored per 2-second tick.
        let drink_regen: f32 = player
            .auras
            .container
            .all_auras()
            .filter(|a| a.aura_type == 85 && a.misc_value == 0)
            .map(|a| a.current_value() as f32)
            .sum();

        // Sync power max from stats (stats can change from gear/buffs at any time)
        player.power.max[PowerType::Mana as usize] = player.stats.max_mana;

        let power = &mut player.power;
        let stats = &player.stats;

        match power.power_type {
            PowerType::Mana => {
                // Check 5-second rule
                power.spirit_regen_active = now >= power.last_mana_use_time + FIVE_SECOND_RULE_MS;

                // Base mana regen (spirit + MP5 from gear)
                let regen = regen::calculate_mana_regen_per_tick(
                    stats.mana_regen_base,
                    power.mp5_from_gear,
                    power.spirit_regen_active,
                    power.casting_regen_pct,
                );

                // Drink regen is flat mana per tick, added on top (like legacy)
                let total_regen = regen + drink_regen;

                // Apply with accumulator for fractional amounts
                power.regen_accumulator += total_regen;
                let whole = power.regen_accumulator as u32;
                if whole > 0 {
                    power.regen_accumulator -= whole as f32;
                    let idx = PowerType::Mana as usize;
                    power.current[idx] = (power.current[idx] + whole).min(power.max[idx]);
                }
            }

            PowerType::Rage => {
                // Rage decays out of combat
                // Note: We need to check combat state from the combat system
                // For now, we'll need to pass this in or access it differently
                // This is a placeholder - actual implementation depends on CombatSystem integration
                let in_combat = false; // TODO: Get from CombatSystem
                if !in_combat {
                    let idx = PowerType::Rage as usize;
                    power.current[idx] =
                        power.current[idx].saturating_sub(regen::RAGE_DECAY_PER_TICK);
                }
            }

            PowerType::Energy => {
                let idx = PowerType::Energy as usize;
                power.current[idx] =
                    (power.current[idx] + regen::ENERGY_REGEN_PER_TICK).min(power.max[idx]);
            }

            PowerType::Focus => {
                let idx = PowerType::Focus as usize;
                power.current[idx] =
                    (power.current[idx] + regen::FOCUS_REGEN_PER_TICK).min(power.max[idx]);
            }

            PowerType::Happiness => {
                // Pet happiness - handled by pet system
            }
        }

        // Health regen (out of combat: spirit-based, in combat: 0 for non-druids)
        // TODO: Health regen formulas when combat system integration is available
    }

    /// Consume power for a spell cast
    /// Returns false if not enough power
    pub fn consume_power(
        &self,
        player_guid: ObjectGuid,
        power_type: PowerType,
        amount: u32,
        world: &World,
    ) -> Result<bool> {
        let player_mgr = world.managers.player_mgr.clone();
        let mut success = false;

        player_mgr.with_player_mut(player_guid, |player| {
            let idx = power_type as usize;
            if player.power.current[idx] >= amount {
                player.power.current[idx] -= amount;
                success = true;

                // Reset 5-second rule timer for mana
                if power_type == PowerType::Mana {
                    player.power.last_mana_use_time = get_time_ms();
                    player.power.spirit_regen_active = false;
                }
            }
        });

        if success {
            self.send_power_update(player_guid, power_type, world)?;
        }

        Ok(success)
    }

    /// Restore power (from potions, spells, etc.)
    pub fn restore_power(
        &self,
        player_guid: ObjectGuid,
        power_type: PowerType,
        amount: u32,
        world: &World,
    ) -> Result<()> {
        let player_mgr = world.managers.player_mgr.clone();

        player_mgr.with_player_mut(player_guid, |player| {
            let idx = power_type as usize;
            player.power.current[idx] =
                (player.power.current[idx] + amount).min(player.power.max[idx]);
        });

        self.send_power_update(player_guid, power_type, world)
    }

    /// Called when player deals damage (rage generation)
    pub fn on_damage_dealt(
        &self,
        player_guid: ObjectGuid,
        damage: u32,
        world: &World,
    ) -> Result<()> {
        let player_mgr = world.managers.player_mgr.clone();

        player_mgr.with_player_mut(player_guid, |player| {
            if player.power.power_type == PowerType::Rage {
                let rage = regen::rage_from_damage_dealt(damage, player.level);
                let idx = PowerType::Rage as usize;
                player.power.current[idx] = (player.power.current[idx] + rage).min(regen::MAX_RAGE);
            }
        });
        Ok(())
    }

    /// Called when player takes damage (rage generation)
    pub fn on_damage_taken(
        &self,
        player_guid: ObjectGuid,
        damage: u32,
        world: &World,
    ) -> Result<()> {
        let player_mgr = world.managers.player_mgr.clone();

        player_mgr.with_player_mut(player_guid, |player| {
            if player.power.power_type == PowerType::Rage {
                let rage = regen::rage_from_damage_taken(damage, player.level);
                let idx = PowerType::Rage as usize;
                player.power.current[idx] = (player.power.current[idx] + rage).min(regen::MAX_RAGE);
            }
        });
        Ok(())
    }

    /// Called on login
    pub fn on_login(&self, player_guid: ObjectGuid, world: &World) -> Result<()> {
        // Initialize power type and max values
        let player_mgr = world.managers.player_mgr.clone();

        player_mgr.with_player_mut(player_guid, |player| {
            player.power.power_type = PowerType::for_class(player.class);

            // Set max values
            let mana_idx = PowerType::Mana as usize;
            player.power.max[mana_idx] = player.stats.max_mana;

            let rage_idx = PowerType::Rage as usize;
            player.power.max[rage_idx] = regen::MAX_RAGE;

            let energy_idx = PowerType::Energy as usize;
            player.power.max[energy_idx] = regen::MAX_ENERGY;

            // Load current values from database
            // TODO: Load saved power values from DB

            // Start with full mana/energy, 0 rage
            if player.power.power_type == PowerType::Mana {
                player.power.current[mana_idx] = player.power.max[mana_idx];
            } else if player.power.power_type == PowerType::Energy {
                player.power.current[energy_idx] = player.power.max[energy_idx];
            }

            player.power.spirit_regen_active = true;
        });

        Ok(())
    }

    /// Send power update to client via SMSG_UPDATE_OBJECT
    fn send_power_update(
        &self,
        player_guid: ObjectGuid,
        power_type: PowerType,
        world: &World,
    ) -> Result<()> {
        let value = world
            .managers
            .player_mgr
            .with_player(player_guid, |player| {
                player.power.current[power_type as usize]
            });

        if let Some(value) = value {
            self.broadcast_power_value(player_guid, power_type, value, world);
        }

        Ok(())
    }

    /// Broadcast a power value update via SMSG_UPDATE_OBJECT
    fn broadcast_power_value(
        &self,
        player_guid: ObjectGuid,
        power_type: PowerType,
        value: u32,
        _world: &World,
    ) {
        let field_offset = UNIT_FIELD_POWER1 + power_type as u32;
        let block =
            ValuesUpdateBlock::new(player_guid, ObjectType::Player).set_field(field_offset, value);
        let update_msg = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(block));
        let packet = update_msg.to_world_packet();
        self.broadcast_mgr
            .broadcast_nearby(player_guid, &packet, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_type_for_class() {
        assert_eq!(PowerType::for_class(1), PowerType::Rage); // Warrior
        assert_eq!(PowerType::for_class(4), PowerType::Energy); // Rogue
        assert_eq!(PowerType::for_class(8), PowerType::Mana); // Mage
        assert_eq!(PowerType::for_class(9), PowerType::Mana); // Warlock
    }

    #[test]
    fn test_power_state_consume_restore() {
        let mut state = PowerState::default();
        state.max[0] = 100;
        state.current[0] = 50;

        // Consume
        assert!(state.consume(PowerType::Mana, 30));
        assert_eq!(state.current[0], 20);

        // Not enough
        assert!(!state.consume(PowerType::Mana, 30));
        assert_eq!(state.current[0], 20);

        // Restore
        state.restore(PowerType::Mana, 50);
        assert_eq!(state.current[0], 70);

        // Restore capped at max
        state.restore(PowerType::Mana, 100);
        assert_eq!(state.current[0], 100);
    }
}
