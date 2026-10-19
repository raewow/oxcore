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
pub mod assist;
mod decision;
pub mod event_ai;
pub mod executor;
pub mod script_target;
mod snapshot;
mod system;
pub mod target;
pub mod types;
pub mod units;

// Public exports
pub use aggro::{
    attack_distance, is_npc, NPC_FLAG_GOSSIP, NPC_FLAG_QUEST_GIVER, NPC_FLAG_TRAINER,
    NPC_FLAG_VENDOR,
};
pub use aggro_scan::{aggro_on_open, scan_for_aggro};
pub use snapshot::{AIDecisionResult, AIInput, CreatureSnapshot, TargetSnapshot, ThreatEntry};
pub use system::{forget_lua_state, process_ai_event, queue_event, update_creature_ai};
pub use types::{
    AIAction, AIEvent, AIEventQueue, AIState, AIStateData, AIType, CombatEndReason,
    CreatureAiProfile, MovementType, PendingAssistance, ReactState,
};
