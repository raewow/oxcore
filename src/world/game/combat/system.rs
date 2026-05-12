//! Combat System - Orchestrates auto-attacks, hit tables, and damage
//!
//! The CombatSystem is stateless - all per-player state lives in Player.combat.
//! Systems access player state through PlayerManager.with_player_mut().

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, trace, warn};

use crate::shared::protocol::{ObjectGuid, Opcode};
use crate::world::game::broadcast_mgr::BroadcastManagerTrait;
use crate::world::game::player::PlayerManager;

use super::auto_attack::{update_auto_attack, PendingAttack};
use super::damage::calculate_melee_damage;
use super::hit_table::{calculate_hit_table, CombatSnapshot};
use super::state::{AttackHand, CombatState, DamageResult};

/// Combat system - stateless, operates on Player.combat
pub struct CombatSystem {
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
}

impl CombatSystem {
    /// Create a new combat system
    pub fn new(broadcast_mgr: Arc<dyn BroadcastManagerTrait>) -> Self {
        Self { broadcast_mgr }
    }

    /// Initialize the system
    pub async fn init(&self) -> Result<()> {

        Ok(())
    }

    /// Update combat timers for all players
    /// Called every world tick
    pub fn update(&self, _diff: Duration, _player_mgr: &PlayerManager) -> Result<()> {
        // Combat update is now handled per-player through PlayerManager
        // This method is kept for system update pattern consistency
        Ok(())
    }

    /// Update auto-attack for a specific player
    /// Returns list of pending attacks to execute
    pub fn update_player_auto_attack(
        &self,
        player_guid: ObjectGuid,
        diff_ms: u32,
        player_mgr: &PlayerManager,
    ) -> Vec<PendingAttack> {
        player_mgr
            .with_player_mut(player_guid, |player| {
                update_auto_attack(player_guid, diff_ms, &mut player.combat)
            })
            .unwrap_or_default()
    }

    /// Start auto-attacking a target
    pub async fn start_attack(
        &self,
        attacker: ObjectGuid,
        target: ObjectGuid,
        player_mgr: &PlayerManager,
    ) -> Result<()> {
        player_mgr.with_player_mut(attacker, |player| {
            player.combat.start_attack(target);
            debug!("Player {} started attacking {}", attacker, target);
        });

        // Notify others that we're attacking
        // SMSG_ATTACKSTART would go here

        Ok(())
    }

    /// Stop auto-attacking
    pub async fn stop_attack(
        &self,
        attacker: ObjectGuid,
        player_mgr: &PlayerManager,
    ) -> Result<()> {
        player_mgr.with_player_mut(attacker, |player| {
            player.combat.stop_attack();
            debug!("Player {} stopped attacking", attacker);
        });

        // SMSG_ATTACKSTOP would go here

        Ok(())
    }

