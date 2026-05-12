//! AuraContainer - manages all active auras on a player

use super::aura::Aura;
use super::state::*;
use std::collections::HashMap;

/// AuraContainer - manages all active auras on a player.
///
/// Vanilla WoW has 64 visible aura slots total:
/// - Slots 0-31:  Positive buffs (shown on buff bar)
/// - Slots 32-47: Negative debuffs (shown on debuff bar)
/// - Slots 48-63: Passive effects (hidden, from talents/racials)
///
/// The container handles:
/// - Slot allocation per category (positive/negative/passive)
/// - Aura lookup by spell_id, aura_type, or slot
/// - Duration tracking and expiry
/// - Stacking and refresh logic
#[derive(Debug, Clone)]
pub struct AuraContainer {
    /// All active auras, keyed by (spell_id, effect_index)
    auras: HashMap<(u32, u8), Aura>,

    /// Slot allocation map: (spell_id, effect_index) -> slot (0-63)
    slot_map: HashMap<(u32, u8), u8>,

    /// Reverse slot lookup: slot -> (spell_id, effect_index)
    reverse_slot_map: HashMap<u8, (u32, u8)>,

    /// Occupied slot bitmask for fast free-slot search (64 bits for 64 slots)
    occupied_slots: u64,
}

impl AuraContainer {
    pub fn new() -> Self {
        Self {
            auras: HashMap::new(),
            slot_map: HashMap::new(),
            reverse_slot_map: HashMap::new(),
            occupied_slots: 0,
        }
    }

    // =========================================================================
    // Slot Management
    // =========================================================================

    /// Find the first free slot in the given range [start, end] inclusive.
    fn find_free_slot_in_range(&self, start: u8, end: u8) -> Option<u8> {
        for slot in start..=end {
            if (self.occupied_slots & (1u64 << slot)) == 0 {
                return Some(slot);
            }
        }
        None
    }

    /// Find a free slot appropriate for the aura category.
    fn find_free_slot(&self, aura: &Aura) -> Option<u8> {
        if aura.is_passive() {
            self.find_free_slot_in_range(PASSIVE_SLOT_START, PASSIVE_SLOT_END)
        } else if aura.is_negative() {
            self.find_free_slot_in_range(NEGATIVE_SLOT_START, NEGATIVE_SLOT_END)
        } else {
            self.find_free_slot_in_range(POSITIVE_SLOT_START, POSITIVE_SLOT_END)
        }
    }

    /// Allocate a slot for an aura key.
    fn allocate_slot(&mut self, key: (u32, u8), aura: &Aura) -> Option<u8> {
        let slot = self.find_free_slot(aura)?;
        self.slot_map.insert(key, slot);
        self.reverse_slot_map.insert(slot, key);
        self.occupied_slots |= 1u64 << slot;
        Some(slot)
    }

    /// Free an allocated slot.
    fn free_slot(&mut self, key: (u32, u8)) {
        if let Some(slot) = self.slot_map.remove(&key) {
            // Only fully free the slot if no other effects of the same spell still use it
            let spell_id = key.0;
            let other_uses_slot = (0..3u8).any(|ei| {
                ei != key.1 && self.slot_map.get(&(spell_id, ei)).copied() == Some(slot)
            });
            if !other_uses_slot {
                self.reverse_slot_map.remove(&slot);
                self.occupied_slots &= !(1u64 << slot);
            }
        }
    }

    // =========================================================================
    // Add / Remove
    // =========================================================================

    /// Add an aura to the container.
    ///
    /// Handles three cases:
    /// 1. New aura: allocate slot, insert
    /// 2. Same spell+effect from same caster: refresh duration
    /// 3. Same spell+effect, stackable: increment stack count
    ///
    /// Multiple effects of the same spell share one visible slot (the first effect's slot).
    /// Returns the assigned slot, or None if no slots available.
    pub fn add_aura(&mut self, mut aura: Aura) -> Option<u8> {
        let key = (aura.spell_id, aura.effect_index);

        // Check if this aura already exists
        if let Some(existing) = self.auras.get_mut(&key) {
            // Same spell + same effect index already active
            if existing.caster_guid == aura.caster_guid {
                // Same caster: refresh duration and try to add stack
                existing.refresh_duration();
                if existing.max_stack_count > 1 {
                    existing.add_stack();
                }
            } else {
                // Different caster, same spell:
                // In vanilla, most buffs from different casters don't stack.
                // The new application replaces the old one if it has a higher value.
                if aura.base_value() >= existing.base_value() {
                    let slot = self.slot_map.get(&key).copied();
                    aura.slot = slot;
                    *existing = aura;
                }
                // Otherwise keep existing (higher value wins)
            }
            return self.slot_map.get(&key).copied();
        }

        // Check if another effect of the same spell already has a visible slot.
        // Multiple effects of the same spell share one visible slot in vanilla.
        let existing_slot = self.find_spell_slot(aura.spell_id);

        let slot = if let Some(shared_slot) = existing_slot {
            // Reuse the existing visible slot — don't allocate a new one
            self.slot_map.insert(key, shared_slot);
            self.reverse_slot_map.insert(shared_slot, key);
            // Don't set occupied bit again — it's already set
            shared_slot
        } else {
            // First effect of this spell — allocate a new visible slot
            self.allocate_slot(key, &aura)?
        };

        aura.slot = Some(slot);
        self.auras.insert(key, aura);
        Some(slot)
    }

