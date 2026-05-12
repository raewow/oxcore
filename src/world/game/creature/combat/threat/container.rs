//! Threat Container - Online/Offline threat separation with lazy sorting
//!
//! The ThreatContainer separates online (accessible) and offline (inaccessible) threats:
//! - Online references: Targets that can be attacked (normal players)
//! - Offline references: Targets that cannot be attacked (GMs, taxi flyers, dead units)

use super::manager::HostileReference;
use crate::shared::protocol::ObjectGuid;

/// Threat container with online/offline separation
///
/// Online references: Targets that can be attacked (normal players)
/// Offline references: Targets that cannot be attacked (GMs, taxi flyers, dead units)
#[derive(Debug, Clone)]
pub struct ThreatContainer {
    /// Online threat references (accessible targets)
    online_refs: Vec<HostileReference>,
    /// Offline threat references (inaccessible targets)
    offline_refs: Vec<HostileReference>,
    /// Whether the online list needs re-sorting
    dirty: bool,
}

impl ThreatContainer {
    pub fn new() -> Self {
        Self {
            online_refs: Vec::new(),
            offline_refs: Vec::new(),
            dirty: false,
        }
    }

    /// Add threat to a target
    pub fn add_threat(&mut self, target: ObjectGuid, threat: f32) {
        // Find or create reference
        if let Some(ref_mut) = self.find_reference_mut(target) {
            ref_mut.threat += threat;
            self.dirty = true;
        } else {
            // Create new online reference
            self.online_refs.push(HostileReference {
                target,
                threat,
                temp_threat_modifier: 0.0,
                online: true,
                suppressed: false,
            });
            self.dirty = true;
        }
    }

    /// Move a target offline (becomes inaccessible)
    ///
    /// Used when:
    /// - Player enters taxi
    /// - Player dies
    /// - GM goes invisible/untargetable
    pub fn set_offline(&mut self, target: ObjectGuid) {
        if let Some(index) = self.online_refs.iter().position(|r| r.target == target) {
            let mut reference = self.online_refs.remove(index);
            reference.online = false;
            self.offline_refs.push(reference);
            self.dirty = true;
        }
    }

    /// Move a target back online (becomes accessible again)
    ///
    /// Used when:
    /// - Player exits taxi
    /// - Player resurrects
    pub fn set_online(&mut self, target: ObjectGuid) {
        if let Some(index) = self.offline_refs.iter().position(|r| r.target == target) {
            let mut reference = self.offline_refs.remove(index);
            reference.online = true;
            self.online_refs.push(reference);
            self.dirty = true;
        }
    }

    /// Get the most threatened target (lazy sorting)
    pub fn get_victim(&mut self) -> Option<ObjectGuid> {
        if self.online_refs.is_empty() {
            return None;
        }

        // Sort if dirty
        if self.dirty {
            self.sort_references();
            self.dirty = false;
        }

        // Return highest threat (first after sort)
        self.online_refs.first().map(|r| r.target)
    }

    /// Get threat for a specific target
    pub fn get_threat(&self, target: ObjectGuid) -> f32 {
        self.find_reference(target).map(|r| r.threat).unwrap_or(0.0)
    }

    /// Get effective threat for a specific target (base + temp modifier)
    pub fn get_effective_threat(&self, target: ObjectGuid) -> f32 {
        self.find_reference(target)
            .map(|r| r.threat + r.temp_threat_modifier)
            .unwrap_or(0.0)
    }

    /// Check if target is online
    pub fn is_online(&self, target: ObjectGuid) -> bool {
        self.online_refs.iter().any(|r| r.target == target)
    }

    /// Remove a target from the threat list
    pub fn remove_target(&mut self, target: ObjectGuid) {
        self.online_refs.retain(|r| r.target != target);
        self.offline_refs.retain(|r| r.target != target);
        self.dirty = true;
    }

    /// Clear all threats
    pub fn clear(&mut self) {
        self.online_refs.clear();
        self.offline_refs.clear();
        self.dirty = false;
    }

