//! Area trigger system
//!
//! Handles area triggers for zone transitions, teleports, quest completion, and taverns.
//! Area triggers are invisible zones that trigger events when players enter them.

use anyhow::Result;
use dashmap::DashMap;
use sqlx::PgPool;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{debug, warn};

use oxcore_shared::protocol::Position;

/// Area trigger entry from the `areatrigger_template` DB table.
/// Defines the trigger zone geometry.
#[derive(Debug, Clone)]
pub struct AreaTriggerEntry {
    pub id: u32,
    pub map_id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub radius: f32,
    pub box_x: f32,
    pub box_y: f32,
    pub box_z: f32,
    pub box_orientation: f32,
}

/// Area trigger teleport destination from the `areatrigger_teleport` DB table.
#[derive(Debug, Clone)]
pub struct AreaTriggerTeleport {
    pub message: String,
    pub required_level: u8,
    pub required_condition: u32,
    pub destination_map: u32,
    pub destination: Position,
}

/// Manages area trigger data loaded from the database at startup.
pub struct AreaTriggerManager {
    world_db: Arc<PgPool>,
    /// Trigger ID -> AreaTriggerEntry (from areatrigger_template)
    templates: DashMap<u32, AreaTriggerEntry>,
    /// Trigger ID -> teleport destination (from areatrigger_teleport)
    teleports: DashMap<u32, AreaTriggerTeleport>,
    /// Trigger IDs that are taverns/inns (from areatrigger_tavern)
    taverns: parking_lot::RwLock<HashSet<u32>>,
    /// Trigger ID -> quest ID (from areatrigger_involvedrelation)
    quest_triggers: DashMap<u32, u32>,
}

impl AreaTriggerManager {
    pub fn new(world_db: Arc<PgPool>) -> Self {
        Self {
            world_db,
            templates: DashMap::new(),
            teleports: DashMap::new(),
            taverns: parking_lot::RwLock::new(HashSet::new()),
            quest_triggers: DashMap::new(),
        }
    }

    /// Load all area trigger data from the database.
    pub async fn load(&self) -> Result<()> {
        self.load_templates().await?;
        self.load_teleports().await?;
        self.load_taverns().await?;
        self.load_quest_triggers().await?;
        Ok(())
    }

