//! Spawn pool data types (port of MaNGOS `PoolManager.h` `PoolGroup`/`PoolObject`).
//!
//! A pool holds a set of candidate spawns and only ever keeps `max_limit` of
//! them in the world at a time. Which candidates are picked is the pool's
//! *roster* (`selected`); the roster is rolled at startup and re-rolled
//! whenever one of its members is ready to respawn.

use oxcore_shared::protocol::ObjectGuid;
use rand::Rng;
use std::collections::{HashMap, HashSet};

/// Pool template flags (`pool_template.flags`).
pub mod pool_flags {
    /// Spawn at pool system start (not part of another pool).
    pub const AUTO_SPAWN: u32 = 0x1;
    /// Scale `max_limit` with the realm population.
    pub const MAXLIMIT_SCALING_LINEAR: u32 = 0x2;
}

/// Pool member types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PoolMemberType {
    Creature,
    GameObject,
    Pool, // Nested pool
}

/// Identifies one member inside a pool: its type plus its spawn id
/// (creature/gameobject DB guid, or the child pool id).
pub type PoolMemberKey = (PoolMemberType, u32);

/// A member of a spawn pool
#[derive(Debug, Clone)]
pub struct PoolMember {
    /// Member ID (DB spawn guid for creatures/gameobjects, pool id for nested pools)
    pub id: u32,
    /// Member type
    pub member_type: PoolMemberType,
    /// Spawn chance in percent; 0 means "equal chance with its peers"
    pub chance: f32,
    /// Description for debugging
    pub description: String,
}

impl PoolMember {
    pub fn key(&self) -> PoolMemberKey {
        (self.member_type, self.id)
    }
}

/// Pool configuration
#[derive(Debug, Clone)]
pub struct PoolTemplate {
    /// Pool ID
    pub pool_id: u32,
    /// Maximum members spawned at once
    pub max_limit: u32,
    /// `pool_template.flags`
    pub flags: u32,
    /// Description
    pub description: String,
}

impl PoolTemplate {
    /// Is this pool spawned on its own at startup?
    pub fn is_auto_spawn(&self) -> bool {
        self.flags & pool_flags::AUTO_SPAWN != 0
    }
}

/// Roster of one pool in one map instance.
///
/// MaNGOS stores this in the map's persistent state, so every instance of a
/// map rolls its own members; continents share instance 0.
#[derive(Debug, Default)]
struct PoolInstance {
    /// Members the pool wants in the world in this instance
    selected: HashSet<PoolMemberKey>,
    /// Roster members that are actually instantiated right now (grid loaded)
    spawned: HashMap<PoolMemberKey, ObjectGuid>,
}

/// Runtime pool state
#[derive(Debug)]
pub struct PoolState {
    /// Pool template
    pub template: PoolTemplate,
    /// Members with an explicit spawn chance (rolled first, in order)
    explicitly_chanced: Vec<PoolMember>,
    /// Members sharing the remaining chance equally
    equal_chanced: Vec<PoolMember>,
    /// Per map-instance rosters
    instances: HashMap<u32, PoolInstance>,
    /// Members excluded from future rolls (all instances)
    excluded: HashSet<PoolMemberKey>,
}

impl PoolState {
    pub fn new(template: PoolTemplate) -> Self {
        Self {
            template,
            explicitly_chanced: Vec::new(),
            equal_chanced: Vec::new(),
            instances: HashMap::new(),
            excluded: HashSet::new(),
        }
    }

    /// Add a member (MaNGOS `PoolGroup::AddEntry`).
    ///
    /// An explicit chance only means anything for pools that spawn a single
    /// member; everything else rolls with equal chance.
    pub fn add_member(&mut self, member: PoolMember) {
        if member.chance != 0.0 && self.template.max_limit == 1 {
            self.explicitly_chanced.push(member);
        } else {
            self.equal_chanced.push(member);
        }
    }

    /// All members of this pool.
    pub fn members(&self) -> impl Iterator<Item = &PoolMember> {
        self.explicitly_chanced.iter().chain(&self.equal_chanced)
    }

    pub fn member_count(&self) -> usize {
        self.explicitly_chanced.len() + self.equal_chanced.len()
    }

    pub fn is_empty(&self) -> bool {
        self.member_count() == 0
    }

    /// Chance integrity check (MaNGOS `PoolGroup::CheckPool`): with no
    /// equal-chanced members the explicit chances must sum to 100 (or 0).
    pub fn check_chances(&self) -> bool {
        if !self.equal_chanced.is_empty() {
            return true;
        }

        let sum: f32 = self.explicitly_chanced.iter().map(|m| m.chance).sum();
        sum == 100.0 || sum == 0.0
    }

    /// Has this instance rolled its roster yet?
    pub fn has_roster(&self, instance_id: u32) -> bool {
        self.instances.contains_key(&instance_id)
    }

    /// Instances that have rolled a roster.
    pub fn instance_ids(&self) -> Vec<u32> {
        self.instances.keys().copied().collect()
    }

