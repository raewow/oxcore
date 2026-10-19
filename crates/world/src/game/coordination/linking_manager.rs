use super::link_flags::LinkFlags;
use super::linking_repository::{CreatureEntryLinkRow, CreatureLinkRow};
use dashmap::DashMap;

/// A spawn linked to a master spawn.
#[derive(Debug, Clone)]
pub struct CreatureLink {
    /// Master spawn id
    pub master_guid: u32,
    /// Slave spawn id
    pub slave_guid: u32,
    pub flags: LinkFlags,
}

/// Static creature linking data, by spawn and by entry.
pub struct LinkingManager {
    links_by_master: DashMap<u32, Vec<CreatureLink>>,
    links_by_slave: DashMap<u32, CreatureLink>,
    entry_links_by_master: DashMap<u32, Vec<CreatureEntryLinkRow>>,
    entry_links_by_slave: DashMap<(u32, u32), CreatureEntryLinkRow>,
}

impl LinkingManager {
    pub fn new() -> Self {
        Self {
            links_by_master: DashMap::new(),
            links_by_slave: DashMap::new(),
            entry_links_by_master: DashMap::new(),
            entry_links_by_slave: DashMap::new(),
        }
    }

    /// Load links between spawns.
    pub fn load_from_repository(&self, links: Vec<CreatureLinkRow>) {
        for row in links {
            let link = CreatureLink {
                master_guid: row.master_guid,
                slave_guid: row.slave_guid,
                flags: row.flags,
            };
            self.links_by_master
                .entry(row.master_guid)
                .or_default()
                .push(link.clone());
            self.links_by_slave.insert(row.slave_guid, link);
        }
    }

    /// Load links between entries.
    pub fn load_entry_links(&self, links: Vec<CreatureEntryLinkRow>) {
        for row in links {
            self.entry_links_by_master
                .entry(row.master_entry)
                .or_default()
                .push(row.clone());
            self.entry_links_by_slave
                .insert((row.slave_entry, row.map_id), row);
        }
    }

    /// Spawns linked to this master spawn.
    pub fn slaves_of_spawn(&self, master_spawn_id: u32) -> Vec<CreatureLink> {
        self.links_by_master
            .get(&master_spawn_id)
            .map(|links| links.clone())
            .unwrap_or_default()
    }

    /// Entries linked to this master entry.
    pub fn slaves_of_entry(&self, master_entry: u32) -> Vec<CreatureEntryLinkRow> {
        self.entry_links_by_master
            .get(&master_entry)
            .map(|links| links.clone())
            .unwrap_or_default()
    }

    /// This spawn's own link to its master spawn.
    pub fn master_of_spawn(&self, slave_spawn_id: u32) -> Option<CreatureLink> {
        self.links_by_slave.get(&slave_spawn_id).map(|l| l.clone())
    }

    /// This entry's own link to its master entry on `map_id`.
    pub fn master_of_entry(&self, slave_entry: u32, map_id: u32) -> Option<CreatureEntryLinkRow> {
        self.entry_links_by_slave
            .get(&(slave_entry, map_id))
            .map(|l| l.clone())
    }

    pub fn link_count(&self) -> usize {
        self.links_by_slave.len() + self.entry_links_by_slave.len()
    }
}

impl Default for LinkingManager {
    fn default() -> Self {
        Self::new()
    }
}
