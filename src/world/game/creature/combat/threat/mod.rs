//! Threat system for creatures
//!
//! This module implements a sophisticated threat management system with:
//! - Online/Offline threat separation (GMs, taxi, dead units)
//! - Threat modifiers (healing, abilities, spell attributes, auras)
//! - Assist threat system with hard CC checks
//! - Taunt mechanics with temporary modifiers
//! - Target switching with 110%/130% thresholds

pub mod assist;
pub mod calc;
pub mod container;
pub mod manager;

pub use assist::AssistThreatHelper;
pub use calc::ThreatCalcHelper;
pub use container::ThreatContainer;
pub use manager::{HostileReference, ThreatManager};