    async fn load_templates(&self) -> Result<()> {
        let query_str = r#"SELECT id, name, map_id, x, y, z, radius,
                           box_x, box_y, box_z, box_orientation
                            FROM world.areatrigger_template"#;

        let rows = match sqlx::query_as::<_, AreaTriggerTemplateRow>(query_str)
            .fetch_all(self.world_db.as_ref())
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                warn!(
                    "Failed to query areatrigger_template: {}. Area triggers will not work.",
                    e
                );
                return Ok(());
            }
        };

        self.templates.clear();
        let mut count = 0u32;

        for row in rows {
            let entry = row.into_entry()?;
            let trigger_id = entry.id;

            self.templates.insert(trigger_id, entry);
            count += 1;
        }

        debug!("Loaded {} area triggers", count);
        Ok(())
    }

    async fn load_teleports(&self) -> Result<()> {
        let query_str = r#"SELECT id, required_level, required_condition, message,
                           target_map, target_position_x, target_position_y,
                           target_position_z, target_orientation
                            FROM world.areatrigger_teleport"#;

        let rows = match sqlx::query_as::<_, AreaTriggerTeleportRow>(query_str)
            .fetch_all(self.world_db.as_ref())
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                warn!(
                    "Failed to query areatrigger_teleport: {}. Teleport triggers will not work.",
                    e
                );
                return Ok(());
            }
        };

        self.teleports.clear();
        let mut count = 0u32;

        for row in rows {
            let (trigger_id, teleport) = row.into_teleport()?;
            let target_x = teleport.destination.x;
            let target_y = teleport.destination.y;
            let target_z = teleport.destination.z;

            if target_x == 0.0 && target_y == 0.0 && target_z == 0.0 {
                warn!(
                    "areatrigger_teleport ID {} has no target coordinates, skipping",
                    trigger_id
                );
                continue;
            }

            self.teleports.insert(trigger_id, teleport);
            count += 1;
        }

        debug!("Loaded {} area trigger teleports", count);
        Ok(())
    }

    async fn load_taverns(&self) -> Result<()> {
        let current_patch: i16 = 10; // WOW_PATCH_112
        let query_str = r#"SELECT id FROM world.areatrigger_tavern WHERE patch_min <= $1"#;

        let rows = match sqlx::query_scalar::<sqlx::Postgres, i64>(query_str)
            .bind(current_patch)
            .fetch_all(self.world_db.as_ref())
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                warn!(
                    "Failed to query areatrigger_tavern: {}. Tavern triggers will not work.",
                    e
                );
                return Ok(());
            }
        };

        let mut taverns = self.taverns.write();
        taverns.clear();
        let mut count = 0u32;

        for row in rows {
            let trigger_id = checked_u32(row, "areatrigger_tavern.id")?;
            taverns.insert(trigger_id);
            count += 1;
        }

        debug!("Loaded {} tavern area triggers", count);
        Ok(())
    }

    async fn load_quest_triggers(&self) -> Result<()> {
        let query_str = r#"SELECT id, quest FROM world.areatrigger_involvedrelation"#;

        let rows = match sqlx::query_as::<_, AreaTriggerQuestRow>(query_str)
            .fetch_all(self.world_db.as_ref())
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                warn!("Failed to query areatrigger_involvedrelation: {}. Quest triggers will not work.", e);
                return Ok(());
            }
        };

        self.quest_triggers.clear();
        let mut count = 0u32;

        for row in rows {
            let trigger_id = checked_u32(row.id, "areatrigger_involvedrelation.id")?;
            let quest_id = checked_u32(row.quest, "areatrigger_involvedrelation.quest")?;
            self.quest_triggers.insert(trigger_id, quest_id);
            count += 1;
        }

        debug!("Loaded {} quest area triggers", count);
        Ok(())
    }

    // --- Accessor methods ---

    /// Get an area trigger template by ID.
    pub fn get_template(&self, trigger_id: u32) -> Option<AreaTriggerEntry> {
        self.templates.get(&trigger_id).map(|r| r.clone())
    }

    /// Get a teleport destination for a trigger.
    pub fn get_teleport(&self, trigger_id: u32) -> Option<AreaTriggerTeleport> {
        self.teleports.get(&trigger_id).map(|r| r.clone())
    }

    /// Check if a trigger is a tavern/inn.
    pub fn is_tavern(&self, trigger_id: u32) -> bool {
        self.taverns.read().contains(&trigger_id)
    }

    /// Get the quest ID associated with a trigger (exploration objective).
    pub fn get_quest_for_trigger(&self, trigger_id: u32) -> Option<u32> {
        self.quest_triggers.get(&trigger_id).map(|r| *r)
    }

    /// Get the map entrance trigger (teleports players into the given map).
    pub fn get_map_entrance_trigger(&self, map_id: u32) -> Option<AreaTriggerTeleport> {
        for entry in self.teleports.iter() {
            if entry.value().destination_map == map_id {
                return Some(entry.value().clone());
            }
        }
        None
    }
}

#[derive(sqlx::FromRow)]
struct AreaTriggerTemplateRow {
    id: i64,
    name: String,
    map_id: i64,
    x: f32,
    y: f32,
    z: f32,
    radius: f32,
    box_x: f32,
    box_y: f32,
    box_z: f32,
    box_orientation: f32,
}

