//! Creature Manager - owns all creatures and templates

use anyhow::{Context, Result};
use dashmap::DashMap;
use sqlx::MySqlPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::death::DeathState;
use super::movement::generators::{RandomMovementGenerator, WaypointMovementGenerator};
use super::movement::WaypointManager;
use super::{Creature, CreatureSpawnData};
use crate::shared::database::world::repositories::CreatureRepository;
use crate::shared::protocol::{ObjectGuid, Position};
use crate::world::core::common::compress_update_packet_if_needed;
use crate::world::map::grid_coords::world_to_grid;

/// Creature model info from creature_display_info_addon table
/// Stores bounding_radius, combat_reach, and speed rates per display_id
#[derive(Debug, Clone)]
pub struct CreatureModelInfo {
    pub bounding_radius: f32,
    pub combat_reach: f32,
    /// Walk speed rate multiplier (default 1.0, actual speed = rate * 2.5)
    pub speed_walk: f32,
    /// Run speed rate multiplier (default 1.14286, actual speed = rate * 7.0)
    pub speed_run: f32,
}

/// Creature template from database
#[derive(Debug, Clone)]
pub struct CreatureTemplate {
    pub entry: u32,
    pub name: String,
    pub subname: Option<String>,
    pub min_level: u8,
    pub max_level: u8,
    pub faction: u32,
    pub model_id_1: u32,
    pub model_id_2: u32,
    pub model_id_3: u32,
    pub model_id_4: u32,
    pub scale: f32,
    pub npc_flags: u32,
    pub unit_flags: u32,    // COMPUTED from static_flags1 (client-visible flags)
    pub static_flags1: u32, // Server-side behavior flags from DB
    pub flags_extra: u32,   // From DB flags_extra column (CREATURE_FLAG_EXTRA_*)
    pub creature_type: u8,  // Creature type (CRITTER=8, etc.) for special handling
    pub unit_class: u8,     // Unit class (1=warrior, 2=paladin, 4=rogue, 8=mage)
    pub health_multiplier: f32,
    pub power_multiplier: f32,
    pub armor_multiplier: f32,
    pub damage_multiplier: f32,
    pub damage_variance: f32,
    pub attack_time: u32,
    /// Creature rank: 0=normal, 1=elite, 2=rareelite, 3=worldboss, 4=rare
    pub rank: u8,
    pub gossip_menu_id: u32, // Default gossip menu ID
    pub vendor_id: u32,      // Maps to npc_vendor_template.entry
    pub trainer_id: u32,     // Maps to npc_trainer_template.entry
    pub trainer_type: u8,    // 0=class, 1=mount, 2=tradeskill, 3=pet
    pub spells: [u32; 4],    // Spell IDs from creature_template spell1-4
}

/// Base stats from creature_classlevelstats table
#[derive(Debug, Clone)]
pub struct ClassLevelStats {
    pub melee_damage: f32,
    pub health: i32,
    pub mana: i32,
    pub armor: i32,
    pub attack_power: i32,
}

impl CreatureTemplate {
    /// Calculate health for a given level using classlevelstats
    pub fn calculate_health(&self, level: u8, class_stats: Option<&ClassLevelStats>) -> u32 {
        if let Some(stats) = class_stats {
            let base = stats.health.max(1) as f32;
            (base * self.health_multiplier).max(1.0) as u32
        } else {
            // Fallback: simple level-based formula
            let base = 20 + (level as u32 * 10);
            (base as f32 * self.health_multiplier).max(1.0) as u32
        }
    }

    /// Calculate mana for a given level using classlevelstats
    pub fn calculate_mana(&self, _level: u8, class_stats: Option<&ClassLevelStats>) -> u32 {
        if let Some(stats) = class_stats {
            let base = stats.mana.max(0) as f32;
            (base * self.power_multiplier) as u32
        } else {
            0
        }
    }

    /// Calculate armor for a given level using classlevelstats
    pub fn calculate_armor(&self, level: u8, class_stats: Option<&ClassLevelStats>) -> u32 {
        if let Some(stats) = class_stats {
            let base = stats.armor.max(0) as f32;
            (base * self.armor_multiplier) as u32
        } else {
            (level as u32) * 30
        }
    }

    /// Calculate damage range for a given level using classlevelstats
    /// Returns (damage_min, damage_max)
    pub fn calculate_damage(&self, level: u8, class_stats: Option<&ClassLevelStats>) -> (u32, u32) {
        let base_dmg = if let Some(stats) = class_stats {
            stats.melee_damage.max(0.1)
        } else {
            level as f32 * 2.0 + 2.0
        };

        let scaled = base_dmg * self.damage_multiplier;
        let min = (scaled * (1.0 - self.damage_variance)).max(1.0) as u32;
        let max = (scaled * (1.0 + self.damage_variance)).max(1.0) as u32;
        (min, max.max(min))
    }

    /// Get the first non-zero display_id from the 4 available options.
    pub fn get_display_id(&self) -> u32 {
        let display_ids = [
            self.model_id_1,
            self.model_id_2,
            self.model_id_3,
            self.model_id_4,
        ];
        display_ids.iter().find(|&&id| id > 0).copied().unwrap_or(0)
    }
}

/// Tracks spawn state for grid loading
#[derive(Debug, Clone)]
struct SpawnState {
    /// Whether this spawn is currently active in the world
    spawned: bool,
    /// When the creature should respawn (if dead)
    respawn_time: Option<std::time::Instant>,
}

/// Manages all creatures and their templates
pub struct CreatureManager {
    /// Database pool for loading data
    world_db: Arc<MySqlPool>,
    /// Templates by entry ID
    templates: DashMap<u32, Arc<CreatureTemplate>>,
    /// Active creatures by GUID
    creatures: DashMap<ObjectGuid, Creature>,
    /// Spawn data by map_id -> list of spawns
    spawns_by_map: DashMap<u32, Vec<CreatureSpawnData>>,
    /// Track spawn states by spawn_id
    spawn_states: DashMap<u32, SpawnState>,
    /// Track which creature GUID belongs to which spawn_id
    guid_to_spawn: DashMap<ObjectGuid, u32>,
    /// GUID counter
    next_guid: AtomicU64,
    /// Current game patch for filtering spawns (e.g., 10 for patch 1.10)
    current_patch: std::sync::RwLock<u8>,
    /// Base stats per (class, level) from creature_classlevelstats
    class_level_stats: DashMap<(u8, u8), ClassLevelStats>,
    /// Model info (bounding_radius, combat_reach) per display_id from creature_display_info_addon
    model_info: DashMap<u32, CreatureModelInfo>,
}