    /// Find the visible slot already allocated for any effect of a given spell.
    fn find_spell_slot(&self, spell_id: u32) -> Option<u8> {
        for effect_index in 0..3u8 {
            if let Some(&slot) = self.slot_map.get(&(spell_id, effect_index)) {
                return Some(slot);
            }
        }
        None
    }

    /// Remove an aura by spell_id and effect_index.
    /// Returns the removed aura and its slot, or None if not found.
    pub fn remove_aura(&mut self, spell_id: u32, effect_index: u8) -> Option<(Aura, u8)> {
        let key = (spell_id, effect_index);
        if let Some(aura) = self.auras.remove(&key) {
            let slot = self.slot_map.get(&key).copied().unwrap_or(0);
            self.free_slot(key);
            Some((aura, slot))
        } else {
            None
        }
    }

    /// Remove all auras from a given spell (all 3 possible effect indices).
    /// Returns a Vec of (Aura, slot) for each removed aura.
    pub fn remove_spell_auras(&mut self, spell_id: u32) -> Vec<(Aura, u8)> {
        let mut removed = Vec::new();
        for effect_index in 0..3u8 {
            if let Some(pair) = self.remove_aura(spell_id, effect_index) {
                removed.push(pair);
            }
        }
        removed
    }

    /// Remove all auras. Returns all removed auras.
    pub fn remove_all(&mut self) -> Vec<Aura> {
        let auras: Vec<Aura> = self.auras.drain().map(|(_, a)| a).collect();
        self.slot_map.clear();
        self.reverse_slot_map.clear();
        self.occupied_slots = 0;
        auras
    }

    /// Remove all auras that are not passive.
    /// Used on death, entering/leaving instances, etc.
    pub fn remove_all_non_passive(&mut self) -> Vec<(Aura, u8)> {
        let keys_to_remove: Vec<(u32, u8)> = self
            .auras
            .iter()
            .filter(|(_, a)| !a.is_passive())
            .map(|(k, _)| *k)
            .collect();

        let mut removed = Vec::new();
        for key in keys_to_remove {
            if let Some(aura) = self.auras.remove(&key) {
                let slot = self.slot_map.get(&key).copied().unwrap_or(0);
                self.free_slot(key);
                removed.push((aura, slot));
            }
        }
        removed
    }

    /// Remove all auras of a specific aura type.
    /// Used for dispelling or mechanic-specific removal.
    pub fn remove_auras_by_type(&mut self, aura_type: u32) -> Vec<(Aura, u8)> {
        let keys_to_remove: Vec<(u32, u8)> = self
            .auras
            .iter()
            .filter(|(_, a)| a.aura_type == aura_type)
            .map(|(k, _)| *k)
            .collect();

        let mut removed = Vec::new();
        for key in keys_to_remove {
            if let Some(aura) = self.auras.remove(&key) {
                let slot = self.slot_map.get(&key).copied().unwrap_or(0);
                self.free_slot(key);
                removed.push((aura, slot));
            }
        }
        removed
    }

    // =========================================================================
    // Queries
    // =========================================================================

    /// Get an aura by spell_id and effect_index.
    pub fn get_aura(&self, spell_id: u32, effect_index: u8) -> Option<&Aura> {
        self.auras.get(&(spell_id, effect_index))
    }

    /// Get a mutable aura by spell_id and effect_index.
    pub fn get_aura_mut(&mut self, spell_id: u32, effect_index: u8) -> Option<&mut Aura> {
        self.auras.get_mut(&(spell_id, effect_index))
    }