impl AreaTriggerTemplateRow {
    fn into_entry(self) -> Result<AreaTriggerEntry> {
        Ok(AreaTriggerEntry {
            id: checked_u32(self.id, "areatrigger_template.id")?,
            map_id: checked_u32(self.map_id, "areatrigger_template.map_id")?,
            name: self.name,
            x: self.x,
            y: self.y,
            z: self.z,
            radius: self.radius,
            box_x: self.box_x,
            box_y: self.box_y,
            box_z: self.box_z,
            box_orientation: self.box_orientation,
        })
    }
}

#[derive(sqlx::FromRow)]
struct AreaTriggerTeleportRow {
    id: i64,
    required_level: i64,
    required_condition: i64,
    message: String,
    target_map: i64,
    target_position_x: f32,
    target_position_y: f32,
    target_position_z: f32,
    target_orientation: f32,
}

impl AreaTriggerTeleportRow {
    fn into_teleport(self) -> Result<(u32, AreaTriggerTeleport)> {
        Ok((
            checked_u32(self.id, "areatrigger_teleport.id")?,
            AreaTriggerTeleport {
                message: self.message,
                required_level: u8::try_from(self.required_level).map_err(|_| {
                    anyhow::anyhow!("areatrigger_teleport.required_level must fit u8")
                })?,
                required_condition: checked_u32(
                    self.required_condition,
                    "areatrigger_teleport.required_condition",
                )?,
                destination_map: checked_u32(self.target_map, "areatrigger_teleport.target_map")?,
                destination: Position::new(
                    self.target_position_x,
                    self.target_position_y,
                    self.target_position_z,
                    self.target_orientation,
                ),
            },
        ))
    }
}

#[derive(sqlx::FromRow)]
struct AreaTriggerQuestRow {
    id: i64,
    quest: i64,
}

fn checked_u32(value: i64, field: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| anyhow::anyhow!("{field} must fit u32"))
}

/// Check if a point is within an area trigger zone.
///
/// Works with both sphere triggers (radius > 0) and box triggers.
/// The `delta` parameter adds tolerance (e.g. 5 yards for anti-cheat).
///
pub fn is_point_in_area_trigger_zone(
    trigger: &AreaTriggerEntry,
    map_id: u32,
    x: f32,
    y: f32,
    z: f32,
    delta: f32,
) -> bool {
    if map_id != trigger.map_id {
        return false;
    }

    if trigger.radius > 0.0 {
        // Sphere trigger
        let dist_sq = (x - trigger.x) * (x - trigger.x)
            + (y - trigger.y) * (y - trigger.y)
            + (z - trigger.z) * (z - trigger.z);

        let radius_with_delta = trigger.radius + delta;
        if dist_sq > radius_with_delta * radius_with_delta {
            return false;
        }
    } else {
        // Box trigger - rotate player into box's local coordinate space
        let rotation = 2.0 * std::f32::consts::PI - trigger.box_orientation;
        let sin_val = rotation.sin();
        let cos_val = rotation.cos();

        let player_box_dist_x = x - trigger.x;
        let player_box_dist_y = y - trigger.y;

        let dx = player_box_dist_x * cos_val - player_box_dist_y * sin_val;
        let dy = player_box_dist_y * cos_val + player_box_dist_x * sin_val;
        let dz = z - trigger.z;

        if dx.abs() > trigger.box_x / 2.0 + delta
            || dy.abs() > trigger.box_y / 2.0 + delta
            || dz.abs() > trigger.box_z / 2.0 + delta
        {
            return false;
        }
    }

    true
}

/// Convert a DBC AreaTriggerEntry into our local AreaTriggerEntry.
pub fn from_dbc_entry(dbc: &crate::dbc::structures::AreaTriggerEntry) -> AreaTriggerEntry {
    AreaTriggerEntry {
        id: dbc.id,
        map_id: dbc.map_id,
        name: String::new(),
        x: dbc.x,
        y: dbc.y,
        z: dbc.z,
        radius: dbc.radius,
        box_x: dbc.box_x,
        box_y: dbc.box_y,
        box_z: dbc.box_z,
        box_orientation: dbc.box_orientation,
    }
}
