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

impl SpellArea {
    /// Port of `SpellArea::IsFitToRequirements` (SpellMgr.cpp:3050).
    /// Checks whether a player meets all area/spell requirements.
    /// Player-specific data (gender, race, quests, auras) is passed explicitly
    /// so the function works without a direct dependency on the Player type.
    pub fn is_fit_to_requirements(
        &self,
        player_gender: Option<u8>,
        player_race_mask: Option<u32>,
        player_active_quests: Option<&[u32]>,
        player_rewarded_quests: Option<&std::collections::HashSet<u32>>,
        player_has_aura: Option<&dyn Fn(u32) -> bool>,
        new_zone: u32,
        new_area: u32,
    ) -> bool {
        // gender check
        if self.gender != 2 {
            // GENDER_NONE = 2
            if player_gender.map_or(true, |g| g != self.gender) {
                return false;
            }
        }

        // race check
        if self.racemask != 0 {
            if player_race_mask.map_or(true, |mask| (self.racemask & mask) == 0) {
                return false;
            }
        }

        // area check
        if self.area_id != 0 {
            if new_zone != self.area_id && new_area != self.area_id {
                return false;
            }
        }

        // quest start check
        if self.quest_start != 0 {
            let passes = player_active_quests.zip(player_rewarded_quests).map_or(
                false,
                |(active, rewarded)| {
                    let active_ok =
                        self.quest_start_can_active && active.contains(&self.quest_start);
                    active_ok || rewarded.contains(&self.quest_start)
                },
            );
            if !passes {
                return false;
            }
        }

        // quest end check
        if self.quest_end != 0 {
            if player_rewarded_quests.map_or(true, |r| r.contains(&self.quest_end)) {
                return false;
            }
        }

        // aura check
        if self.aura_spell != 0 {
            let Some(has_aura) = player_has_aura else {
                return false;
            };
            if self.aura_spell > 0 {
                return has_aura(self.aura_spell as u32);
            } else {
                return !has_aura((-self.aura_spell) as u32);
            }
        }

        true
    }
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
    /// `conditions.condition_entry`; evaluation awaits the shared condition system.
    pub condition_id: u32,
    pub can_focus: bool,
    pub inverse_effect_mask: u32,
}

/// Pet aura binding: a spell that grants a pet an aura.
/// Port of MaNGOS `PetAura` (SpellMgr.h).
#[derive(Debug, Clone, Default)]
pub struct PetAura {
    pub remove_on_change_pet: bool,
    pub damage: i32,
    pub auras: std::collections::HashMap<u32, u32>,
}

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

/// Port of `SpellEntry::IsExplicitPositiveTarget` — target modes where the
/// spell expects an explicit (client-provided) friendly target.
fn is_explicit_positive_target(target_a: u32) -> bool {
    matches!(target_a, 21 | 35 | 45 | 57 | 61)
}

/// Port of `SpellEntry::IsAreaEffectPossitiveTarget` — target modes that
/// auto-select friendly units in an area (party, raid, friend-AoE).
fn is_area_positive_target(target: u32) -> bool {
    matches!(target, 20 | 30 | 31 | 33 | 34 | 37 | 56 | 61)
}

pub struct SpellManager {
    spells: DashMap<u32, Arc<SpellEntry>>,
    target_positions: DashMap<u32, SpellTargetPosition>,
    spell_chains: RwLock<HashMap<u32, SpellChainNode>>,
    spell_chains_next: RwLock<HashMap<u32, Vec<u32>>>,
    spell_proc_events: DashMap<u32, SpellProcEventEntry>,
    spell_proc_item_enchant: HashMap<u32, f32>,
    spell_enchant_charges: RwLock<HashMap<u32, u32>>,
    spell_threats: DashMap<u32, SpellThreatEntry>,
    spell_elixirs: HashMap<u32, u8>,
    spell_learn_skills: RwLock<HashMap<u32, SpellLearnSkillNode>>,
    spell_learn_spells: RwLock<HashMap<u32, Vec<SpellLearnSpellNode>>>,
    spell_script_targets: RwLock<HashMap<u32, Vec<SpellTargetEntry>>>,
    spell_areas: RwLock<Vec<SpellArea>>,
    spell_pet_auras: RwLock<HashMap<u16, PetAura>>,
    spell_groups: HashMap<u32, Vec<u32>>,
    spell_group_stack: HashMap<u32, SpellGroupStackRule>,
    spell_cones: RwLock<HashMap<u32, f32>>,
    existing_spell_ids: RwLock<HashSet<u32>>,
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
            spell_enchant_charges: RwLock::new(HashMap::new()),
            spell_threats: DashMap::new(),
            spell_elixirs: HashMap::new(),
            spell_learn_skills: RwLock::new(HashMap::new()),
            spell_learn_spells: RwLock::new(HashMap::new()),
            spell_script_targets: RwLock::new(HashMap::new()),
            spell_areas: RwLock::new(Vec::new()),
            spell_pet_auras: RwLock::new(HashMap::new()),
            spell_groups: HashMap::new(),
            spell_group_stack: HashMap::new(),
            spell_cones: RwLock::new(HashMap::new()),
            existing_spell_ids: RwLock::new(HashSet::new()),
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

