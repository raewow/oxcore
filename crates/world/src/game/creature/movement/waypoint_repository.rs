//! WaypointRepository - loads waypoint data from database
//!
//! Waypoints are stored in two tables:
//! - creature_movement: Per-GUID waypoints (FromGuid)
//! - creature_movement_template: Per-entry waypoints (FromEntry)

use super::generators::Waypoint;
use super::waypoint_manager;
use anyhow::Context;
use oxcore_shared::protocol::Position;
use sqlx::PgPool;
use std::collections::HashMap;

/// Repository for loading waypoint data from database
pub struct WaypointRepository {
    pool: PgPool,
}

impl WaypointRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Load all waypoints from database
    pub async fn load_all(&self) -> anyhow::Result<WaypointData> {
        let guid_waypoints = self.load_guid_waypoints().await?;
        let template_waypoints = self.load_template_waypoints().await?;

        tracing::debug!(
            "Loaded {} GUID waypoint paths, {} template waypoint paths",
            guid_waypoints.len(),
            template_waypoints.len()
        );

        Ok(WaypointData {
            guid_waypoints,
            template_waypoints,
        })
    }

    /// Load per-GUID waypoints (creature_movement table)
    async fn load_guid_waypoints(&self) -> anyhow::Result<HashMap<u32, Vec<Waypoint>>> {
        let rows = sqlx::query_as::<_, WaypointRow>(
            r#"SELECT id, point, position_x, position_y, position_z, orientation, waittime, wander_distance, script_id
                FROM world.creature_movement
                ORDER BY id, point"#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(Self::group_waypoints(rows)?)
    }

    /// Load per-entry waypoints (creature_movement_template table)
    async fn load_template_waypoints(&self) -> anyhow::Result<HashMap<u32, Vec<Waypoint>>> {
        let rows = sqlx::query_as::<_, WaypointRow>(
            r#"SELECT entry as id, point, position_x, position_y, position_z, orientation, waittime, wander_distance, script_id
                FROM world.creature_movement_template
                ORDER BY entry, point"#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(Self::group_waypoints(rows)?)
    }

    /// Group waypoint rows by ID
    fn group_waypoints(rows: Vec<WaypointRow>) -> anyhow::Result<HashMap<u32, Vec<Waypoint>>> {
        let mut grouped: HashMap<u32, Vec<Waypoint>> = HashMap::new();

        for row in rows {
            grouped
                .entry(u32::try_from(row.id).context("waypoint id is outside u32 range")?)
                .or_default()
                .push(Self::build_waypoint(row)?);
        }

        Ok(grouped)
    }

    /// Convert one DB row into a waypoint, normalizing bad data.
    fn build_waypoint(row: WaypointRow) -> anyhow::Result<Waypoint> {
        // The DB stores 100 to mean "no orientation override at this node".
        let orientation = row
            .orientation
            .filter(|value| *value != waypoint_manager::NO_ORIENTATION);

        let mut x = row.position_x;
        let mut y = row.position_y;
        let z = row.position_z;

        if !waypoint_manager::is_valid_map_coord(x, y, z, orientation.unwrap_or(0.0)) {
            tracing::error!(
                "Waypoint path {} point {} has invalid coordinates (X: {}, Y: {})",
                row.id,
                row.point,
                x,
                y
            );
            x = waypoint_manager::normalize_map_coord(x);
            y = waypoint_manager::normalize_map_coord(y);
        }

        Ok(Waypoint {
            point_id: u32::try_from(row.point).context("waypoint point is outside u32 range")?,
            position: Position {
                x,
                y,
                z,
                o: orientation.unwrap_or(0.0),
            },
            wait_time: row
                .waittime
                .map(|value| u32::try_from(value).context("waypoint waittime is outside u32 range"))
                .transpose()?
                .unwrap_or(0),
            wander_distance: row.wander_distance.unwrap_or(0.0),
            script_id: row
                .script_id
                .map(|value| {
                    u32::try_from(value).context("waypoint script_id is outside u32 range")
                })
                .transpose()?
                .unwrap_or(0),
            orientation,
        })
    }
}

/// Waypoint data loaded from database
pub struct WaypointData {
    pub guid_waypoints: HashMap<u32, Vec<Waypoint>>,
    pub template_waypoints: HashMap<u32, Vec<Waypoint>>,
}

#[derive(sqlx::FromRow)]
struct WaypointRow {
    id: i64,
    point: i64,
    position_x: f32,
    position_y: f32,
    position_z: f32,
    orientation: Option<f32>,
    waittime: Option<i64>,
    wander_distance: Option<f32>,
    script_id: Option<i64>,
}
