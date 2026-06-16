//! Combat submodule for creatures

pub mod damage;
pub mod state;
pub mod threat;

pub use damage::{
    apply_hit_outcome, calculate_melee_damage, hit_outcome_to_hit_info,
    hit_outcome_to_victim_state, roll_melee_hit_outcome, MeleeHitOutcome,
};
pub use state::{CombatState, ThreatEntry};
pub use threat::{AssistThreatHelper, ThreatCalcHelper, ThreatContainer, ThreatManager};
