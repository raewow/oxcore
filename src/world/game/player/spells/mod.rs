//! Spell Casting System (Phase 5)
//!
//! Complete spell casting pipeline from button press to effect application,
//! with cooldowns, GCD, cast bar, interrupts, and spell learning.
//!
//! ## Architecture
//!
//! ```text
//! Spell Cast Request
//!     |
//! Validation (pure) -> SpellCastError or continue
//!     |
//! Start Cast -> SMSG_SPELL_START (broadcast)
//!     |
//! Cast Timer (if cast time > 0) -> interruptible
//!     |
//! Execute Effects -> dispatch to effect handlers
//!     +-- damage.rs -> Combat damage pipeline
//!     +-- healing.rs -> Heal + update stats
//!     +-- aura.rs -> AuraSystem.apply_aura()
//!     +-- power.rs -> Energize/drain
//!     +-- misc.rs -> Teleport, summon, etc.
//!     |
//! SMSG_SPELL_GO (broadcast)
//!     |
//! Apply Cooldown + GCD
//! ```

pub mod cooldowns;
pub mod diminishing;
pub mod effects;
pub mod hit;
pub mod learning;
pub mod modifiers;
pub mod state;
pub mod system;
pub mod targets;
pub mod validation;

// Re-exports for convenience
pub use state::{
    ActiveCast, SpellCastError, SpellCastResult, SpellCastTargets, SpellMod, SpellModOp,
    SpellModType, SpellSchool, SpellsState, NUM_SPELL_SCHOOLS,
};
pub use system::SpellSystem;
pub use validation::{spell_cast_error_to_u8, validate_cast};