    /// Execute a pending attack
    pub async fn execute_attack(
        &self,
        attack: &PendingAttack,
        player_mgr: &PlayerManager,
    ) -> Result<Option<DamageResult>> {
        // Get attacker data
        let attacker_data = player_mgr.with_player(attack.attacker, |player| {
            AttackerData {
                level: player.level,
                weapon_skill: player.level as u16 * 5,
                hit_bonus: 0.0, // TODO: Get from stats
                crit_chance: player.stats.melee_crit_pct,
                ap: player.stats.melee_attack_power,
                weapon_min: player.combat.main_hand_min_dmg,
                weapon_max: player.combat.main_hand_max_dmg,
                weapon_speed: player.combat.main_hand_speed,
                is_dual_wielding: player.combat.can_dual_wield,
            }
        });

        let Some(attacker_data) = attacker_data else {
            return Ok(None);
        };

        // Get defender data (for now assume it's another player)
        // TODO: Handle creature targets
        let defender_data = player_mgr.with_player(attack.target, |player| {
            DefenderData {
                level: player.level,
                defense_skill: player.level as u16 * 5,
                dodge_chance: player.stats.dodge_pct,
                parry_chance: player.stats.parry_pct,
                block_chance: player.stats.block_pct,
                block_value: 0, // TODO: Get from shield
                armor: player.stats.armor,
                can_parry: player.combat.can_parry,
                can_block: player.combat.can_block,
            }
        });

        let Some(defender_data) = defender_data else {
            return Ok(None);
        };

        // Build combat snapshot
        let snapshot = CombatSnapshot {
            attacker_level: attacker_data.level,
            attacker_weapon_skill: attacker_data.weapon_skill,
            attacker_hit_bonus: attacker_data.hit_bonus,
            attacker_crit_chance: attacker_data.crit_chance,
            attacker_ap: attacker_data.ap,
            is_dual_wielding: attacker_data.is_dual_wielding,
            defender_level: defender_data.level,
            defender_defense_skill: defender_data.defense_skill,
            defender_dodge_chance: defender_data.dodge_chance,
            defender_parry_chance: defender_data.parry_chance,
            defender_block_chance: defender_data.block_chance,
            defender_block_value: defender_data.block_value,
            defender_armor: defender_data.armor,
            defender_is_player: true,
            defender_can_parry: defender_data.can_parry,
            defender_can_block: defender_data.can_block,
            hand: attack.hand,
            is_ranged: attack.hand == AttackHand::Ranged,
        };

        // Determine outcome via hit table
        let outcome = calculate_hit_table(&snapshot);
        trace!("Attack outcome: {:?}", outcome);

        // Get weapon damage for the appropriate hand
        let (weapon_min, weapon_max, weapon_speed) = match attack.hand {
            AttackHand::MainHand => (
                attacker_data.weapon_min,
                attacker_data.weapon_max,
                attacker_data.weapon_speed,
            ),
            AttackHand::OffHand => (
                attacker_data.weapon_min * 0.5, // Offhand penalty in damage calc
                attacker_data.weapon_max * 0.5,
                attacker_data.weapon_speed,
            ),
            AttackHand::Ranged => (
                attacker_data.weapon_min,
                attacker_data.weapon_max,
                attacker_data.weapon_speed,
            ),
        };

        // Calculate damage
        let damage_result =
            calculate_melee_damage(&snapshot, outcome, weapon_min, weapon_max, weapon_speed);

        // Apply damage to target
        if damage_result.damage > 0 {
            self.apply_damage(attack.target, damage_result.damage, player_mgr)
                .await?;
            // NOTE: Honor contributor tracking (Phase 8) currently lives in
            // the combat spells damage pipeline and on-killed hooks rather
            // than here — CombatSystem doesn't hold a World handle, so we
            // attribute the killer as the sole contributor in on_killed.
        }

        // Broadcast attack result
        self.broadcast_attack_result(attack, &damage_result)?;

        Ok(Some(damage_result))
    }

    /// Apply damage to a target
    async fn apply_damage(
        &self,
        target: ObjectGuid,
        damage: u32,
        player_mgr: &PlayerManager,
    ) -> Result<()> {
        player_mgr.with_player_mut(target, |player| {
            // Apply damage to health
            let current_health = player.stats.health;
            let new_health = current_health.saturating_sub(damage);
            player.stats.health = new_health;

            // Enter combat
            player.combat.enter_combat(target);

            debug!(
                "Player {} took {} damage, health: {} -> {}",
                target, damage, current_health, new_health
            );

            // TODO: Broadcast health update
            // TODO: Check for death
        });

        Ok(())
    }

    /// Broadcast attack result to nearby players
    fn broadcast_attack_result(
        &self,
        attack: &PendingAttack,
        result: &DamageResult,
    ) -> Result<()> {
        // Build SMSG_ATTACKERSTATEUPDATE packet
        // This would use the messages system
        trace!(
            "Attack result: {} -> {}: {:?} for {} damage",
            attack.attacker,
            attack.target,
            result.outcome,
            result.damage
        );

        // TODO: Implement SMSG_ATTACKERSTATEUPDATE message

        Ok(())
    }

