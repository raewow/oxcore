//! Threat Manager - Main threat management for creatures
//!
//! This is the main public API for the threat system. Features:
//! - Online/Offline threat separation
//! - GM immunity (always offline)
//! - Spell attribute-based threat modifiers
//! - Aura-based threat modifiers
//! - Taunt with temporary threat modifiers
//! - Pet owner threat auto-generation
//! - Accessibility checks (in water, unreachable)

use super::calc::ThreatCalcHelper;
use super::container::ThreatContainer;
use crate::shared::protocol::ObjectGuid;

/// Hostile reference - represents a single target on the threat list
#[derive(Debug, Clone)]
pub struct HostileReference {
    pub target: ObjectGuid,
    /// Base threat value
    pub threat: f32,
    /// Temporary threat modifier (used for taunt)
    /// Added to threat only during target selection
    pub temp_threat_modifier: f32,
    /// Whether this reference is online (accessible)
    pub online: bool,
    /// Whether this reference is suppressed (e.g., GM)
    pub suppressed: bool,
}

/// Manages threat list for a creature
#[derive(Debug, Clone)]
pub struct ThreatManager {
    /// GUID of the creature that owns this threat manager
    owner: ObjectGuid,
    /// Threat container (online/offline separation with lazy sorting)
    container: ThreatContainer,
    /// Current victim (highest online threat target)
    current_victim: Option<ObjectGuid>,
    /// Whether this creature is a ranged attacker (affects switch threshold)
    is_ranged_attacker: bool,
    /// Taunt expiry times: target_guid -> expiry_timestamp_ms
    taunt_expiry: std::collections::HashMap<ObjectGuid, u64>,
}

impl ThreatManager {
    pub fn new(owner: ObjectGuid) -> Self {
        Self {
            owner,
            container: ThreatContainer::new(),
            current_victim: None,
            is_ranged_attacker: false,
            taunt_expiry: std::collections::HashMap::new(),
        }
    }

    /// Add threat to a unit (simplified version for damage)
    ///
    /// This is the main method for adding threat from damage.
    pub fn add_damage_threat(&mut self, source: ObjectGuid, damage: u32, threat_modifier: f32) {
        // Don't add threat to self
        if source == self.owner {
            return;
        }

        let threat = damage as f32 * threat_modifier;

        if threat > 0.0 {
            self.container.add_threat(source, threat);
            self.update_victim();
        }
    }

    /// Add healing threat (assist threat)
    ///
    /// Healing threat is typically 0.5x the amount healed.
    /// This should be called through AssistThreatHelper which checks for hard CC.
    pub fn add_healing_threat(&mut self, healer: ObjectGuid, healing: u32, threat_modifier: f32) {
        // Don't add threat to self
        if healer == self.owner {
            return;
        }

        // Healing threat is typically 0.5x the amount healed
        let threat = healing as f32 * 0.5 * threat_modifier;

        if threat > 0.0 {
            self.container.add_threat(healer, threat);
            self.update_victim();
        }
    }

    /// Add raw threat (for spell effects, taunt, etc.)
    pub fn add_threat(&mut self, target: ObjectGuid, amount: f32) {
        if target != self.owner && amount > 0.0 {
            self.container.add_threat(target, amount);
            self.update_victim();
        }
    }

    /// Apply taunt effect (temporary threat modifier)
    ///
    /// Taunt uses a temporary threat modifier system, not fixed threat setting.
    /// The modifier is cleared after a short duration.
    pub fn taunt(&mut self, taunter: ObjectGuid, current_time_ms: u64, duration_ms: u32) {
        // Ensure reference exists
        self.container.ensure_reference(taunter);

        // Set temporary threat modifier to force target switch
        let temp_modifier = 1000000.0; // Very high value
        self.container
            .set_temp_threat_modifier(taunter, temp_modifier);

        // Schedule removal of temp modifier after duration
        let expiry_time = current_time_ms + duration_ms as u64;
        self.taunt_expiry.insert(taunter, expiry_time);

        // Force victim update
        self.update_victim();
    }

    /// Update taunt states (called on tick)
    pub fn update(&mut self, current_time_ms: u64) {
        // Clear expired taunts
        let expired: Vec<ObjectGuid> = self
            .taunt_expiry
            .iter()
            .filter(|(_, &expiry)| current_time_ms >= expiry)
            .map(|(guid, _)| *guid)
            .collect();

        for guid in expired {
            self.taunt_expiry.remove(&guid);
            self.container.clear_temp_threat_modifier(guid);
        }

        self.update_victim();
    }

    /// Remove a target from threat list
    pub fn remove_target(&mut self, target: ObjectGuid) {
        self.container.remove_target(target);
        self.taunt_expiry.remove(&target);
        if self.current_victim == Some(target) {
            self.update_victim();
        }
    }

    /// Clear all threat
    pub fn clear(&mut self) {
        self.container.clear();
        self.current_victim = None;
        self.taunt_expiry.clear();
    }

    /// Get current victim
    pub fn get_victim(&self) -> Option<ObjectGuid> {
        self.current_victim
    }

    /// Get sorted threat list
    pub fn get_threat_list(&self) -> Vec<(ObjectGuid, f32)> {
        self.container.get_threat_list()
    }

