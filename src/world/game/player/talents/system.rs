use crate::shared::protocol::ObjectGuid;
use crate::world::World;
use anyhow::Result;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use super::dbc::TalentStore;
use super::effects;
use super::points;
use super::reset;
use super::validation::{self, TalentLearnResult};

/// TalentSystem orchestrates all talent operations.
///
/// Holds a reference to the global TalentStore (loaded from DBC)
/// and provides methods for learning, resetting, and managing talents.
///
/// The system follows the same pattern as other game systems:
/// - Validate input (pure functions)
/// - Mutate state (brief locks)
/// - Apply effects (delegate to other systems)
pub struct TalentSystem {
    /// Global talent data loaded from DBC files at startup.
    /// Uses RwLock to allow reloading after DBC files are loaded.
    store: RwLock<Arc<TalentStore>>,
}

impl TalentSystem {
    pub fn new(store: Arc<TalentStore>) -> Self {
        Self {
            store: RwLock::new(store),
        }
    }

    /// Reload the talent store from DBC data.
    ///
    /// This is called after DBC files are loaded to populate the talent store
    /// with actual data from Talent.dbc and TalentTab.dbc.
    pub fn reload_from_dbc(&self, store: Arc<TalentStore>) {
        let talent_count = store.talents.len();
        let tab_count = store.tabs.len();
        *self.store.write() = store;
        tracing::info!(
            "TalentSystem reloaded with {} talents and {} tabs",
            talent_count,
            tab_count
        );
    }

    /// Get a reference to the talent store.
    pub fn store(&self) -> Arc<TalentStore> {
        Arc::clone(&*self.store.read())
    }

    /// Learn a talent rank for a player.
    ///
    /// This is the main entry point called from the CMSG_LEARN_TALENT handler.
    /// Performs validation, updates state, and applies effects.
    ///
    /// # Arguments
    /// * `player_guid` - The player learning the talent
    /// * `talent_id` - DBC talent ID to learn
    /// * `world` - World reference for system access
    ///
    /// # Returns
    /// Ok(()) on success, Err on failure (logged, not sent to client --
    /// the client already validates locally)
    pub async fn learn_talent(
        &self,
        player_guid: ObjectGuid,
        talent_id: u32,
        world: &World,
    ) -> Result<()> {
        // 1. Snapshot state for validation (brief lock)
        let Some((state_snapshot, class_id)) = world
            .systems
            .player
            .manager()
            .with_player(player_guid, |player| (player.talents.clone(), player.class))
        else {
            return Ok(()); // Player not found
        };

        // 2. Pure validation (no locks)
        let store = self.store.read();
        let result =
            validation::validate_learn_talent(&state_snapshot, talent_id, class_id, &store);

        if result != TalentLearnResult::Ok {
            tracing::warn!(
                "Player {:?} failed to learn talent {}: {:?}",
                player_guid,
                talent_id,
                result
            );
            return Ok(()); // Silently reject (client should prevent this)
        }

        // 3. Get talent info for effect application
        let talent_info = store
            .get_talent(talent_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Talent {} not found", talent_id))?;
        drop(store); // Release lock before async operations

        let old_rank = state_snapshot.talent_rank(talent_id);
        let new_rank = old_rank + 1;

        // 4. Update state (brief lock)
        world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                player.talents.talents.insert(talent_id, new_rank);
                player.talents.used_talent_count += 1;
                player.talents.free_talent_points -= 1;
            });

        // 5. Apply talent spell effects (may acquire other locks)
        let store = self.store.read();
        effects::apply_talent_rank(player_guid, talent_id, new_rank, old_rank, &store, world)
            .await?;
        drop(store);

        // 6. Update client (PLAYER_CHARACTER_POINTS1)
        // Note: Player doesn't have set_need_update(), we need to mark for update differently
        // For now, we'll rely on the periodic update or other systems to sync

        tracing::debug!(
            "Player {:?} learned talent {} rank {}/{}",
            player_guid,
            talent_id,
            new_rank,
            talent_info.max_rank()
        );

