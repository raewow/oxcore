//! Graveyard selection system
//!
//! Graveyard selection determines where the ghost spawns after releasing spirit.
//! The system loads zone-to-graveyard mappings from the `game_graveyard_zone`
//! database table and resolves coordinates from `WorldSafeLocs.dbc`.

use crate::dbc::DbcManager;
use anyhow::Result;
use oxcore_db::database::world::repositories::GraveyardRepository;
use oxcore_shared::protocol::Position;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

/// Team constants for graveyard filtering.
/// These match the faction IDs stored in game_graveyard_zone.
pub const FACTION_NONE: u16 = 0; // Any team
pub const FACTION_HORDE: u16 = 67;
pub const FACTION_ALLIANCE: u16 = 469;

/// Internal team representation used in graveyard lookups.
pub const TEAM_ALLIANCE: u8 = 0;
pub const TEAM_HORDE: u8 = 1;
pub const TEAM_BOTH: u8 = 2;

/// Graveyard data linking a WorldSafeLocs entry to a zone and faction.
#[derive(Debug, Clone)]
pub struct GraveyardData {
    /// WorldSafeLocs.dbc entry ID.
    pub safe_loc_id: u32,
    /// Zone or area ID this graveyard serves.
    pub zone_id: u32,
    /// Team filter: 0 = Alliance, 1 = Horde, 2 = Both.
    pub team: u8,
    /// World position from WorldSafeLocs.dbc.
    pub position: Position,
    /// Map ID from WorldSafeLocs.dbc.
    pub map_id: u32,
}

/// Find the closest graveyard for a dead player.
///
/// Algorithm:
/// 1. Look up all graveyards linked to the player's current area ID
/// 2. If none found, fall back to the parent zone ID
/// 3. Filter by team (Alliance/Horde) - graveyards marked "Both" always pass
/// 4. For same-map graveyards: compute 3D Euclidean distance
/// 5. For cross-map graveyards: compute 2D distance (Z is unreliable across maps)
/// 6. Return the closest match
///
/// If no graveyard is found at all, returns None and the caller should
/// fall back to the player's homebind location.
pub fn find_closest_graveyard(
    graveyards_by_zone: &std::collections::HashMap<u32, Vec<GraveyardData>>,
    x: f32,
    y: f32,
    z: f32,
    map_id: u32,
    zone_id: u32,
    area_id: u32,
    team: u8,
) -> Option<GraveyardData> {
    let mut candidates = Vec::new();

    // Prefer area-specific graveyards (more precise)
    if let Some(area_gys) = graveyards_by_zone.get(&area_id) {
        candidates.extend(area_gys.iter().cloned());
    }

    // Fall back to zone-level graveyards
    if candidates.is_empty() {
        if let Some(zone_gys) = graveyards_by_zone.get(&zone_id) {
            candidates.extend(zone_gys.iter().cloned());
        }
    }

    // Filter by team: allow matching team or "Both" (TEAM_BOTH = 2)
    let filtered: Vec<GraveyardData> = candidates
        .into_iter()
        .filter(|gy| gy.team == TEAM_BOTH || gy.team == team)
        .collect();

    if filtered.is_empty() {
        return None;
    }

    pick_closest(filtered, x, y, z, map_id)
}

/// Pick the nearest graveyard from a candidate list.
///
/// For same-map graveyards the full 3D distance is used; for cross-map graveyards
/// the Z coordinate is unreliable, so only 2D distance is compared.
fn pick_closest(
    candidates: Vec<GraveyardData>,
    x: f32,
    y: f32,
    z: f32,
    map_id: u32,
) -> Option<GraveyardData> {
    let mut closest: Option<GraveyardData> = None;
    let mut closest_dist_sq = f32::INFINITY;

    for gy in candidates {
        let dx = gy.position.x - x;
        let dy = gy.position.y - y;
        let dist_sq = if gy.map_id == map_id {
            // Same map: use full 3D distance
            let dz = gy.position.z - z;
            dx * dx + dy * dy + dz * dz
        } else {
            // Cross-map: use 2D distance only
            dx * dx + dy * dy
        };

        if dist_sq < closest_dist_sq {
            closest_dist_sq = dist_sq;
            closest = Some(gy);
        }
    }

    closest
}

/// Determine team from player race.
/// Used when looking up faction-filtered graveyards.
pub fn team_from_race(race: u8) -> u8 {
    match race {
        // Alliance: Human(1), Dwarf(3), Night Elf(4), Gnome(7)
        1 | 3 | 4 | 7 => TEAM_ALLIANCE,
        // Horde: Orc(2), Undead(5), Tauren(6), Troll(8)
        2 | 5 | 6 | 8 => TEAM_HORDE,
        _ => TEAM_BOTH,
    }
}

/// Convert database faction value to internal team constant.
fn faction_to_team(faction: u16) -> u8 {
    match faction {
        FACTION_ALLIANCE => TEAM_ALLIANCE,
        FACTION_HORDE => TEAM_HORDE,
        _ => TEAM_BOTH, // 0 or any other value = both factions
    }
}

/// Manages graveyard data loaded from DB + DBC.
///
/// Loaded once at startup and used by the death system to find the
/// nearest graveyard when a player releases spirit.
pub struct GraveyardManager {
    /// Zone/area ID -> list of graveyards serving that zone
    graveyards_by_zone: HashMap<u32, Vec<GraveyardData>>,
}

impl GraveyardManager {
    /// Create an empty manager (call `load()` to populate)
    pub fn new() -> Self {
        Self {
            graveyards_by_zone: HashMap::new(),
        }
    }

