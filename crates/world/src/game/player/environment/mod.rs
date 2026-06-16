//! Environment System - Rest XP and Environmental Hazards
//!
//! This module implements:
//! - Rest XP accumulation in inns and cities
//! - Mirror timers (breath, fatigue, environmental)
//! - Environmental damage (drowning, lava, slime, fall damage)

pub mod fall;
pub mod mirror_timers;
pub mod rest;
pub mod state;
pub mod system;

// Re-export commonly used types
pub use fall::{calculate_fall_damage, handle_fall_landing, SAFE_FALL_DISTANCE};
pub use mirror_timers::{
    get_max_duration, get_water_breathing_interval, on_mirror_timer_expiration_pulse,
    should_activate_breath, should_activate_environmental, should_activate_fatigue,
    should_deactivate, update_mirror_timers, MirrorTimerAction, MirrorTimerEvent,
    BREATH_MAX_SECONDS, ENVIRONMENTAL_DAMAGE_MAX, ENVIRONMENTAL_DAMAGE_MIN,
    ENVIRONMENTAL_MAX_SECONDS, FATIGUE_MAX_SECONDS,
};
pub use rest::{
    apply_rest_bonus, calculate_offline_rest, on_player_login as on_rest_login, set_rest_type,
    update_rest_bonus, PLAYER_FLAGS_RESTING, REST_RATE_PER_SECOND,
};
pub use state::{
    EnvironmentFlags, EnvironmentState, EnvironmentalDamageType, MirrorTimer, MirrorTimerStatus,
    MirrorTimerType, RestType, MIRROR_TIMER_PULSE_INTERVAL, NUM_CLIENT_TIMERS, NUM_TIMERS,
};
pub use system::{update_environment_flags_internal, EnvironmentSystem, LiquidStatus, LiquidType};