impl CreatureManager {
    pub fn new(world_db: Arc<MySqlPool>) -> Self {
        Self {
            world_db,
            templates: DashMap::new(),
            creatures: DashMap::new(),
            spawns_by_map: DashMap::new(),
            spawn_states: DashMap::new(),
            guid_to_spawn: DashMap::new(),
            next_guid: AtomicU64::new(1),
            current_patch: std::sync::RwLock::new(10),
            class_level_stats: DashMap::new(),
            model_info: DashMap::new(),
        }
    }

    /// Set the current game patch for filtering creature spawns
    pub fn set_patch(&self, patch: u8) {
        if let Ok(mut guard) = self.current_patch.write() {
            *guard = patch;
            tracing::debug!("CreatureManager patch set to {}", patch);
        }
    }

    /// Get the current game patch
    pub fn get_patch(&self) -> u8 {
        self.current_patch.read().map(|g| *g).unwrap_or(10)
    }

    /// Translate static_flags1 (server-side flags) to unit_flags (client-visible flags)
    ///
    /// Based on legacy_world toggle_unit_flags_from_static_flags() with special handling:
    /// - Critters (type=8) and guards always have NOT_SELECTABLE removed
    /// - Creatures with NPC flags always have NOT_SELECTABLE removed
    fn compute_unit_flags_from_static(
        static_flags1: u32,
        creature_type: u8,
        npc_flags: u32,
        flags_extra: u32,
    ) -> u32 {
        use crate::world::game::common::creature_flags::{
            CREATURE_STATIC_FLAG_IMMUNE_TO_NPC, CREATURE_STATIC_FLAG_IMMUNE_TO_PC,
            CREATURE_STATIC_FLAG_UNINTERACTIBLE,
        };
        use crate::world::game::common::unit_flags::{
            IMMUNE_TO_NPC, IMMUNE_TO_PLAYER, NOT_SELECTABLE,
        };

        let mut unit_flags = 0u32;

        if (static_flags1 & CREATURE_STATIC_FLAG_IMMUNE_TO_PC) != 0 {
            unit_flags |= IMMUNE_TO_PLAYER;
        }

        if (static_flags1 & CREATURE_STATIC_FLAG_IMMUNE_TO_NPC) != 0 {
            unit_flags |= IMMUNE_TO_NPC;
        }

        if (static_flags1 & CREATURE_STATIC_FLAG_UNINTERACTIBLE) != 0 {
            unit_flags |= NOT_SELECTABLE;
        }

        // Critters must have unit_flags=0 (fully attackable/selectable)
        // MaNGOS: critters (Rabbit, Squirrel, etc.) have unit_flags=0 in DB
        // The static_flags IMMUNE_TO_NPC/UNINTERACTIBLE are server-side AI flags
        // that should NOT translate to client unit_flags for critters
        const CREATURE_TYPE_CRITTER: u8 = 8;
        if creature_type == CREATURE_TYPE_CRITTER {
            unit_flags = 0;
            return unit_flags;
        }

        // Guards must always be selectable
        const CREATURE_FLAG_EXTRA_GUARD: u32 = 0x00000400;
        if (flags_extra & CREATURE_FLAG_EXTRA_GUARD) != 0 {
            unit_flags &= !NOT_SELECTABLE;
        }

        // Creatures with NPC flags must be selectable (vendors, trainers, quest givers)
        if npc_flags != 0 {
            unit_flags &= !NOT_SELECTABLE;
        }

        unit_flags
    }

    /// Get a creature template by entry
    pub fn get_template(&self, entry: u32) -> Option<Arc<CreatureTemplate>> {
        self.templates.get(&entry).map(|r| Arc::clone(&r))
    }

    /// Iterate over all loaded templates
    pub fn all_templates(&self) -> Vec<Arc<CreatureTemplate>> {
        self.templates
            .iter()
            .map(|r| Arc::clone(r.value()))
            .collect()
    }

    /// Add a template
    pub fn add_template(&self, template: CreatureTemplate) {
        self.templates.insert(template.entry, Arc::new(template));
    }