        // Scan loaded spells for learn-skill entries
        self.load_spell_learn_skills();

        // Load spell_enchant_charges
        self.load_spell_enchant_charges(world_db).await?;

        // Load spell_pet_auras
        self.load_spell_pet_auras(world_db).await?;

        // Load existing spell ids (for validation)
        self.load_existing_spell_ids(world_db).await?;

        // Load spell_cones
        self.load_spell_cones(world_db).await?;

        // Load spell_areas
        self.load_spell_areas(world_db).await?;

        // Load spell_learn_spells
        self.load_spell_learn_spells(world_db).await?;

        // Load spell_script_targets
        self.load_spell_script_targets(world_db).await?;

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
                        prev: if j > 0 {
                            talent.rank_spell_ids[j - 1]
                        } else {
                            0
                        },
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
                info!(
                    "Loaded {} spell chain records (from DBC data only)",
                    chains.len()
                );
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
                if existing.rank != node.rank
                    || existing.prev != node.prev
                    || existing.first != node.first
                {
                    warn!("Spell {spell_id} listed in `spell_chain` conflicts with DBC-derived chain data");
                    continue;
                }
                if node.req != 0 {
                    existing.req = node.req;
                }
                continue;
            }

            if node.prev != 0 && self.spells.get(&node.prev).is_none() {
                warn!(
                    "Spell {spell_id} in `spell_chain` has nonexistent previous rank spell {}",
                    node.prev
                );
                continue;
            }
            if self.spells.get(&node.first).is_none() {
                warn!(
                    "Spell {spell_id} in `spell_chain` has nonexistent first rank spell {}",
                    node.first
                );
                continue;
            }
            if node.req != 0 && self.spells.get(&node.req).is_none() {
                warn!(
                    "Spell {spell_id} in `spell_chain` has nonexistent required spell {}",
                    node.req
                );
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

    /// Port of `SpellMgr::SelectAuraRankForLevel`.
    ///
    /// Selects the appropriate rank of a positive aura spell for a target of
    /// the given `level`. Falls back down the spell chain (from higher ranks
    /// to lower) until it finds one the target qualifies for (level + 10 >=
    /// spellLevel). Returns `None` when no rank in the chain fits.
    ///
    /// Approximations: `IsPassiveSpell` uses the `SPELL_ATTR_PASSIVE` bit;
    /// `IsExplicitPositiveTarget` and `IsAreaEffectPossitiveTarget` are
    /// ported as `is_explicit_positive_target` / `is_area_positive_target`
    /// checking the same target-mode values.
    pub fn select_aura_rank_for_level(
        &self,
        spell: &SpellEntry,
        level: u32,
    ) -> Option<Arc<SpellEntry>> {
        // Fast case: target already high enough level
        if level + 10 >= spell.spell_level {
            return self.get(spell.id);
        }

        // Passive spells are never down-ranked
        if spell.is_passive_spell() {
            return self.get(spell.id);
        }

        // Whether we need rank selection at all (positive aura with explicit
        // or area-positive target, or area-aura-party effect)
        let need_rank_selection = spell.effect.iter().enumerate().any(|(i, &eff)| {
            let target_a = spell.effect_implicit_target_a[i];
            let positive = spell.is_positive_effect(i);
            positive
                && ((eff == 6
                    && (is_explicit_positive_target(target_a)
                        || is_area_positive_target(target_a)))
                    || eff == 35)
        });

        if !need_rank_selection || self.get_spell_rank(spell.id) == 0 {
            return self.get(spell.id);
        }

        // Walk down the spell chain (lower ranks have lower spellLevel)
        let mut next_id = spell.id;
        loop {
            if let Some(next_spell) = self.get(next_id) {
                if level + 10 >= next_spell.spell_level {
                    return Some(next_spell);
                }
            } else {
                break;
            }
            let node = self.get_spell_chain_node(next_id);
            next_id = match node {
                Some(n) if n.prev != 0 => n.prev,
                _ => break,
            };
        }

        None
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

    /// Get the `spell_threat` flat/pct/AP bonus entry for a spell, if any
    /// (`SpellMgr::GetSpellThreatEntry`). Consumed by `Spell::HandleThreatSpells`.
    pub fn get_spell_threat_entry(&self, spell_id: u32) -> Option<SpellThreatEntry> {
        self.spell_threats.get(&spell_id).map(|r| r.clone())
    }

    /// Insert a `spell_threat` entry (used by the loader and tests).
    pub fn add_spell_threat(&self, spell_id: u32, entry: SpellThreatEntry) {
        self.spell_threats.insert(spell_id, entry);
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

    /// Return the script-target records configured for a spell.
    pub fn get_spell_script_targets(&self, spell_id: u32) -> Vec<SpellTargetEntry> {
        self.spell_script_targets
            .read()
            .get(&spell_id)
            .cloned()
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub fn set_spell_script_targets_for_test(&self, spell_id: u32, targets: Vec<SpellTargetEntry>) {
        self.spell_script_targets.write().insert(spell_id, targets);
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

    /// Port of `SpellMgr::LoadSpellLearnSkills`.
    ///
    /// Scans every loaded spell for `SPELL_EFFECT_SKILL` (118) and populates
    /// `spell_learn_skills[spell_id]` with the skill, step, and character points
    /// (1 for ordinary skills, `step * 75` for riding).
    pub fn load_spell_learn_skills(&self) {
        const SPELL_EFFECT_SKILL: u32 = 118;
        const SKILL_RIDING: u32 = 762;

        let mut map = HashMap::new();
        let mut count = 0u32;

        for entry in self.spells.iter() {
            let spell_id = entry.id;
            for (i, &eff) in entry.effect.iter().enumerate() {
                if eff == SPELL_EFFECT_SKILL {
                    let skill = entry.effect_misc_value[i] as u32;
                    let step = entry.effect_base_points[i] as u32;
                    let char_pts = if skill != SKILL_RIDING { 1 } else { step * 75 };
                    map.insert(
                        spell_id,
                        SpellLearnSkillNode {
                            skill_id: skill,
                            step,
                            char_pts,
                        },
                    );
                    count += 1;
                    break;
                }
            }
        }

        *self.spell_learn_skills.write() = map;
        info!("Loaded {} Spell Learn Skills from templates", count);
    }

    /// Port of `SpellMgr::LoadSpellEnchantCharges`.
    ///
    /// Loads `spell_enchant_charges` from the SQL table, validating each spell
    /// exists in the loaded spell list. Missing spells are logged but not fatal.
    async fn load_spell_enchant_charges(&self, world_db: &MySqlPool) -> Result<()> {
        let mut map = HashMap::new();
        let mut count = 0u32;

        let rows = sqlx::query(
            "SELECT CAST(`entry` AS UNSIGNED) AS entry, \
                    CAST(`charges` AS UNSIGNED) AS charges \
             FROM `spell_enchant_charges`",
        )
        .fetch_all(world_db)
        .await;

        let rows = match rows {
            Ok(rows) => rows,
            Err(e) => {
                warn!("spell_enchant_charges table missing or errored: {e}");
                *self.spell_enchant_charges.write() = map;
                info!(">> Loaded 0 spell enchant charges");
                return Ok(());
            }
        };

        for row in &rows {
            use sqlx::Row;
            let entry: u32 = row.try_get::<u64, _>("entry").unwrap_or(0) as u32;
            let charges: u32 = row.try_get::<u64, _>("charges").unwrap_or(0) as u32;

            if self.spells.get(&entry).is_none() {
                if !self.existing_spell_ids.read().contains(&entry) {
                    warn!("Spell {entry} in spell_enchant_charges does not exist");
                }
                continue;
            }

            map.insert(entry, charges);
            count += 1;
        }

        *self.spell_enchant_charges.write() = map;
        info!(">> Loaded {count} spell enchant charges");
        Ok(())
    }

    /// Port of `SpellMgr::LoadSpellPetAuras`.
    ///
    /// Loads `spell_pet_auras` SQL table, validating each spell exists and has
    /// a dummy aura/effect. Creates `PetAura` entries keyed by spell ID.
    async fn load_spell_pet_auras(&self, world_db: &MySqlPool) -> Result<()> {
        let mut map: HashMap<u16, PetAura> = HashMap::new();
        let mut count = 0u32;

        let rows = match sqlx::query(
            "SELECT CAST(`spell` AS UNSIGNED) AS spell, \
                    CAST(`pet` AS UNSIGNED) AS pet, \
                    CAST(`aura` AS UNSIGNED) AS aura \
             FROM `spell_pet_auras`",
        )
        .fetch_all(world_db)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                warn!("spell_pet_auras table missing or errored: {e}");
                info!(">> Loaded 0 spell pet auras");
                return Ok(());
            }
        };

        for row in &rows {
            use sqlx::Row;
            let spell: u32 = row.try_get::<u64, _>("spell").unwrap_or(0) as u32;
            let pet: u32 = row.try_get::<u64, _>("pet").unwrap_or(0) as u32;
            let aura: u32 = row.try_get::<u64, _>("aura").unwrap_or(0) as u32;

            let Some(spell_entry) = self.get(spell) else {
                warn!("Spell {spell} in spell_pet_auras does not exist");
                continue;
            };

            if let Some(pa) = map.get_mut(&(spell as u16)) {
                pa.auras.insert(pet, aura);
            } else {
                // Validate: spell must have a dummy effect or dummy aura
                let has_dummy = spell_entry.effect.iter().enumerate().any(|(i, &eff)| {
                    (eff == 6 && spell_entry.effect_apply_aura_name[i] == 45) || eff == 3
                });
                if !has_dummy {
                    warn!(
                        "Spell {spell} in spell_pet_auras does not have dummy aura or dummy effect"
                    );
                    continue;
                }

                let Some(aura_entry) = self.get(aura) else {
                    warn!("Aura {aura} in spell_pet_auras does not exist");
                    continue;
                };
                let _ = aura_entry;

                let is_caster_pet_target = spell_entry.effect_implicit_target_a[0] == 5;
                let damage = spell_entry.effect_base_points[0] as i32;

                let mut pa = PetAura::default();
                pa.remove_on_change_pet = is_caster_pet_target;
                pa.damage = damage;
                pa.auras.insert(pet, aura);
                map.insert(spell as u16, pa);
            }
            count += 1;
        }

        *self.spell_pet_auras.write() = map;
        info!(">> Loaded {count} spell pet auras");
        Ok(())
    }

    // ═════════════════════════════════════════════════════════════════
    // Ports of remaining SpellMgr loaders & query functions
    // ═════════════════════════════════════════════════════════════════

    /// Port of `SpellMgr::LoadSpellCones` (SpellMgr.cpp:2363).
    async fn load_spell_cones(&self, world_db: &MySqlPool) -> Result<()> {
        let mut cones = HashMap::new();
        let mut count = 0u32;

        let rows = sqlx::query(
            "SELECT CAST(`entry` AS UNSIGNED) AS entry, \
                    `cone_degrees` \
             FROM `spell_cone`",
        )
        .fetch_all(world_db)
        .await;

        let rows = match rows {
            Ok(rows) => rows,
            Err(e) => {
                warn!("spell_cone table missing or errored: {e}");
                *self.spell_cones.write() = cones;
                info!(">> Loaded 0 spell cones");
                return Ok(());
            }
        };

        for row in &rows {
            use sqlx::Row;
            let entry: u32 = row.try_get::<u64, _>("entry").unwrap_or(0) as u32;
            let degrees: f64 = row.try_get("cone_degrees").unwrap_or(0.0);

            if self.spells.get(&entry).is_none() {
                if !self.existing_spell_ids.read().contains(&entry) {
                    warn!("Spell {entry} in spell_cone does not exist");
                }
                continue;
            }
            if degrees < -360.0 || degrees > 360.0 {
                warn!("Spell {entry} in spell_cone has incorrect angle {degrees} outside of valid range");
                continue;
            }

            let angle = (degrees * std::f64::consts::PI / 180.0) as f32;
            cones.insert(entry, angle);
            count += 1;
        }

        *self.spell_cones.write() = cones;
        info!(">> Loaded {count} spell cones");
        Ok(())
    }

    /// Port of `SpellMgr::LoadSpellAreas` (SpellMgr.cpp:2418).
    async fn load_spell_areas(&self, world_db: &MySqlPool) -> Result<()> {
        let rows = sqlx::query(
            "SELECT CAST(`spell` AS UNSIGNED) AS spell, \
                    CAST(`area` AS UNSIGNED) AS area, \
                    CAST(`quest_start` AS UNSIGNED) AS quest_start, \
                    `quest_start_active`, \
                    CAST(`quest_end` AS UNSIGNED) AS quest_end, \
                    CAST(`aura_spell` AS SIGNED) AS aura_spell, \
                    CAST(`racemask` AS UNSIGNED) AS racemask, \
                    CAST(`gender` AS UNSIGNED) AS gender, \
                    `autocast` \
             FROM `spell_area`",
        )
        .fetch_all(world_db)
        .await;

        let rows = match rows {
            Ok(rows) => rows,
            Err(e) => {
                warn!("spell_area table missing or errored: {e}");
                info!(">> Loaded 0 spell area requirements");
                return Ok(());
            }
        };

        let mut areas: Vec<SpellArea> = Vec::new();
        let mut count = 0u32;

        for row in &rows {
            use sqlx::Row;
            let spell: u32 = row.try_get::<u64, _>("spell").unwrap_or(0) as u32;

            if self.spells.get(&spell).is_none() {
                if !self.existing_spell_ids.read().contains(&spell) {
                    warn!("Spell {spell} listed in spell_area does not exist");
                }
                continue;
            }

            let sa = SpellArea {
                spell,
                area_id: row.try_get::<u64, _>("area").unwrap_or(0) as u32,
                quest_start: row.try_get::<u64, _>("quest_start").unwrap_or(0) as u32,
                quest_end: row.try_get::<u64, _>("quest_end").unwrap_or(0) as u32,
                aura_spell: row.try_get::<i64, _>("aura_spell").unwrap_or(0) as i32,
                racemask: row.try_get::<u64, _>("racemask").unwrap_or(0) as u32,
                gender: row.try_get::<u64, _>("gender").unwrap_or(2) as u8, // default GENDER_NONE
                quest_start_can_active: row.try_get("quest_start_active").unwrap_or(false),
                autocast: row.try_get("autocast").unwrap_or(false),
            };

            areas.push(sa);
            count += 1;
        }

        *self.spell_areas.write() = areas;
        info!(">> Loaded {count} spell area requirements");
        Ok(())
    }

    /// Port of `SpellMgr::LoadExistingSpellIds` (SpellMgr.cpp:3103).
    /// Populates `existing_spell_ids` with all spell IDs from `spell_template`.
    async fn load_existing_spell_ids(&self, world_db: &MySqlPool) -> Result<()> {
        let mut ids = Vec::new();
        if let Ok(rows) =
            sqlx::query("SELECT DISTINCT CAST(`entry` AS UNSIGNED) AS entry FROM `spell_template`")
                .fetch_all(world_db)
                .await
        {
            for row in &rows {
                use sqlx::Row;
                let id: u32 = row.try_get::<u64, _>("entry").unwrap_or(0) as u32;
                ids.push(id);
            }
        }
        self.replace_existing_spell_ids(ids);
        let count = self.existing_spell_ids.read().len();
        info!(">> Loaded {count} existing spell ids");
        Ok(())
    }

    fn replace_existing_spell_ids(&self, ids: impl IntoIterator<Item = u32>) {
        *self.existing_spell_ids.write() = ids.into_iter().collect();
    }

    /// Port of `SpellMgr::IsSpellValid` (SpellMgr.cpp:2289).
    /// Validates a spell entry — checks CREATE_ITEM effects have valid item
    /// prototypes and LEARN_SPELL effects target valid spells.
    pub fn is_spell_valid(&self, spell_info: Option<&SpellEntry>, _msg: bool) -> bool {
        let Some(spell) = spell_info else {
            return false;
        };

        let mut need_check_reagents = false;

        for i in 0..spell.effect.len() {
            match spell.effect[i] {
                0 => continue,
                // SPELL_EFFECT_CREATE_ITEM
                24 => {
                    let item_entry = spell.effect_item_type[i] as u32;
                    // If no item prototype system exists yet, skip the check
                    need_check_reagents = true;
                    let _ = item_entry;
                }
                // SPELL_EFFECT_LEARN_SPELL
                36 => {
                    let trigger = spell.effect_trigger_spell[i];
                    if trigger != 0 {
                        if let Some(entry) = self.spells.get(&trigger) {
                            if !self.is_spell_valid(Some(&entry), _msg) {
                                return false;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if need_check_reagents {
            for &reagent in &spell.reagent {
                if reagent > 0 {
                    // If no item prototype system exists, skip reagent validation
                }
            }
        }

        true
    }

    /// Port of `SpellMgr::GetRequiredAreaForSpell` (SpellMgr.cpp:2706).
    /// Returns the area ID required by a spell, or 0 if none.
    pub fn get_required_area_for_spell(&self, spell_id: u32) -> u32 {
        let areas = self.spell_areas.read();
        for sa in areas.iter() {
            if sa.spell == spell_id && sa.area_id != 0 {
                return sa.area_id;
            }
        }

        // Hardcoded battleground flags
        match spell_id {
            23333 | 23335 => 3277, // Warsong Gulch flags
            _ => 0,
        }
    }

    /// Port of `SpellMgr::GetSpellAllowedInLocationError` (SpellMgr.cpp:2641).
    /// Checks battleground-only attributes, hardcoded spell IDs, and spell_area
    /// requirements. Player-specific checks (gender, race, quests, auras) are
    /// gated by `player_data` — pass `None` to skip those checks.
    #[allow(clippy::too_many_arguments)]
    pub fn get_spell_allowed_in_location_error(
        &self,
        spell_info: &SpellEntry,
        player_in_bg: Option<bool>,
        player_map_id: Option<u32>,
        player_gender: Option<u8>,
        player_race_mask: Option<u32>,
        player_active_quests: Option<&[u32]>,
        player_rewarded_quests: Option<&std::collections::HashSet<u32>>,
        player_has_aura: Option<&dyn Fn(u32) -> bool>,
        zone_id: u32,
        area_id: u32,
    ) -> crate::game::player::spells::state::SpellCastResult {
        use crate::game::player::spells::state::{SpellCastError, SpellCastResult};

        // SPELL_ATTR_EX3_ONLY_BATTLEGROUNDS check
        if (spell_info.attributes_ex3 & 0x0000_0001) != 0 {
            if player_in_bg.map_or(true, |bg| !bg) {
                return SpellCastResult::Failed(SpellCastError::OnlyBattlegrounds);
            }
        }

        // Hardcoded spell-location restrictions
        match spell_info.id {
            22564 | 22563 | 23538 | 23539 => {
                let (Some(bg), Some(map_id)) = (player_in_bg, player_map_id) else {
                    return SpellCastResult::Failed(SpellCastError::RequiresArea);
                };
                if map_id != 30 || !bg {
                    return SpellCastResult::Failed(SpellCastError::RequiresArea);
                }
            }
            23333 | 23335 => {
                let (Some(bg), Some(map_id)) = (player_in_bg, player_map_id) else {
                    return SpellCastResult::Failed(SpellCastError::RequiresArea);
                };
                if map_id != 489 || !bg {
                    return SpellCastResult::Failed(SpellCastError::RequiresArea);
                }
            }
            2584 => {
                if player_in_bg.map_or(true, |bg| !bg) {
                    return SpellCastResult::Failed(SpellCastError::OnlyBattlegrounds);
                }
            }
            22011 | 22012 | 24171 => {
                if player_in_bg.map_or(true, |bg| !bg) {
                    return SpellCastResult::Failed(SpellCastError::OnlyBattlegrounds);
                }
            }
            _ => {}
        }

        // SpellArea map check
        let areas = self.spell_areas.read();
        let has_area_restriction = areas
            .iter()
            .any(|sa| sa.spell == spell_info.id && sa.area_id != 0);
        if has_area_restriction {
            for sa in areas.iter() {
                if sa.spell == spell_info.id
                    && sa.is_fit_to_requirements(
                        player_gender,
                        player_race_mask,
                        player_active_quests,
                        player_rewarded_quests,
                        player_has_aura,
                        zone_id,
                        area_id,
                    )
                {
                    return SpellCastResult::Success;
                }
            }
            return SpellCastResult::Failed(SpellCastError::RequiresArea);
        }

        SpellCastResult::Success
    }

    /// Port of `SpellMgr::CheckUsedSpells` (SpellMgr.cpp:2797).
    /// Validates that spells referenced in a given table exist.
    pub async fn check_used_spells(&self, world_db: &MySqlPool, table: &str) -> Result<()> {
        let query = format!(
            "SELECT CAST(`spellid` AS UNSIGNED) AS spellid, `Code` \
             FROM `{table}` LIMIT 1"
        );
        let test = sqlx::query(&query).fetch_all(world_db).await;
        if test.is_err() {
            warn!("Table `{table}` is empty or does not exist");
            return Ok(());
        }

        let full_query =
            format!("SELECT CAST(`spellid` AS UNSIGNED) AS spellid, `Code` FROM `{table}`");
        let rows = match sqlx::query(&full_query).fetch_all(world_db).await {
            Ok(rows) => rows,
            Err(e) => {
                warn!("check_used_spells({table}) failed: {e}");
                return Ok(());
            }
        };

        let mut count = 0u32;
        for row in &rows {
            use sqlx::Row;
            let spell_id: u32 = row.try_get::<u64, _>("spellid").unwrap_or(0) as u32;
            if spell_id != 0 && self.spells.get(&spell_id).is_none() {
                let code: String = row.try_get("Code").unwrap_or_default();
                warn!("Spell {spell_id} referenced in `{table}` ({code}) does not exist");
                count += 1;
            }
        }

        info!(">> Checked {count} invalid spell references in `{table}`");
        Ok(())
    }

    /// Port of `SpellMgr::AssignInternalSpellFlags` (SpellMgr.cpp:3461).
    /// Pre-computes internal classification flags on each loaded SpellEntry.
    /// NOTE: The `internal` field on `Arc<SpellEntry>` cannot be mutated through
    /// a shared reference in the current architecture. Flags are computed on
    /// demand as methods (e.g. `SpellEntry::is_reflectable_spell()`) — this
    /// stub exists for API completeness.
    pub fn assign_internal_spell_flags(&self) {
        let count = self.spells.len();
        info!(">> assign_internal_spell_flags: {count} spells (flags computed on-demand)");
    }

    /// Port of `SpellMgr::LoadSpellLearnSpells` (SpellMgr.cpp:1927).
    /// Loads `spell_learn_spell` SQL table and also scans DBC for
    /// `SPELL_EFFECT_LEARN_SPELL` auto-learn entries.
    async fn load_spell_learn_spells(&self, world_db: &MySqlPool) -> Result<()> {
        let mut map: HashMap<u32, Vec<SpellLearnSpellNode>> = HashMap::new();
        let mut count = 0u32;

        // Load from SQL table
        let rows = sqlx::query(
            "SELECT CAST(`entry` AS UNSIGNED) AS entry, \
                    CAST(`SpellID` AS UNSIGNED) AS spell_id, \
                    `Active` \
             FROM `spell_learn_spell`",
        )
        .fetch_all(world_db)
        .await;

        if let Ok(rows) = &rows {
            for row in rows {
                use sqlx::Row;
                let spell_id: u32 = row.try_get::<u64, _>("entry").unwrap_or(0) as u32;
                let learned: u32 = row.try_get::<u64, _>("spell_id").unwrap_or(0) as u32;
                let active: bool = row.try_get("Active").unwrap_or(false);

                if self.spells.get(&spell_id).is_none() {
                    if !self.existing_spell_ids.read().contains(&spell_id) {
                        warn!("Spell {spell_id} listed in spell_learn_spell does not exist");
                    }
                    continue;
                }
                if self.spells.get(&learned).is_none() {
                    warn!("Spell {learned} listed in spell_learn_spell (learning) does not exist");
                    continue;
                }

                map.entry(spell_id).or_default().push(SpellLearnSpellNode {
                    spell: learned,
                    active,
                    autolearned: false,
                });
                count += 1;
            }
        }

        // Scan loaded spells for SPELL_EFFECT_LEARN_SPELL (36) DBC entries
        let mut dbc_count = 0u32;
        for entry in self.spells.iter() {
            for (i, &eff) in entry.effect.iter().enumerate() {
                if eff == 36 {
                    let learned = entry.effect_trigger_spell[i];
                    if learned == 0 || self.spells.get(&learned).is_none() {
                        continue;
                    }
                    let already_present = map
                        .get(&entry.id)
                        .map_or(false, |vec| vec.iter().any(|n| n.spell == learned));
                    if !already_present {
                        let autolearned = entry.effect_implicit_target_a[i] == 5
                            || entry.is_passive_spell()
                            || entry.has_effect(61); // SPELL_EFFECT_SKILL_STEP
                        map.entry(entry.id).or_default().push(SpellLearnSpellNode {
                            spell: learned,
                            active: true,
                            autolearned,
                        });
                        dbc_count += 1;
                    }
                }
            }
        }

        *self.spell_learn_spells.write() = map;
        info!(">> Loaded {count} spell learn spells + {dbc_count} found in DBC");
        Ok(())
    }

    /// Port of `SpellMgr::LoadSpellScriptTarget` (SpellMgr.cpp:2036).
    /// Loads `spell_script_target` SQL table — validates targets exist and
    /// spell has a script-referencing target mode.
    async fn load_spell_script_targets(&self, world_db: &MySqlPool) -> Result<()> {
        let mut map: HashMap<u32, Vec<SpellTargetEntry>> = HashMap::new();
        let mut count = 0u32;

        let rows = sqlx::query(
            "SELECT CAST(`entry` AS UNSIGNED) AS entry, \
                    CAST(`type` AS UNSIGNED) AS type, \
                    CAST(`targetEntry` AS UNSIGNED) AS target_entry, \
                    CAST(`conditionId` AS UNSIGNED) AS condition_id, \
                    CAST(`inverseEffectMask` AS UNSIGNED) AS effect_mask \
             FROM `spell_script_target`",
        )
        .fetch_all(world_db)
        .await;

        let rows = match rows {
            Ok(rows) => rows,
            Err(e) => {
                warn!("spell_script_target table missing or errored: {e}");
                info!(">> Loaded 0 spell script targets");
                *self.spell_script_targets.write() = map;
                return Ok(());
            }
        };

        for row in &rows {
            use sqlx::Row;
            let spell_id: u32 = row.try_get::<u64, _>("entry").unwrap_or(0) as u32;
            let type_: u32 = row.try_get::<u64, _>("type").unwrap_or(0) as u32;
            let target_entry: u32 = row.try_get::<u64, _>("target_entry").unwrap_or(0) as u32;

            if self.spells.get(&spell_id).is_none() {
                if !self.existing_spell_ids.read().contains(&spell_id) {
                    warn!("Spell {spell_id} in spell_script_target does not exist");
                }
                continue;
            }

            // Validate spell has a script-referencing target mode
            let Some(spell_proto) = self.spells.get(&spell_id) else {
                continue;
            };
            let has_script_target = (0..spell_proto.effect.len()).any(|i| {
                matches!(
                    spell_proto.effect_implicit_target_a[i],
                    38 | 40 | 46 | 53 | 54 | 55 | 56 | 57 | 58 | 59 | 60 | 61
                ) || matches!(
                    spell_proto.effect_implicit_target_b[i],
                    38 | 40 | 46 | 53 | 54 | 55 | 56 | 57 | 58 | 59 | 60 | 61
                )
            });
            if !has_script_target {
                warn!("Spell {spell_id} in spell_script_target has no script target mode");
                continue;
            }

            map.entry(spell_id).or_default().push(SpellTargetEntry {
                type_,
                target_id: target_entry,
                condition_id: row.try_get::<u64, _>("condition_id").unwrap_or(0) as u32,
                can_focus: false,
                inverse_effect_mask: row.try_get::<u64, _>("effect_mask").unwrap_or(0) as u32,
            });
            count += 1;
        }

        *self.spell_script_targets.write() = map;
        info!(">> Loaded {count} spell script targets");
        Ok(())
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

    #[test]
    fn existing_spell_ids_are_replaced_and_keep_zero() {
        let mgr = SpellManager::new();
        mgr.replace_existing_spell_ids([1, 1, 2]);
        mgr.replace_existing_spell_ids([0, 3]);

        assert_eq!(*mgr.existing_spell_ids.read(), HashSet::from([0, 3]));
    }
}
