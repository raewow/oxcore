//! AuraState - Per-player aura state embedded in Player struct

use crate::shared::protocol::ObjectGuid;
use std::collections::HashMap;

use super::container::AuraContainer;

/// Vanilla WoW aura slot limits
pub const MAX_POSITIVE_AURA_SLOTS: usize = 32;
pub const MAX_NEGATIVE_AURA_SLOTS: usize = 16;
pub const MAX_PASSIVE_AURA_SLOTS: usize = 16;
pub const MAX_TOTAL_AURA_SLOTS: usize =
    MAX_POSITIVE_AURA_SLOTS + MAX_NEGATIVE_AURA_SLOTS + MAX_PASSIVE_AURA_SLOTS;

/// Slot ranges for each aura category
pub const POSITIVE_SLOT_START: u8 = 0;
pub const POSITIVE_SLOT_END: u8 = 31; // inclusive
pub const NEGATIVE_SLOT_START: u8 = 32;
pub const NEGATIVE_SLOT_END: u8 = 47; // inclusive
pub const PASSIVE_SLOT_START: u8 = 48;
pub const PASSIVE_SLOT_END: u8 = 63; // inclusive

/// Per-player aura state, embedded in Player struct
#[derive(Debug, Clone)]
pub struct AuraState {
    /// Active aura container (manages auras and slot allocation)
    pub container: AuraContainer,

    /// Proc trigger tracking: spell_id -> last_proc_time_ms
    /// Used to enforce internal cooldowns on proc effects
    pub proc_cooldowns: HashMap<u32, u64>,

    /// Whether aura state needs to be sent to client
    pub needs_client_update: bool,

    /// Whether stats need recalculation due to aura changes
    pub needs_stat_recalc: bool,

    /// Aura interrupt flags (accumulated from all active auras)
    /// Used to check if an action should remove certain auras
    pub interrupt_flags: u32,
}

impl Default for AuraState {
    fn default() -> Self {
        Self {
            container: AuraContainer::new(),
            proc_cooldowns: HashMap::new(),
            needs_client_update: false,
            needs_stat_recalc: false,
            interrupt_flags: 0,
        }
    }
}