    /// Get a creature by GUID
    pub fn get_creature(
        &self,
        guid: ObjectGuid,
    ) -> Option<dashmap::mapref::one::Ref<'_, ObjectGuid, Creature>> {
        self.creatures.get(&guid)
    }

    /// Add a creature
    pub fn add_creature(&self, creature: Creature) {
        self.creatures.insert(creature.guid, creature);
    }

    /// Remove a creature
    pub fn remove_creature(&self, guid: ObjectGuid) -> Option<(ObjectGuid, Creature)> {
        // Also remove from spawn tracking
        if let Some(spawn_id) = self.guid_to_spawn.remove(&guid) {
            if let Some(mut state) = self.spawn_states.get_mut(&spawn_id.1) {
                state.spawned = false;
            }
        }
        self.creatures.remove(&guid)
    }

    /// Check if a spawn is already active
    pub fn has_spawn(&self, spawn_id: u32) -> bool {
        self.spawn_states
            .get(&spawn_id)
            .map(|s| s.spawned)
            .unwrap_or(false)
    }

    /// Generate a new creature GUID
    pub fn generate_guid(&self) -> u64 {
        self.next_guid.fetch_add(1, Ordering::Relaxed)
    }

    /// Load creature templates from database
    pub async fn load_templates(&self) -> Result<()> {
        let repo = CreatureRepository::new(Arc::clone(&self.world_db));

        // Load class level stats first (needed for stat calculations)
        let cls_rows = repo
            .load_class_level_stats()
            .await
            .context("Failed to load creature class level stats")?;

        for row in &cls_rows {
            self.class_level_stats.insert(
                (row.class, row.level),
                ClassLevelStats {
                    melee_damage: row.melee_damage,
                    health: row.health,
                    mana: row.mana,
                    armor: row.armor,
                    attack_power: row.attack_power,
                },
            );
        }
        tracing::info!(
            "Loaded {} creature class level stat entries",
            self.class_level_stats.len()
        );

        // Load templates
        let rows = repo
            .load_all_templates()
            .await
            .context("Failed to load creature templates from database")?;

        for row in rows {
            let unit_flags = Self::compute_unit_flags_from_static(
                row.static_flags1,
                row.creature_type,
                row.npc_flags,
                row.flags_extra,
            );

            let template = CreatureTemplate {
                entry: row.entry,
                name: row.name,
                subname: row.subname,
                min_level: row.level_min,
                max_level: row.level_max,
                faction: row.faction as u32,
                model_id_1: row.display_id1,
                model_id_2: row.display_id2,
                model_id_3: row.display_id3,
                model_id_4: row.display_id4,
                scale: row.display_scale1,
                npc_flags: row.npc_flags,
                unit_flags,
                static_flags1: row.static_flags1,
                flags_extra: row.flags_extra,
                creature_type: row.creature_type,
                unit_class: row.unit_class,
                health_multiplier: row.health_multiplier,
                power_multiplier: row.mana_multiplier,
                armor_multiplier: row.armor_multiplier,
                damage_multiplier: row.damage_multiplier,
                damage_variance: row.damage_variance,
                attack_time: row.base_attack_time,
                rank: row.rank,
                gossip_menu_id: row.gossip_menu_id,
                vendor_id: row.vendor_id,
                trainer_id: row.trainer_id,
                trainer_type: row.trainer_type,
                spells: [row.spell_id1, row.spell_id2, row.spell_id3, row.spell_id4],
            };
            self.templates.insert(template.entry, Arc::new(template));
        }
        tracing::info!("Loaded {} creature templates", self.templates.len());

        Ok(())
    }

    /// Get class level stats for a given class and level
    pub fn get_class_level_stats(&self, unit_class: u8, level: u8) -> Option<ClassLevelStats> {
        self.class_level_stats
            .get(&(unit_class, level))
            .map(|r| r.clone())
    }

    /// Load creature_display_info_addon from database (bounding_radius, combat_reach, speeds per display_id)
    pub async fn load_model_info(&self) -> Result<()> {
        let rows: Result<Vec<(u32, f32, f32, f32, f32)>, _> = sqlx::query_as(
            "SELECT display_id, bounding_radius, combat_reach, speed_walk, speed_run FROM creature_display_info_addon"
        )
        .fetch_all(self.world_db.as_ref())
        .await;

        match rows {
            Ok(rows) => {
                for (display_id, bounding_radius, combat_reach, speed_walk, speed_run) in &rows {
                    self.model_info.insert(
                        *display_id,
                        CreatureModelInfo {
                            bounding_radius: if *bounding_radius > 0.0 {
                                *bounding_radius
                            } else {
                                0.5
                            },
                            combat_reach: if *combat_reach > 0.0 {
                                *combat_reach
                            } else {
                                1.5
                            },
                            speed_walk: if *speed_walk > 0.0 { *speed_walk } else { 1.0 },
                            speed_run: if *speed_run > 0.0 {
                                *speed_run
                            } else {
                                1.14286
                            },
                        },
                    );
                }
                tracing::info!(
                    "Loaded {} creature model info entries",
                    self.model_info.len()
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Could not load creature_display_info_addon (table may not exist): {}. Using defaults.",
                    e
                );
            }
        }

        Ok(())
    }

    /// Get model info for a display_id
    pub fn get_model_info(&self, display_id: u32) -> Option<CreatureModelInfo> {
        self.model_info.get(&display_id).map(|r| r.clone())
    }

    /// Load creature spawns from database
    pub async fn load_spawns(&self) -> Result<()> {
        let repo = CreatureRepository::new(Arc::clone(&self.world_db));
        let rows = repo
            .load_all_spawns()
            .await
            .context("Failed to load creature spawns from database")?;

        let current_patch = self.get_patch();
        let mut skipped_patch = 0;

        for row in rows {
            // Check patch compatibility
            let exists_in_patch = row.patch_min <= current_patch && current_patch <= row.patch_max;

            // Skip creatures not in current patch
            if !exists_in_patch {
                skipped_patch += 1;
                continue;
            }

            // NOTE: Not filtering by game_event or pool - world doesn't have these systems yet
            // so we spawn ALL creatures regardless of event/pool membership

            let spawn = CreatureSpawnData {
                spawn_id: row.guid,
                entry: row.id,
                map_id: row.map,
                position: Position {
                    x: row.position_x,
                    y: row.position_y,
                    z: row.position_z,
                    o: row.orientation,
                },
                spawntimesecs: row.spawntimesecsmin,
                wander_distance: row.wander_distance,
                movement_type: row.movement_type,
                phase_mask: 1,
                spawn_flags: super::spawn::spawn_flags::RANDOM_RESPAWN_TIME, // Default to random variance
            };

            self.spawns_by_map
                .entry(spawn.map_id)
                .or_insert_with(Vec::new)
                .push(spawn);
        }

        let total: usize = self.spawns_by_map.iter().map(|e| e.value().len()).sum();
        tracing::info!(
            "Loaded {} creature spawns across {} maps (skipped {} for patch)",
            total,
            self.spawns_by_map.len(),
            skipped_patch
        );

        Ok(())
    }

    /// Get spawns for a specific grid on a map
    pub fn get_spawns_for_grid(
        &self,
        map_id: u32,
        grid_x: u8,
        grid_y: u8,
    ) -> Vec<CreatureSpawnData> {
        let mut result = Vec::new();

        if let Some(spawns) = self.spawns_by_map.get(&map_id) {
            for spawn in spawns.iter() {
                let (spawn_gx, spawn_gy) = world_to_grid(spawn.position.x, spawn.position.y);
                if spawn_gx == grid_x && spawn_gy == grid_y {
                    result.push(spawn.clone());
                }
            }
        }

        result
    }

    /// Spawn a single creature from spawn data
    /// Returns the GUID of the spawned creature, or None if failed
    pub fn spawn_creature(
        &self,
        spawn: &CreatureSpawnData,
        instance_id: u32,
    ) -> Option<ObjectGuid> {
        // spawn_id == 0 means a transient script-created summon — no dedup check.
        // For DB spawns (spawn_id > 0) skip if already spawned.
        if spawn.spawn_id != 0 && self.has_spawn(spawn.spawn_id) {
            return None;
        }

        let template = self.get_template(spawn.entry)?;
        let class_stats = self.get_class_level_stats(template.unit_class, template.min_level);

        let counter = self.next_guid.fetch_add(1, Ordering::Relaxed);
        let guid = ObjectGuid::new_creature(spawn.entry, counter as u32);

        let mut creature = Creature::new(
            guid,
            spawn.entry,
            spawn.spawn_id,
            spawn.position,
            spawn.map_id,
            instance_id,
            &template,
            spawn.phase_mask,
            class_stats.as_ref(),
        );

        // Apply model info (bounding_radius, combat_reach, speeds) from creature_display_info_addon
        if let Some(model_info) = self.get_model_info(creature.display_id) {
            creature.bounding_radius = model_info.bounding_radius;
            creature.combat_reach = model_info.combat_reach;
            creature.speed_walk = model_info.speed_walk;
            creature.speed_run = model_info.speed_run;
        }

        // Store wander distance for restoring wander after combat
        creature.wander_distance = spawn.wander_distance;

        self.creatures.insert(guid, creature);

        // Track spawn state
        // Only track spawn state for DB-backed spawns (spawn_id > 0).
        // Transient script summons (spawn_id == 0) are not tracked for respawn.
        if spawn.spawn_id != 0 {
            self.spawn_states.insert(
                spawn.spawn_id,
                SpawnState {
                    spawned: true,
                    respawn_time: None,
                },
            );
            self.guid_to_spawn.insert(guid, spawn.spawn_id);
        }

        Some(guid)
    }

    /// Initialize default movement for a spawned creature based on spawn data
    ///
    /// Called after spawn_creature() to set up random/waypoint movement.
    /// Must be called separately because it needs WaypointManager.
    pub fn initialize_creature_movement(
        &self,
        guid: ObjectGuid,
        spawn: &CreatureSpawnData,
        waypoint_mgr: Option<&WaypointManager>,
    ) {
        self.with_creature_mut(guid, |creature| {
            match spawn.movement_type {
                0 => {
                    // Idle - default, no special movement
                }
                1 => {
                    // Random movement
                    if spawn.wander_distance > 0.0 {
                        let walk_speed = creature.walk_speed();
                        let gen = RandomMovementGenerator::new(
                            spawn.position,
                            spawn.wander_distance,
                            walk_speed,
                        );
                        creature
                            .motion_master
                            .add_generator(Box::new(gen), guid, spawn.position);
                    }
                }
                2 => {
                    // Waypoint movement
                    if let Some(wp_mgr) = waypoint_mgr {
                        if let Some(waypoints) = wp_mgr.get_waypoints(spawn.spawn_id, spawn.entry) {
                            let walk_speed = creature.walk_speed();
                            let gen = WaypointMovementGenerator::new(
                                (*waypoints).clone(),
                                true, // repeating
                                walk_speed,
                            );
                            creature.motion_master.add_generator(
                                Box::new(gen),
                                guid,
                                spawn.position,
                            );
                        }
                    }
                }
                _ => {}
            }
        });
    }

    /// Save respawn state for a creature
    pub fn save_respawn_state(&self, guid: ObjectGuid) {
        if let Some((_, spawn_id)) = self.guid_to_spawn.remove(&guid) {
            if let Some(mut state) = self.spawn_states.get_mut(&spawn_id) {
                state.spawned = false;
                // TODO: Calculate proper respawn time based on template
                state.respawn_time =
                    Some(std::time::Instant::now() + std::time::Duration::from_secs(300));
            }
        }
    }

    /// Spawn all creatures for a given map (legacy method for initial load)
    /// DEPRECATED: Use grid-based loading instead
    pub fn spawn_creatures_for_map(&self, map_id: u32) -> Vec<ObjectGuid> {
        let mut spawned = Vec::new();

        if let Some(spawns) = self.spawns_by_map.get(&map_id) {
            for spawn in spawns.iter() {
                let Some(template) = self.get_template(spawn.entry) else {
                    tracing::warn!(
                        "Skipping spawn {}: template {} not found",
                        spawn.spawn_id,
                        spawn.entry
                    );
                    continue;
                };

                let class_stats =
                    self.get_class_level_stats(template.unit_class, template.min_level);
                let counter = self.next_guid.fetch_add(1, Ordering::Relaxed);
                let guid = ObjectGuid::new_creature(spawn.entry, counter as u32);

                let mut creature = Creature::new(
                    guid,
                    spawn.entry,
                    spawn.spawn_id,
                    spawn.position,
                    spawn.map_id,
                    0, // instance_id = 0 for continents (legacy method)
                    &template,
                    spawn.phase_mask,
                    class_stats.as_ref(),
                );

                // Apply model info (bounding_radius, combat_reach, speeds) from creature_display_info_addon
                if let Some(model_info) = self.get_model_info(creature.display_id) {
                    creature.bounding_radius = model_info.bounding_radius;
                    creature.combat_reach = model_info.combat_reach;
                    creature.speed_walk = model_info.speed_walk;
                    creature.speed_run = model_info.speed_run;
                }

                self.creatures.insert(guid, creature);

                // Track spawn state
                self.spawn_states.insert(
                    spawn.spawn_id,
                    SpawnState {
                        spawned: true,
                        respawn_time: None,
                    },
                );
                self.guid_to_spawn.insert(guid, spawn.spawn_id);

                spawned.push(guid);
            }
        }

        tracing::info!(
            "Spawned {} creatures for map {} (legacy method)",
            spawned.len(),
            map_id
        );

        spawned
    }

    /// Get creature position
    pub fn get_position(&self, guid: ObjectGuid) -> Option<Position> {
        self.creatures.get(&guid).map(|c| c.position)
    }

    /// Get creature combat reach (for melee range calculations)
    pub fn get_combat_reach(&self, guid: ObjectGuid) -> f32 {
        self.creatures
            .get(&guid)
            .map(|c| c.combat_reach)
            .unwrap_or(1.5)
    }

    /// Get creature phase mask
    pub fn get_phase_mask(&self, guid: ObjectGuid) -> Option<u32> {
        self.creatures.get(&guid).map(|c| c.phase_mask)
    }

    /// Get creature static_flags1 (includes VISIBLE_TO_GHOSTS for spirit healers)
    pub fn get_static_flags1(&self, guid: ObjectGuid) -> Option<u32> {
        self.creatures.get(&guid).map(|c| c.static_flags1)
    }

    /// Get creature entry by GUID
    pub fn get_entry(&self, guid: ObjectGuid) -> Option<u32> {
        self.creatures.get(&guid).map(|c| c.entry)
    }

    /// Read-only access to a creature
    pub fn with_creature<F, R>(&self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&Creature) -> R,
    {
        self.creatures.get(&guid).map(|c| f(&*c))
    }

    /// Batched access pattern for thread-safe creature modification
    pub fn with_creature_mut<F, R>(&self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&mut Creature) -> R,
    {
        self.creatures.get_mut(&guid).map(|mut c| f(&mut *c))
    }

    /// Apply damage to a creature with threat tracking (Phase 5: ThreatManager)
    /// Returns (actual_damage, is_dead)
    pub fn apply_damage(
        &self,
        guid: ObjectGuid,
        damage: u32,
        attacker: ObjectGuid,
        timestamp: u64,
    ) -> Option<(u32, bool)> {
        self.creatures.get_mut(&guid).map(|mut creature| {
            // Enter combat if not already
            creature.combat.enter_combat(attacker, timestamp);

            // Add threat using ThreatManager (Phase 5)
            creature
                .threat_manager
                .add_damage_threat(attacker, damage, 1.0);

            // Also update legacy combat threat for backward compatibility
            creature
                .combat
                .add_threat(attacker, damage as f32, timestamp);

            // Apply damage
            let actual_damage = creature.take_damage(damage);
            let is_dead = creature.is_dead();

            (actual_damage, is_dead)
        })
    }

    /// Get creature health info
    pub fn get_health(&self, guid: ObjectGuid) -> Option<(u32, u32)> {
        self.creatures
            .get(&guid)
            .map(|c| (c.current_health, c.max_health))
    }

    /// Check if creature is alive
    pub fn is_alive(&self, guid: ObjectGuid) -> bool {
        self.creatures
            .get(&guid)
            .map(|c| c.is_alive())
            .unwrap_or(false)
    }

    /// Get creature's highest threat target (for AI - Phase 5: ThreatManager)
    pub fn get_highest_threat_target(&self, guid: ObjectGuid) -> Option<ObjectGuid> {
        self.creatures
            .get(&guid)
            .and_then(|c| c.threat_manager.get_victim())
    }

    /// Get creature's threat list (for AI snapshot)
    pub fn get_threat_list(&self, guid: ObjectGuid) -> Vec<(ObjectGuid, f32)> {
        self.creatures
            .get(&guid)
            .map(|c| c.threat_manager.get_threat_list())
            .unwrap_or_default()
    }

    /// Check if creature is in combat
    pub fn is_in_combat(&self, guid: ObjectGuid) -> bool {
        self.creatures
            .get(&guid)
            .map(|c| c.combat.in_combat)
            .unwrap_or(false)
    }

    /// Iterate over all creatures
    pub fn iter_creatures(&self) -> dashmap::iter::Iter<'_, ObjectGuid, Creature> {
        self.creatures.iter()
    }

    /// Get full spawn data for a creature
    pub fn get_spawn_data(&self, guid: ObjectGuid) -> Option<super::spawn::CreatureSpawnData> {
        let creature = self.creatures.get(&guid)?;
        let spawn_id = creature.spawn_id;
        let map_id = creature.map_id;

        // Find spawn data
        self.spawns_by_map
            .get(&map_id)
            .and_then(|spawns| spawns.iter().find(|s| s.spawn_id == spawn_id).cloned())
    }

    /// Get spawn time for a creature from its spawn data (convenience method)
    pub fn get_spawn_time(&self, guid: ObjectGuid) -> Option<u32> {
        self.get_spawn_data(guid).map(|s| s.spawntimesecs)
    }

    /// Get spawn data by spawn_id (for pool system)
    pub fn get_spawn_data_by_id(&self, spawn_id: u32) -> Option<super::spawn::CreatureSpawnData> {
        // Search all maps for this spawn_id
        for map_spawns in self.spawns_by_map.iter() {
            if let Some(spawn) = map_spawns.value().iter().find(|s| s.spawn_id == spawn_id) {
                return Some(spawn.clone());
            }
        }
        None
    }

    /// Get spawn_id from creature GUID (for linking system)
    pub fn get_spawn_id(&self, guid: ObjectGuid) -> Option<u32> {
        self.creatures.get(&guid).map(|c| c.spawn_id)
    }

    /// Clear the lootable flag on a creature corpse
    pub fn clear_lootable_flag(&self, guid: ObjectGuid, world: &crate::world::World) {
        use super::death::{UNIT_DYNFLAG_DEAD, UNIT_DYNFLAG_TAPPED, UNIT_DYNFLAG_TAPPED_BY_PLAYER};

        // Get creature's loot recipient to determine tapped flag
        let has_loot_recipient = self
            .with_creature(guid, |c| c.loot_recipient.is_some())
            .unwrap_or(false);

        // Calculate new flags: Keep DEAD and TAPPED flags, but remove LOOTABLE
        let new_flags = if has_loot_recipient {
            UNIT_DYNFLAG_DEAD | UNIT_DYNFLAG_TAPPED_BY_PLAYER
        } else {
            UNIT_DYNFLAG_DEAD | UNIT_DYNFLAG_TAPPED
        };

        tracing::debug!(
            "Clearing lootable flag for creature {:?}, new flags: 0x{:04X}",
            guid,
            new_flags
        );

        // Send update to clients
        super::death::send_dynamic_flags_update(world, guid, new_flags);
    }
}

