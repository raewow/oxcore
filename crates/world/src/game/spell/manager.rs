//! SpellManager - owns spell template data loaded from SQL

use crate::database::repositories::SpellRepository;
use crate::dbc::structures::SpellEntry;
use anyhow::Result;
use dashmap::DashMap;
use sqlx::MySqlPool;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct SpellChainNode {
    pub first: u32,
    pub prev: u32,
    pub next: u32,
    pub rank: u32,
    pub last: u32,
}

#[derive(Debug, Clone)]
pub struct SpellProcEventEntry {
    pub school_mask: u32,
    pub spell_family: u32,
    pub spell_family_mask: u64,
    pub proc_flags: u32,
    /// Extra proc requirement (ProcFlagsEx) — gates which hit outcome / cast-end triggers it.
    pub proc_ex: u32,
    pub ppm_rate: f32,
    pub custom_chance: f32,
    pub cooldown: u32,
}

#[derive(Debug, Clone)]
pub struct SpellThreatEntry {
    pub flat: i32,
    pub pct: f32,
    pub ap_bonus: f32,
}

#[derive(Debug, Clone)]
pub struct SpellLearnSkillNode {
    pub skill_id: u32,
    pub step: u32,
    pub char_pts: u32,
}

#[derive(Debug, Clone)]
pub struct SpellLearnSpellNode {
    pub spell: u32,
    pub active: bool,
    pub autolearned: bool,
}

#[derive(Debug, Clone)]
pub struct SpellArea {
    pub spell: u32,
    pub area_id: u32,
    pub quest_start: u32,
    pub quest_end: u32,
    pub aura_spell: i32,
    pub racemask: u32,
    pub gender: u8,
    pub quest_start_can_active: bool,
    pub autocast: bool,
}

#[derive(Debug, Clone)]
pub struct SpellTargetEntry {
    pub type_: u32,
    pub target_id: u32,
    pub can_focus: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PetAura;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpellGroupStackRule(pub u32);

/// Destination coordinates loaded from spell_target_position table
#[derive(Debug, Clone)]
pub struct SpellTargetPosition {
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
}

pub struct SpellManager {
    spells: DashMap<u32, Arc<SpellEntry>>,
    target_positions: DashMap<u32, SpellTargetPosition>,
    spell_chains: HashMap<u32, SpellChainNode>,
    spell_chains_next: HashMap<u32, Vec<u32>>,
    spell_proc_events: DashMap<u32, SpellProcEventEntry>,
    spell_proc_item_enchant: HashMap<u32, f32>,
    spell_enchant_charges: HashMap<u32, u32>,
    spell_threats: HashMap<u32, SpellThreatEntry>,
    spell_elixirs: HashMap<u32, u8>,
    spell_learn_skills: HashMap<u32, SpellLearnSkillNode>,
    spell_learn_spells: HashMap<u32, Vec<SpellLearnSpellNode>>,
    spell_script_targets: HashMap<u32, Vec<SpellTargetEntry>>,
    spell_areas: Vec<SpellArea>,
    spell_pet_auras: HashMap<u16, PetAura>,
    spell_groups: HashMap<u32, Vec<u32>>,
    spell_group_stack: HashMap<u32, SpellGroupStackRule>,
    spell_cones: HashMap<u32, f32>,
    existing_spell_ids: HashSet<u32>,
}

impl SpellManager {
    pub fn new() -> Self {
        Self {
            spells: DashMap::new(),
            target_positions: DashMap::new(),
            spell_chains: HashMap::new(),
            spell_chains_next: HashMap::new(),
            spell_proc_events: DashMap::new(),
            spell_proc_item_enchant: HashMap::new(),
            spell_enchant_charges: HashMap::new(),
            spell_threats: HashMap::new(),
            spell_elixirs: HashMap::new(),
            spell_learn_skills: HashMap::new(),
            spell_learn_spells: HashMap::new(),
            spell_script_targets: HashMap::new(),
            spell_areas: Vec::new(),
            spell_pet_auras: HashMap::new(),
            spell_groups: HashMap::new(),
            spell_group_stack: HashMap::new(),
            spell_cones: HashMap::new(),
            existing_spell_ids: HashSet::new(),
        }
    }

    /// Load all spells from the spell_template SQL table
    pub async fn load(&self, world_db: &MySqlPool) -> Result<()> {
        let repo = SpellRepository::new(Arc::new(world_db.clone()));
        let entries = repo.load_all().await?;
        let count = entries.len();

        for entry in entries {
            let id = entry.id;
            self.spells.insert(id, Arc::new(entry));
        }

        info!("Loaded {} spells from spell_template", count);

        // Load spell_target_position
        self.load_target_positions(world_db).await?;

        // Load spell_proc_event (custom proc gating)
        self.load_spell_proc_events(world_db).await?;

        Ok(())
    }

    /// Load spell_target_position table (coordinates for teleport spells)
    async fn load_target_positions(&self, world_db: &MySqlPool) -> Result<()> {
        let rows = sqlx::query(
            "SELECT CAST(id AS UNSIGNED) AS id, \
                    CAST(target_map AS UNSIGNED) AS target_map, \
                    target_position_x, target_position_y, target_position_z, target_orientation \
             FROM spell_target_position",
        )
        .fetch_all(world_db)
        .await?;

        let count = rows.len();
        for row in rows {
            use sqlx::Row;
            let id: u32 = row.try_get::<u64, _>("id").unwrap_or(0) as u32;
            let map_id: u32 = row.try_get::<u64, _>("target_map").unwrap_or(0) as u32;
            let x: f32 = row.try_get("target_position_x").unwrap_or(0.0);
            let y: f32 = row.try_get("target_position_y").unwrap_or(0.0);
            let z: f32 = row.try_get("target_position_z").unwrap_or(0.0);
            let orientation: f32 = row.try_get("target_orientation").unwrap_or(0.0);

            self.target_positions.insert(
                id,
                SpellTargetPosition {
                    map_id,
                    x,
                    y,
                    z,
                    orientation,
                },
            );
        }

        info!("Loaded {} spell_target_position entries", count);
        Ok(())
    }

