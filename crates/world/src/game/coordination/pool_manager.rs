//! Spawn pool state (port of MaNGOS `PoolManager` + `SpawnedPoolData`).
//!
//! The manager owns *which* pool members should exist; instantiating them is
//! the world's job — grid loading spawns selected members, and the pool system
//! swaps them out when one is ready to respawn.

use super::pool_repository::PoolData;
use super::pool_types::{pool_flags, PoolMember, PoolMemberKey, PoolMemberType, PoolState};
use dashmap::DashMap;
use oxcore_shared::protocol::ObjectGuid;

/// Instance id shared by all continents (MaNGOS continent map persistent state).
pub const CONTINENT_INSTANCE: u32 = 0;

/// Guard against `pool_pool` rows forming a cycle.
const MAX_POOL_NESTING: u32 = 32;

/// A member that left a pool's roster, along with the object it had in the
/// world (if it was instantiated at all).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolDespawn {
    pub key: PoolMemberKey,
    pub guid: Option<ObjectGuid>,
}

/// Outcome of re-rolling a pool: which members left the roster and which
/// joined it. Both lists only contain creatures and gameobjects — nested pools
/// are resolved into their own members.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PoolRoll {
    /// Members that are no longer part of the roster and must be despawned.
    pub to_despawn: Vec<PoolDespawn>,
    /// Members newly added to the roster that should be spawned.
    pub to_spawn: Vec<PoolMemberKey>,
    /// The triggering member stayed on the roster: it just respawns in place.
    pub trigger_respawns: bool,
}

/// Manages all spawn pools - state only, no database
pub struct PoolManager {
    /// Pools by ID
    pools: DashMap<u32, PoolState>,
    /// Creature spawn -> pool mapping
    creature_to_pool: DashMap<u32, u32>,
    /// Gameobject spawn -> pool mapping
    gameobject_to_pool: DashMap<u32, u32>,
    /// Pool -> parent pool mapping (for nested pools)
    pool_to_parent: DashMap<u32, u32>,
    /// Runtime object -> (owning pool, map instance)
    guid_to_pool: DashMap<ObjectGuid, (u32, u32)>,
}

impl PoolManager {
    pub fn new() -> Self {
        Self {
            pools: DashMap::new(),
            creature_to_pool: DashMap::new(),
            gameobject_to_pool: DashMap::new(),
            pool_to_parent: DashMap::new(),
            guid_to_pool: DashMap::new(),
        }
    }

    /// Load pools from repository data.
    ///
    /// Every pool auto-spawns unless it is a member of another pool (MaNGOS
    /// sets AUTO_SPAWN on load and clears it for nested pools); pools whose
    /// chances cannot produce a pick are reported and disabled.
    pub fn load_from_repository(&self, data: PoolData) {
        self.pools.clear();
        self.creature_to_pool.clear();
        self.gameobject_to_pool.clear();
        self.pool_to_parent.clear();
        self.guid_to_pool.clear();

        // Templates
        for mut template in data.templates {
            if template.max_limit == 0 {
                tracing::warn!("[POOL] Pool {} has no max limit set", template.pool_id);
            }
            template.flags |= pool_flags::AUTO_SPAWN;
            let pool_id = template.pool_id;
            self.pools.insert(pool_id, PoolState::new(template));
        }

        // Creature members
        for member in data.creature_members {
            let added = self.add_member(
                member.pool_id,
                PoolMember {
                    id: member.spawn_id,
                    member_type: PoolMemberType::Creature,
                    chance: member.chance,
                    description: member.description,
                },
            );
            if added {
                self.creature_to_pool
                    .insert(member.spawn_id, member.pool_id);
            }
        }

        // Gameobject members
        for member in data.gameobject_members {
            let added = self.add_member(
                member.pool_id,
                PoolMember {
                    id: member.spawn_id,
                    member_type: PoolMemberType::GameObject,
                    chance: member.chance,
                    description: member.description,
                },
            );
            if added {
                self.gameobject_to_pool
                    .insert(member.spawn_id, member.pool_id);
            }
        }

        // Nested pools
        for member in data.pool_members {
            if member.child_pool_id == member.parent_pool_id {
                tracing::error!(
                    "[POOL] Pool {} is its own mother pool, skipped",
                    member.child_pool_id
                );
                continue;
            }

            let added = self.add_member(
                member.parent_pool_id,
                PoolMember {
                    id: member.child_pool_id,
                    member_type: PoolMemberType::Pool,
                    chance: member.chance,
                    description: member.description,
                },
            );
            if !added {
                continue;
            }

            self.pool_to_parent
                .insert(member.child_pool_id, member.parent_pool_id);

            // A pool inside another pool is spawned by its mother, not on its own
            if let Some(mut child) = self.pools.get_mut(&member.child_pool_id) {
                child.template.flags &= !pool_flags::AUTO_SPAWN;
            }
        }

        // Chance integrity: a pool that can never pick anything is disabled
        for mut pool in self.pools.iter_mut() {
            if !pool.template.is_auto_spawn() || pool.check_chances() {
                continue;
            }
            tracing::error!(
                "[POOL] Pool {} has explicit chances that do not sum to 100 and no \
                 equal-chanced members; it cannot pick a member to spawn",
                pool.template.pool_id
            );
            pool.template.flags &= !pool_flags::AUTO_SPAWN;
        }

        tracing::info!("PoolManager loaded {} pools", self.pools.len());
    }