    /// Check if target is on threat list
    pub fn has_target(&self, target: ObjectGuid) -> bool {
        self.container.get_threat(target) > 0.0
    }

    /// Get threat for a specific target
    pub fn get_threat(&self, target: ObjectGuid) -> f32 {
        self.container.get_threat(target)
    }

    /// Set whether this creature is a ranged attacker (affects switch threshold)
    pub fn set_ranged_attacker(&mut self, is_ranged: bool) {
        self.is_ranged_attacker = is_ranged;
    }

    /// Move a target offline (inaccessible)
    pub fn set_target_offline(&mut self, target: ObjectGuid) {
        self.container.set_offline(target);
        if self.current_victim == Some(target) {
            self.update_victim();
        }
    }

    /// Move a target back online (accessible)
    pub fn set_target_online(&mut self, target: ObjectGuid) {
        self.container.set_online(target);
        self.update_victim();
    }

    /// Update current victim based on threat
    ///
    /// Uses 110% threshold for melee, 130% for ranged to prevent ping-ponging
    fn update_victim(&mut self) {
        // Get current victim's effective threat
        let current_threat = self
            .current_victim
            .map(|v| self.container.get_effective_threat(v))
            .unwrap_or(0.0);

        // Get new highest threat target
        let new_victim = self.container.get_victim();
        let new_threat = new_victim
            .map(|v| self.container.get_effective_threat(v))
            .unwrap_or(0.0);

        // Determine threshold based on creature attack type
        let threshold = if self.is_ranged_attacker {
            1.30 // 130% for ranged attackers
        } else {
            1.10 // 110% for melee attackers
        };

        // Only switch if new threat exceeds threshold OR no current victim
        if new_threat > current_threat * threshold || self.current_victim.is_none() {
            if let Some(new_target) = new_victim {
                self.current_victim = Some(new_target);
            } else {
                self.current_victim = None;
            }
        }
    }
}

impl Default for ThreatManager {
    fn default() -> Self {
        // Use a default owner GUID - this is only for testing
        Self::new(ObjectGuid::new_creature(0, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_damage_threat() {
        let owner = ObjectGuid::new_creature(1, 1);
        let mut manager = ThreatManager::new(owner);
        let attacker = ObjectGuid::new_player(1);

        manager.add_damage_threat(attacker, 100, 1.0);
        assert_eq!(manager.get_threat(attacker), 100.0);

        manager.add_damage_threat(attacker, 50, 1.0);
        assert_eq!(manager.get_threat(attacker), 150.0);
    }

    #[test]
    fn test_target_switch_threshold() {
        let owner = ObjectGuid::new_creature(1, 1);
        let mut manager = ThreatManager::new(owner);
        let attacker1 = ObjectGuid::new_player(1);
        let attacker2 = ObjectGuid::new_player(2);

        // First attacker becomes victim
        manager.add_damage_threat(attacker1, 100, 1.0);
        assert_eq!(manager.get_victim(), Some(attacker1));

        // Second attacker needs 110% threat to pull aggro (melee)
        manager.add_damage_threat(attacker2, 105, 1.0); // 105 < 110
        assert_eq!(manager.get_victim(), Some(attacker1)); // Still attacker1

        manager.add_damage_threat(attacker2, 10, 1.0); // 115 > 110
        assert_eq!(manager.get_victim(), Some(attacker2)); // Now attacker2
    }

    #[test]
    fn test_ranged_threshold() {
        let owner = ObjectGuid::new_creature(1, 1);
        let mut manager = ThreatManager::new(owner);
        manager.set_ranged_attacker(true);

        let attacker1 = ObjectGuid::new_player(1);
        let attacker2 = ObjectGuid::new_player(2);

        manager.add_damage_threat(attacker1, 100, 1.0);
        assert_eq!(manager.get_victim(), Some(attacker1));

        // Ranged needs 130% threat
        manager.add_damage_threat(attacker2, 120, 1.0); // 120 < 130
        assert_eq!(manager.get_victim(), Some(attacker1));

        manager.add_damage_threat(attacker2, 15, 1.0); // 135 > 130
        assert_eq!(manager.get_victim(), Some(attacker2));
    }

    #[test]
    fn test_taunt() {
        let owner = ObjectGuid::new_creature(1, 1);
        let mut manager = ThreatManager::new(owner);
        let attacker1 = ObjectGuid::new_player(1);
        let attacker2 = ObjectGuid::new_player(2);

        // Attacker1 has high threat
        manager.add_damage_threat(attacker1, 1000, 1.0);
        assert_eq!(manager.get_victim(), Some(attacker1));

        // Attacker2 taunts (even with 0 threat)
        manager.taunt(attacker2, 0, 3000);
        assert_eq!(manager.get_victim(), Some(attacker2));

        // After taunt expires, should return to attacker1
        manager.update(3001);
        assert_eq!(manager.get_victim(), Some(attacker1));
    }

    #[test]
    fn test_clear() {
        let owner = ObjectGuid::new_creature(1, 1);
        let mut manager = ThreatManager::new(owner);
        let attacker = ObjectGuid::new_player(1);

        manager.add_damage_threat(attacker, 100, 1.0);
        assert!(manager.has_target(attacker));

        manager.clear();
        assert!(!manager.has_target(attacker));
        assert_eq!(manager.get_victim(), None);
    }
}