    /// Get sorted threat list (for AI snapshot)
    pub fn get_threat_list(&self) -> Vec<(ObjectGuid, f32)> {
        let mut list: Vec<_> = self
            .online_refs
            .iter()
            .chain(self.offline_refs.iter())
            .map(|r| (r.target, r.threat))
            .collect();
        list.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        list
    }

    /// Set temporary threat modifier (used for taunt)
    pub fn set_temp_threat_modifier(&mut self, target: ObjectGuid, modifier: f32) {
        if let Some(ref_mut) = self.find_reference_mut(target) {
            ref_mut.temp_threat_modifier = modifier;
            self.dirty = true;
        }
    }

    /// Clear temporary threat modifier
    pub fn clear_temp_threat_modifier(&mut self, target: ObjectGuid) {
        if let Some(ref_mut) = self.find_reference_mut(target) {
            ref_mut.temp_threat_modifier = 0.0;
            self.dirty = true;
        }
    }

    /// Ensure a reference exists (creates if not found)
    pub fn ensure_reference(&mut self, target: ObjectGuid) {
        if self.find_reference(target).is_none() {
            self.online_refs.push(HostileReference {
                target,
                threat: 0.0,
                temp_threat_modifier: 0.0,
                online: true,
                suppressed: false,
            });
            self.dirty = true;
        }
    }

    /// Sort references by effective threat (base + temp modifier)
    fn sort_references(&mut self) {
        self.online_refs.sort_by(|a, b| {
            let a_threat = a.threat + a.temp_threat_modifier;
            let b_threat = b.threat + b.temp_threat_modifier;
            b_threat.partial_cmp(&a_threat).unwrap()
        });
    }

    fn find_reference(&self, target: ObjectGuid) -> Option<&HostileReference> {
        self.online_refs
            .iter()
            .chain(self.offline_refs.iter())
            .find(|r| r.target == target)
    }

    fn find_reference_mut(&mut self, target: ObjectGuid) -> Option<&mut HostileReference> {
        self.online_refs
            .iter_mut()
            .chain(self.offline_refs.iter_mut())
            .find(|r| r.target == target)
    }
}

impl Default for ThreatContainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_threat() {
        let mut container = ThreatContainer::new();
        let target = ObjectGuid::new_player(1);

        container.add_threat(target, 100.0);
        assert_eq!(container.get_threat(target), 100.0);

        container.add_threat(target, 50.0);
        assert_eq!(container.get_threat(target), 150.0);
    }

    #[test]
    fn test_online_offline() {
        let mut container = ThreatContainer::new();
        let target = ObjectGuid::new_player(1);

        container.add_threat(target, 100.0);
        assert!(container.is_online(target));

        container.set_offline(target);
        assert!(!container.is_online(target));
        assert_eq!(container.get_threat(target), 100.0); // Threat preserved

        container.set_online(target);
        assert!(container.is_online(target));
    }

    #[test]
    fn test_get_victim() {
        let mut container = ThreatContainer::new();
        let target1 = ObjectGuid::new_player(1);
        let target2 = ObjectGuid::new_player(2);

        container.add_threat(target1, 100.0);
        container.add_threat(target2, 200.0);

        // target2 has higher threat
        assert_eq!(container.get_victim(), Some(target2));
    }

    #[test]
    fn test_temp_modifier() {
        let mut container = ThreatContainer::new();
        let target1 = ObjectGuid::new_player(1);
        let target2 = ObjectGuid::new_player(2);

        container.add_threat(target1, 1000.0);
        container.add_threat(target2, 500.0);

        // Without modifier, target1 is victim
        assert_eq!(container.get_victim(), Some(target1));

        // Add temp modifier to target2 (taunt effect)
        container.set_temp_threat_modifier(target2, 10000.0);

        // Now target2 should be victim due to temp modifier
        assert_eq!(container.get_victim(), Some(target2));
    }
}