    /// Roll the continent rosters of every auto-spawn pool
    /// (MaNGOS `PoolManager::Initialize`, called per map persistent state).
    ///
    /// Instanced maps roll their own roster the first time a grid in that
    /// instance asks whether a pooled spawn may spawn.
    ///
    /// Returns the number of members selected for the continents.
    pub fn initialize_rosters(&self) -> usize {
        let pool_ids: Vec<u32> = self
            .pools
            .iter()
            .filter(|pool| pool.template.is_auto_spawn())
            .map(|pool| *pool.key())
            .collect();

        for pool_id in &pool_ids {
            self.roll_roster(*pool_id, CONTINENT_INSTANCE);
        }

        let selected: usize = self
            .pools
            .iter()
            .map(|pool| pool.selected_count(CONTINENT_INSTANCE) as usize)
            .sum();

        tracing::info!(
            "[POOL] Initialized {} pools, {} members selected to spawn",
            pool_ids.len(),
            selected
        );
        selected
    }

    /// Fill a pool's roster for one instance, descending into nested pools.
    fn roll_roster(&self, pool_id: u32, instance_id: u32) {
        let mut children = Vec::new();

        {
            let Some(mut pool) = self.pools.get_mut(&pool_id) else {
                return;
            };

            pool.open_roster(instance_id);

            while pool.can_select_more(instance_id) {
                let Some(member) = pool.roll_one(instance_id, None) else {
                    break;
                };
                pool.select(instance_id, member.key());
                if member.member_type == PoolMemberType::Pool {
                    children.push(member.id);
                }
            }
        }

        for child in children {
            self.roll_roster(child, instance_id);
        }
    }

    /// Make sure a pool has a roster for this instance, rolling it from the
    /// top-level pool down if this is the first time the instance is seen.
    fn ensure_roster(&self, pool_id: u32, instance_id: u32) {
        let has_roster = self
            .pools
            .get(&pool_id)
            .map(|pool| pool.has_roster(instance_id))
            .unwrap_or(true);
        if has_roster {
            return;
        }

        // Nested pools are filled by their mother pool.
        let top = self.top_pool(pool_id);
        self.roll_roster(top, instance_id);
    }

    /// Walk up to the pool that is not a member of another pool.
    fn top_pool(&self, pool_id: u32) -> u32 {
        let mut current = pool_id;
        let mut guard = 0;
        while let Some(parent) = self.pool_to_parent.get(&current).map(|r| *r) {
            current = parent;
            guard += 1;
            if guard > MAX_POOL_NESTING {
                tracing::error!("[POOL] Pool {} nests too deeply, giving up", pool_id);
                break;
            }
        }
        current
    }

    /// Pool a creature spawn belongs to, if any.
    pub fn get_pool_for_creature(&self, spawn_id: u32) -> Option<u32> {
        self.creature_to_pool.get(&spawn_id).map(|r| *r)
    }

    /// Pool a gameobject spawn belongs to, if any.
    pub fn get_pool_for_gameobject(&self, spawn_id: u32) -> Option<u32> {
        self.gameobject_to_pool.get(&spawn_id).map(|r| *r)
    }

