//! Experience System
//!
//! Handles player XP gain, level-up calculations, and packet sending.
//! Ported from server/src/world/game/experience.rs with world patterns.

use anyhow::Result;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tracing::{debug, info};

use crate::shared::game::experience::{
    XpColor, XpSource, BASE_CREATURE_XP, BASE_XP, MAX_PLAYER_LEVEL,
};
use crate::shared::messages::experience::{SmsgLevelupInfo, SmsgLogXpGain};
use crate::shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::ObjectGuid;
use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
use crate::world::game::broadcast_mgr::{BroadcastManager, BroadcastManagerExt};
use crate::world::game::common::update_fields::{
    PLAYER_NEXT_LEVEL_XP, PLAYER_XP, UNIT_FIELD_LEVEL,
};
use crate::world::game::player::stats::StatsSystem;
use crate::world::game::player::PlayerManager;

// ========== Standalone Calculation Functions ==========
// These are pure functions for testing without system dependencies

/// Calculate XP required for a given level
///
/// Formula: BASE_XP * level^2.5
pub fn calculate_xp_for_level(level: u8) -> u32 {
    if level == 0 {
        return 0;
    }
    if level >= MAX_PLAYER_LEVEL {
        return u32::MAX;
    }

    let level_f = level as f32;
    (BASE_XP * level_f.powf(2.5)).round() as u32
}

/// Get the gray level threshold for a player
///
/// Creatures at or below this level give no XP.
/// Matches MaNGOS XP::GetGrayLevel()
pub fn get_gray_level(player_level: u8) -> u8 {
    if player_level <= 5 {
        0
    } else if player_level <= 39 {
        player_level - 5 - player_level / 10
    } else {
        player_level - 1 - player_level / 5
    }
}

/// Get the zero difference value for XP calculation
///
/// Matches MaNGOS XP::GetZeroDifference()
pub fn get_zero_difference(player_level: u8) -> u8 {
    match player_level {
        0..=7 => 5,
        8..=9 => 6,
        10..=11 => 7,
        12..=15 => 8,
        16..=19 => 9,
        20..=29 => 11,
        30..=39 => 12,
        40..=44 => 13,
        45..=49 => 14,
        50..=54 => 15,
        55..=59 => 16,
        _ => 17,
    }
}

/// Calculate XP for killing a creature
///
/// Based on MaNGOS Formulas.h XP::BaseGain() and XP::Gain()
pub fn calculate_creature_xp(creature_level: u8, player_level: u8, is_elite: bool) -> u32 {
    if creature_level == 0 || player_level == 0 {
        return 0;
    }

    // Calculate level factor based on level difference
    let level_factor = if creature_level >= player_level {
        // Creature is same level or higher: +5% per level diff, max +20%
        let level_diff = (creature_level - player_level).min(4);
        1.0 + 0.05 * level_diff as f32
    } else {
        // Creature is lower level
        let gray_lvl = get_gray_level(player_level);
        if creature_level > gray_lvl {
            let zero_diff = get_zero_difference(player_level);
            (zero_diff as i32 + creature_level as i32 - player_level as i32) as f32
                / zero_diff as f32
        } else {
            // Gray mob, no XP
            return 0;
        }
    };

    // Base XP: (player_level * 5 + 45) * level_factor
    let base_xp = (player_level as f32 * 5.0 + BASE_CREATURE_XP) * level_factor;

    // Apply elite multiplier
    let multiplier = if is_elite { 2.0 } else { 1.0 };

    (base_xp * multiplier).round() as u32
}

/// Get XP color code for level difference display
///
/// Matches MaNGOS XP::GetColorCode()
pub fn get_xp_color(player_level: u8, target_level: u8) -> XpColor {
    if target_level >= player_level.saturating_add(5) {
        XpColor::Red
    } else if target_level >= player_level.saturating_add(3) {
        XpColor::Orange
    } else if target_level.saturating_add(2) >= player_level {
        XpColor::Yellow
    } else if target_level > get_gray_level(player_level) {
        XpColor::Green
    } else {
        XpColor::Gray
    }
}

/// Check if target gives XP (not gray)
pub fn gives_xp(player_level: u8, target_level: u8) -> bool {
    target_level > get_gray_level(player_level)
}

/// Experience System - handles XP gain and level-up
pub struct ExperienceSystem {
    broadcast_mgr: Arc<BroadcastManager>,
    player_mgr: Arc<PlayerManager>,
    stats: OnceLock<Arc<StatsSystem>>,
}

impl ExperienceSystem {
    /// Create a new ExperienceSystem
    pub fn new(broadcast_mgr: Arc<BroadcastManager>, player_mgr: Arc<PlayerManager>) -> Self {
        Self {
            broadcast_mgr,
            player_mgr,
            stats: OnceLock::new(),
        }
    }

