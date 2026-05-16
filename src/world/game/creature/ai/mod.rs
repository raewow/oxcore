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

mod aggro;
mod aggro_scan;
mod decision;
pub mod executor;
mod snapshot;
mod system;
mod types;

// Public exports
pub use aggro_scan::scan_for_aggro;
pub use snapshot::{AIDecisionResult, AIInput, CreatureSnapshot, TargetSnapshot, ThreatEntry};
pub use system::{process_ai_event, queue_event, update_creature_ai};
pub use types::{
    AIAction, AIEvent, AIEventQueue, AIState, AIStateData, AIType, CombatEndReason, MovementType,
    ReactState,
};
