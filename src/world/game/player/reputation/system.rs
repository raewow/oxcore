//! Reputation System for world
//!
//! This is a stateless system that operates on player reputation state
//! through the PlayerManager. All reputation modifications flow through here.

use super::state::{
    rank_cap_to_reputation_rank, FactionEntry, FactionStanding, ReputationSpilloverTemplate,
    ReputationState,
};
use crate::shared::game::reputation::{
    apply_level_reduction, ReputationListID, ReputationRank, FACTION_FLAG_AT_WAR,
    FACTION_FLAG_VISIBLE, MAX_REPUTATION_LIST_SLOTS, REPUTATION_BOTTOM, REPUTATION_CAP,
};
use crate::shared::messages::reputation::{
    SmsgInitializeFactions, SmsgSetFactionStanding, SmsgSetForcedReactions,
};
use crate::shared::protocol::ObjectGuid;
use crate::world::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::world::World;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

/// Stateless reputation system
///
/// All methods take player_guid and World to access player state through PlayerManager.
/// The system itself holds no per-player state.
pub struct ReputationSystem {
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
    /// Cached faction map built from DBC data (built once on first use)
    faction_map_cache: OnceLock<HashMap<u32, FactionEntry>>,
}

impl ReputationSystem {
    /// Create a new reputation system
    pub fn new(broadcast_mgr: Arc<dyn BroadcastManagerTrait>) -> Self {
        Self {
            broadcast_mgr,
            faction_map_cache: OnceLock::new(),
        }
    }

    /// Get or build the faction map from DBC data (cached after first call)
    fn get_or_build_faction_map(&self, world: &World) -> &HashMap<u32, FactionEntry> {
        self.faction_map_cache.get_or_init(|| {
            let dbc_guard = world.dbc.read();
            let mut map = HashMap::new();
            for (faction_id, dbc_entry) in dbc_guard.get_all_factions() {
                map.insert(*faction_id, FactionEntry::from_dbc(dbc_entry));
            }
            tracing::info!(
                "[REPUTATION] Built faction map cache with {} factions",
                map.len()
            );
            map
        })
    }

    /// Initialize all 64 faction slots from Faction.dbc using cached faction map.
    ///
    /// Called during character creation and on first login if reputation data
    /// is missing. Sets standing=0 (relative) for all factions, which means
    /// the absolute reputation equals the DBC base value for the player's
    /// race and class.
    ///
    /// This version uses the cached faction map built from DBC data on first use.
    pub fn initialize(&self, player_guid: ObjectGuid, world: &World) -> Result<()> {
        let faction_map = self.get_or_build_faction_map(world);
        self.initialize_with_faction_map(player_guid, faction_map, world)
    }

