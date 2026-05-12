//! Combat System - Melee and ranged combat for world
//!
//! Implements the full vanilla WoW auto-attack system:
//! - Single-roll hit table (miss → dodge → parry → glancing → block → crit → crush → hit)
//! - Damage calculation with AP scaling and armor reduction
//! - Swing timer management for main-hand, off-hand, and ranged
//! - Threat generation
//!
//! # Architecture
//! - `CombatState` is embedded in `Player` struct
//! - `CombatSystem` is stateless, operates via `PlayerManager`
//! - Pure functions for hit table and damage calculations

// Module exports
pub mod auto_attack;
pub mod creature_attacks;
pub mod damage;
pub mod hit_table;
pub mod melee_range;
pub mod state;
pub mod system;

// Re-exports for convenience
pub use state::{AttackHand, AttackOutcome, CombatState, DamageResult};

pub use hit_table::{
    calculate_dodge_chance, calculate_effective_crit, calculate_hit_table, calculate_miss_chance,
    calculate_parry_chance, CombatSnapshot,
};

pub use damage::{
    apply_armor_reduction, calculate_armor_reduction_pct, calculate_melee_damage,
    calculate_ranged_damage, calculate_spell_resistance,
};

pub use auto_attack::{
    adjust_attack_speed, apply_haste, initialize_swing_timers, reset_swing_timer,
    update_auto_attack, update_auto_shoot, PendingAttack,
};

pub use system::CombatSystem;
