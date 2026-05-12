use super::state::POINTS_PER_ROW;
use std::collections::HashMap;

/// Talent entry loaded from Talent.dbc.
///
/// Each entry defines one talent node in a talent tree.
/// A talent can have up to 5 ranks, each granting a different spell.
///
/// DBC columns (Talent.dbc):
///   0: id (u32)
///   1: tab_id (u32) - which talent tab (tree) this belongs to
///   2: row (u32) - row position in the tree (0-6)
///   3: column (u32) - column position in the tree (0-3)
///   4-8: rank_spell_id[5] (u32) - spell ID granted at each rank (0 = unused)
///   9: depends_on_talent (u32) - prerequisite talent ID (0 = none)
///  10: depends_on_rank (u32) - required rank of prerequisite
///  17: depends_on_talent_2 (u32) - second prerequisite (rare, some TBC talents)
///  18: depends_on_rank_2 (u32) - required rank of second prerequisite
#[derive(Debug, Clone)]
pub struct TalentInfo {
    pub id: u32,
    pub tab_id: u32,
    pub row: u32,
    pub column: u32,

    /// Spell IDs for ranks 1-5. Index 0 = rank 1 spell, etc.
    /// A value of 0 means that rank does not exist.
    pub rank_spell_ids: [u32; 5],

    /// Prerequisite talent that must be fully ranked before this can be learned.
    /// 0 means no prerequisite.
    pub prerequisite_talent_id: u32,

    /// Required rank of the prerequisite talent (usually its max rank).
    pub prerequisite_rank: u32,
}

impl TalentInfo {
    /// Get the maximum rank for this talent (1-5).
    /// Determined by counting non-zero entries in rank_spell_ids.
    pub fn max_rank(&self) -> u8 {
        self.rank_spell_ids.iter().filter(|&&id| id != 0).count() as u8
    }

    /// Get the spell ID for a specific rank (1-indexed).
    /// Returns None if the rank does not exist.
    pub fn spell_id_for_rank(&self, rank: u8) -> Option<u32> {
        if rank == 0 || rank as usize > self.rank_spell_ids.len() {
            return None;
        }
        let id = self.rank_spell_ids[(rank - 1) as usize];
        if id == 0 {
            None
        } else {
            Some(id)
        }
    }

    /// Get the required points spent in this talent's tab before it unlocks.
    /// Formula: row * POINTS_PER_ROW (row 0 = 0, row 1 = 5, ..., row 6 = 30)
    pub fn required_points_in_tab(&self) -> u32 {
        self.row * POINTS_PER_ROW
    }

    /// Check if this talent has a prerequisite.
    pub fn has_prerequisite(&self) -> bool {
        self.prerequisite_talent_id != 0
    }
}

/// Talent tab (tree) definition loaded from TalentTab.dbc.
///
/// Each class has exactly 3 talent tabs (e.g., Warrior: Arms, Fury, Protection).
///
/// DBC columns (TalentTab.dbc):
///   0: id (u32)
///   1: name (string) - localized name of the tab
///   8: class_mask (u32) - bitmask of classes that have this tab
///   9: tab_page (u32) - order within class (0, 1, 2)
///  10: spell_icon (string) - icon path for the tab
#[derive(Debug, Clone)]
pub struct TalentTabInfo {
    pub id: u32,
    pub name: String,
    pub class_mask: u32,
    pub tab_page: u32,
}

impl TalentTabInfo {
    /// Check if a given class has access to this talent tab.
    ///
    /// # Arguments
    /// * `class_id` - Player class (1=Warrior, 2=Paladin, ..., 11=Druid)
    pub fn is_for_class(&self, class_id: u8) -> bool {
        let class_bit = 1u32 << (class_id - 1);
        (self.class_mask & class_bit) != 0
    }
}