    /// Enter combat state
    pub fn enter_combat(
        &self,
        player: ObjectGuid,
        attacker: ObjectGuid,
        player_mgr: &PlayerManager,
    ) {
        player_mgr.with_player_mut(player, |p| {
            p.combat.enter_combat(attacker);
        });
    }

    /// Leave combat
    pub fn leave_combat(&self, player: ObjectGuid, player_mgr: &PlayerManager) {
        player_mgr.with_player_mut(player, |p| {
            p.combat.in_combat = false;
            p.combat.combat_timer = 0;
            p.combat.attackers.clear();
            p.combat.stop_attack();
        });
    }

    /// Check if player is in combat
    pub fn is_in_combat(&self, player: ObjectGuid, player_mgr: &PlayerManager) -> bool {
        player_mgr
            .with_player(player, |p| p.combat.in_combat)
            .unwrap_or(false)
    }

    /// Get attack target
    pub fn get_attack_target(
        &self,
        player: ObjectGuid,
        player_mgr: &PlayerManager,
    ) -> Option<ObjectGuid> {
        player_mgr
            .with_player(player, |p| p.combat.attack_target)
            .flatten()
    }

    /// Update weapon stats from equipment
    pub fn update_weapon_stats(
        &self,
        player: ObjectGuid,
        player_mgr: &PlayerManager,
    ) -> Result<()> {
        player_mgr.with_player_mut(player, |p| {
            // TODO: Read weapon stats from equipment
            // For now use defaults from stats system
            p.combat.main_hand_min_dmg = p.stats.min_damage;
            p.combat.main_hand_max_dmg = p.stats.max_damage;
            p.combat.off_hand_min_dmg = p.stats.min_offhand_damage;
            p.combat.off_hand_max_dmg = p.stats.max_offhand_damage;
            p.combat.ranged_min_dmg = p.stats.min_ranged_damage;
            p.combat.ranged_max_dmg = p.stats.max_ranged_damage;
        });

        Ok(())
    }

    /// Set dual wield capability
    pub fn set_can_dual_wield(
        &self,
        player: ObjectGuid,
        can_dual_wield: bool,
        player_mgr: &PlayerManager,
    ) {
        player_mgr.with_player_mut(player, |p| {
            p.combat.can_dual_wield = can_dual_wield;
        });
    }

    /// Add combo points
    pub fn add_combo_points(
        &self,
        player: ObjectGuid,
        target: ObjectGuid,
        points: u8,
        player_mgr: &PlayerManager,
    ) {
        player_mgr.with_player_mut(player, |p| {
            p.combat.add_combo_points(target, points);
        });
    }

    /// Clear combo points
    pub fn clear_combo_points(&self, player: ObjectGuid, player_mgr: &PlayerManager) {
        player_mgr.with_player_mut(player, |p| {
            p.combat.clear_combo_points();
        });
    }
}

/// Data snapshot for attacker
#[derive(Debug, Clone)]
struct AttackerData {
    level: u8,
    weapon_skill: u16,
    hit_bonus: f32,
    crit_chance: f32,
    ap: i32,
    weapon_min: f32,
    weapon_max: f32,
    weapon_speed: u32,
    is_dual_wielding: bool,
}

/// Data snapshot for defender
#[derive(Debug, Clone)]
struct DefenderData {
    level: u8,
    defense_skill: u16,
    dodge_chance: f32,
    parry_chance: f32,
    block_chance: f32,
    block_value: u32,
    armor: u32,
    can_parry: bool,
    can_block: bool,
}

impl CombatSystem {
    /// Shutdown the system
    pub async fn shutdown(&self) -> Result<()> {

        Ok(())
    }

    /// Called when a player logs in
    pub fn on_player_login(&self, _guid: ObjectGuid) -> Result<()> {
        // Combat state is already initialized in Player::new
        Ok(())
    }

    /// Called when a player logs out
    pub fn on_player_logout(&self, player: ObjectGuid, player_mgr: &PlayerManager) -> Result<()> {
        // Stop any ongoing combat
        self.leave_combat(player, player_mgr);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require mocking PlayerManager and BroadcastManager
    // Integration tests would be more appropriate for this system
}
