//! Aura System - Buff/Debuff management for players
//!
//! This module implements the aura (buff/debuff) system following the world patterns:
//! - State lives ON the Player struct (AuraState)
//! - AuraSystem is stateless, operating on player state via PlayerManager
//! - Snapshot pattern for periodic ticks and proc checks to avoid deadlocks
//!
//! ## Architecture
//!
//! ```text
//! Player {
//!     auras: AuraState {
//!         container: AuraContainer,  // HashMap storage + slot allocation
//!         proc_cooldowns: HashMap,   // Internal cooldown tracking
//!         ...
//!     }
//! }
//!
//! AuraSystem::apply_aura() ->
//!     1. Create Aura struct from spell data
//!     2. container.add_aura() handles stacking/refresh
//!     3. Apply stat modifiers if needed
//!     4. Send SMSG_AURA_UPDATE to client
//! ```

pub mod aura;
pub mod container;
pub mod effects;
pub mod interrupt;
pub mod periodic;
pub mod proc;
pub mod stacking;
pub mod state;
pub mod system;

// Re-exports
pub use aura::{Aura, AuraFlags, MAX_SPELL_EFFECTS};
pub use container::AuraContainer;
pub use state::{
    AuraState, MAX_NEGATIVE_AURA_SLOTS, MAX_PASSIVE_AURA_SLOTS, MAX_POSITIVE_AURA_SLOTS,
    MAX_TOTAL_AURA_SLOTS, NEGATIVE_SLOT_END, NEGATIVE_SLOT_START, PASSIVE_SLOT_END,
    PASSIVE_SLOT_START, POSITIVE_SLOT_END, POSITIVE_SLOT_START,
};
pub use system::{AuraSystem, AuraTickSnapshot, ProcCandidate};
