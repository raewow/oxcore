//! Graveyard selection system
//!
//! Graveyard selection determines where the ghost spawns after releasing spirit.
//! The system loads zone-to-graveyard mappings from the `game_graveyard_zone`
//! database table and resolves coordinates from `WorldSafeLocs.dbc`.

use crate::shared::database::world::repositories::GraveyardRepository;
use crate::shared::protocol::Position;
use crate::world::dbc::DbcManager;
use anyhow::Result;
use sqlx::MySqlPool;
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

    // Find closest by distance
    let mut closest: Option<GraveyardData> = None;
    let mut closest_dist_sq = f32::INFINITY;

    for gy in filtered {
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
    pub async fn load(&mut self, world_pool: Arc<MySqlPool>, dbc_mgr: &DbcManager) -> Result<()> {
        let repo = GraveyardRepository::new(world_pool);
        let rows = repo.load_graveyard_zones().await?;

        let mut loaded = 0u32;
        let mut skipped = 0u32;

        for row in &rows {
            // Look up coordinates from WorldSafeLocs.dbc
            let safe_loc = match dbc_mgr.get_world_safe_locs(row.id) {
                Some(loc) => loc,
                None => {
                    skipped += 1;
                    continue;
                }
            };

            let data = GraveyardData {
                safe_loc_id: row.id,
                zone_id: row.ghost_zone,
                team: faction_to_team(row.faction),
                position: Position {
                    x: safe_loc.x,
                    y: safe_loc.y,
                    z: safe_loc.z,
                    o: 0.0,
                },
                map_id: safe_loc.map_id,
            };

            self.graveyards_by_zone
                .entry(row.ghost_zone)
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
        find_closest_graveyard(
            &self.graveyards_by_zone,
            x,
            y,
            z,
            map_id,
            zone_id,
            area_id,
            team,
        )
    }

    /// Check if any graveyard data has been loaded
    pub fn is_loaded(&self) -> bool {
        !self.graveyards_by_zone.is_empty()
    }
}