    /// May this creature spawn in this instance? Non-pooled spawns always can;
    /// pooled ones only when the pool has them on the instance's roster.
    pub fn can_creature_spawn(&self, spawn_id: u32, instance_id: u32) -> bool {
        self.is_selected(
            self.get_pool_for_creature(spawn_id),
            instance_id,
            (PoolMemberType::Creature, spawn_id),
        )
    }

    /// May this gameobject spawn? See [`Self::can_creature_spawn`].
    pub fn can_gameobject_spawn(&self, spawn_id: u32, instance_id: u32) -> bool {
        self.is_selected(
            self.get_pool_for_gameobject(spawn_id),
            instance_id,
            (PoolMemberType::GameObject, spawn_id),
        )
    }

    fn is_selected(&self, pool_id: Option<u32>, instance_id: u32, key: PoolMemberKey) -> bool {
        let Some(pool_id) = pool_id else {
            return true; // Not in a pool
        };

        self.ensure_roster(pool_id, instance_id);

        let Some(pool) = self.pools.get(&pool_id) else {
            return true;
        };
        pool.is_selected(instance_id, key)
    }

    /// Members the pool wants in the world in this instance (creatures and
    /// gameobjects only, nested pools resolved recursively).
    pub fn selected_objects(&self, pool_id: u32, instance_id: u32) -> Vec<PoolMemberKey> {
        let mut result = Vec::new();

        let selected = {
            let Some(pool) = self.pools.get(&pool_id) else {
                return result;
            };
            pool.selected_members(instance_id)
        };

        for key in selected {
            match key.0 {
                PoolMemberType::Pool => result.extend(self.selected_objects(key.1, instance_id)),
                _ => result.push(key),
            }
        }
        result
    }

    /// Record the runtime object created for a pooled spawn.
    pub fn mark_spawned(
        &self,
        pool_id: u32,
        instance_id: u32,
        key: PoolMemberKey,
        guid: ObjectGuid,
    ) {
        if let Some(mut pool) = self.pools.get_mut(&pool_id) {
            pool.mark_spawned(instance_id, key, guid);
            self.guid_to_pool.insert(guid, (pool_id, instance_id));
        }
    }

    /// Forget a runtime object (grid unload/despawn); the member keeps its
    /// roster slot so it comes back when the grid is loaded again.
    pub fn mark_despawned(&self, guid: ObjectGuid) -> Option<(u32, PoolMemberKey)> {
        let (pool_id, instance_id) = *self.guid_to_pool.get(&guid)?;

        let key = {
            let mut pool = self.pools.get_mut(&pool_id)?;
            pool.mark_despawned(instance_id, guid)?
        };

        self.guid_to_pool.remove(&guid);
        Some((pool_id, key))
    }

    /// Pool, instance and member key for a runtime object.
    pub fn get_pool_membership(&self, guid: ObjectGuid) -> Option<(u32, u32, PoolMemberKey)> {
        let (pool_id, instance_id) = *self.guid_to_pool.get(&guid)?;
        let pool = self.pools.get(&pool_id)?;
        pool.key_for_guid(instance_id, guid)
            .map(|key| (pool_id, instance_id, key))
    }

    /// Re-roll a pool because one of its members is ready to respawn
    /// (MaNGOS `PoolManager::UpdatePool`).
    ///
    /// If the trigger is rolled again it simply respawns; otherwise it leaves
    /// the roster and the newly rolled member takes its place. Only the
    /// roster of `instance_id` is touched — other instances of the map keep
    /// their own members.
    pub fn update_pool(&self, pool_id: u32, instance_id: u32, trigger: PoolMemberKey) -> PoolRoll {
        // A pool inside another pool is rolled at the mother's level.
        if let Some(parent) = self.pool_to_parent.get(&pool_id).map(|r| *r) {
            return self.update_pool(parent, instance_id, (PoolMemberType::Pool, pool_id));
        }

        let mut roll = PoolRoll::default();

        let rolled = {
            let Some(pool) = self.pools.get(&pool_id) else {
                return roll;
            };
            pool.roll_one(instance_id, Some(trigger))
        };

        // Nothing else can be picked: keep the trigger where it is.
        let Some(rolled) = rolled else {
            roll.trigger_respawns = true;
            return roll;
        };

        if rolled.key() == trigger {
            roll.trigger_respawns = true;
            return roll;
        }

        // Swap: the trigger leaves the roster, the rolled member joins it.
        roll.to_despawn = self.deselect(pool_id, instance_id, trigger);

        if let Some(mut pool) = self.pools.get_mut(&pool_id) {
            pool.select(instance_id, rolled.key());
        }

        match rolled.member_type {
            PoolMemberType::Pool => {
                self.roll_roster(rolled.id, instance_id);
                roll.to_spawn = self.selected_objects(rolled.id, instance_id);
            }
            _ => roll.to_spawn.push(rolled.key()),
        }

        roll
    }