    /// Load the `spell_proc_event` table — custom proc gating (procEx / school / family /
    /// PPM / cooldown) keyed by spell id. Tolerant: a missing table is logged, not fatal.
    async fn load_spell_proc_events(&self, world_db: &MySqlPool) -> Result<()> {
        let query = sqlx::query(
            "SELECT CAST(entry AS UNSIGNED) AS entry, \
                    CAST(SchoolMask AS UNSIGNED) AS school_mask, \
                    CAST(SpellFamilyName AS UNSIGNED) AS spell_family, \
                    CAST(SpellFamilyMask0 AS UNSIGNED) AS family_mask, \
                    CAST(procFlags AS UNSIGNED) AS proc_flags, \
                    CAST(procEx AS UNSIGNED) AS proc_ex, \
                    ppmRate, CustomChance, \
                    CAST(Cooldown AS UNSIGNED) AS cooldown \
             FROM spell_proc_event",
        )
        .fetch_all(world_db)
        .await;

        let rows = match query {
            Ok(rows) => rows,
            Err(e) => {
                warn!("spell_proc_event not loaded (table missing or query failed): {e}");
                return Ok(());
            }
        };

        use sqlx::Row;
        let mut count = 0u32;
        for row in rows {
            let entry: u32 = row.try_get::<u64, _>("entry").unwrap_or(0) as u32;
            if entry == 0 {
                continue;
            }
            self.spell_proc_events.insert(
                entry,
                SpellProcEventEntry {
                    school_mask: row.try_get::<u64, _>("school_mask").unwrap_or(0) as u32,
                    spell_family: row.try_get::<u64, _>("spell_family").unwrap_or(0) as u32,
                    spell_family_mask: row.try_get::<u64, _>("family_mask").unwrap_or(0),
                    proc_flags: row.try_get::<u64, _>("proc_flags").unwrap_or(0) as u32,
                    proc_ex: row.try_get::<u64, _>("proc_ex").unwrap_or(0) as u32,
                    ppm_rate: row.try_get("ppmRate").unwrap_or(0.0),
                    custom_chance: row.try_get("CustomChance").unwrap_or(0.0),
                    cooldown: row.try_get::<u64, _>("cooldown").unwrap_or(0) as u32,
                },
            );
            count += 1;
        }

        info!("Loaded {} spell_proc_event conditions", count);
        Ok(())
    }

    /// Get a spell entry by ID
    pub fn get(&self, spell_id: u32) -> Option<Arc<SpellEntry>> {
        self.spells.get(&spell_id).map(|r| Arc::clone(&r))
    }

    /// Get spell target position (for teleport spells using TARGET_LOCATION_DATABASE)
    pub fn get_spell_target_position(&self, spell_id: u32) -> Option<SpellTargetPosition> {
        self.target_positions.get(&spell_id).map(|r| r.clone())
    }

    /// Get the custom proc-event configuration for a spell (`spell_proc_event` table), if any.
    pub fn get_proc_event(&self, spell_id: u32) -> Option<SpellProcEventEntry> {
        self.spell_proc_events.get(&spell_id).map(|r| r.clone())
    }

    /// Get spell count
    pub fn len(&self) -> usize {
        self.spells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }

    /// Search spells by name (case-insensitive substring match)
    /// Insert a spell entry (used by tests)
    pub fn add_spell(&self, entry: SpellEntry) {
        self.spells.insert(entry.id, Arc::new(entry));
    }

    pub fn search_by_name(&self, search: &str) -> Vec<Arc<SpellEntry>> {
        let search_lower = search.to_lowercase();
        let mut results: Vec<Arc<SpellEntry>> = self
            .spells
            .iter()
            .filter(|entry| entry.value().name.to_lowercase().contains(&search_lower))
            .map(|entry| Arc::clone(entry.value()))
            .collect();
        results.sort_by_key(|s| s.id);
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::mysql::MySqlPoolOptions;

    fn lazy_pool() -> MySqlPool {
        MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    #[test]
    fn proc_event_round_trips_through_the_map() {
        let mgr = SpellManager::new();
        assert!(mgr.get_proc_event(17).is_none());
        mgr.spell_proc_events.insert(
            17,
            SpellProcEventEntry {
                school_mask: 0,
                spell_family: 0,
                spell_family_mask: 0,
                proc_flags: 0,
                proc_ex: crate::game::player::auras::proc::proc_flags_ex::CRITICAL_HIT,
                ppm_rate: 0.0,
                custom_chance: 0.0,
                cooldown: 0,
            },
        );
        let got = mgr.get_proc_event(17).expect("inserted entry");
        assert_eq!(got.proc_ex, 0x02);
    }

    #[tokio::test]
    async fn missing_proc_event_table_is_not_fatal() {
        // A lazy pool with no reachable DB: the loader must degrade gracefully (Ok + empty map),
        // never propagate an error that would abort startup.
        let mgr = SpellManager::new();
        let result = mgr.load_spell_proc_events(&lazy_pool()).await;
        assert!(result.is_ok());
        assert!(mgr.get_proc_event(1).is_none());
    }
}
