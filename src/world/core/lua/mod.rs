//! Lua Scripting System
//!
//! This module provides hot-reloadable Lua scripting support for:
//! - Creature AI (boss fights, mob behavior)
//! - Instance scripts (dungeon/raid logic) [planned]
//! - Zone scripts (world events) [planned]
//! - Gossip scripts (NPC dialogue) [planned]
//!
//! Scripts are auto-discovered from the `/scripts/` directory and self-register
//! via `RegisterCreatureAI()`, `RegisterInstanceScript()`, etc.
//!
//! # Architecture
//!
//! The system follows a deadlock-free pattern:
//! 1. Scripts receive readonly snapshots of game state
//! 2. Scripts return action tables (pure functions)
//! 3. Actions are queued and executed with deterministic lock ordering
//!
//! # Hot Reload
//!
//! Use `.reload lua` GM command to reload all scripts without server restart.
//! Instance state (encounter progress) is preserved across reloads.

pub mod actions;
pub mod api;
pub mod bridge;
pub mod error;
pub mod gossip_executor;
pub mod loader;
pub mod manager;
pub mod scripts;
pub mod snapshot;

pub use actions::{LuaAction, ReactState, SpellTarget, SummonType};
pub use error::{LuaError, LuaResult};
pub use gossip_executor::{build_player_snapshot, execute_gossip_actions};
pub use loader::LoadResult;
pub use manager::LuaScriptManager;
pub use scripts::{
    CreatureScriptState, LuaAreaTriggerScript, LuaCreatureAI, LuaEffectDummyScript,
    LuaGameObjectScript, LuaGossipScript, LuaProcessEventScript,
};
pub use snapshot::{
    AIEvent, LuaCreatureSnapshot, LuaGuid, LuaSnapshot, PlayerSnapshot, ThreatEntry,
};