    /// Initialize all 64 faction slots from provided faction map.
    ///
    /// Internal method used by initialize(). Kept for compatibility if needed.
    fn initialize_with_faction_map(
        &self,
        player_guid: ObjectGuid,
        faction_map: &HashMap<u32, FactionEntry>,
        world: &World,
    ) -> Result<()> {
        world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                let race = player.race;
                let class = player.class;

                player.reputation.factions.clear();

                for (faction_id, entry) in faction_map {
                    if entry.reputation_list_id < 0 {
                        continue; // Not a trackable faction
                    }

                    let rep_list_id = entry.reputation_list_id as u32;

                    // Get default flags from DBC for this race/class
                    let flags = entry.get_reputation_flags(race, class);

                    // Standing starts at 0 (relative to base).
                    // Absolute = base + 0 = base reputation from DBC.
                    let standing = FactionStanding::new(*faction_id, rep_list_id, 0, flags);
                    player.reputation.factions.insert(rep_list_id, standing);
                }

                player.reputation.need_send = true;
            });

        Ok(())
    }

    /// Modify reputation for a faction (incremental, with spillover).
    ///
    /// This is the primary entry point for reputation gains/losses from
    /// quests, creature kills, item turn-ins, etc.
    ///
    /// Returns (rank_changed, new_rank).
    pub fn modify_reputation(
        &self,
        player_guid: ObjectGuid,
        faction_id: u32,
        delta: i32,
        world: &World,
    ) -> Result<(bool, ReputationRank)> {
        // First apply spillover (if any) before the primary change
        self.apply_spillover(player_guid, faction_id, delta, world)?;

        // Then apply the primary reputation change
        let result = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                let race = player.race;
                let class = player.class;

                // Find the faction standing
                let standing = match player.reputation.get_standing_mut_by_faction_id(faction_id) {
                    Some(s) => s,
                    None => {
                        // Faction not initialized - need to look up from DBC
                        return (false, ReputationRank::Neutral);
                    }
                };

                // Get base reputation from DBC
                let dbc_guard = world.dbc.read();
                let base_rep = dbc_guard
                    .get_faction(faction_id)
                    .map(|e| e.get_base_reputation(race, class))
                    .unwrap_or(0);

                modify_standing(standing, delta, base_rep)
            })
            .unwrap_or((false, ReputationRank::Neutral));

        // Send update packet if reputation changed
        self.send_dirty_reputations(player_guid, world)?;

        Ok(result)
    }

    /// Modify reputation with level-based reduction (for creature kills).
    ///
    /// This applies the level reduction formula before modifying reputation.
    pub fn modify_reputation_with_level(
        &self,
        player_guid: ObjectGuid,
        faction_id: u32,
        base_delta: i32,
        player_level: u8,
        creature_level: u8,
        world: &World,
    ) -> Result<(bool, ReputationRank)> {
        let adjusted_delta = apply_level_reduction(base_delta, player_level, creature_level);
        self.modify_reputation(player_guid, faction_id, adjusted_delta, world)
    }

    /// Apply spillover to allied factions.
    ///
    /// Spillover is applied BEFORE the primary reputation change and is NOT recursive.
    fn apply_spillover(
        &self,
        player_guid: ObjectGuid,
        faction_id: u32,
        delta: i32,
        world: &World,
    ) -> Result<()> {
        // TODO: Implement spillover using DBC or database
        // For now, skip spillover since we don't have ObjectMgr in world
        // This functionality needs to be re-implemented using DBC data or the world database
        let _ = (player_guid, faction_id, delta, world);
        Ok(())
    }

    /// Toggle at-war flag for a faction.
    ///
    /// Players can right-click a faction in the reputation panel to toggle
    /// at-war status. This enforces the rules:
    /// - Cannot toggle if INVISIBLE_FORCED or HIDDEN
    /// - Cannot declare war if PEACE_FORCED (own faction protection)
    /// - Cannot remove at-war if rank is Hostile or below (auto-war)
    pub fn set_at_war(
        &self,
        player_guid: ObjectGuid,
        rep_list_id: ReputationListID,
        at_war: bool,
        world: &World,
    ) -> Result<bool> {
        let changed = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                let standing = match player.reputation.get_standing_mut(rep_list_id) {
                    Some(s) => s,
                    None => return false,
                };

                // Cannot change if invisible or hidden
                if standing.is_invisible_forced() || standing.is_hidden() {
                    return false;
                }

                // Already in desired state
                if standing.is_at_war() == at_war {
                    return false;
                }

                // Cannot declare war if peace forced (own faction)
                if at_war && standing.is_peace_forced() {
                    return false;
                }

                // Cannot remove at-war if rank is Hostile or below
                if !at_war {
                    let dbc_guard = world.dbc.read();
                    let base_rep = dbc_guard
                        .get_faction(standing.faction_id)
                        .map(|e| e.get_base_reputation(player.race, player.class))
                        .unwrap_or(0);
                    let rank = standing.get_rank(base_rep);
                    if rank <= ReputationRank::Hostile {
                        return false;
                    }
                }

                standing.set_at_war(at_war);
                true
            })
            .unwrap_or(false);

        if changed {
            self.send_dirty_reputations(player_guid, world)?;
        }

        Ok(changed)
    }

    /// Toggle inactive flag for a faction.
    ///
    /// Players can collapse factions in the reputation panel by marking
    /// them inactive. Only visible, non-hidden factions can be set inactive.
    pub fn set_inactive(
        &self,
        player_guid: ObjectGuid,
        rep_list_id: ReputationListID,
        inactive: bool,
        world: &World,
    ) -> Result<bool> {
        let changed = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                let standing = match player.reputation.get_standing_mut(rep_list_id) {
                    Some(s) => s,
                    None => return false,
                };

                // Cannot set inactive if forced invisible, hidden, or not visible
                if inactive
                    && (standing.is_invisible_forced()
                        || standing.is_hidden()
                        || !standing.is_visible())
                {
                    return false;
                }

                // Already in desired state
                if standing.is_inactive() == inactive {
                    return false;
                }

                standing.set_inactive(inactive);
                true
            })
            .unwrap_or(false);

        if changed {
            self.send_dirty_reputations(player_guid, world)?;
        }

        Ok(changed)
    }

    /// Send SMSG_INITIALIZE_FACTIONS on login.
    ///
    /// Sends all 64 faction slots to the client. Each slot contains:
    /// - flags (u8): visibility, at-war, inactive state
    /// - absolute_standing (u32): base_rep + standing (what the client displays)
    ///
    /// Empty slots (no faction in that list position) send (0, 0).
    pub fn send_initialize_factions(&self, player_guid: ObjectGuid, world: &World) -> Result<()> {
        let faction_data = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                let race = player.race;
                let class = player.class;
                let dbc_guard = world.dbc.read();

                let mut slots: Vec<(u8, u32)> = Vec::with_capacity(MAX_REPUTATION_LIST_SLOTS);

                for rep_list_id in 0..MAX_REPUTATION_LIST_SLOTS as u32 {
                    if let Some(standing) = player.reputation.get_standing(rep_list_id) {
                        // Look up base rep from FactionEntry
                        let base_rep = dbc_guard
                            .get_faction(standing.faction_id)
                            .map(|e| e.get_base_reputation(race, class))
                            .unwrap_or(0);

                        // ABSOLUTE standing for the packet
                        let absolute = base_rep + standing.standing;
                        slots.push((standing.flags as u8, absolute as u32));
                    } else {
                        slots.push((0, 0)); // Empty slot
                    }
                }

                slots
            })
            .unwrap_or_default();

        // Build and send SMSG_INITIALIZE_FACTIONS
        let msg = SmsgInitializeFactions {
            factions: faction_data
                .into_iter()
                .enumerate()
                .map(|(i, (flags, standing))| (i as u32, (flags, standing as i32)))
                .collect(),
        };

        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

        // Also send forced reactions if any
        self.send_forced_reactions(player_guid, world)?;

        Ok(())
    }

    /// Send SMSG_SET_FACTION_STANDING for all dirty factions.
    ///
    /// Called after any reputation modification. Collects all factions
    /// with need_send=true, builds a single update packet, and clears
    /// the dirty flags.
    fn send_dirty_reputations(&self, player_guid: ObjectGuid, world: &World) -> Result<()> {
        let updates = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                let race = player.race;
                let class = player.class;
                let dbc_guard = world.dbc.read();

                let mut pending: Vec<(u32, i32)> = Vec::new();

                for (rep_list_id, standing) in &mut player.reputation.factions {
                    if !standing.need_send {
                        continue;
                    }

                    let base_rep = dbc_guard
                        .get_faction(standing.faction_id)
                        .map(|e| e.get_base_reputation(race, class))
                        .unwrap_or(0);

                    let absolute = base_rep + standing.standing;
                    pending.push((*rep_list_id, absolute));
                    standing.need_send = false;
                }

                player.reputation.need_send = false;
                pending
            })
            .unwrap_or_default();

        if !updates.is_empty() {
            let msg = SmsgSetFactionStanding { factions: updates };
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
        }

        Ok(())
    }

    /// Send SMSG_SET_FORCED_REACTIONS for all forced reaction entries.
    fn send_forced_reactions(&self, player_guid: ObjectGuid, world: &World) -> Result<()> {
        let reactions = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                player
                    .reputation
                    .forced_reactions
                    .iter()
                    .map(|(faction_id, rank)| (*faction_id, *rank as u32))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !reactions.is_empty() {
            let forced_reactions = reactions.into_iter().collect();
            let msg = SmsgSetForcedReactions { forced_reactions };
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
        }

        Ok(())
    }

    /// Load reputation data from database rows.
    ///
    /// Called during character login after initialize(). Overwrites the
    /// default standings from DBC with saved values from character_reputation.
    pub fn load_from_db(
        &self,
        player_guid: ObjectGuid,
        data: Vec<(u32, i32, i32)>, // (faction_id, standing, flags)
        world: &World,
    ) -> Result<()> {
        world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                let race = player.race;
                let class = player.class;
                let dbc_guard = world.dbc.read();

                for (faction_id, standing, db_flags) in &data {
                    let entry = match dbc_guard.get_faction(*faction_id) {
                        Some(e) if e.reputation_list_id >= 0 => e,
                        _ => continue,
                    };

                    let rep_list_id = entry.reputation_list_id as u32;
                    let state = match player.reputation.get_standing_mut(rep_list_id) {
                        Some(s) => s,
                        None => continue, // Not initialized (shouldn't happen)
                    };

                    // Overwrite standing from DB
                    state.standing = *standing;

                    // Apply flags with validation (matching C++ LoadFromDB)
                    let db_flags_u32 = *db_flags as u32;
                    if (db_flags_u32 & FACTION_FLAG_VISIBLE) != 0 {
                        if !state.is_invisible_forced() && !state.is_hidden() {
                            state.flags |= FACTION_FLAG_VISIBLE;
                        }
                    }

                    if (db_flags_u32 & FACTION_FLAG_AT_WAR) != 0 {
                        if !state.is_invisible_forced() && !state.is_hidden() {
                            state.flags |= FACTION_FLAG_AT_WAR;
                        }
                    } else if state.is_visible() && !state.is_peace_forced() {
                        state.flags &= !FACTION_FLAG_AT_WAR;
                    }

                    // Reset dirty flags if DB state matches
                    if state.flags == db_flags_u32 {
                        state.need_send = false;
                        state.need_save = false;
                    }
                }

                // Post-process: auto-set AT_WAR for hostile factions
                let rep_list_ids: Vec<u32> = player.reputation.factions.keys().copied().collect();

                for rep_list_id in rep_list_ids {
                    let faction_id = player
                        .reputation
                        .get_standing(rep_list_id)
                        .map(|s| s.faction_id)
                        .unwrap_or(0);

                    let base_rep = dbc_guard
                        .get_faction(faction_id)
                        .map(|e| e.get_base_reputation(race, class))
                        .unwrap_or(0);

                    if let Some(standing) = player.reputation.get_standing_mut(rep_list_id) {
                        let rank = standing.get_rank(base_rep);
                        if rank <= ReputationRank::Hostile {
                            standing.flags |= FACTION_FLAG_AT_WAR;
                        }
                    }
                }
            });

        Ok(())
    }

    /// Get all faction data that needs to be saved to the database.
    ///
    /// Returns Vec of (faction_id, standing, flags) for factions with need_save=true.
    pub fn get_save_data(
        &self,
        player_guid: ObjectGuid,
        world: &World,
    ) -> Result<Vec<(u32, i32, u32)>> {
        let data = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                player
                    .reputation
                    .factions
                    .values()
                    .filter(|s| s.need_save)
                    .map(|s| (s.faction_id, s.standing, s.flags))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(data)
    }

    /// Mark all reputations as saved (clears need_save flags).
    pub fn mark_saved(&self, player_guid: ObjectGuid, world: &World) -> Result<()> {
        world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                player.reputation.mark_all_saved();
            });

        Ok(())
    }

    /// Get the reputation rank for a specific faction.
    pub fn get_rank(
        &self,
        player_guid: ObjectGuid,
        faction_id: u32,
        world: &World,
    ) -> Option<ReputationRank> {
        world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                let standing = player.reputation.get_standing_by_faction_id(faction_id)?;
                let dbc_guard = world.dbc.read();
                let base_rep = dbc_guard
                    .get_faction(faction_id)
                    .map(|e| e.get_base_reputation(player.race, player.class))
                    .unwrap_or(0);

                Some(standing.get_rank(base_rep))
            })
            .flatten()
    }
}

