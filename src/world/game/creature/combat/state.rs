//! Combat State for Creatures - Threat tracking and combat status
//!
//! Architecture Note: Per Option 3 decision, combat state is embedded in Creature
//! for performance (50k creatures, avoid DashMap lookups). Players use CombatSystem.

use crate::shared::protocol::ObjectGuid;
use std::collections::HashSet;

/// Threat entry for basic threat tracking
/// Phase 2: Simple threat = damage dealt
/// Phase 5: Will add threat modifiers, taunt, fade, etc.
#[derive(Debug, Clone)]
pub struct ThreatEntry {
    pub guid: ObjectGuid,
    pub threat: f32,
    pub timestamp: u64,
}

/// Combat state for a creature (embedded in Creature struct)
///
/// Architecture Note: Per Option 3 decision, combat state is embedded in Creature
/// for performance (50k creatures, avoid DashMap lookups). Players use CombatSystem.
#[derive(Debug, Clone, Default)]
pub struct CombatState {
    /// Currently in combat
    pub in_combat: bool,

    /// Current attack target (for when creature fights back - Phase 4 AI)
    pub attacking: Option<ObjectGuid>,

    /// All entities currently attacking us
    pub attackers: HashSet<ObjectGuid>,

    /// Timestamp when combat started (milliseconds)
    pub combat_start_time: u64,

    /// Threat list for target selection (Phase 2: basic, Phase 5: full system)
    pub threat_list: Vec<ThreatEntry>,
}

impl CombatState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enter combat with an attacker
    pub fn enter_combat(&mut self, attacker: ObjectGuid, timestamp: u64) {
        self.in_combat = true;
        self.attackers.insert(attacker);
        if self.combat_start_time == 0 {
            self.combat_start_time = timestamp;
        }
    }

    /// Leave combat
    pub fn leave_combat(&mut self) {
        self.in_combat = false;
        self.attacking = None;
        self.attackers.clear();
        self.combat_start_time = 0;
        self.threat_list.clear();
    }

    /// Remove an attacker (they died, left range, etc.)
    pub fn remove_attacker(&mut self, attacker: ObjectGuid) {
        self.attackers.remove(&attacker);

        // Remove from threat list
        self.threat_list.retain(|entry| entry.guid != attacker);

        if self.attackers.is_empty() {
            self.leave_combat();
        }
    }

    /// Add threat for an attacker (Phase 2: basic damage = threat)
    /// Phase 5 TODO: Add threat modifiers (taunt = 100x, healing = 0.5x, etc.)
    pub fn add_threat(&mut self, attacker: ObjectGuid, amount: f32, timestamp: u64) {
        // Find existing entry or create new
        if let Some(entry) = self.threat_list.iter_mut().find(|e| e.guid == attacker) {
            entry.threat += amount;
            entry.timestamp = timestamp;
        } else {
            self.threat_list.push(ThreatEntry {
                guid: attacker,
                threat: amount,
                timestamp,
            });
        }
    }

    /// Get highest threat target for AI target selection
    /// Returns None if no valid threats
    pub fn get_highest_threat(&self) -> Option<ObjectGuid> {
        self.threat_list
            .iter()
            .max_by(|a, b| a.threat.partial_cmp(&b.threat).unwrap())
            .map(|entry| entry.guid)
    }

    /// Get threat amount for a specific attacker
    pub fn get_threat(&self, guid: ObjectGuid) -> f32 {
        self.threat_list
            .iter()
            .find(|e| e.guid == guid)
            .map(|e| e.threat)
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combat_state_enter_combat() {
        let mut state = CombatState::new();
        let attacker = ObjectGuid::new_player(1);
        let timestamp = 1000;

        state.enter_combat(attacker, timestamp);

        assert!(state.in_combat);
        assert!(state.attackers.contains(&attacker));
        assert_eq!(state.combat_start_time, timestamp);
    }

    #[test]
    fn test_combat_state_leave_combat() {
        let mut state = CombatState::new();
        let attacker = ObjectGuid::new_player(1);

        state.enter_combat(attacker, 1000);
        state.leave_combat();

        assert!(!state.in_combat);
        assert!(state.attackers.is_empty());
        assert_eq!(state.combat_start_time, 0);
        assert!(state.threat_list.is_empty());
    }

    #[test]
    fn test_combat_state_add_threat() {
        let mut state = CombatState::new();
        let attacker = ObjectGuid::new_player(1);
        let timestamp = 1000;

        state.add_threat(attacker, 100.0, timestamp);

        assert_eq!(state.get_threat(attacker), 100.0);

        // Add more threat
        state.add_threat(attacker, 50.0, timestamp + 100);
        assert_eq!(state.get_threat(attacker), 150.0);
    }

    #[test]
    fn test_combat_state_get_highest_threat() {
        let mut state = CombatState::new();
        let attacker1 = ObjectGuid::new_player(1);
        let attacker2 = ObjectGuid::new_player(2);
        let timestamp = 1000;

        state.add_threat(attacker1, 100.0, timestamp);
        state.add_threat(attacker2, 50.0, timestamp);

        assert_eq!(state.get_highest_threat(), Some(attacker1));
    }

    #[test]
    fn test_combat_state_remove_attacker() {
        let mut state = CombatState::new();
        let attacker = ObjectGuid::new_player(1);

        state.enter_combat(attacker, 1000);
        state.add_threat(attacker, 100.0, 1000);
        state.remove_attacker(attacker);

        assert!(!state.in_combat);
        assert!(state.attackers.is_empty());
        assert_eq!(state.get_threat(attacker), 0.0);
    }
}
