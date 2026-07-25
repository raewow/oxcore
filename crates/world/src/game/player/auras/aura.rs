//! Aura struct - represents a single active buff/debuff on a unit

use crate::game::player::spells::diminishing::DRGroup;
use oxcore_shared::protocol::ObjectGuid;

/// Number of effects per spell in vanilla WoW
pub const MAX_SPELL_EFFECTS: usize = 3;

/// Aura - represents one active buff/debuff effect on a unit.
/// A single spell with 3 apply-aura effects creates 3 Aura instances.
#[derive(Debug, Clone)]
pub struct Aura {
    // === Identity ===
    /// Spell ID that created this aura (from Spell.dbc)
    pub spell_id: u32,

    /// GUID of the unit that cast this aura
    pub caster_guid: ObjectGuid,

    /// GUID of the item that cast this aura (if from Use effect)
    pub cast_item_guid: Option<ObjectGuid>,

    /// Equipped weapon slot that originated this aura (15 main-hand, 16 off-hand).
    pub weapon_buff_slot: Option<u8>,

    /// Which effect index of the spell created this aura (0, 1, or 2)
    pub effect_index: u8,

    // === Slot ===
    /// Assigned aura slot (0-63). None if not yet assigned.
    /// Slots 0-31: positive, 32-47: negative, 48-63: passive
    pub slot: Option<u8>,

    /// Whether this aura is subject to the visible buff/debuff slot limit.
    pub affected_by_visible_slot_limit: bool,

    /// Priority used when visible buff slots are exhausted.
    pub visible_slot_limit_score: i32,

    // === Timing ===
    /// Remaining duration in milliseconds. None = permanent (passive/talent).
    pub duration_ms: Option<u32>,

    /// Maximum duration in milliseconds (for refresh capping)
    pub max_duration_ms: Option<u32>,

    /// Spell.dbc duration record used to distinguish passive zero-duration spells.
    pub duration_index: u32,

    /// Accumulated time since last periodic tick (for DoT/HoT)
    pub periodic_timer_ms: u32,

    /// Interval between periodic ticks in milliseconds (e.g., 3000 for most DoTs)
    /// 0 means this aura has no periodic component.
    pub periodic_interval_ms: u32,

    /// Number of periodic ticks already applied
    pub ticks_applied: u32,

    /// Total number of periodic ticks expected over the full duration
    pub total_ticks: u32,

    // === Stacking ===
    /// Current stack count (1 = no stacking)
    pub stack_count: u8,

    /// Maximum stack count from spell data
    pub max_stack_count: u8,

    /// Current charge count (0 = unlimited charges)
    pub charges: u8,

    /// Maximum charges (0 = unlimited)
    pub max_charges: u8,

    // === Effect Values ===
    /// Base values for each effect (from spell data, scaled by level/SP at apply time)
    /// Index maps to effect_index. Only [self.effect_index] is relevant for this Aura,
    /// but we store all 3 for convenience when multiple auras from the same spell interact.
    pub base_values: [i32; MAX_SPELL_EFFECTS],

    /// Current effective values (base * stack_count, modified by recalculation)
    pub current_values: [i32; MAX_SPELL_EFFECTS],

    // === Classification ===
    /// Aura type (from spell DBC EffectApplyAuraName)
    pub aura_type: u32,

    /// Misc value from spell effect (e.g., stat index for MOD_STAT, school mask for resist)
    pub misc_value: i32,
    /// Diminishing-return group registered for this aura's lifetime.
    pub diminishing_group: Option<DRGroup>,

    /// Aura flags
    pub flags: AuraFlags,

    /// Whether this effect changed target state when it was applied.
    ///
    /// `AURA_MOD_RANGED_AMMO_HASTE` remains stored when no ammo-requiring ranged weapon is
    /// equipped, but its removal must not adjust attack time.
    pub is_applied: bool,
}

/// Aura classification flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuraFlags {
    /// Positive buff (shows in buff bar, green border)
    pub is_positive: bool,

    /// Negative debuff (shows in debuff bar, red border)
    pub is_negative: bool,

    /// Passive aura (from talents, racials - no icon, no duration)
    pub is_passive: bool,

    /// Whether the player can right-click to cancel this aura
    pub can_be_cancelled: bool,

    /// Whether this aura is hidden from the UI entirely
    pub is_hidden: bool,

    /// Whether this aura is permanent (no duration expiry)
    pub is_permanent: bool,
}

