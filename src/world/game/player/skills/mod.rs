//! Weapon & Defense Skills System
//!
//! Manages player weapon and defense skills including:
//! - Skill tracking (current/max values)
//! - Skill-up mechanics on combat
//! - Proficiency management
//! - UPDATE_OBJECT field packing

pub mod constants;
pub mod defaults;
pub mod formulas;
pub mod skill_up;
pub mod state;
pub mod system;
pub mod update_fields;

pub use constants::*;
pub use defaults::*;
pub use formulas::*;
pub use skill_up::*;
pub use state::*;
pub use system::*;
pub use update_fields::*;