    /// Load graveyard data from the `game_graveyard_zone` DB table,
    /// cross-referencing coordinates from WorldSafeLocs.dbc.
    pub async fn load(
        &mut self,
        world_pool: Arc<sqlx::PgPool>,
        dbc_mgr: &DbcManager,
    ) -> Result<()> {
        let repo = GraveyardRepository::new(world_pool);
        let rows = repo.load_graveyard_zones().await?;

        let mut loaded = 0u32;
        let mut skipped = 0u32;

        for row in &rows {
            // Look up coordinates from WorldSafeLocs.dbc
            let safe_loc_id = u32::try_from(row.id)?;
            let zone_id = u32::try_from(row.ghost_zone)?;
            let faction = u16::try_from(row.faction)?;
            let safe_loc = match dbc_mgr.get_world_safe_locs(safe_loc_id) {
                Some(loc) => loc,
                None => {
                    skipped += 1;
                    continue;
                }
            };

            let data = GraveyardData {
                safe_loc_id,
                zone_id,
                team: faction_to_team(faction),
                position: Position {
                    x: safe_loc.x,
                    y: safe_loc.y,
                    z: safe_loc.z,
                    o: 0.0,
                },
                map_id: safe_loc.map_id,
            };

            self.graveyards_by_zone
                .entry(zone_id)
                .or_default()
                .push(data);
            loaded += 1;
        }

        if skipped > 0 {
            warn!(
                "Skipped {} graveyard zone entries (missing WorldSafeLocs.dbc reference)",
                skipped
            );
        }
        info!(
            "Loaded {} graveyard zone entries across {} zones",
            loaded,
            self.graveyards_by_zone.len()
        );
        Ok(())
    }

    /// Find the closest graveyard for a player at the given position.
    ///
    /// The zone/area-scoped lookup runs first. When the player's zone has no graveyard
    /// mapping (custom areas, missing DBC, ...) it falls back to the nearest graveyard
    /// anywhere — preferring the same map — rather than reporting no result and letting
    /// the caller dump the ghost at its homebind (which is usually the death spot).
    pub fn get_closest_graveyard(
        &self,
        x: f32,
        y: f32,
        z: f32,
        map_id: u32,
        zone_id: u32,
        area_id: u32,
        team: u8,
    ) -> Option<GraveyardData> {
        if let Some(gy) = find_closest_graveyard(
            &self.graveyards_by_zone,
            x,
            y,
            z,
            map_id,
            zone_id,
            area_id,
            team,
        ) {
            return Some(gy);
        }

        let all_candidates: Vec<GraveyardData> = self
            .graveyards_by_zone
            .values()
            .flatten()
            .filter(|gy| gy.team == TEAM_BOTH || gy.team == team)
            .cloned()
            .collect();

        pick_closest(all_candidates, x, y, z, map_id)
    }

    /// Check if any graveyard data has been loaded
    pub fn is_loaded(&self) -> bool {
        !self.graveyards_by_zone.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gy(safe_loc_id: u32, zone_id: u32, team: u8, x: f32, y: f32, z: f32) -> GraveyardData {
        GraveyardData {
            safe_loc_id,
            zone_id,
            team,
            position: Position { x, y, z, o: 0.0 },
            map_id: 0,
        }
    }

    fn manager_with(mut zones: HashMap<u32, Vec<GraveyardData>>) -> GraveyardManager {
        GraveyardManager {
            graveyards_by_zone: zones,
        }
    }

    #[test]
    fn zone_scoped_lookup_picks_nearest_in_zone() {
        let mut mapping = HashMap::new();
        mapping.insert(
            85, // Orgrimmar
            vec![
                gy(1, 85, TEAM_BOTH, 1600.0, -4400.0, 30.0),
                gy(2, 85, TEAM_BOTH, 0.0, -4700.0, 40.0),
            ],
        );
        let mgr = manager_with(mapping);

        let found = mgr
            .get_closest_graveyard(1700.0, -4450.0, 25.0, 0, 85, 85, TEAM_HORDE)
            .unwrap();
        assert_eq!(found.safe_loc_id, 1, "nearest in-zone graveyard wins");
    }

    #[test]
    fn unmapped_zone_falls_back_to_any_graveyard() {
        let mut mapping = HashMap::new();
        mapping.insert(85, vec![gy(1, 85, TEAM_BOTH, 1600.0, -4400.0, 30.0)]);
        let mgr = manager_with(mapping);

        // Death in zone 999, no mapping — must not return None, must return the
        // only loaded graveyard rather than dumping the ghost at homebind.
        let found = mgr
            .get_closest_graveyard(2000.0, -4000.0, 0.0, 0, 999, 999, TEAM_HORDE)
            .unwrap();
        assert_eq!(found.safe_loc_id, 1);
    }

    #[test]
    fn team_filtering_excludes_opposing_faction_graveyards() {
        let mut mapping = HashMap::new();
        mapping.insert(85, vec![gy(1, 85, TEAM_ALLIANCE, 1600.0, -4400.0, 30.0)]);
        let mgr = manager_with(mapping);

        // A horde player must not be sent to an alliance-only graveyard, even alone.
        assert!(mgr
            .get_closest_graveyard(2000.0, -4000.0, 10.0, 0, 85, 85, TEAM_HORDE)
            .is_none());
    }

    #[test]
    fn team_both_passes_through_any_faction_filter() {
        let mut mapping = HashMap::new();
        mapping.insert(85, vec![gy(1, 85, TEAM_BOTH, 1600.0, -4400.0, 30.0)]);
        let mgr = manager_with(mapping);

        let found = mgr
            .get_closest_graveyard(2000.0, -4000.0, 10.0, 0, 85, 85, TEAM_HORDE)
            .unwrap();
        assert_eq!(found.safe_loc_id, 1);
    }
}
