//! Power System - handles regeneration and power consumption
//!
//! Stateless system that operates on PowerState embedded in Player.

use crate::game::broadcast_mgr::BroadcastManagerTrait;
use crate::game::common::update_fields::*;
use crate::game::player::auras::effects::AURA_MOD_CONFUSE;
use crate::game::player::manager::PlayerManager;
use crate::game::player::Player;
use crate::World;
use anyhow::Result;
use oxcore_shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::ObjectGuid;
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

    fn apply_damage_dealt_rage(player: &mut Player, damage: u32) -> bool {
        if player.power.power_type != PowerType::Rage {
            return false;
        }

        let rage = regen::rage_from_damage_dealt(damage, player.level);
        player.power.modify_power(PowerType::Rage, rage as i32) != 0
    }

    fn apply_damage_taken_rage(player: &mut Player, damage: u32) -> bool {
        if player.power.power_type != PowerType::Rage {
            return false;
        }

        let rage = regen::rage_from_damage_taken(damage, player.level);
        player.power.modify_power(PowerType::Rage, rage as i32) != 0
    }

    fn power_value_update_block(
        player_guid: ObjectGuid,
        power_type: PowerType,
        value: u32,
    ) -> ValuesUpdateBlock {
        let field_offset = UNIT_FIELD_POWER1 + power_type as u32;
        ValuesUpdateBlock::new(player_guid, ObjectType::Player).set_field(field_offset, value)
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
        let mut health_updates: Vec<(ObjectGuid, u32)> = Vec::new();
        let player_mgr = world.managers.player_mgr.clone();
        player_mgr.for_each_player(|guid, player| {
            let power_type = player.power.power_type;
            let idx = power_type as usize;
            let old_value = player.power.current[idx];
            let old_health = player.stats.health;
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
            if player.stats.health != old_health {
                health_updates.push((guid, player.stats.health));
            }
        });

        // Broadcast power updates outside the player lock
        for (guid, power_type, value) in power_updates {
            self.broadcast_power_value(guid, power_type, value, world);
        }
        for (guid, value) in health_updates {
            self.broadcast_health_value(guid, value);
        }

        Ok(())
    }

    /// Process one regen tick for a player
    fn regen_tick(&self, guid: ObjectGuid, player: &mut Player, now: u64) {
        // Sync power max from stats (stats can change from gear/buffs at any time)
        player.power.set_max(PowerType::Mana, player.stats.max_mana);

        let power = &mut player.power;
        let stats = &player.stats;

        match power.power_type {
            PowerType::Mana => {
                // Check 5-second rule
                power.spirit_regen_active = now >= power.last_mana_use_time + FIVE_SECOND_RULE_MS;

                // StatsSystem precomputes aura-adjusted full and interrupt regen.
                let mut total_regen = regen::calculate_mana_regen_per_tick(
                    stats.mana_regen_base,
                    stats.mana_regen_interrupt,
                    power.spirit_regen_active,
                );
                total_regen += power.mp5_from_gear * 2.0 / 5.0;

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
                if !player.combat.in_combat {
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

        // Health regen (Player::RegenerateHealth). Skipped when at max health.
        // Full implementation also requires MOD_REGEN_DURING_COMBAT and
        // MOD_HEALTH_REGEN_IN_COMBAT aura totals.
        {
            let cur_health = player.stats.health;
            let max_health = player.stats.max_health;
            if cur_health < max_health {
                let in_combat = player.combat.in_combat;
                let mut add_value: f32 = 0.0;

                // Polymorph: regen 10% of max health per 2-second tick regardless of combat.
                let is_polymorphed = player.auras.container.has_aura_type(AURA_MOD_CONFUSE);

                if is_polymorphed {
                    add_value = max_health as f32 / 10.0;
                } else if !in_combat {
                    // Spirit-based regen out of combat (MOD_REGEN_DURING_COMBAT talent variant
                    // is omitted until aura totals are wired in).
                    add_value =
                        regen::calculate_health_regen_per_tick(player.stats.spirit, player.level);
                    add_value *= player.power.health_regen_multiplier;
                    // C++ IsStandingUp is true only for standing and dead units.
                    if player.stand_state != 0 && player.stand_state != 7 {
                        add_value *= 1.5;
                    }
                    // MOD_REGEN stores health restored per five seconds.
                    add_value += 2.0 * (player.power.health_regen_per_5 / 5.0);
                }

                // Always-active combat regen bonus (SPELL_AURA_MOD_HEALTH_REGEN_IN_COMBAT).
                // 2.0 because the tick fires every 2 seconds; the aura stores HP per 5 sec
                // so we scale: rate * 2 * (total / 5). Defaults to 0 until aura totals wired.
                let mod_regen_in_combat: f32 = 0.0;
                add_value += 2.0 * (mod_regen_in_combat / 5.0);

                // Carry fractional regen across ticks so sub-integer amounts aren't silently lost.
                let with_carry = add_value + player.power.carry_health_regen;
                let whole = with_carry as i32;
                player.power.carry_health_regen = with_carry - whole as f32;

                if whole > 0 {
                    if player.stats.modify_health(whole) != 0 {
                        player.stats.dirty = true;
                    }
                }
            }
        }
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

    /// Spend power for a finished spell cast (faithful `Spell::TakePower` deduction).
    ///
    /// Unlike [`consume_power`], this never fails: it deducts the amount clamped at
    /// zero (mirroring C++ `Unit::ModifyPower`), since the affordability check already
    /// happened during cast validation. When `reset_mana_timer` is true and the power
    /// is mana, the five-second-rule timer is reset (caller decides based on the
    /// spell's `DONT_BLOCK_MANA_REGEN` attribute and a positive cost).
    pub fn spend_spell_power(
        &self,
        player_guid: ObjectGuid,
        power_type: PowerType,
        amount: u32,
        reset_mana_timer: bool,
        world: &World,
    ) -> Result<()> {
        let player_mgr = world.managers.player_mgr.clone();

        player_mgr.with_player_mut(player_guid, |player| {
            player.power.modify_power(power_type, -(amount as i32));

            if power_type == PowerType::Mana && reset_mana_timer {
                player.power.last_mana_use_time = get_time_ms();
                player.power.spirit_regen_active = false;
            }
        });

        self.send_power_update(player_guid, power_type, world)
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
        let mut changed = false;

        player_mgr.with_player_mut(player_guid, |player| {
            changed = Self::apply_damage_dealt_rage(player, damage);
        });

        if changed {
            self.send_power_update(player_guid, PowerType::Rage, world)?;
        }
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
        let mut changed = false;

        player_mgr.with_player_mut(player_guid, |player| {
            changed = Self::apply_damage_taken_rage(player, damage);
        });

        if changed {
            self.send_power_update(player_guid, PowerType::Rage, world)?;
        }
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
        let block = Self::power_value_update_block(player_guid, power_type, value);
        let update_msg = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(block));
        let packet = update_msg.to_world_packet();
        self.broadcast_mgr
            .broadcast_nearby(player_guid, &packet, true);
    }

    /// Broadcast a health update produced by passive regeneration.
    fn broadcast_health_value(&self, player_guid: ObjectGuid, value: u32) {
        let block = ValuesUpdateBlock::new(player_guid, ObjectType::Player)
            .set_field(UNIT_FIELD_HEALTH, value);
        let update_msg = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(block));
        let packet = update_msg.to_world_packet();
        self.broadcast_mgr
            .broadcast_nearby(player_guid, &packet, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::game::broadcast_mgr::MockBroadcastManagerTrait;
    use crate::game::player::auras::{Aura, AuraFlags};
    use crate::game::player::power::regen::MAX_RAGE;
    use oxcore_shared::database::Databases;
    use sqlx::mysql::MySqlPoolOptions;
    use std::path::PathBuf;

    fn test_world() -> World {
        let pool = || {
            MySqlPoolOptions::new()
                .connect_lazy("mysql://test:test@localhost/test")
                .expect("lazy pool should be constructible")
        };
        let databases = Arc::new(Databases {
            world: pool(),
            character: pool(),
            auth: pool(),
            logs: pool(),
        });

        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

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

    fn warrior(level: u8) -> Player {
        let mut player = Player::new(
            ObjectGuid::new_player(1),
            "Warrior".to_string(),
            0,
            0,
            0,
            level,
            1,
            1,
            0,
        );
        player.power.power_type = PowerType::Rage;
        player.power.max[PowerType::Rage as usize] = MAX_RAGE;
        player
    }

    #[test]
    fn damage_dealt_hook_generates_internal_rage_for_warriors() {
        let mut player = warrior(60);

        assert!(PowerSystem::apply_damage_dealt_rage(&mut player, 100));
        assert_eq!(player.power.current[PowerType::Rage as usize], 32);
    }

    #[test]
    fn damage_taken_hook_generates_internal_rage_for_warriors() {
        let mut player = warrior(60);

        assert!(PowerSystem::apply_damage_taken_rage(&mut player, 100));
        assert_eq!(player.power.current[PowerType::Rage as usize], 10);
    }

    #[test]
    fn rage_generation_clamps_at_internal_max() {
        let mut player = warrior(60);
        player.power.current[PowerType::Rage as usize] = 990;

        assert!(PowerSystem::apply_damage_dealt_rage(&mut player, 100));
        assert_eq!(player.power.current[PowerType::Rage as usize], MAX_RAGE);
    }

    #[test]
    fn non_rage_power_type_does_not_generate_rage() {
        let mut player = warrior(60);
        player.power.power_type = PowerType::Mana;

        assert!(!PowerSystem::apply_damage_dealt_rage(&mut player, 100));
        assert_eq!(player.power.current[PowerType::Rage as usize], 0);
    }

    #[test]
    fn rage_does_not_decay_in_combat() {
        let system = PowerSystem::new(Arc::new(MockBroadcastManagerTrait::new()));
        let mut player = warrior(60);
        player.power.current[PowerType::Rage as usize] = 100;
        player.combat.in_combat = true;

        system.regen_tick(player.guid, &mut player, 0);

        assert_eq!(player.power.current[PowerType::Rage as usize], 100);
    }

    #[test]
    fn rage_decays_out_of_combat() {
        let system = PowerSystem::new(Arc::new(MockBroadcastManagerTrait::new()));
        let mut player = warrior(60);
        player.power.current[PowerType::Rage as usize] = 100;

        system.regen_tick(player.guid, &mut player, 0);

        assert_eq!(
            player.power.current[PowerType::Rage as usize],
            100 - regen::RAGE_DECAY_PER_TICK
        );
    }

    #[test]
    fn polymorphed_player_regenerates_ten_percent_max_health_in_combat() {
        let system = PowerSystem::new(Arc::new(MockBroadcastManagerTrait::new()));
        let mut player = warrior(60);
        player.stats.health = 100;
        player.stats.max_health = 1_000;
        player.combat.in_combat = true;
        player.auras.container.add_aura(Aura::new(
            118,
            player.guid,
            0,
            AURA_MOD_CONFUSE,
            0,
            0,
            Some(10_000),
            0,
            1,
            0,
            AuraFlags::default(),
        ));

        system.regen_tick(player.guid, &mut player, 0);

        assert_eq!(player.stats.health, 200);
    }

    #[test]
    fn seated_player_gets_fifty_percent_more_spirit_health_regen() {
        let system = PowerSystem::new(Arc::new(MockBroadcastManagerTrait::new()));
        let mut player = warrior(60);
        player.stats.health = 100;
        player.stats.max_health = 1_000;
        player.stats.spirit = 100;
        player.stand_state = 1;

        system.regen_tick(player.guid, &mut player, 0);

        // Level 60: 100 spirit * 0.30 = 30; seated multiplier yields 45.
        assert_eq!(player.stats.health, 145);
        assert!(player.stats.dirty);
    }

    #[test]
    fn flat_health_regen_aura_applies_out_of_combat_with_fractional_carry() {
        let system = PowerSystem::new(Arc::new(MockBroadcastManagerTrait::new()));
        let mut player = warrior(60);
        player.stats.health = 100;
        player.stats.max_health = 1_000;
        player.power.health_regen_per_5 = 2.0;

        for _ in 0..5 {
            system.regen_tick(player.guid, &mut player, 0);
        }

        // Two HP per five seconds is 0.8 HP per two-second tick, totaling four HP.
        assert_eq!(player.stats.health, 104);
        assert_eq!(player.power.carry_health_regen, 0.0);
    }

    #[test]
    fn percent_health_regen_scales_spirit_but_not_flat_hp5() {
        let system = PowerSystem::new(Arc::new(MockBroadcastManagerTrait::new()));
        let mut player = warrior(60);
        player.stats.health = 100;
        player.stats.max_health = 1_000;
        player.stats.spirit = 100;
        player.power.health_regen_multiplier = 1.5;
        player.power.health_regen_per_5 = 5.0;

        system.regen_tick(player.guid, &mut player, 0);

        // Spirit regen is 30 * 1.5; the unscaled 5 HP5 contributes 2 per tick.
        assert_eq!(player.stats.health, 147);
        assert!(player.stats.dirty);
    }

    #[test]
    fn flat_health_regen_aura_clamps_at_max_health() {
        let system = PowerSystem::new(Arc::new(MockBroadcastManagerTrait::new()));
        let mut player = warrior(60);
        player.stats.health = 998;
        player.stats.max_health = 1_000;
        player.power.health_regen_per_5 = 10.0;

        system.regen_tick(player.guid, &mut player, 0);

        assert_eq!(player.stats.health, 1_000);
    }

    #[test]
    fn passive_health_regen_broadcasts_health_update() {
        let world = test_world();
        let mut broadcast_mgr = MockBroadcastManagerTrait::new();
        let guid = ObjectGuid::new_player(1);
        broadcast_mgr
            .expect_broadcast_nearby()
            .withf(move |sender_guid, _, include_self| *sender_guid == guid && *include_self)
            .times(1)
            .returning(|_, _, _| ());
        let system = PowerSystem::new(Arc::new(broadcast_mgr));

        let mut player = warrior(60);
        player.stats.health = 100;
        player.stats.max_health = 1_000;
        player.stats.spirit = 100;
        world.managers.player_mgr.add_player(player, 1);

        system
            .update(Duration::from_millis(REGEN_TICK_MS as u64), &world)
            .expect("health regen update should succeed");

        world
            .managers
            .player_mgr
            .with_player(guid, |player| {
                assert_eq!(player.stats.health, 130);
                assert!(player.stats.dirty);
            })
            .expect("regenerating player should remain registered");
    }

    #[test]
    fn regen_tick_syncs_mana_max_and_clamps_current_power() {
        let system = PowerSystem::new(Arc::new(MockBroadcastManagerTrait::new()));
        let mut player = warrior(60);
        let mana_idx = PowerType::Mana as usize;
        player.stats.max_mana = 100;
        player.power.max[mana_idx] = 200;
        player.power.current[mana_idx] = 150;

        system.regen_tick(player.guid, &mut player, 0);

        assert_eq!(player.power.max[mana_idx], 100);
        assert_eq!(player.power.current[mana_idx], 100);
    }

    #[test]
    fn rage_power_update_uses_power2_field_for_client_sync() {
        let block =
            PowerSystem::power_value_update_block(ObjectGuid::new_player(1), PowerType::Rage, 256);

        assert_eq!(block.fields, vec![(UNIT_FIELD_POWER1 + 1, 256)]);
    }
}