    /// Set the stats system reference (called after SystemManager construction)
    pub fn set_stats_system(&self, stats: Arc<StatsSystem>) {
        let _ = self.stats.set(stats);
    }

    // ========== Lifecycle Methods ==========

    /// Initialize the experience system
    pub async fn init(&self) -> Result<()> {
        Ok(())
    }

    /// Periodic update (for rest state calculation, etc.)
    pub fn update(&self, _diff: Duration) -> Result<()> {
        // Rest state calculations could go here in the future
        Ok(())
    }

    /// Shutdown the experience system
    pub async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    /// Handle player login
    pub fn on_player_login(&self, guid: ObjectGuid) -> Result<()> {
        // Initialize next_level_xp if not set
        if let Some(mut player) = self.player_mgr.get_player_mut(guid) {
            if player.next_level_xp == 0 {
                player.next_level_xp = calculate_xp_for_level(player.level);
            }
        }
        Ok(())
    }

    /// Handle player logout
    pub fn on_player_logout(&self, _guid: ObjectGuid) -> Result<()> {
        // XP is saved via character save system, nothing to do here
        Ok(())
    }

    // ========== Core XP Methods ==========

    /// Add XP to player
    ///
    /// Returns `(xp_gained, leveled_up, new_level)` tuple.
    pub async fn add_xp(
        &self,
        player_guid: ObjectGuid,
        xp_amount: u32,
        source: XpSource,
        victim_guid: Option<ObjectGuid>,
        group_bonus: f32,
    ) -> Result<(u32, bool, u8)> {
        if xp_amount == 0 {
            return Ok((0, false, 0));
        }

        // Get player data
        let (current_level, current_xp, next_level_xp) = {
            let player = self
                .player_mgr
                .get_player(player_guid)
                .ok_or_else(|| anyhow::anyhow!("Player not found"))?;

            // Can't gain XP at max level
            if player.level >= MAX_PLAYER_LEVEL {
                return Ok((0, false, player.level));
            }

            (player.level, player.xp, player.next_level_xp)
        };

        let mut leveled_up = false;
        let mut new_level = current_level;
        let mut new_xp = current_xp.saturating_add(xp_amount);
        let mut xp_for_next_level = next_level_xp;

        // Check for level up (can level up multiple times if XP is large)
        // TODO have max level in config to go up to 255
        while new_level < MAX_PLAYER_LEVEL && new_xp >= xp_for_next_level {
            new_xp -= xp_for_next_level;
            new_level += 1;
            leveled_up = true;
            xp_for_next_level = calculate_xp_for_level(new_level);
        }

        // Update player state
        if let Some(mut player) = self.player_mgr.get_player_mut(player_guid) {
            player.xp = new_xp;
            player.level = new_level;
            player.next_level_xp = xp_for_next_level;
        }

        // Send XP gain packet
        let xp_type = match source {
            XpSource::Kill | XpSource::Pvp => 0,
            XpSource::Quest | XpSource::Discovery => 1,
        };

        let xp_msg = SmsgLogXpGain {
            victim_guid: victim_guid.unwrap_or_else(ObjectGuid::empty),
            total_xp: xp_amount,
            xp_type,
            group_bonus,
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, xp_msg);

        // Send level up packet if needed
        if leveled_up {
            // Recalculate stats for new level (sets health/mana to max)
            if let Some(stats) = self.stats.get() {
                stats.on_level_up(player_guid);
            }

            self.send_levelup_info(player_guid, current_level, new_level)?;

            // Broadcast updated stats (health, mana, attributes) to client
            if let Some(stats) = self.stats.get() {
                stats.send_stat_update(player_guid);
            }
        }

        // Send field updates to client (updates XP bar and level display)
        let world_guid = WorldObjectGuid::from_low(player_guid.counter());
        let mut values_block = ValuesUpdateBlock::new(world_guid, ObjectType::Player)
            .set_field(PLAYER_XP, new_xp)
            .set_field(PLAYER_NEXT_LEVEL_XP, xp_for_next_level);

        // Add level field if leveled up
        if leveled_up {
            values_block = values_block.set_field(UNIT_FIELD_LEVEL, new_level as u32);
        }

        let update_msg = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(values_block));
        let packet = update_msg.to_world_packet();

        // Broadcast to self and nearby players (include_self = true)
        self.broadcast_mgr
            .broadcast_nearby(player_guid, &packet, true);

        // Log XP gain
        if leveled_up {
            info!(
                "Player {} leveled up from {} to {} (gained {} XP)",
                player_guid.counter(),
                current_level,
                new_level,
                xp_amount
            );
        } else {
            debug!(
                "Player {} gained {} XP ({}/{})",
                player_guid.counter(),
                xp_amount,
                new_xp,
                xp_for_next_level
            );
        }