/// Information about a creature death
#[derive(Debug)]
pub struct DeathInfo {
    pub guid: ObjectGuid,
    pub position: Position,
    pub loot_recipient: Option<ObjectGuid>,
    pub entry: u32,
}

impl CreatureManager {
    /// Process creature death
    /// Called when damage brings health to 0
    pub fn handle_death(&self, guid: ObjectGuid, killer: Option<ObjectGuid>) -> Option<DeathInfo> {
        tracing::info!(
            "[CREATURE_MGR] handle_death ENTRY: guid={:?}, killer={:?}",
            guid,
            killer
        );

        self.creatures.get_mut(&guid).map(|mut creature| {
            tracing::info!(
                "[CREATURE_MGR] handle_death: calling creature.kill(), current death_state={:?}",
                creature.death_state
            );
            creature.kill(killer);
            tracing::info!(
                "[CREATURE_MGR] handle_death: after kill(), death_state={:?}, loot_recipient={:?}",
                creature.death_state,
                creature.loot_recipient
            );

            DeathInfo {
                guid,
                position: creature.position,
                loot_recipient: creature.loot_recipient,
                entry: creature.entry,
            }
        })
    }

    /// Transition creature to corpse state
    pub fn set_corpse_state(&self, guid: ObjectGuid, decay_time_ms: u32) {
        if let Some(mut creature) = self.creatures.get_mut(&guid) {
            creature.set_corpse_state(decay_time_ms);
        }
    }

