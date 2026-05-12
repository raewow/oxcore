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

mod types;
mod snapshot;
mod decision;
pub mod executor;
mod system;
mod aggro;
mod aggro_scan;

// Public exports
pub use types::{
    AIState, AIType, AIEvent, AIAction, CombatEndReason,
    MovementType, ReactState, AIStateData, AIEventQueue,
};
pub use snapshot::{
    CreatureSnapshot, TargetSnapshot, ThreatEntry,
    AIInput, AIDecisionResult,
};
pub use system::{
    update_creature_ai, queue_event, process_ai_event,
};
pub use aggro_scan::scan_for_aggro;