    /// Check if an aura exists for a given spell_id (any effect index).
    pub fn has_aura(&self, spell_id: u32) -> bool {
        (0..3u8).any(|ei| self.auras.contains_key(&(spell_id, ei)))
    }

    /// Check if an aura exists for a given spell_id and effect_index.
    pub fn has_aura_effect(&self, spell_id: u32, effect_index: u8) -> bool {
        self.auras.contains_key(&(spell_id, effect_index))
    }

    /// Check if any aura of a given aura_type is active.
    pub fn has_aura_type(&self, aura_type: u32) -> bool {
        self.auras.values().any(|a| a.aura_type == aura_type)
    }

    /// Get all auras of a given aura_type.
    pub fn get_auras_by_type(&self, aura_type: u32) -> Vec<&Aura> {
        self.auras
            .values()
            .filter(|a| a.aura_type == aura_type)
            .collect()
    }

    /// Get all active auras (immutable).
    pub fn all_auras(&self) -> impl Iterator<Item = &Aura> {
        self.auras.values()
    }

    /// Get all active auras (mutable).
    pub fn all_auras_mut(&mut self) -> impl Iterator<Item = &mut Aura> {
        self.auras.values_mut()
    }

    /// Get the aura slot for a given key.
    pub fn get_slot(&self, spell_id: u32, effect_index: u8) -> Option<u8> {
        self.slot_map.get(&(spell_id, effect_index)).copied()
    }

    /// Get the aura at a given slot.
    pub fn get_aura_at_slot(&self, slot: u8) -> Option<&Aura> {
        self.reverse_slot_map
            .get(&slot)
            .and_then(|key| self.auras.get(key))
    }

    /// Get total additive modifier for all auras of a given type.
    /// Used for: total stat bonus, total resistance bonus, etc.
    pub fn get_total_aura_modifier(&self, aura_type: u32) -> i32 {
        self.auras
            .values()
            .filter(|a| a.aura_type == aura_type)
            .map(|a| a.current_value())
            .sum()
    }

    /// Get total modifier for auras of a given type with a specific misc_value.
    /// Used for: MOD_STAT where misc_value = stat index, MOD_RESISTANCE where misc_value = school.
    pub fn get_total_aura_modifier_by_misc(&self, aura_type: u32, misc_value: i32) -> i32 {
        self.auras
            .values()
            .filter(|a| a.aura_type == aura_type && a.misc_value == misc_value)
            .map(|a| a.current_value())
            .sum()
    }

    /// Get total number of active auras.
    pub fn count(&self) -> usize {
        self.auras.len()
    }

    /// Get number of positive aura slots in use.
    pub fn positive_count(&self) -> usize {
        self.auras.values().filter(|a| a.is_positive()).count()
    }

    /// Get number of negative aura slots in use.
    pub fn negative_count(&self) -> usize {
        self.auras.values().filter(|a| a.is_negative()).count()
    }

    /// Get number of passive aura slots in use.
    pub fn passive_count(&self) -> usize {
        self.auras.values().filter(|a| a.is_passive()).count()
    }

    /// Update all aura durations and collect expired aura keys.
    /// Returns list of (spell_id, effect_index) for expired auras.
    pub fn tick_durations(&mut self, diff_ms: u32) -> Vec<(u32, u8)> {
        let mut expired = Vec::new();

        for (key, aura) in self.auras.iter_mut() {
            if let Some(ref mut duration) = aura.duration_ms {
                if *duration <= diff_ms {
                    *duration = 0;
                    expired.push(*key);
                } else {
                    *duration -= diff_ms;
                }
            }
        }

        expired
    }

    /// Advance periodic timers and collect ticks that are ready.
    /// Returns list of (spell_id, effect_index) for auras that should tick now.
    pub fn tick_periodic(&mut self, diff_ms: u32) -> Vec<(u32, u8)> {
        let mut ready_to_tick = Vec::new();

        for (key, aura) in self.auras.iter_mut() {
            if aura.periodic_interval_ms == 0 {
                continue; // Not a periodic aura
            }

            aura.periodic_timer_ms += diff_ms;

            while aura.periodic_timer_ms >= aura.periodic_interval_ms {
                aura.periodic_timer_ms -= aura.periodic_interval_ms;
                aura.ticks_applied += 1;
                ready_to_tick.push(*key);
            }
        }

        ready_to_tick
    }
}

impl Default for AuraContainer {
    fn default() -> Self {
        Self::new()
    }
}