    /// Get all creatures in JustDied state (for death processing)
    pub fn get_just_died_creatures(&self) -> Vec<ObjectGuid> {
        let result: Vec<ObjectGuid> = self
            .creatures
            .iter()
            .filter(|e| e.death_state == DeathState::JustDied)
            .map(|e| *e.key())
            .collect();

        if !result.is_empty() {
            tracing::info!(
                "[CREATURE_MGR] get_just_died_creatures: found {} creatures",
                result.len()
            );
        }

        result
    }

    /// Get all creatures with expired corpse timers
    pub fn get_expired_corpses(&self) -> Vec<ObjectGuid> {
        self.creatures
            .iter()
            .filter(|e| e.death_state == DeathState::Corpse && e.corpse_decay_timer == 0)
            .map(|e| *e.key())
            .collect()
    }

    /// Update corpse timers for all corpses
    pub fn update_corpse_timers(&self, diff_ms: u32) {
        for mut entry in self.creatures.iter_mut() {
            entry.update_corpse_timer(diff_ms);
        }
    }

    /// Build CREATE_OBJECT2 packet for a creature
    ///
    /// Uses world shared message pattern for type-safe packet construction.
    /// This follows the same pattern as PlayerManager::build_create_msg().
    ///
    /// CRITICAL: This must include all fields that the old world's
    /// Creature::populate_create_fields() sets, otherwise the client
    /// will not render NPCs properly (especially UNIT_FIELD_BYTES_0/1/2).
    pub fn build_create_msg(
        &self,
        guid: ObjectGuid,
        _world: &crate::world::World,
    ) -> Option<crate::shared::messages::update::SmsgUpdateObject> {
        use crate::shared::messages::update::*;
        use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
        use crate::world::core::common::position::Position as WorldPosition;
        use crate::world::game::common::object_type::MovementSpeeds;
        use crate::world::game::common::object_type::ObjectTypeId;
        use crate::world::game::common::update_fields::*;

        let creature = self.creatures.get(&guid)?;

        // CRITICAL: Use new_creature() to embed entry ID in GUID
        // Format: counter (bits 0-23) | entry (bits 24-47) | HighGuid::Unit (bits 48-63)
        // The client extracts entry from GUID to look up creature template
        // Use guid.entry() to ensure consistency with VALUES updates (health, combat, etc.)
        let world_guid = WorldObjectGuid::new_creature(guid.entry(), guid.counter());
        let world_position = WorldPosition::new(
            creature.position.x,
            creature.position.y,
            creature.position.z,
            creature.position.o,
        );

        // Use unit_flags directly from DB - trust the database values
        // MaNGOS loads unit_flags from DB and does NOT blanket-clear flags
        let unit_flags_val = creature.unit_flags;

        // UNIT_FIELD_BYTES_0: race/class/gender/power_type
        // Byte 0: Race (always 0 for creatures - they don't have player races)
        // Byte 1: Class (1=Warrior default for creatures)
        // Byte 2: Gender (0=Male, 1=Female, 2=None)
        // Byte 3: Power Type (0=Mana, 1=Rage, 2=Focus, 3=Energy, 4=Happiness)
        // CRITICAL: Client uses this to determine if creature is valid/attackable
        let bytes_0: u32 = (0u32 << 24)  // power_type = 0 (mana)
            | (0u32 << 16)               // gender = 0 (male)
            | (1u32 << 8)                // class = 1 (warrior)
            | 0u32; // race = 0 (none for creatures)

        // UNIT_FIELD_BYTES_1: stand state, pet loyalty, shapeshift, stealth
        // Byte 0: Stand state (0 = standing, 7 = dead)
        // CRITICAL: This field is REQUIRED for the client to show interaction cursor!
        // For dead/corpse creatures, use stand state Dead (7) so the client
        // renders them as dead when they first enter visibility range.
        use super::death::DeathState;
        let is_dead = creature.death_state != DeathState::Alive;
        let bytes_1: u32 = if is_dead { 7 } else { 0 };

        // UNIT_FIELD_BYTES_2: sheath state, misc flags
        // Byte 0: Sheath state (1 = melee weapon sheathed)
        // Byte 1: Misc flags (0x10 = UNIT_BYTE2_FLAG_AURAS - has aura icons)
        // Working old implementation uses 0x00001001
        let bytes_2: u32 = 0x00001001;

        // Build CREATE_OBJECT block with all required fields
        // This matches the old world's Creature::populate_create_fields()
        let mut block = CreateObjectBlock::new(world_guid, ObjectTypeId::Unit, ObjectType::Unit)
            .with_movement(world_position, 0, Some(MovementSpeeds::default()))
            // Object fields
            .set_guid_field(OBJECT_FIELD_GUID, world_guid)
            .set_field(OBJECT_FIELD_TYPE, 0x09) // TYPEMASK_OBJECT | TYPEMASK_UNIT
            .set_field(OBJECT_FIELD_ENTRY, creature.entry) // CRITICAL: Client needs entry!
            // Scale: default to 1.0 if database has 0 (matches old world creature.rs:1565)
            .set_float_field(
                OBJECT_FIELD_SCALE_X,
                if creature.scale > 0.0 {
                    creature.scale
                } else {
                    1.0
                },
            )
            // Unit stats
            .set_field(UNIT_FIELD_HEALTH, creature.current_health)
            .set_field(UNIT_FIELD_MAXHEALTH, creature.max_health)
            .set_field(UNIT_FIELD_LEVEL, creature.level as u32)
            .set_field(UNIT_FIELD_FACTIONTEMPLATE, creature.faction)
            // CRITICAL bytes fields - required for creature validity and interaction
            .set_required(UNIT_FIELD_BYTES_0, bytes_0)
            .set_required(UNIT_FIELD_FLAGS, unit_flags_val)
            .set_required(UNIT_FIELD_BYTES_1, bytes_1) // CRITICAL for interaction cursor!
            // Collision and combat (from creature_display_info_addon or defaults)
            .set_float_field(UNIT_FIELD_BOUNDINGRADIUS, creature.bounding_radius)
            .set_float_field(UNIT_FIELD_COMBATREACH, creature.combat_reach)
            // Display
            .set_field(UNIT_FIELD_DISPLAYID, creature.display_id)
            .set_field(UNIT_FIELD_NATIVEDISPLAYID, creature.native_display_id)
            // Mana (for caster creatures)
            .set_field(UNIT_FIELD_POWER1, creature.current_mana)
            .set_field(UNIT_FIELD_MAXPOWER1, creature.max_mana)
            // Attack timing (from template)
            .set_field(UNIT_FIELD_BASEATTACKTIME, creature.base_attack_time)
            .set_field(UNIT_FIELD_BASEATTACKTIME + 1, creature.base_attack_time) // Off-hand
            // Melee damage range (float fields, used by client for tooltip/combat log)
            .set_float_field(UNIT_FIELD_MINDAMAGE, creature.damage_min as f32)
            .set_float_field(UNIT_FIELD_MAXDAMAGE, creature.damage_max as f32)
            // Attack power (from classlevelstats, base value before mods)
            .set_field(UNIT_FIELD_ATTACK_POWER, creature.attack_power as u32)
            .set_field(UNIT_FIELD_ATTACK_POWER_MODS, 0u32)
            .set_float_field(UNIT_FIELD_ATTACK_POWER_MULTIPLIER, 0.0)
            // Physical armor (resistance school 0)
            .set_field(UNIT_FIELD_RESISTANCES, creature.armor)
            // Cast speed
            .set_float_field(UNIT_MOD_CAST_SPEED, 1.0)
            // More bytes fields
            .set_field(UNIT_FIELD_BYTES_2, bytes_2)
            // NPC interaction flags (cleared on death so dead creatures aren't interactable)
            .set_required(UNIT_NPC_FLAGS, if is_dead { 0 } else { creature.npc_flags });

        // Calculate dynamic flags for dead creatures
        // CRITICAL: Include LOOTABLE flag for dead creatures with loot
        let dynamic_flags = if is_dead {
            let mut flags = super::death::UNIT_DYNFLAG_DEAD;

            // Add LOOTABLE flag if creature has loot
            if creature.has_loot() {
                flags |= super::death::UNIT_DYNFLAG_LOOTABLE;

                // Add tapped flags based on loot recipient
                if creature.loot_recipient.is_some() {
                    flags |= super::death::UNIT_DYNFLAG_TAPPED_BY_PLAYER;
                } else {
                    flags |= super::death::UNIT_DYNFLAG_TAPPED;
                }
            } else if creature.loot_recipient.is_some() {
                // Dead but no loot - still show tapped
                flags |= super::death::UNIT_DYNFLAG_TAPPED_BY_PLAYER;
            } else {
                flags |= super::death::UNIT_DYNFLAG_TAPPED;
            }

            flags
        } else {
            0
        };

        // Dynamic flags (CRITICAL: includes LOOTABLE for dead creatures with loot)
        block = block.set_field(UNIT_DYNAMIC_FLAGS, dynamic_flags);

        // Set UNIT_FIELD_TARGET and UPDATEFLAG_MELEE_ATTACKING if creature is in combat
        if creature.combat.in_combat {
            if let Some(target) = creature.combat.attacking {
                let target_world_guid = WorldObjectGuid::from_raw(target.raw());
                block = block.set_guid_field(UNIT_FIELD_TARGET, target_world_guid);
                // MaNGOS adds UPDATEFLAG_MELEE_ATTACKING when unit has a victim.
                // The client expects a packed GUID of the victim after the ALL u32
                // in the movement block. Without this, the creature's attack state
                // is not properly communicated on CREATE.
                block = block.with_melee_attacking(target_world_guid);
            }
        }

        // Return SMSG_UPDATE_OBJECT with CreateObject block (matches old world behavior)
        Some(SmsgUpdateObject::new().add_block(UpdateBlockData::CreateObject(block)))
    }

