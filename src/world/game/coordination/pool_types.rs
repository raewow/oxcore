use crate::shared::protocol::ObjectGuid;

/// Pool member types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolMemberType {
    Creature,
    GameObject,
    Pool, // Nested pool
}

/// A member of a spawn pool
#[derive(Debug, Clone)]
pub struct PoolMember {
    /// Member ID (spawn GUID for creatures, entry for nested pools)
    pub id: u32,
    /// Member type
    pub member_type: PoolMemberType,
    /// Spawn chance (relative weight)
    pub chance: f32,
    /// Description for debugging
    pub description: String,
}

/// Pool configuration
#[derive(Debug, Clone)]
pub struct PoolTemplate {
    /// Pool ID
    pub pool_id: u32,
    /// Maximum spawned members at once
    pub max_limit: u32,
    /// Description
    pub description: String,
}

/// Runtime pool state
#[derive(Debug)]
pub struct PoolState {
    /// Pool template
    pub template: PoolTemplate,
    /// Members of this pool
    pub members: Vec<PoolMember>,
    /// Currently spawned members (GUID -> member_id)
    pub spawned: Vec<(ObjectGuid, u32)>,
    /// Explicitly disabled members
    pub disabled: Vec<u32>,
}

impl PoolState {
    pub fn new(template: PoolTemplate) -> Self {
        Self {
            template,
            members: Vec::new(),
            spawned: Vec::new(),
            disabled: Vec::new(),
        }
    }

    /// Current spawn count
    pub fn spawn_count(&self) -> u32 {
        self.spawned.len() as u32
    }

    /// Can spawn more?
    pub fn can_spawn(&self) -> bool {
        self.spawn_count() < self.template.max_limit
    }

    /// Get available members (not spawned, not disabled)
    pub fn available_members(&self) -> Vec<&PoolMember> {
        let spawned_ids: Vec<_> = self.spawned.iter().map(|(_, id)| *id).collect();

        self.members
            .iter()
            .filter(|m| !spawned_ids.contains(&m.id) && !self.disabled.contains(&m.id))
            .collect()
    }

    /// Select a random member based on weights
    pub fn select_random(&self) -> Option<&PoolMember> {
        let available = self.available_members();
        if available.is_empty() {
            return None;
        }

        // Calculate total weight
        let total: f32 = available.iter().map(|m| m.chance).sum();
        if total <= 0.0 {
            return available.first().copied();
        }

        // Random selection
        let mut roll = rand::random::<f32>() * total;
        for member in &available {
            roll -= member.chance;
            if roll <= 0.0 {
                return Some(member);
            }
        }

        available.last().copied()
    }

    /// Mark member as spawned
    pub fn mark_spawned(&mut self, guid: ObjectGuid, member_id: u32) {
        self.spawned.push((guid, member_id));
    }

    /// Mark member as despawned
    pub fn mark_despawned(&mut self, guid: ObjectGuid) {
        self.spawned.retain(|(g, _)| *g != guid);
    }
}
