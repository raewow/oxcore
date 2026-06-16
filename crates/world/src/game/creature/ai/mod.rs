//! Creature AI System - Phase 4: AI Foundation
//!
//! This module implements the core AI architecture:
//! - Pure function decision system (deadlock-free)
//! - AI state machine with 7 states
//! - Event queue system with 20+ event types
//! - Action system with 20+ action types
//! - Multiple AI type implementations
//!
//! Architecture:
//! ```text
//! ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
//! │   SNAPSHOT      │     │    DECISION     │     │    EXECUTE      │
//! │   CAPTURE       │────▶│    (pure fn)    │────▶│    ACTIONS      │
//! │   (brief lock)  │     │   (no locks)    │     │  (batch apply)  │
//! └─────────────────┘     └─────────────────┘     └─────────────────┘
//! ```

pub mod aggro;
mod aggro_scan;
mod decision;
pub mod executor;
mod snapshot;
mod system;
mod types;

// Public exports
pub use aggro::{
    is_hostile_faction, is_npc, is_valid_aggro_target, should_aggro_creature, NPC_FLAG_GOSSIP,
    NPC_FLAG_QUEST_GIVER, NPC_FLAG_TRAINER, NPC_FLAG_VENDOR,
};
pub use aggro_scan::scan_for_aggro;
pub use snapshot::{AIDecisionResult, AIInput, CreatureSnapshot, TargetSnapshot, ThreatEntry};
pub use system::{process_ai_event, queue_event, update_creature_ai};
pub use types::{
    AIAction, AIEvent, AIEventQueue, AIState, AIStateData, AIType, CombatEndReason, MovementType,
    ReactState,
};
