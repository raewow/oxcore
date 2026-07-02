//! SpellManager - owns spell template data loaded from SQL

use crate::database::repositories::SpellRepository;
use crate::dbc::manager::DbcManager;
use crate::dbc::structures::SpellEntry;
use anyhow::Result;
use dashmap::DashMap;
use parking_lot::RwLock;
use sqlx::MySqlPool;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{info, warn};

/// Faithful `SpellChainNode` (MaNGOS `SpellMgr.h`): prev/first/req + 1-based rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellChainNode {
    pub prev: u32,
    pub first: u32,
    pub req: u32,
    pub rank: u8,
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
    spell_chains: RwLock<HashMap<u32, SpellChainNode>>,
    spell_chains_next: RwLock<HashMap<u32, Vec<u32>>>,
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
            spell_chains: RwLock::new(HashMap::new()),
            spell_chains_next: RwLock::new(HashMap::new()),
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

    /// Load spell rank chains: talent ranks + skill-ability forward-rank chains (both derived
    /// from DBC data), merged/overridden by the `spell_chain` SQL table (custom cases).
    /// Faithful `SpellMgr::LoadSpellChains` port (validation logging trimmed to warnings).
    pub async fn load_spell_chains(&self, world_db: &MySqlPool, dbc: &DbcManager) -> Result<()> {
        let mut chains: HashMap<u32, SpellChainNode> = HashMap::new();

        // 1. Talent DBC: ranks 2..5 form a chain rooted at rank_spell_ids[0].
        for (_, talent) in dbc.talent.entries() {
            if talent.rank_spell_ids[1] == 0 {
                // Single-rank talents don't need chain data (handled by table data if present).
                continue;
            }
            for j in 0..5usize {
                let spell_id = talent.rank_spell_ids[j];
                if spell_id == 0 {
                    continue;
                }
                if self.spells.get(&spell_id).is_none() {
                    continue;
                }
                chains.insert(
                    spell_id,
                    SpellChainNode {
                        prev: if j > 0 { talent.rank_spell_ids[j - 1] } else { 0 },
                        first: talent.rank_spell_ids[0],
                        rank: (j + 1) as u8,
                        req: 0,
                    },
                );
            }
        }

        // 2. SkillLineAbility forward_spell_id chains (e.g. profession/secondary-skill ranks).
        let mut by_spell_id: HashMap<u32, Vec<u32>> = HashMap::new(); // spell_id -> [forward_spell_id]
        for (_, ability) in dbc.skill_line_ability.entries() {
            by_spell_id
                .entry(ability.spell_id)
                .or_default()
                .push(ability.forward_spell_id);
        }
        {
            let mut prev_ranks: HashMap<u32, u32> = HashMap::new(); // forward_id -> spell_id
            for (&spell_id, forwards) in &by_spell_id {
                if self.spells.get(&spell_id).is_none() {
                    continue;
                }
                for &raw_forward_id in forwards {
                    let mut forward_id = raw_forward_id;
                    if spell_id == 2366 {
                        // Herb Gathering, Apprentice: pre-3.x clients miss the forward link.
                        forward_id = 2368;
                    }
                    if spell_id == 20154 {
                        // Seal of Righteousness: forward link duplicates the spellbook entry.
                        continue;
                    }
                    if forward_id == 0 {
                        continue;
                    }
                    if self.spells.get(&forward_id).is_none() {
                        continue;
                    }
                    // forward_id must itself be a known ability spell_id (has further data).
                    if !by_spell_id.contains_key(&forward_id) {
                        continue;
                    }
                    if chains.contains_key(&forward_id) {
                        continue;
                    }
                    if prev_ranks.contains_key(&forward_id) {
                        continue;
                    }
                    if let Some(prev_node) = chains.get(&spell_id).copied() {
                        chains.insert(
                            forward_id,
                            SpellChainNode {
                                prev: spell_id,
                                first: prev_node.first,
                                rank: prev_node.rank + 1,
                                req: 0,
                            },
                        );
                        continue;
                    }
                    prev_ranks.insert(forward_id, spell_id);
                }
            }

            // Resolve the deferred (rank not yet known) forward chains.
            while let Some(spell_id) = prev_ranks.keys().next().copied() {
                let prev_id = prev_ranks.remove(&spell_id).unwrap();
                let (first, rank) = match chains.get(&prev_id) {
                    Some(prev_node) => (prev_node.first, prev_node.rank + 1),
                    None => (prev_id, 2), // prev is itself the (unranked) first spell.
                };
                chains.insert(
                    spell_id,
                    SpellChainNode {
                        prev: prev_id,
                        first,
                        rank,
                        req: 0,
                    },
                );
            }
        }

        let dbc_count = chains.len();
        let mut new_count = 0u32;

        // 3. `spell_chain` SQL table: authoritative custom cases + `req` field updates.
        let rows = sqlx::query(
            "SELECT CAST(spell_id AS UNSIGNED) AS spell_id, \
                    CAST(prev_spell AS UNSIGNED) AS prev_spell, \
                    CAST(first_spell AS UNSIGNED) AS first_spell, \
                    CAST(`rank` AS UNSIGNED) AS `rank`, \
                    CAST(req_spell AS UNSIGNED) AS req_spell \
             FROM spell_chain",
        )
        .fetch_all(world_db)
        .await;

        let rows = match rows {
            Ok(rows) => rows,
            Err(e) => {
                warn!("spell_chain not loaded (table missing or query failed): {e}");
                info!("Loaded {} spell chain records (from DBC data only)", chains.len());
                self.rebuild_spell_chains_next(&chains);
                *self.spell_chains.write() = chains;
                return Ok(());
            }
        };

        use sqlx::Row;
        for row in rows {
            let spell_id: u32 = row.try_get::<u64, _>("spell_id").unwrap_or(0) as u32;
            let node = SpellChainNode {
                prev: row.try_get::<u64, _>("prev_spell").unwrap_or(0) as u32,
                first: row.try_get::<u64, _>("first_spell").unwrap_or(0) as u32,
                rank: row.try_get::<u64, _>("rank").unwrap_or(0) as u8,
                req: row.try_get::<u64, _>("req_spell").unwrap_or(0) as u32,
            };

            if self.spells.get(&spell_id).is_none() {
                warn!("Spell {spell_id} listed in `spell_chain` does not exist");
                continue;
            }

            if let Some(existing) = chains.get_mut(&spell_id) {
                if existing.rank != node.rank || existing.prev != node.prev || existing.first != node.first {
                    warn!("Spell {spell_id} listed in `spell_chain` conflicts with DBC-derived chain data");
                    continue;
                }
                if node.req != 0 {
                    existing.req = node.req;
                }
                continue;
            }

            if node.prev != 0 && self.spells.get(&node.prev).is_none() {
                warn!("Spell {spell_id} in `spell_chain` has nonexistent previous rank spell {}", node.prev);
                continue;
            }
            if self.spells.get(&node.first).is_none() {
                warn!("Spell {spell_id} in `spell_chain` has nonexistent first rank spell {}", node.first);
                continue;
            }
            if node.req != 0 && self.spells.get(&node.req).is_none() {
                warn!("Spell {spell_id} in `spell_chain` has nonexistent required spell {}", node.req);
                continue;
            }

            chains.insert(spell_id, node);
            new_count += 1;
        }

        info!(
            "Loaded {} spell chain records ({} from DBC data, {} loaded from table)",
            chains.len(),
            dbc_count,
            new_count
        );

        self.rebuild_spell_chains_next(&chains);
        *self.spell_chains.write() = chains;
        Ok(())
    }

    /// Rebuild the `prev`/`req` -> spell_id reverse-lookup map (MaNGOS `mSpellChainsNext`).
    fn rebuild_spell_chains_next(&self, chains: &HashMap<u32, SpellChainNode>) {
        let mut next: HashMap<u32, Vec<u32>> = HashMap::new();
        for (&spell_id, node) in chains {
            if node.prev != 0 {
                next.entry(node.prev).or_default().push(spell_id);
            }
            if node.req != 0 {
                next.entry(node.req).or_default().push(spell_id);
            }
        }
        *self.spell_chains_next.write() = next;
    }

    /// Get the rank (1-based) of a spell within its spell chain, or 0 if it has no chain data.
    /// Faithful `SpellMgr::GetSpellRank`.
    pub fn get_spell_rank(&self, spell_id: u32) -> u8 {
        self.spell_chains
            .read()
            .get(&spell_id)
            .map(|n| n.rank)
            .unwrap_or(0)
    }

    /// Get the chain node (prev/first/req/rank) for a spell, if it belongs to a rank chain.
    pub fn get_spell_chain_node(&self, spell_id: u32) -> Option<SpellChainNode> {
        self.spell_chains.read().get(&spell_id).copied()
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

    #[test]
    fn spell_rank_reads_from_chain_map() {
        let mgr = SpellManager::new();
        assert_eq!(mgr.get_spell_rank(999), 0);

        *mgr.spell_chains.write() = HashMap::from([(
            999,
            SpellChainNode {
                prev: 998,
                first: 997,
                rank: 3,
                req: 0,
            },
        )]);

        assert_eq!(mgr.get_spell_rank(999), 3);
        let node = mgr.get_spell_chain_node(999).expect("inserted node");
        assert_eq!(node.first, 997);
        assert_eq!(node.prev, 998);
    }

    #[tokio::test]
    async fn missing_spell_chain_table_is_not_fatal() {
        let mgr = SpellManager::new();
        let dbc = DbcManager::new();
        let result = mgr.load_spell_chains(&lazy_pool(), &dbc).await;
        assert!(result.is_ok());
        assert_eq!(mgr.get_spell_rank(1), 0);
    }
}
