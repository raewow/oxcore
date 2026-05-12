//! Aura struct - represents a single active buff/debuff on a unit

use crate::shared::protocol::ObjectGuid;

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

    /// Which effect index of the spell created this aura (0, 1, or 2)
    pub effect_index: u8,

    // === Slot ===
    /// Assigned aura slot (0-63). None if not yet assigned.
    /// Slots 0-31: positive, 32-47: negative, 48-63: passive
    pub slot: Option<u8>,

    // === Timing ===
    /// Remaining duration in milliseconds. None = permanent (passive/talent).
    pub duration_ms: Option<u32>,

    /// Maximum duration in milliseconds (for refresh capping)
    pub max_duration_ms: Option<u32>,

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

    /// Aura flags
    pub flags: AuraFlags,
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
            effect_index,
            slot: None,
            duration_ms,
            max_duration_ms: duration_ms,
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
            flags,
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
