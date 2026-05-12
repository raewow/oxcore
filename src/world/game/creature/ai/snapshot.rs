//! AI Snapshot - Read-only state for AI decisions
//!
//! The snapshot pattern is critical for deadlock-free AI:
//! 1. Brief lock to copy creature state into snapshot
//! 2. Release lock
//! 3. Pure function makes decisions using snapshot (no locks held)
//! 4. Execute actions with proper locking

use super::types::{AIState, AIStateData};
use crate::shared::protocol::{ObjectGuid, Position};

/// Read-only snapshot of creature state for AI decisions
/// Captured with brief lock, then used without holding any locks
#[derive(Debug, Clone)]
pub struct CreatureSnapshot {
    pub guid: ObjectGuid,
    pub entry: u32,
    pub map_id: u32,
    pub instance_id: u32,
    pub position: Position,
    pub home_position: Position,
    pub ai_state: AIState,
    pub ai_type: super::types::AIType,
    pub current_target: Option<ObjectGuid>,
    pub threat_list: Vec<ThreatEntry>,
    pub health_pct: f32,
    pub current_health: u32,
    pub max_health: u32,
    pub in_combat: bool,
    pub is_alive: bool,
    pub attack_timer_ready: bool,
    pub ai_state_data: AIStateData,
    pub combat_reach: f32,
    pub spells: [u32; 4],
    pub current_mana: u32,
    pub max_mana: u32,
    pub level: u8,
    pub unit_class: u8,
    /// Active auras: (spell_id, remaining_ms, stacks)
    pub auras: Vec<(u32, u32, u8)>,
}

/// Threat entry in the snapshot
#[derive(Debug, Clone)]
pub struct ThreatEntry {
    pub target: ObjectGuid,
    pub threat: f32,
}

/// Snapshot of a potential target
#[derive(Debug, Clone)]
pub struct TargetSnapshot {
    pub guid: ObjectGuid,
    pub position: Position,
    pub is_alive: bool,
    pub is_player: bool,
    pub health_pct: f32,
    pub has_mana: bool,
}

impl CreatureSnapshot {
    /// Calculate distance to a position
    pub fn distance_to(&self, pos: &Position) -> f32 {
        let dx = self.position.x - pos.x;
        let dy = self.position.y - pos.y;
        let dz = self.position.z - pos.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Calculate distance to home
    pub fn distance_to_home(&self) -> f32 {
        self.distance_to(&self.home_position)
    }

    /// Calculate 2D distance to a position (ignoring Z)
    pub fn distance_to_2d(&self, pos: &Position) -> f32 {
        let dx = self.position.x - pos.x;
        let dy = self.position.y - pos.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Get highest threat target
    pub fn get_top_threat_target(&self) -> Option<ObjectGuid> {
        self.threat_list
            .iter()
            .max_by(|a, b| a.threat.partial_cmp(&b.threat).unwrap())
            .map(|e| e.target)
    }

    /// Get threat amount for a specific target
    pub fn get_threat(&self, target: ObjectGuid) -> f32 {
        self.threat_list
            .iter()
            .find(|e| e.target == target)
            .map(|e| e.threat)
            .unwrap_or(0.0)
    }

    /// Check if we have any valid targets
    pub fn has_targets(&self) -> bool {
        !self.threat_list.is_empty()
    }

    /// Get number of targets
    pub fn target_count(&self) -> usize {
        self.threat_list.len()
    }
}

/// Input to the AI decision function
/// Contains all data needed for pure decision making
#[derive(Debug, Clone)]
pub struct AIInput {
    /// Snapshot of creature state
    pub snapshot: CreatureSnapshot,
    /// Events to process
    pub events: Vec<super::types::AIEvent>,
    /// Time delta since last update (milliseconds)
    pub diff_ms: u32,
    /// Snapshots of nearby targets
    pub nearby_targets: Vec<TargetSnapshot>,
}

/// Result of AI decision making
#[derive(Debug, Clone, Default)]
pub struct AIDecisionResult {
    /// Actions to execute
    pub actions: Vec<super::types::AIAction>,
    /// Whether to override base AI behavior (for Lua scripts)
    pub override_base_ai: bool,
    /// Updated AI state data (cooldowns, timers, etc.)
    pub updated_state_data: Option<AIStateData>,
}

impl AIDecisionResult {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an action to the result
    pub fn add_action(mut self, action: super::types::AIAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Add multiple actions
    pub fn add_actions(mut self, actions: Vec<super::types::AIAction>) -> Self {
        self.actions.extend(actions);
        self
    }

    /// Merge another result into this one
    pub fn merge(&mut self, other: AIDecisionResult) {
        self.actions.extend(other.actions);
        if other.override_base_ai {
            self.override_base_ai = true;
        }
        if let Some(state_data) = other.updated_state_data {
            self.updated_state_data = Some(state_data);
        }
    }

    /// Set the override flag
    pub fn set_override(mut self, override_base: bool) -> Self {
        self.override_base_ai = override_base;
        self
    }

    /// Set updated state data
    pub fn with_state_data(mut self, state_data: AIStateData) -> Self {
        self.updated_state_data = Some(state_data);
        self
    }
}