    /// Build an SMSG_MONSTER_MOVE packet that syncs the creature's current movement
    /// state to a client that just received CREATE_OBJECT2.
    ///
    /// When a creature enters visibility while mid-movement, CREATE_OBJECT2 sends
    /// the creature's current position but no spline/movement data. Without this
    /// sync packet, the client shows the creature standing still until the next
    /// natural SMSG_MONSTER_MOVE fires (when the creature picks a new destination).
    pub fn build_movement_sync_packet(
        &self,
        guid: ObjectGuid,
    ) -> Option<crate::shared::protocol::WorldPacket> {
        use crate::shared::messages::movement::{spline_flags, SmsgMonsterMove};
        use crate::shared::messages::ToWorldPacket;

        let creature = self.creatures.get(&guid)?;

        if !creature.move_spline.is_active() {
            return None;
        }

        let remaining = creature.move_spline.time_remaining();
        if remaining == 0 {
            return None;
        }

        let current_pos = creature.position;
        let final_dest = creature.move_spline.final_position();

        // Build a monster move from current position to final destination
        // with remaining duration. Use walking flag since most idle movement
        // is walking; chase movements will get fresh packets anyway.
        let msg = SmsgMonsterMove {
            guid,
            position: current_pos,
            spline_id: creature.move_spline.id(),
            move_type: 0, // Normal
            facing_target: None,
            facing_angle: None,
            spline_flags: spline_flags::WALKMODE,
            duration: remaining,
            waypoints: vec![final_dest],
        };

        Some(msg.to_world_packet())
    }

