//! Reputation system module for world
//!
//! This module implements the vanilla WoW reputation system:
//! - 64 faction slots indexed by ReputationListID
//! - Relative standing storage (absolute = base + standing)
//! - Spillover to allied factions
//! - At-war and inactive flag management
//! - Vendor discounts based on reputation rank
//!
//! ## Architecture
//!
//! - [`ReputationState`] - Per-player state embedded in the Player struct
//! - [`FactionStanding`] - Standing for a single faction
//! - [`FactionEntry`] - DBC faction data
//! - [`ReputationSystem`] - Stateless system that operates on player state
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Initialize factions on character creation
//! world.systems.reputation.initialize(player_guid, &faction_map, world)?;
//!
//! // Modify reputation (with spillover)
//! world.systems.reputation.modify_reputation(player_guid, faction_id, delta, world)?;
//!
//! // Toggle at-war status
//! world.systems.reputation.set_at_war(player_guid, rep_list_id, true, world)?;
//! ```

pub mod state;
pub mod system;

pub use state::{
    FactionEntry,
    FactionStanding,
    ReputationSpilloverTemplate,
    ReputationState,
    rank_cap_to_reputation_rank,
};

pub use system::ReputationSystem;