/// Talent store - holds all talent and tab data loaded from DBC files.
///
/// Built once at server startup, then shared as read-only across all sessions.
/// Provides O(1) lookups by talent ID and class-specific tab lists.
#[derive(Debug, Clone)]
pub struct TalentStore {
    /// All talents indexed by talent_id
    pub talents: HashMap<u32, TalentInfo>,

    /// All talent tabs indexed by tab_id
    pub tabs: HashMap<u32, TalentTabInfo>,

    /// Talent ID -> tab_id mapping for quick lookup
    pub talent_to_tab: HashMap<u32, u32>,

    /// Class ID -> ordered list of tab IDs (always 3 per class)
    pub class_tabs: HashMap<u8, Vec<u32>>,
}

impl TalentStore {
    /// Load all talent data from DBC files.
    ///
    /// Called once during server startup. The store is then shared
    /// via Arc for concurrent read access.
    pub fn load(talent_entries: Vec<TalentInfo>, tab_entries: Vec<TalentTabInfo>) -> Self {
        Self::load_internal(talent_entries, tab_entries)
    }

    /// Load talent data from DBC manager entries.
    ///
    /// This is called after DBC files are loaded to populate the talent store
    /// with actual data from Talent.dbc and TalentTab.dbc.
    pub fn from_dbc(
        talent_dbc_entries: &[(u32, crate::world::dbc::structures::TalentEntry)],
        talent_tab_dbc_entries: &[(u32, crate::world::dbc::structures::TalentTabEntry)],
    ) -> Self {
        // Convert DBC entries to internal format
        let talent_entries: Vec<TalentInfo> = talent_dbc_entries
            .iter()
            .map(|(_, entry)| TalentInfo {
                id: entry.id,
                tab_id: entry.tab_id,
                row: entry.row,
                column: entry.column,
                rank_spell_ids: entry.rank_spell_ids,
                prerequisite_talent_id: entry.prerequisite_talent_id,
                prerequisite_rank: entry.prerequisite_rank,
            })
            .collect();

        let tab_entries: Vec<TalentTabInfo> = talent_tab_dbc_entries
            .iter()
            .map(|(_, entry)| TalentTabInfo {
                id: entry.id,
                name: String::new(), // DBC doesn't have name in our struct, would need string table
                class_mask: entry.class_mask,
                tab_page: entry.tab_page,
            })
            .collect();

        Self::load_internal(talent_entries, tab_entries)
    }

    fn load_internal(talent_entries: Vec<TalentInfo>, tab_entries: Vec<TalentTabInfo>) -> Self {
        let mut store = Self {
            talents: HashMap::new(),
            tabs: HashMap::new(),
            talent_to_tab: HashMap::new(),
            class_tabs: HashMap::new(),
        };

        // Index tabs
        for tab in tab_entries {
            store.tabs.insert(tab.id, tab.clone());

            // Build class -> tabs mapping
            for class_id in 1u8..=11 {
                if tab.is_for_class(class_id) {
                    store
                        .class_tabs
                        .entry(class_id)
                        .or_insert_with(Vec::new)
                        .push(tab.id);
                }
            }
        }

        // Sort tabs by tab_page for each class
        for tabs in store.class_tabs.values_mut() {
            tabs.sort_by_key(|tab_id| store.tabs.get(tab_id).map(|t| t.tab_page).unwrap_or(0));
        }

        // Index talents
        for talent in talent_entries {
            store.talent_to_tab.insert(talent.id, talent.tab_id);
            store.talents.insert(talent.id, talent);
        }

        store
    }

    /// Get all talent tabs for a specific class.
    pub fn tabs_for_class(&self, class_id: u8) -> &[u32] {
        self.class_tabs
            .get(&class_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get talent info by ID.
    pub fn get_talent(&self, talent_id: u32) -> Option<&TalentInfo> {
        self.talents.get(&talent_id)
    }

    /// Get tab info by ID.
    pub fn get_tab(&self, tab_id: u32) -> Option<&TalentTabInfo> {
        self.tabs.get(&tab_id)
    }
}
