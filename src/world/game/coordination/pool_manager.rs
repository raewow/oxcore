use super::pool_repository::PoolData;
use super::pool_types::{PoolMember, PoolMemberType, PoolState, PoolTemplate};
use crate::shared::protocol::ObjectGuid;
use dashmap::DashMap;
use std::sync::Arc;

/// Manages all spawn pools - state only, no database
pub struct PoolManager {
    /// Pools by ID
    pools: DashMap<u32, PoolState>,
    /// Creature spawn -> pool mapping
    creature_to_pool: DashMap<u32, u32>,
    /// Pool -> parent pool mapping (for nested pools)
    pool_to_parent: DashMap<u32, u32>,
}

impl PoolManager {
    pub fn new() -> Self {
        Self {
            pools: DashMap::new(),
            creature_to_pool: DashMap::new(),
            pool_to_parent: DashMap::new(),
        }
    }

    /// Load pools from repository data
    pub fn load_from_repository(&self, data: PoolData) {
        // Load templates
        for template in data.templates {
            let pool_id = template.pool_id;
            self.pools.insert(pool_id, PoolState::new(template));
        }

        // Load creature members
        for member in data.creature_members {
            if let Some(mut pool) = self.pools.get_mut(&member.pool_id) {
                pool.members.push(PoolMember {
                    id: member.spawn_id,
                    member_type: PoolMemberType::Creature,
                    chance: member.chance,
                    description: member.description,
                });
            }
            self.creature_to_pool
                .insert(member.spawn_id, member.pool_id);
        }

        // Load nested pool members
        for member in data.pool_members {
            if let Some(mut parent) = self.pools.get_mut(&member.parent_pool_id) {
                parent.members.push(PoolMember {
                    id: member.child_pool_id,
                    member_type: PoolMemberType::Pool,
                    chance: member.chance,
                    description: member.description,
                });
            }
            self.pool_to_parent
                .insert(member.child_pool_id, member.parent_pool_id);
        }

        tracing::info!("PoolManager loaded {} pools", self.pools.len());
    }

    /// Get pool for a creature spawn
    pub fn get_pool_for_creature(&self, spawn_id: u32) -> Option<u32> {
        self.creature_to_pool.get(&spawn_id).map(|r| *r)
    }

    /// Check if creature can spawn (pool allows it)
    pub fn can_creature_spawn(&self, spawn_id: u32) -> bool {
        if let Some(pool_id) = self.get_pool_for_creature(spawn_id) {
            if let Some(pool) = self.pools.get(&pool_id) {
                return pool.can_spawn();
            }
        }
        true // Not in pool, can spawn
    }

    /// Select spawn members for a pool (up to limit)
    pub fn select_spawn_members(&self, pool_id: u32) -> Vec<u32> {
        let mut members = Vec::new();

        let Some(mut pool) = self.pools.get_mut(&pool_id) else {
            return members;
        };

        while pool.can_spawn() {
            if let Some(member) = pool.select_random() {
                match member.member_type {
                    PoolMemberType::Creature => {
                        members.push(member.id);
                    }
                    PoolMemberType::Pool => {
                        // Recursively select from nested pool
                        let child_id = member.id;
                        drop(pool); // Release lock
                        let nested = self.select_spawn_members(child_id);
                        members.extend(nested);
                        pool = self.pools.get_mut(&pool_id).unwrap();
                    }
                    PoolMemberType::GameObject => {
                        // Handle game objects if needed
                    }
                }
            } else {
                break; // No available members
            }
        }

        members
    }

    /// Mark member as spawned
    pub fn mark_spawned(&self, pool_id: u32, guid: ObjectGuid, member_id: u32) {
        if let Some(mut pool) = self.pools.get_mut(&pool_id) {
            pool.mark_spawned(guid, member_id);
        }
    }

    /// Mark member as despawned
    pub fn mark_despawned(&self, pool_id: u32, guid: ObjectGuid) {
        if let Some(mut pool) = self.pools.get_mut(&pool_id) {
            pool.mark_despawned(guid);
        }
    }

    /// Get pool membership for a creature
    pub fn get_pool_membership(&self, guid: ObjectGuid) -> Option<(u32, u32)> {
        // Find pool and spawn_id for this GUID
        for pool in self.pools.iter() {
            for (spawned_guid, spawn_id) in &pool.spawned {
                if *spawned_guid == guid {
                    return Some((*pool.key(), *spawn_id));
                }
            }
        }
        None
    }

    /// Select replacement member from pool
    pub fn select_replacement(&self, pool_id: u32) -> Option<u32> {
        let pool = self.pools.get(&pool_id)?;
        pool.select_random().map(|m| m.id)
    }
}

impl Default for PoolManager {
    fn default() -> Self {
        Self::new()
    }
}