    /// Remove a member from a pool's roster, cascading into nested pools.
    /// Returns the object members that have to be despawned, each with the
    /// runtime object it held (if any) — the caller needs it to remove the
    /// object from the world.
    fn deselect(&self, pool_id: u32, instance_id: u32, key: PoolMemberKey) -> Vec<PoolDespawn> {
        let mut removed = Vec::new();

        // A child pool takes its whole roster with it.
        if key.0 == PoolMemberType::Pool {
            let child_keys = self
                .pools
                .get(&key.1)
                .map(|child| child.selected_members(instance_id))
                .unwrap_or_default();

            for child_key in child_keys {
                removed.extend(self.deselect(key.1, instance_id, child_key));
            }
        }

        let guid = self
            .pools
            .get_mut(&pool_id)
            .and_then(|mut pool| pool.deselect(instance_id, key));
        if let Some(guid) = guid {
            self.guid_to_pool.remove(&guid);
        }

        if key.0 != PoolMemberType::Pool {
            removed.push(PoolDespawn { key, guid });
        }

        removed
    }

    /// Fill a pool's roster for an instance back up to its limit
    /// (MaNGOS `SpawnPool`).
    pub fn fill_roster(&self, pool_id: u32, instance_id: u32) {
        self.roll_roster(pool_id, instance_id);
    }

    /// Drop a pool's whole roster for an instance (MaNGOS `DespawnPool`).
    /// Returns the members that have to be removed from the world.
    pub fn clear_roster(&self, pool_id: u32, instance_id: u32) -> Vec<PoolDespawn> {
        let keys = self
            .pools
            .get(&pool_id)
            .map(|pool| pool.selected_members(instance_id))
            .unwrap_or_default();

        keys.into_iter()
            .flat_map(|key| self.deselect(pool_id, instance_id, key))
            .collect()
    }

    /// Runtime object of a roster member, if it is instantiated.
    pub fn spawned_guid(
        &self,
        pool_id: u32,
        instance_id: u32,
        key: PoolMemberKey,
    ) -> Option<ObjectGuid> {
        self.pools.get(&pool_id)?.spawned_guid(instance_id, key)
    }

    /// Number of loaded pools.
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }

    /// Roster size of a pool in one instance.
    pub fn selected_count(&self, pool_id: u32, instance_id: u32) -> u32 {
        self.pools
            .get(&pool_id)
            .map(|pool| pool.selected_count(instance_id))
            .unwrap_or(0)
    }

    /// Add a member to a pool, reporting members pointing at unknown pools.
    fn add_member(&self, pool_id: u32, member: PoolMember) -> bool {
        let Some(mut pool) = self.pools.get_mut(&pool_id) else {
            tracing::error!(
                "[POOL] Member {:?} {} references non-existent pool {}, skipped",
                member.member_type,
                member.id,
                pool_id
            );
            return false;
        };
        pool.add_member(member);
        true
    }
}

