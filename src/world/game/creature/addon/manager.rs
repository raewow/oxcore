use super::addon::CreatureAddon;
use super::repository::AddonData;
use dashmap::DashMap;
use std::sync::Arc;

/// Manages creature addons - state only, no database
pub struct AddonManager {
    /// Addons by spawn GUID (highest priority)
    guid_addons: DashMap<u32, Arc<CreatureAddon>>,
    /// Addons by entry ID (template defaults)
    template_addons: DashMap<u32, Arc<CreatureAddon>>,
}

impl AddonManager {
    pub fn new() -> Self {
        Self {
            guid_addons: DashMap::new(),
            template_addons: DashMap::new(),
        }
    }

    /// Load addons from repository data
    pub fn load_from_repository(&self, data: AddonData) {
        for (guid, addon) in data.guid_addons {
            self.guid_addons.insert(guid, Arc::new(addon));
        }

        for (entry, addon) in data.template_addons {
            self.template_addons.insert(entry, Arc::new(addon));
        }

        tracing::info!(
            "AddonManager loaded {} GUID addons, {} template addons",
            self.guid_addons.len(),
            self.template_addons.len()
        );
    }

    /// Get addon for a creature (GUID addon takes priority)
    pub fn get_addon(&self, spawn_id: u32, entry: u32) -> Option<Arc<CreatureAddon>> {
        // Try GUID-specific first
        if let Some(addon) = self.guid_addons.get(&spawn_id) {
            return Some(Arc::clone(&addon));
        }

        // Fall back to template addon
        self.template_addons.get(&entry).map(|a| Arc::clone(&a))
    }
}

impl Default for AddonManager {
    fn default() -> Self {
        Self::new()
    }
}
