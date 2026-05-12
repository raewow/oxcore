use crate::shared::protocol::ObjectGuid;
use dashmap::DashMap;
use super::link_flags::{LinkFlags, LinkEvent};
use super::linking_repository::CreatureLinkRow;

/// A link between two creatures
#[derive(Debug, Clone)]
pub struct CreatureLink {
    /// Master creature spawn GUID
    pub master_guid: u32,
    /// Slave creature spawn GUID
    pub slave_guid: u32,
    /// Link behavior flags
    pub flags: LinkFlags,
}

/// Manages creature links - state only, no database
pub struct LinkingManager {
    /// Links by master spawn GUID
    links_by_master: DashMap<u32, Vec<CreatureLink>>,
    /// Links by slave spawn GUID (for reverse lookups)
    links_by_slave: DashMap<u32, Vec<CreatureLink>>,
    /// Spawn GUID -> runtime GUID mapping
    spawn_to_runtime: DashMap<u32, ObjectGuid>,
}

impl LinkingManager {
    pub fn new() -> Self {
        Self {
            links_by_master: DashMap::new(),
            links_by_slave: DashMap::new(),
            spawn_to_runtime: DashMap::new(),
        }
    }

    /// Load links from repository data
    pub fn load_from_repository(&self, links: Vec<CreatureLinkRow>) {
        for row in links {
            let link = CreatureLink {
                master_guid: row.master_guid,
                slave_guid: row.slave_guid,
                flags: row.flags,
            };

            self.links_by_master
                .entry(row.master_guid)
                .or_insert_with(Vec::new)
                .push(link.clone());

            self.links_by_slave
                .entry(row.slave_guid)
                .or_insert_with(Vec::new)
                .push(link);
        }

        tracing::info!(
            "LinkingManager loaded {} masters, {} slaves",
            self.links_by_master.len(),
            self.links_by_slave.len()
        );
    }

    /// Register a spawned creature
    pub fn register_creature(&self, spawn_id: u32, runtime_guid: ObjectGuid) {
        self.spawn_to_runtime.insert(spawn_id, runtime_guid);
    }

    /// Unregister a despawned creature
    pub fn unregister_creature(&self, spawn_id: u32) {
        self.spawn_to_runtime.remove(&spawn_id);
    }

    /// Get runtime GUID from spawn ID
    pub fn get_runtime_guid(&self, spawn_id: u32) -> Option<ObjectGuid> {
        self.spawn_to_runtime.get(&spawn_id).map(|r| *r)
    }

    /// Get slaves for a master
    pub fn get_slaves(&self, master_spawn_id: u32) -> Vec<(ObjectGuid, LinkFlags)> {
        self.links_by_master
            .get(&master_spawn_id)
            .map(|links| {
                links
                    .iter()
                    .filter_map(|link| {
                        self.get_runtime_guid(link.slave_guid)
                            .map(|guid| (guid, link.flags))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get masters for a slave
    pub fn get_masters(&self, slave_spawn_id: u32) -> Vec<(ObjectGuid, LinkFlags)> {
        self.links_by_slave
            .get(&slave_spawn_id)
            .map(|links| {
                links
                    .iter()
                    .filter_map(|link| {
                        self.get_runtime_guid(link.master_guid)
                            .map(|guid| (guid, link.flags))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Process a link event from master to slaves
    pub fn propagate_event_to_slaves(
        &self,
        master_spawn_id: u32,
        event: LinkEvent,
    ) -> Vec<(ObjectGuid, LinkEvent)> {
        let mut events = Vec::new();

        if let Some(links) = self.links_by_master.get(&master_spawn_id) {
            for link in links.iter() {
                let should_propagate = match event {
                    LinkEvent::Aggro { .. } | LinkEvent::EnterCombat { .. } => {
                        link.flags.contains(LinkFlags::AGGRO_ON_AGGRO)
                    }
                    LinkEvent::Death => {
                        link.flags.contains(LinkFlags::DIE_ON_MASTER_DEATH)
                    }
                    LinkEvent::Respawn => {
                        link.flags.contains(LinkFlags::RESPAWN_ON_RESPAWN)
                    }
                    LinkEvent::Evade | LinkEvent::LeaveCombat => {
                        link.flags.contains(LinkFlags::EVADE_ON_MASTER_EVADE)
                    }
                };

                if should_propagate {
                    if let Some(slave_guid) = self.get_runtime_guid(link.slave_guid) {
                        events.push((slave_guid, event));
                    }
                }
            }
        }

        events
    }

    /// Process a link event from slave to masters
    pub fn propagate_event_to_masters(
        &self,
        slave_spawn_id: u32,
        event: LinkEvent,
    ) -> Vec<(ObjectGuid, LinkEvent)> {
        let mut events = Vec::new();

        if let Some(links) = self.links_by_slave.get(&slave_spawn_id) {
            for link in links.iter() {
                let should_propagate = match event {
                    LinkEvent::Aggro { .. } | LinkEvent::EnterCombat { .. } => {
                        link.flags.contains(LinkFlags::TO_AGGRO_ON_AGGRO)
                    }
                    LinkEvent::Respawn => {
                        link.flags.contains(LinkFlags::TO_RESPAWN_ON_RESPAWN)
                    }
                    _ => false,
                };

                if should_propagate {
                    if let Some(master_guid) = self.get_runtime_guid(link.master_guid) {
                        events.push((master_guid, event));
                    }
                }
            }
        }

        events
    }
}

impl Default for LinkingManager {
    fn default() -> Self {
        Self::new()
    }
}