        Ok((xp_amount, leveled_up, new_level))
    }

    /// Send level up info packet to player
    fn send_levelup_info(
        &self,
        player_guid: ObjectGuid,
        old_level: u8,
        new_level: u8,
    ) -> Result<()> {
        // Get race/class from player for stat gain calculation
        let player_info = self
            .player_mgr
            .get_player(player_guid)
            .map(|p| (p.race, p.class));

        let (hp_gain, mana_gain, stat_gains) =
            if let (Some((race, class)), Some(stats)) = (player_info, self.stats.get()) {
                stats.get_level_up_gains(race, class, old_level, new_level)
            } else {
                // Fallback if stats system not available
                let level_diff = (new_level - old_level) as u32;
                (level_diff * 10, level_diff * 5, [level_diff; 5])
            };

        let msg = SmsgLevelupInfo {
            level: new_level as u32,
            hp_gain,
            mana_gain,
            stat_gains,
        };

        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

        Ok(())
    }

    // ========== XP Calculation Methods (delegate to standalone functions) ==========

    /// Calculate XP required for a given level
    pub fn xp_for_level(&self, level: u8) -> u32 {
        calculate_xp_for_level(level)
    }

    /// Calculate XP for killing a creature
    pub fn creature_xp(&self, creature_level: u8, player_level: u8, is_elite: bool) -> u32 {
        calculate_creature_xp(creature_level, player_level, is_elite)
    }

    /// Get the gray level threshold for a player
    pub fn gray_level(&self, player_level: u8) -> u8 {
        get_gray_level(player_level)
    }

    /// Get XP color code for level difference display
    pub fn xp_color(&self, player_level: u8, target_level: u8) -> XpColor {
        get_xp_color(player_level, target_level)
    }

    /// Check if target gives XP (not gray)
    pub fn target_gives_xp(&self, player_level: u8, target_level: u8) -> bool {
        gives_xp(player_level, target_level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gray_level() {
        // Level 1-5: gray level is 0
        assert_eq!(get_gray_level(1), 0);
        assert_eq!(get_gray_level(5), 0);

        // Level 10: gray level is 10 - 5 - 1 = 4
        assert_eq!(get_gray_level(10), 4);

        // Level 20: gray level is 20 - 5 - 2 = 13
        assert_eq!(get_gray_level(20), 13);

        // Level 40: gray level is 40 - 1 - 8 = 31
        assert_eq!(get_gray_level(40), 31);

        // Level 60: gray level is 60 - 1 - 12 = 47
        assert_eq!(get_gray_level(60), 47);
    }

    #[test]
    fn test_xp_for_level() {
        // Level 1: 400 * 1^2.5 = 400
        assert_eq!(calculate_xp_for_level(1), 400);

        // Level 10: 400 * 10^2.5 = 400 * 316.23 = 126491 (approx)
        let xp_10 = calculate_xp_for_level(10);
        assert!(xp_10 > 100000 && xp_10 < 150000);

        // Max level returns u32::MAX
        assert_eq!(calculate_xp_for_level(60), u32::MAX);
    }

    #[test]
    fn test_creature_xp() {
        // Same level creature (level 10 player vs level 10 mob)
        // XP = (10 * 5 + 45) * 1.0 = 95
        let xp = calculate_creature_xp(10, 10, false);
        assert_eq!(xp, 95);

        // Higher level creature (+2 levels)
        // XP = (10 * 5 + 45) * 1.1 = 104.5 -> 105
        let xp = calculate_creature_xp(12, 10, false);
        assert_eq!(xp, 105);

        // Elite creature (2x)
        let xp = calculate_creature_xp(10, 10, true);
        assert_eq!(xp, 190);

        // Gray creature (no XP)
        let xp = calculate_creature_xp(1, 20, false);
        assert_eq!(xp, 0);
    }

    #[test]
    fn test_xp_color() {
        // Red: +5 or more levels
        assert_eq!(get_xp_color(10, 15), XpColor::Red);

        // Orange: +3 to +4 levels
        assert_eq!(get_xp_color(10, 13), XpColor::Orange);

        // Yellow: -2 to +2 levels
        assert_eq!(get_xp_color(10, 10), XpColor::Yellow);
        assert_eq!(get_xp_color(10, 8), XpColor::Yellow);

        // Green: below yellow but above gray
        assert_eq!(get_xp_color(10, 5), XpColor::Green);

        // Gray: at or below gray level
        assert_eq!(get_xp_color(10, 3), XpColor::Gray);
    }
}
