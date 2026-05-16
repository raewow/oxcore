//! WaypointRepository - loads waypoint data from database
//!
//! MaNGOS stores waypoints in three tables:
//! - creature_movement: Per-GUID waypoints (FromGuid)
//! - creature_movement_template: Per-entry waypoints (FromEntry)

use super::generators::Waypoint;
use crate::shared::protocol::Position;
use sqlx::MySqlPool;
use std::collections::HashMap;

/// Repository for loading waypoint data from database
pub struct WaypointRepository {
    pool: MySqlPool,
}

impl WaypointRepository {
    pub fn new(pool: MySqlPool) -> Self {
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
            r#"SELECT id, point, position_x, position_y, position_z, orientation, waittime
               FROM creature_movement
               ORDER BY id, point"#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(Self::group_waypoints(rows))
    }

    /// Load per-entry waypoints (creature_movement_template table)
    async fn load_template_waypoints(&self) -> anyhow::Result<HashMap<u32, Vec<Waypoint>>> {
        let rows = sqlx::query_as::<_, WaypointRow>(
            r#"SELECT entry as id, point, position_x, position_y, position_z, orientation, waittime
               FROM creature_movement_template
               ORDER BY entry, point"#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(Self::group_waypoints(rows))
    }

    /// Group waypoint rows by ID
    fn group_waypoints(rows: Vec<WaypointRow>) -> HashMap<u32, Vec<Waypoint>> {
        let mut grouped: HashMap<u32, Vec<Waypoint>> = HashMap::new();

        for row in rows {
            grouped.entry(row.id).or_default().push(Waypoint {
                point_id: row.point,
                position: Position {
                    x: row.position_x,
                    y: row.position_y,
                    z: row.position_z,
                    o: row.orientation.unwrap_or(0.0),
                },
                wait_time: row.waittime.unwrap_or(0),
                script_id: 0,
                orientation: row.orientation,
            });
        }

        grouped
    }
}

/// Waypoint data loaded from database
pub struct WaypointData {
    pub guid_waypoints: HashMap<u32, Vec<Waypoint>>,
    pub template_waypoints: HashMap<u32, Vec<Waypoint>>,
}

#[derive(sqlx::FromRow)]
struct WaypointRow {
    id: u32,
    point: u32,
    position_x: f32,
    position_y: f32,
    position_z: f32,
    orientation: Option<f32>,
    waittime: Option<u32>,
}