impl Default for AuraFlags {
    fn default() -> Self {
        Self {
            is_positive: false,
            is_negative: false,
            is_passive: false,
            can_be_cancelled: true,
            is_hidden: false,
            is_permanent: false,
        }
    }
}

impl Aura {
    /// Create a new aura from spell data.
    /// `base_value` is the pre-computed effect value (already scaled by spell power, level, etc.)
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        spell_id: u32,
        caster_guid: ObjectGuid,
        effect_index: u8,
        aura_type: u32,
        misc_value: i32,
        base_value: i32,
        duration_ms: Option<u32>,
        periodic_interval_ms: u32,
        max_stack_count: u8,
        max_charges: u8,
        flags: AuraFlags,
    ) -> Self {
        let total_ticks = if periodic_interval_ms > 0 {
            duration_ms.map(|d| d / periodic_interval_ms).unwrap_or(0)
        } else {
            0
        };

        let mut base_values = [0i32; MAX_SPELL_EFFECTS];
        if (effect_index as usize) < MAX_SPELL_EFFECTS {
            base_values[effect_index as usize] = base_value;
        }

        let mut current_values = [0i32; MAX_SPELL_EFFECTS];
        if (effect_index as usize) < MAX_SPELL_EFFECTS {
            current_values[effect_index as usize] = base_value;
        }

        Self {
            spell_id,
            caster_guid,
            cast_item_guid: None,
            weapon_buff_slot: None,
            effect_index,
            slot: None,
            affected_by_visible_slot_limit: false,
            visible_slot_limit_score: 0,
            duration_ms,
            max_duration_ms: duration_ms,
            duration_index: 0,
            periodic_timer_ms: 0,
            periodic_interval_ms,
            ticks_applied: 0,
            total_ticks,
            stack_count: 1,
            max_stack_count: max_stack_count.max(1),
            charges: max_charges,
            max_charges,
            base_values,
            current_values,
            aura_type,
            misc_value,
            diminishing_group: None,
            flags,
            is_applied: true,
        }
    }

    /// Check if this aura has expired (duration ran out)
    pub fn is_expired(&self) -> bool {
        match self.duration_ms {
            Some(0) => true,
            Some(_) => false,
            None => false, // Permanent auras never expire
        }
    }

    /// Check if this aura has periodic effects
    pub fn is_periodic(&self) -> bool {
        self.periodic_interval_ms > 0
    }

    /// Check if this aura is positive (buff)
    pub fn is_positive(&self) -> bool {
        self.flags.is_positive
    }

    /// Check if this aura is negative (debuff)
    pub fn is_negative(&self) -> bool {
        self.flags.is_negative
    }

    /// Check if this aura is passive (talent/racial)
    pub fn is_passive(&self) -> bool {
        self.flags.is_passive
    }

    /// Mark this aura as subject to the visible buff/debuff slot limit.
    pub fn set_affected_by_visible_slot_limit(&mut self) {
        self.affected_by_visible_slot_limit = true;
    }

    /// Mark this aura for visible-slot eviction and calculate its priority.
    pub fn calculate_for_buff_limit(&mut self, target_guid: ObjectGuid) {
        self.set_affected_by_visible_slot_limit();
        self.visible_slot_limit_score = if self.duration_ms.is_none() {
            3
        } else if self.caster_guid != target_guid {
            2
        } else if self.cast_item_guid.is_some() {
            1
        } else {
            0
        };
    }

    /// Get the current effect value for this aura's effect index
    pub fn current_value(&self) -> i32 {
        self.current_values[self.effect_index as usize]
    }

    /// Get the base effect value for this aura's effect index
    pub fn base_value(&self) -> i32 {
        self.base_values[self.effect_index as usize]
    }

    /// Refresh duration back to max (same caster reapplies)
    pub fn refresh_duration(&mut self) {
        self.duration_ms = self.max_duration_ms;
        self.ticks_applied = 0;
        self.periodic_timer_ms = 0;
    }

    /// Mirror `SpellAuraHolder::SetAuraMaxDuration` for this effect of a holder.
    pub fn set_max_duration(&mut self, duration_ms: i32) {
        self.max_duration_ms = (duration_ms >= 0).then_some(duration_ms as u32);

        // A passive spell with no SpellDuration record remains permanent even if a caller
        // supplies a positive cap. This is the C++ DurationIndex == 0 exception.
        if duration_ms > 0 && !(self.is_passive() && self.duration_index == 0) {
            self.flags.is_permanent = false;
        }
    }

    /// Align the elapsed periodic timer with a holder duration change.
    pub fn refresh_periodic_timer(&mut self, duration_ms: i32) {
        if !self.is_periodic() {
            return;
        }

        let duration_ms = if duration_ms > 0 {
            duration_ms
        } else {
            self.duration_ms.unwrap_or(0) as i32
        };
        let until_next =
            super::periodic::update_periodic_timer(duration_ms, self.periodic_interval_ms as i32)
                as u32;

        // C++ stores time until the next tick; Rust stores elapsed time since the last one.
        // A full interval remaining maps to zero elapsed, not to a complete interval elapsed.
        self.periodic_timer_ms = self.periodic_interval_ms.saturating_sub(until_next);
    }

    /// Refresh this aura using a freshly-cast application of the same spell/effect.
    ///
    /// Mirrors `SpellAuraHolder::Refresh` / `Aura::Refresh` (SpellAuras.cpp:311-378): duration
    /// and max duration are taken from the *new* cast (`reapplied`), not the old aura's stored
    /// max — a spell recast with a different computed duration (haste change, different rank,
    /// etc.) must adopt the new duration, not just reset the timer to the previous cap. The
    /// periodic tick counter and timer restart from zero (`m_periodicTick = 0` +
    /// `CalculatePeriodic`), and the effect value/misc value are taken over from the new
    /// application (`m_modifier.m_amount`/`m_miscvalue` copied from `pHolderAura`).
    pub fn refresh_with(&mut self, reapplied: &Aura) {
        self.duration_ms = reapplied.duration_ms;
        self.max_duration_ms = reapplied.max_duration_ms;
        self.ticks_applied = 0;
        self.periodic_timer_ms = 0;
        self.misc_value = reapplied.misc_value;
        let idx = self.effect_index as usize;
        if idx < MAX_SPELL_EFFECTS {
            self.base_values[idx] = reapplied.base_value();
            self.current_values[idx] = reapplied.base_value() * self.stack_count as i32;
        }
    }

    /// Whether this aura can be refreshed-in-place by a new application of the same spell
    /// from the same caster, rather than being stacked or replaced.
    ///
    /// Mirrors `SpellAuraHolder::CanBeRefreshedBy` (SpellAuras.cpp:380-394): requires the same
    /// caster and same spell id (checked by the caller via the `(spell_id, effect_index)` key
    /// and `caster_guid` equality — see `AuraContainer::add_aura`), and requires the spell to
    /// have neither a stack amount nor proc charges (both preclude simple refresh: a stackable
    /// spell goes through `ModStackAmount`/`add_stack` instead, and a charge-based spell must
    /// not have its charge count silently reset by a refresh).
    pub fn can_be_refreshed_by(&self, other_caster_guid: ObjectGuid) -> bool {
        if self.caster_guid != other_caster_guid {
            return false;
        }
        if self.max_stack_count > 1 {
            return false;
        }
        if self.max_charges != 0 {
            return false;
        }
        true
    }

    /// Increment stack count. Returns true if stack was added.
    pub fn add_stack(&mut self) -> bool {
        if self.stack_count < self.max_stack_count {
            self.stack_count += 1;
            // Recalculate current value based on new stack count
            let idx = self.effect_index as usize;
            self.current_values[idx] = self.base_values[idx] * self.stack_count as i32;
            true
        } else {
            false
        }
    }

    /// Consume one charge. Returns true if charges remain, false if depleted.
    pub fn consume_charge(&mut self) -> bool {
        if self.max_charges == 0 {
            return true; // Unlimited charges
        }
        if self.charges > 0 {
            self.charges -= 1;
        }
        self.charges > 0
    }

    /// Get remaining duration in milliseconds (None = permanent)
    pub fn remaining_duration_ms(&self) -> Option<u32> {
        self.duration_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guid(low: u32) -> ObjectGuid {
        ObjectGuid::new_player(low)
    }

    fn make_aura(caster: ObjectGuid, base_value: i32, duration_ms: Option<u32>) -> Aura {
        Aura::new(
            1000,
            caster,
            0,
            /* aura_type */ 13,
            /* misc_value */ 0,
            base_value,
            duration_ms,
            /* periodic_interval_ms */ 0,
            /* max_stack_count */ 1,
            /* max_charges */ 0,
            AuraFlags::default(),
        )
    }

    // --- refresh_with: SpellAuraHolder::Refresh / Aura::Refresh (SpellAuras.cpp:311-378) ---

    #[test]
    fn refresh_with_adopts_new_duration_not_old_max() {
        // C++: m_duration = m_maxDuration = pRefreshWithHolder->GetAuraDuration()/GetAuraMaxDuration()
        // i.e. the *new* cast's duration wins, not the existing aura's stored max.
        let mut existing = make_aura(guid(1), 10, Some(5_000));
        existing.duration_ms = Some(1_234); // ticked down since it was applied
        let reapplied = make_aura(guid(1), 10, Some(8_000)); // e.g. recast with more haste/rank

        existing.refresh_with(&reapplied);

        assert_eq!(existing.duration_ms, Some(8_000));
        assert_eq!(existing.max_duration_ms, Some(8_000));
    }

    #[test]
    fn refresh_with_resets_periodic_tick_state() {
        let mut existing = make_aura(guid(1), 10, Some(5_000));
        existing.ticks_applied = 3;
        existing.periodic_timer_ms = 900;
        let reapplied = make_aura(guid(1), 10, Some(5_000));

        existing.refresh_with(&reapplied);

        assert_eq!(existing.ticks_applied, 0);
        assert_eq!(existing.periodic_timer_ms, 0);
    }

    #[test]
    fn refresh_with_takes_new_effect_value_scaled_by_existing_stacks() {
        let mut existing = make_aura(guid(1), 10, Some(5_000));
        existing.stack_count = 3;
        existing.current_values[0] = 30;
        let reapplied = make_aura(guid(1), 20, Some(5_000));

        existing.refresh_with(&reapplied);

        assert_eq!(existing.base_value(), 20);
        assert_eq!(existing.current_value(), 60); // 20 * 3 stacks
    }

    // --- can_be_refreshed_by: SpellAuraHolder::CanBeRefreshedBy (SpellAuras.cpp:380-394) ---

    #[test]
    fn can_be_refreshed_by_same_caster_no_stacks_no_charges() {
        let aura = make_aura(guid(1), 10, Some(5_000));
        assert!(aura.can_be_refreshed_by(guid(1)));
    }

    #[test]
    fn can_be_refreshed_by_false_for_different_caster() {
        let aura = make_aura(guid(1), 10, Some(5_000));
        assert!(!aura.can_be_refreshed_by(guid(2)));
    }

    #[test]
    fn can_be_refreshed_by_false_when_stackable() {
        // C++: `if (m_spellProto->StackAmount) return false;` — stackable auras go through
        // ModStackAmount instead of a plain refresh.
        let mut aura = make_aura(guid(1), 10, Some(5_000));
        aura.max_stack_count = 5;
        assert!(!aura.can_be_refreshed_by(guid(1)));
    }

    #[test]
    fn can_be_refreshed_by_false_when_has_charges() {
        // C++: `if (m_spellProto->procCharges) return false;` — charge-based auras must not
        // have their charge count reset by a plain refresh.
        let mut aura = make_aura(guid(1), 10, Some(5_000));
        aura.max_charges = 3;
        aura.charges = 3;
        assert!(!aura.can_be_refreshed_by(guid(1)));
    }

    #[test]
    fn set_affected_by_visible_slot_limit_marks_aura() {
        let mut aura = make_aura(guid(1), 10, Some(5_000));
        assert!(!aura.affected_by_visible_slot_limit);

        aura.set_affected_by_visible_slot_limit();

        assert!(aura.affected_by_visible_slot_limit);
    }

    #[test]
    fn calculate_for_buff_limit_assigns_priority_by_aura_source() {
        let target = guid(1);

        let mut permanent = make_aura(target, 10, None);
        permanent.calculate_for_buff_limit(target);
        assert_eq!(permanent.visible_slot_limit_score, 3);

        let mut external = make_aura(guid(2), 10, Some(5_000));
        external.calculate_for_buff_limit(target);
        assert_eq!(external.visible_slot_limit_score, 2);

        let mut item = make_aura(target, 10, Some(5_000));
        item.cast_item_guid = Some(ObjectGuid::new_item(3));
        item.calculate_for_buff_limit(target);
        assert_eq!(item.visible_slot_limit_score, 1);

        let mut self_cast = make_aura(target, 10, Some(5_000));
        self_cast.calculate_for_buff_limit(target);
        assert_eq!(self_cast.visible_slot_limit_score, 0);
        assert!(self_cast.affected_by_visible_slot_limit);
    }

    // --- refresh_duration (existing helper, same-caster non-stacked refresh path) ---

    #[test]
    fn refresh_duration_resets_to_stored_max_and_clears_ticks() {
        let mut aura = make_aura(guid(1), 10, Some(5_000));
        aura.duration_ms = Some(100);
        aura.ticks_applied = 2;
        aura.periodic_timer_ms = 500;

        aura.refresh_duration();

        assert_eq!(aura.duration_ms, Some(5_000));
        assert_eq!(aura.ticks_applied, 0);
        assert_eq!(aura.periodic_timer_ms, 0);
    }

    #[test]
    fn set_max_duration_keeps_passive_zero_duration_spell_permanent() {
        let mut aura = make_aura(guid(1), 10, None);
        aura.flags.is_passive = true;
        aura.flags.is_permanent = true;
        aura.duration_index = 0;

        aura.set_max_duration(5_000);

        assert_eq!(aura.max_duration_ms, Some(5_000));
        assert!(aura.flags.is_permanent);
    }

    #[test]
    fn set_max_duration_makes_other_positive_duration_auras_non_permanent() {
        let mut aura = make_aura(guid(1), 10, None);
        aura.flags.is_passive = true;
        aura.flags.is_permanent = true;
        aura.duration_index = 1;

        aura.set_max_duration(5_000);

        assert!(!aura.flags.is_permanent);
    }

    #[test]
    fn refresh_periodic_timer_translates_until_next_to_elapsed_time() {
        let mut aura = Aura::new(
            1000,
            guid(1),
            0,
            3,
            0,
            10,
            Some(7_000),
            3_000,
            1,
            0,
            AuraFlags::default(),
        );

        aura.refresh_periodic_timer(7_000);

        // C++ computes 1000 ms until the next tick; elapsed storage is 2000 ms.
        assert_eq!(aura.periodic_timer_ms, 2_000);
    }

    // --- add_stack / consume_charge / is_expired sanity (existing behavior) ---

    #[test]
    fn add_stack_caps_at_max_and_scales_current_value() {
        let mut aura = make_aura(guid(1), 10, Some(5_000));
        aura.max_stack_count = 2;

        assert!(aura.add_stack());
        assert_eq!(aura.stack_count, 2);
        assert_eq!(aura.current_value(), 20);

        assert!(!aura.add_stack()); // already at max
        assert_eq!(aura.stack_count, 2);
    }

    #[test]
    fn consume_charge_unlimited_when_max_charges_zero() {
        let mut aura = make_aura(guid(1), 10, Some(5_000));
        assert_eq!(aura.max_charges, 0);
        assert!(aura.consume_charge());
        assert!(aura.consume_charge());
    }

    #[test]
    fn consume_charge_depletes_and_reports_false_at_zero() {
        let mut aura = make_aura(guid(1), 10, Some(5_000));
        aura.max_charges = 2;
        aura.charges = 2;

        assert!(aura.consume_charge()); // 2 -> 1, charges remain
        assert!(!aura.consume_charge()); // 1 -> 0, depleted
    }

    #[test]
    fn is_expired_only_when_duration_hits_zero() {
        let mut aura = make_aura(guid(1), 10, Some(5_000));
        assert!(!aura.is_expired());
        aura.duration_ms = Some(0);
        assert!(aura.is_expired());
        aura.duration_ms = None;
        assert!(!aura.is_expired()); // permanent never expires
    }
}