    /// Send nearby creatures to a player (called during login)
    ///
    /// This sends CREATE_OBJECT2 packets for all creatures within visibility range,
    /// batched to avoid overwhelming the client or causing server lockups.
    pub fn send_nearby_creatures(
        &self,
        player_guid: ObjectGuid,
        position: Position,
        map_id: u32,
        instance_id: u32,
        world: &crate::world::World,
    ) -> anyhow::Result<()> {
        use crate::shared::messages::update::{SmsgUpdateObject, UpdateBlockData};
        use crate::shared::messages::ToWorldPacket;

        const MAX_BLOCKS_PER_PACKET: usize = 50;

        // Get nearby objects from map grid
        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);
        let nearby = map.get_objects_in_range(position, map.visibility_distance());

        // Filter to creatures only (is_unit but not is_player)
        let creatures: Vec<_> = nearby
            .into_iter()
            .filter(|g| g.is_unit() && !g.is_player())
            .collect();

        if creatures.is_empty() {
            return Ok(());
        }

        tracing::info!(
            "[CREATURE] Sending {} creatures to player {:?}",
            creatures.len(),
            player_guid
        );

        // Build and send batched packets
        let mut current_msg = SmsgUpdateObject::new();
        let mut count = 0;
        let mut total_sent = 0;