    /// Members currently on an instance's roster.
    pub fn selected_count(&self, instance_id: u32) -> u32 {
        self.instances
            .get(&instance_id)
            .map(|inst| inst.selected.len() as u32)
            .unwrap_or(0)
    }

    /// Can another member be added to this instance's roster?
    pub fn can_select_more(&self, instance_id: u32) -> bool {
        self.selected_count(instance_id) < self.template.max_limit
    }

    pub fn is_selected(&self, instance_id: u32, key: PoolMemberKey) -> bool {
        self.instances
            .get(&instance_id)
            .map(|inst| inst.selected.contains(&key))
            .unwrap_or(false)
    }

    pub fn selected_members(&self, instance_id: u32) -> Vec<PoolMemberKey> {
        self.instances
            .get(&instance_id)
            .map(|inst| inst.selected.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn select(&mut self, instance_id: u32, key: PoolMemberKey) {
        self.instances
            .entry(instance_id)
            .or_default()
            .selected
            .insert(key);
    }

    /// Start an (empty) roster for an instance, so it counts as rolled even
    /// when the pool cannot pick anything.
    pub fn open_roster(&mut self, instance_id: u32) {
        self.instances.entry(instance_id).or_default();
    }

    /// Drop a member from an instance's roster; returns its runtime object if
    /// it had one.
    pub fn deselect(&mut self, instance_id: u32, key: PoolMemberKey) -> Option<ObjectGuid> {
        let inst = self.instances.get_mut(&instance_id)?;
        inst.selected.remove(&key);
        inst.spawned.remove(&key)
    }

    /// Exclude/allow a member for future rolls (MaNGOS `SetExcludeObject`).
    pub fn set_excluded(&mut self, key: PoolMemberKey, excluded: bool) {
        if excluded {
            self.excluded.insert(key);
        } else {
            self.excluded.remove(&key);
        }
    }

    /// Pick one member that is not on the instance's roster yet
    /// (MaNGOS `PoolGroup::RollOne`).
    ///
    /// `trigger` (the member that is being replaced) may be rolled again even
    /// though it is still on the roster — that is the "respawn in place" case.
    pub fn roll_one(&self, instance_id: u32, trigger: Option<PoolMemberKey>) -> Option<PoolMember> {
        self.roll_one_with(instance_id, trigger, &mut rand::thread_rng())
    }

    fn roll_one_with<R: Rng>(
        &self,
        instance_id: u32,
        trigger: Option<PoolMemberKey>,
        rng: &mut R,
    ) -> Option<PoolMember> {
        let selected = self.instances.get(&instance_id);
        let rollable = |member: &PoolMember| {
            let key = member.key();
            let already_selected = selected
                .map(|inst| inst.selected.contains(&key))
                .unwrap_or(false);
            !self.excluded.contains(&key) && (Some(key) == trigger || !already_selected)
        };

        if !self.explicitly_chanced.is_empty() {
            let mut roll = rng.gen::<f32>() * 100.0;
            for member in &self.explicitly_chanced {
                roll -= member.chance;
                if roll < 0.0 && rollable(member) {
                    return Some(member.clone());
                }
            }
        }

        let candidates: Vec<&PoolMember> =
            self.equal_chanced.iter().filter(|m| rollable(m)).collect();
        if candidates.is_empty() {
            return None;
        }

        let index = rng.gen_range(0..candidates.len());
        Some(candidates[index].clone())
    }

    /// Record the runtime object created for a roster member.
    pub fn mark_spawned(&mut self, instance_id: u32, key: PoolMemberKey, guid: ObjectGuid) {
        self.instances
            .entry(instance_id)
            .or_default()
            .spawned
            .insert(key, guid);
    }

    /// Forget the runtime object of a roster member (grid unload, despawn).
    /// The member stays on the roster so it comes back when the grid reloads.
    pub fn mark_despawned(&mut self, instance_id: u32, guid: ObjectGuid) -> Option<PoolMemberKey> {
        let inst = self.instances.get_mut(&instance_id)?;
        let key = *inst
            .spawned
            .iter()
            .find(|(_, spawned_guid)| **spawned_guid == guid)
            .map(|(key, _)| key)?;
        inst.spawned.remove(&key);
        Some(key)
    }

    /// Runtime object of a roster member, if it is instantiated.
    pub fn spawned_guid(&self, instance_id: u32, key: PoolMemberKey) -> Option<ObjectGuid> {
        self.instances.get(&instance_id)?.spawned.get(&key).copied()
    }

    /// Number of instantiated members in an instance.
    pub fn spawn_count(&self, instance_id: u32) -> u32 {
        self.instances
            .get(&instance_id)
            .map(|inst| inst.spawned.len() as u32)
            .unwrap_or(0)
    }

    /// Member key for a runtime object.
    pub fn key_for_guid(&self, instance_id: u32, guid: ObjectGuid) -> Option<PoolMemberKey> {
        self.instances
            .get(&instance_id)?
            .spawned
            .iter()
            .find(|(_, spawned_guid)| **spawned_guid == guid)
            .map(|(key, _)| *key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template(pool_id: u32, max_limit: u32) -> PoolTemplate {
        PoolTemplate {
            pool_id,
            max_limit,
            flags: pool_flags::AUTO_SPAWN,
            description: String::new(),
        }
    }

    fn creature(id: u32, chance: f32) -> PoolMember {
        PoolMember {
            id,
            member_type: PoolMemberType::Creature,
            chance,
            description: String::new(),
        }
    }

    #[test]
    fn explicit_chance_only_applies_to_single_spawn_pools() {
        let mut single = PoolState::new(template(1, 1));
        single.add_member(creature(10, 60.0));
        single.add_member(creature(11, 40.0));
        assert_eq!(single.explicitly_chanced.len(), 2);
        assert!(single.equal_chanced.is_empty());

        // Same members in a pool that spawns two: chance is ignored.
        let mut multi = PoolState::new(template(1, 2));
        multi.add_member(creature(10, 60.0));
        multi.add_member(creature(11, 40.0));
        assert!(multi.explicitly_chanced.is_empty());
        assert_eq!(multi.equal_chanced.len(), 2);
    }

    /// Continents (and everything not in an instance) use instance 0.
    const CONTINENT: u32 = 0;

    #[test]
    fn roll_never_picks_a_selected_member() {
        let mut pool = PoolState::new(template(1, 2));
        for id in 10..14 {
            pool.add_member(creature(id, 0.0));
        }
        pool.select(CONTINENT, (PoolMemberType::Creature, 10));
        pool.select(CONTINENT, (PoolMemberType::Creature, 11));

        for _ in 0..100 {
            let rolled = pool
                .roll_one(CONTINENT, None)
                .expect("two members are still free");
            assert!(matches!(rolled.id, 12 | 13));
        }
    }

    #[test]
    fn roll_can_repick_the_trigger() {
        let mut pool = PoolState::new(template(1, 1));
        pool.add_member(creature(10, 0.0));
        pool.select(CONTINENT, (PoolMemberType::Creature, 10));

        // Without the trigger exemption the only member is unavailable.
        assert!(pool.roll_one(CONTINENT, None).is_none());
        let rolled = pool
            .roll_one(CONTINENT, Some((PoolMemberType::Creature, 10)))
            .expect("the trigger itself may be rolled again");
        assert_eq!(rolled.id, 10);
    }

    #[test]
    fn rosters_are_independent_per_instance() {
        let mut pool = PoolState::new(template(1, 1));
        pool.add_member(creature(10, 0.0));
        pool.select(CONTINENT, (PoolMemberType::Creature, 10));

        // Another instance of the same map starts from a clean roster.
        assert!(!pool.is_selected(7, (PoolMemberType::Creature, 10)));
        assert_eq!(
            pool.roll_one(7, None).map(|m| m.id),
            Some(10),
            "instance 7 may pick a member instance 0 already holds"
        );
    }

    #[test]
    fn excluded_members_are_never_rolled() {
        let mut pool = PoolState::new(template(1, 2));
        pool.add_member(creature(10, 0.0));
        pool.add_member(creature(11, 0.0));
        pool.set_excluded((PoolMemberType::Creature, 11), true);

        for _ in 0..50 {
            assert_eq!(pool.roll_one(CONTINENT, None).unwrap().id, 10);
        }

        pool.set_excluded((PoolMemberType::Creature, 11), false);
        pool.select(CONTINENT, (PoolMemberType::Creature, 10));
        assert_eq!(pool.roll_one(CONTINENT, None).unwrap().id, 11);
    }

    #[test]
    fn chance_check_matches_mangos_rules() {
        let mut valid = PoolState::new(template(1, 1));
        valid.add_member(creature(10, 60.0));
        valid.add_member(creature(11, 40.0));
        assert!(valid.check_chances());

        let mut invalid = PoolState::new(template(1, 1));
        invalid.add_member(creature(10, 60.0));
        invalid.add_member(creature(11, 20.0));
        assert!(!invalid.check_chances());

        // Any equal-chanced member makes the pool rollable regardless.
        let mut mixed = PoolState::new(template(1, 1));
        mixed.add_member(creature(10, 60.0));
        mixed.add_member(creature(11, 0.0));
        assert!(mixed.check_chances());
    }

    #[test]
    fn despawn_keeps_the_member_on_the_roster() {
        let mut pool = PoolState::new(template(1, 1));
        pool.add_member(creature(10, 0.0));
        let key = (PoolMemberType::Creature, 10);
        let guid = ObjectGuid::new_creature(100, 1);

        pool.select(CONTINENT, key);
        pool.mark_spawned(CONTINENT, key, guid);
        assert_eq!(pool.spawned_guid(CONTINENT, key), Some(guid));

        assert_eq!(pool.mark_despawned(CONTINENT, guid), Some(key));
        assert!(
            pool.is_selected(CONTINENT, key),
            "grid unload must not clear the roster"
        );
        assert_eq!(pool.spawned_guid(CONTINENT, key), None);
    }
}
