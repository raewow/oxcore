//! Power System - Mana, Rage, Energy, Focus, Happiness regeneration
//!
//! This module handles:
//! - Power state storage (current/max values per power type)
//! - Regeneration formulas with 5-second rule for mana
//! - Power consumption for spells and abilities
//! - Rage generation from dealing/taking damage

pub mod regen;
pub mod state;
pub mod system;

pub use regen::*;
pub use state::{PowerState, PowerType};
pub use system::PowerSystem;