/// Modify a faction standing by a delta amount.
///
/// This is a pure function that operates on a single standing.
/// Handles clamping, flag updates, and rank transitions.
///
/// Returns (rank_changed, new_rank).
fn modify_standing(
    standing: &mut FactionStanding,
    delta: i32,
    base_rep: i32,
) -> (bool, ReputationRank) {
    let old_rank = ReputationRank::from_value(base_rep + standing.standing);

    // Apply delta and clamp to absolute limits
    let min_standing = REPUTATION_BOTTOM - base_rep;
    let max_standing = REPUTATION_CAP - base_rep;
    standing.standing = (standing.standing + delta).clamp(min_standing, max_standing);

    let new_rank = ReputationRank::from_value(base_rep + standing.standing);

    // Mark dirty for packet send and database save
    standing.need_send = true;
    standing.need_save = true;

    // Auto-set VISIBLE on first reputation change
    // (unless forced invisible or hidden)
    if !standing.is_invisible_forced() && !standing.is_hidden() {
        if !standing.is_visible() {
            standing.flags |= FACTION_FLAG_VISIBLE;
        }
    }

    // Auto-set AT_WAR when reputation drops to Hostile or below
    if new_rank <= ReputationRank::Hostile && !standing.is_at_war() {
        standing.flags |= FACTION_FLAG_AT_WAR;
    }

    (old_rank != new_rank, new_rank)
}