        Ok(())
    }

    /// Reset all talents for a player.
    ///
    /// Called from the trainer NPC handler or GM commands.
    /// Removes all talent effects, clears talent data, refunds points,
    /// and charges gold (unless no_cost is true).
    ///
    /// Ported from MaNGOS Player::resetTalents() (Player.cpp:3340-3420).
    ///
    /// # Arguments
    /// * `player_guid` - The player resetting talents
    /// * `no_cost` - If true, skip gold charge (GM command, automatic reset)
    /// * `world` - World reference for system access
    ///
    /// # Returns
    /// Ok(true) if reset succeeded, Ok(false) if failed (no talents or no gold)
    pub async fn reset_talents(
        &self,
        player_guid: ObjectGuid,
        no_cost: bool,
        world: &World,
    ) -> Result<bool> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // 1. Snapshot state (brief lock)
        let Some((state_snapshot, money)) = world
            .systems
            .player
            .manager()
            .with_player(player_guid, |player| (player.talents.clone(), player.money))
        else {
            return Ok(false); // Player not found
        };

        // 2. Check if there's anything to reset
        if state_snapshot.used_talent_count == 0 {
            return Ok(false);
        }

        // 3. Calculate cost (pure)
        let cost = if no_cost {
            0
        } else {
            let config = reset::ResetCostConfig::default(); // TODO: load from config
            let decayed_multi = reset::decay_multiplier(
                state_snapshot.reset_cost_multiplier,
                state_snapshot.last_reset_time,
                now,
                config.min_multiplier,
            );
            reset::calculate_reset_cost(
                decayed_multi,
                config.base_cost,
                config.multi_cost,
                config.max_multiplier,
            )
        };

        // 4. Check gold (pure)
        if cost > 0 && money < cost {
            tracing::warn!(
                "Player {:?} cannot afford talent reset: need {} copper, have {}",
                player_guid,
                cost,
                money
            );
            return Ok(false);
        }

        // 5. Remove all talent effects (acquires locks)
        let store = self.store.read();
        effects::remove_all_talent_effects(player_guid, &state_snapshot, &store, world).await?;
        drop(store);

        // 6. Update state (brief lock)
        let Some(level) = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                // Clear talent data
                player.talents.talents.clear();
                player.talents.used_talent_count = 0;

                // Charge gold
                if cost > 0 {
                    player.money -= cost;
                }

                // Update reset tracking (only for paid resets)
                if !no_cost {
                    player.talents.reset_cost_multiplier += 1;
                    player.talents.last_reset_time = now;
                }

                player.level
            })
        else {
            return Ok(false); // Player not found
        };

        // 7. Recalculate free points
        world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                points::update_free_talent_points(
                    &mut player.talents,
                    level,
                    1.0, // rate - TODO: load from config
                    false,
                );
            });

        tracing::info!(
            "Player {:?} reset talents (cost: {} copper, no_cost: {})",
            player_guid,
            cost,
            no_cost
        );

        Ok(true)
    }

    /// Handle level-up: grant new talent point if at level 10+.
    ///
    /// Called from the level-up system after the player's level has
    /// been incremented.
    ///
    /// # Arguments
    /// * `player_guid` - The player who leveled up
    /// * `new_level` - The level the player just reached
    /// * `world` - World reference
    pub fn on_level_up(&self, player_guid: ObjectGuid, new_level: u8, world: &World) -> Result<()> {
        let rate = 1.0; // TODO: load from config

        world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                points::on_level_up(&mut player.talents, new_level, rate);
            });

        Ok(())
    }

    /// Reapply all talent effects on player login.
    ///
    /// After loading talent data from the database, passive auras and
    /// spell modifiers must be reconstructed since they are not persisted.
    ///
    /// Called during the player login sequence, after _load_talents().
    ///
    /// # Arguments
    /// * `player_guid` - The player logging in
    /// * `world` - World reference
    pub async fn on_player_login(&self, player_guid: ObjectGuid, world: &World) -> Result<()> {
        let Some(state_snapshot) = world
            .systems
            .player
            .manager()
            .with_player(player_guid, |player| player.talents.clone())
        else {
            return Ok(()); // Player not found
        };

        let store = self.store.read();
        effects::reapply_all_talent_effects(player_guid, &state_snapshot, &store, world).await?;
        drop(store);

        tracing::debug!(
            "Reapplied {} talent effects for player {:?}",
            state_snapshot.talents.len(),
            player_guid
        );

        Ok(())
    }
}