impl Default for PoolManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::coordination::pool_repository::{PoolObjectMember, PoolPoolMember};
    use crate::game::coordination::pool_types::PoolTemplate;

    fn template(pool_id: u32, max_limit: u32) -> PoolTemplate {
        PoolTemplate {
            pool_id,
            max_limit,
            flags: 0,
            description: format!("pool {}", pool_id),
        }
    }

    fn object_member(pool_id: u32, spawn_id: u32) -> PoolObjectMember {
        PoolObjectMember {
            pool_id,
            spawn_id,
            chance: 0.0,
            description: String::new(),
        }
    }

    fn nested(child_pool_id: u32, parent_pool_id: u32) -> PoolPoolMember {
        PoolPoolMember {
            child_pool_id,
            parent_pool_id,
            chance: 0.0,
            description: String::new(),
        }
    }

    /// One pool holding `member_count` creatures and spawning `max_limit`.
    fn simple_manager(max_limit: u32, member_count: u32) -> PoolManager {
        let mgr = PoolManager::new();
        mgr.load_from_repository(PoolData {
            templates: vec![template(1, max_limit)],
            creature_members: (10..10 + member_count)
                .map(|id| object_member(1, id))
                .collect(),
            gameobject_members: Vec::new(),
            pool_members: Vec::new(),
        });
        mgr
    }

    fn despawned_keys(roll: &PoolRoll) -> Vec<PoolMemberKey> {
        roll.to_despawn.iter().map(|d| d.key).collect()
    }

    /// Which of two members currently holds the single roster slot.
    fn selected_of(mgr: &PoolManager, a: u32, b: u32) -> u32 {
        if mgr.can_creature_spawn(a, CONTINENT_INSTANCE) {
            a
        } else {
            b
        }
    }

    #[test]
    fn only_max_limit_members_are_selected() {
        let mgr = simple_manager(2, 5);
        assert_eq!(mgr.initialize_rosters(), 2);

        let selected = (10..15)
            .filter(|id| mgr.can_creature_spawn(*id, CONTINENT_INSTANCE))
            .count();
        assert_eq!(selected, 2, "pool must keep exactly max_limit members");
    }

    #[test]
    fn instanced_maps_roll_their_own_roster() {
        let mgr = simple_manager(1, 2);
        mgr.initialize_rosters();
        let on_continent = selected_of(&mgr, 10, 11);

        // First grid load inside instance 42 rolls that instance's roster.
        let in_instance: Vec<u32> = [10, 11]
            .into_iter()
            .filter(|id| mgr.can_creature_spawn(*id, 42))
            .collect();
        assert_eq!(in_instance.len(), 1, "the limit applies per instance");
        assert_eq!(mgr.selected_count(1, 42), 1);

        // ... and leaves the continent roster alone.
        assert_eq!(mgr.selected_count(1, CONTINENT_INSTANCE), 1);
        assert!(mgr.can_creature_spawn(on_continent, CONTINENT_INSTANCE));
    }

    #[test]
    fn unpooled_spawns_always_spawn() {
        let mgr = simple_manager(1, 2);
        mgr.initialize_rosters();
        assert!(mgr.can_creature_spawn(9999, CONTINENT_INSTANCE));
    }

    #[test]
    fn nested_pools_select_through_their_mother() {
        let mgr = PoolManager::new();
        mgr.load_from_repository(PoolData {
            templates: vec![template(1, 1), template(2, 1), template(3, 1)],
            creature_members: vec![object_member(2, 20), object_member(3, 30)],
            gameobject_members: Vec::new(),
            pool_members: vec![nested(2, 1), nested(3, 1)],
        });

        mgr.initialize_rosters();

        // The mother pool picks one child pool, which picks its own creature.
        let spawnable: Vec<u32> = [20, 30]
            .into_iter()
            .filter(|id| mgr.can_creature_spawn(*id, CONTINENT_INSTANCE))
            .collect();
        assert_eq!(spawnable.len(), 1, "only one child pool may be spawned");
        assert_eq!(mgr.selected_objects(1, CONTINENT_INSTANCE).len(), 1);
    }

    #[test]
    fn respawn_reroll_keeps_the_roster_size() {
        let mgr = simple_manager(1, 2);
        mgr.initialize_rosters();

        let mut saw_swap = false;
        let mut saw_respawn = false;

        for _ in 0..100 {
            let trigger = (PoolMemberType::Creature, selected_of(&mgr, 10, 11));
            let roll = mgr.update_pool(1, CONTINENT_INSTANCE, trigger);

            if roll.trigger_respawns {
                saw_respawn = true;
                assert!(roll.to_spawn.is_empty() && roll.to_despawn.is_empty());
            } else {
                saw_swap = true;
                assert_eq!(despawned_keys(&roll), vec![trigger]);
                assert_eq!(roll.to_spawn.len(), 1);
                assert_ne!(roll.to_spawn[0], trigger);
            }

            assert_eq!(
                mgr.selected_count(1, CONTINENT_INSTANCE),
                1,
                "roster size must stay at the limit"
            );
        }

        assert!(saw_swap, "100 rolls never swapped the member");
        assert!(saw_respawn, "100 rolls never kept the member");
    }

    #[test]
    fn respawn_reroll_of_a_single_member_pool_respawns_in_place() {
        let mgr = simple_manager(1, 1);
        mgr.initialize_rosters();

        let roll = mgr.update_pool(1, CONTINENT_INSTANCE, (PoolMemberType::Creature, 10));
        assert!(roll.trigger_respawns);
        assert!(mgr.can_creature_spawn(10, CONTINENT_INSTANCE));
    }

    #[test]
    fn respawn_reroll_of_a_nested_pool_swaps_whole_child_pools() {
        let mgr = PoolManager::new();
        mgr.load_from_repository(PoolData {
            templates: vec![template(1, 1), template(2, 1), template(3, 1)],
            creature_members: vec![object_member(2, 20), object_member(3, 30)],
            gameobject_members: Vec::new(),
            pool_members: vec![nested(2, 1), nested(3, 1)],
        });
        mgr.initialize_rosters();

        let spawned = if mgr.can_creature_spawn(20, CONTINENT_INSTANCE) {
            20
        } else {
            30
        };
        let child_pool = if spawned == 20 { 2 } else { 3 };

        // Re-rolling the child pool goes through the mother pool.
        let mut swapped = false;
        for _ in 0..100 {
            let roll = mgr.update_pool(
                child_pool,
                CONTINENT_INSTANCE,
                (PoolMemberType::Creature, spawned),
            );
            if !roll.trigger_respawns {
                assert_eq!(
                    despawned_keys(&roll),
                    vec![(PoolMemberType::Creature, spawned)]
                );
                assert_eq!(roll.to_spawn.len(), 1);
                assert_ne!(roll.to_spawn[0], (PoolMemberType::Creature, spawned));
                swapped = true;
                break;
            }
        }
        assert!(swapped, "the mother pool never rolled the other child pool");
        assert_eq!(mgr.selected_objects(1, CONTINENT_INSTANCE).len(), 1);
    }

    #[test]
    fn despawned_member_keeps_its_roster_slot() {
        let mgr = simple_manager(1, 2);
        mgr.initialize_rosters();

        let selected = selected_of(&mgr, 10, 11);
        let key = (PoolMemberType::Creature, selected);
        let guid = ObjectGuid::new_creature(100, 1);

        mgr.mark_spawned(1, CONTINENT_INSTANCE, key, guid);
        assert_eq!(
            mgr.get_pool_membership(guid),
            Some((1, CONTINENT_INSTANCE, key))
        );

        assert_eq!(mgr.mark_despawned(guid), Some((1, key)));
        assert!(
            mgr.can_creature_spawn(selected, CONTINENT_INSTANCE),
            "a grid unload must not free the roster slot"
        );
        assert_eq!(mgr.selected_count(1, CONTINENT_INSTANCE), 1);
    }

    #[test]
    fn members_of_unknown_pools_are_rejected() {
        let mgr = PoolManager::new();
        mgr.load_from_repository(PoolData {
            templates: vec![template(1, 1)],
            creature_members: vec![object_member(1, 10), object_member(999, 11)],
            gameobject_members: Vec::new(),
            pool_members: Vec::new(),
        });

        assert_eq!(mgr.get_pool_for_creature(11), None);
        assert!(
            mgr.can_creature_spawn(11, CONTINENT_INSTANCE),
            "orphan spawns are not pooled"
        );
    }

    #[test]
    fn gameobjects_are_pooled_too() {
        let mgr = PoolManager::new();
        mgr.load_from_repository(PoolData {
            templates: vec![template(1, 1)],
            creature_members: Vec::new(),
            gameobject_members: vec![object_member(1, 50), object_member(1, 51)],
            pool_members: Vec::new(),
        });
        mgr.initialize_rosters();

        let spawnable = [50, 51]
            .into_iter()
            .filter(|id| mgr.can_gameobject_spawn(*id, CONTINENT_INSTANCE))
            .count();
        assert_eq!(spawnable, 1);
    }
}
