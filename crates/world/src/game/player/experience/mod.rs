//! Experience System
//!
//! Handles player XP gain, level-up calculations, and packet sending.

pub mod system;
pub mod types;

pub use system::ExperienceSystem;
pub use system::{
    calculate_creature_xp, calculate_xp_for_level, get_gray_level, get_xp_color,
    get_zero_difference, gives_xp,
};
pub use types::ExperienceState;
