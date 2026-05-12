//! WaypointManager - manages waypoint state, no database access
//!
//! Stores loaded waypoint data and provides lookup by spawn GUID or entry.
//! Follows the Manager/Repository separation pattern.

use super::generators::Waypoint;
use super::waypoint_repository::WaypointData;
use dashmap::DashMap;
use std::sync::Arc;

/// Waypoint data manager - state only, no database
pub struct WaypointManager {
    /// Waypoints by creature spawn GUID
    guid_waypoints: DashMap<u32, Arc<Vec<Waypoint>>>,
    /// Waypoints by creature entry (template)
    template_waypoints: DashMap<u32, Arc<Vec<Waypoint>>>,
}

impl WaypointManager {
    pub fn new() -> Self {
        Self {
            guid_waypoints: DashMap::new(),
            template_waypoints: DashMap::new(),
        }
    }

    /// Load waypoints from repository data
    pub fn load_from_data(&self, data: WaypointData) {
        for (id, waypoints) in data.guid_waypoints {
            self.guid_waypoints.insert(id, Arc::new(waypoints));
        }

        for (entry, waypoints) in data.template_waypoints {
            self.template_waypoints.insert(entry, Arc::new(waypoints));
        }

        tracing::debug!(
            "WaypointManager loaded {} GUID paths, {} template paths",
            self.guid_waypoints.len(),
            self.template_waypoints.len()
        );
    }

    /// Get waypoints for a creature spawn (checks GUID first, then entry)
    pub fn get_waypoints(&self, spawn_id: u32, entry: u32) -> Option<Arc<Vec<Waypoint>>> {
        // Try per-GUID waypoints first (FromGuid)
        if let Some(waypoints) = self.guid_waypoints.get(&spawn_id) {
            return Some(Arc::clone(&waypoints));
        }

        // Fall back to template waypoints (FromEntry)
        self.template_waypoints.get(&entry).map(|w| Arc::clone(&w))
    }

    /// Check if a creature has waypoints defined
    pub fn has_waypoints(&self, spawn_id: u32, entry: u32) -> bool {
        self.guid_waypoints.contains_key(&spawn_id) || self.template_waypoints.contains_key(&entry)
    }
}

impl Default for WaypointManager {
    fn default() -> Self {
        Self::new()
    }
}