        for guid in creatures {
            if let Some(msg) = self.build_create_msg(guid, world) {
                for block in msg.blocks {
                    if count >= MAX_BLOCKS_PER_PACKET {
                        // Send current batch with compression via broadcast manager
                        let packet = current_msg.to_world_packet();
                        let compressed = compress_update_packet_if_needed(packet)?;
                        world
                            .managers
                            .broadcast_mgr
                            .send_to_player(player_guid, compressed);
                        total_sent += count;
                        current_msg = SmsgUpdateObject::new();
                        count = 0;
                    }
                    current_msg = current_msg.add_block(block);
                    count += 1;
                }
            }
        }

        // Send remaining blocks with compression via broadcast manager
        if !current_msg.blocks.is_empty() {
            let packet = current_msg.to_world_packet();
            let compressed = compress_update_packet_if_needed(packet)?;
            world
                .managers
                .broadcast_mgr
                .send_to_player(player_guid, compressed);
            total_sent += count;
        }

        tracing::info!(
            "[CREATURE] Sent {} creature blocks to player {:?}",
            total_sent,
            player_guid
        );

        // Proactively send creature query responses for all unique entries
        // This ensures the client has creature names/types immediately without querying
        // IMPORTANT: Collect entries first WITHOUT holding DashMap refs across await points
        let mut unique_entries = std::collections::HashSet::new();
        let nearby = map.get_objects_in_range(position, map.visibility_distance());
        for guid in nearby {
            if guid.is_unit() && !guid.is_player() {
                if let Some(creature) = self.creatures.get(&guid) {
                    unique_entries.insert(creature.entry);
                }
                // DashMap ref is dropped here before any await
            }
        }

        // Now send query packets without holding any DashMap refs
        let mut sent_entries = std::collections::HashSet::new();
        for entry in unique_entries {
            if let Some(query_packet) = self.build_creature_query_packet(entry) {
                world
                    .managers
                    .broadcast_mgr
                    .send_to_player(player_guid, query_packet);
                sent_entries.insert(entry);
            }
        }

        tracing::info!(
            "[CREATURE] Sent {} creature query responses to player {:?}",
            sent_entries.len(),
            player_guid
        );

        Ok(())
    }

    /// Build SMSG_CREATURE_QUERY_RESPONSE packet for a creature entry
    ///
    /// This is sent proactively so the client has creature names/types without querying.
    pub fn build_creature_query_packet(
        &self,
        entry: u32,
    ) -> Option<crate::shared::protocol::WorldPacket> {
        use crate::shared::protocol::Opcode;

        let template = self.templates.get(&entry)?;

        let mut packet =
            crate::shared::protocol::WorldPacket::new(Opcode::SMSG_CREATURE_QUERY_RESPONSE);
        packet.write_u32(entry);
        packet.write_cstring(&template.name);
        packet.write_u8(0); // name2
        packet.write_u8(0); // name3
        packet.write_u8(0); // name4
        packet.write_cstring(template.subname.as_deref().unwrap_or(""));
        // type_flags: Not stored in rcore DB schema. Use 0 for now.
        // Ghost visibility is handled server-side via static_flags1 VISIBLE_TO_GHOSTS.
        packet.write_u32(0);
        packet.write_u32(template.creature_type as u32);
        packet.write_u32(0); // creature_family
        packet.write_u32(0); // rank
        packet.write_u32(0); // unknown
        packet.write_u32(0); // pet_spell_data_id
        packet.write_u32(template.get_display_id()); // Use first non-zero display_id
        packet.write_u8(0); // civilian
        packet.write_u8(0); // racial_leader

        Some(packet)
    }
}
