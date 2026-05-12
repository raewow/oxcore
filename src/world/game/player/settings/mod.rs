//! Settings System - Action buttons, macros, account data, and tutorials
//!
//! This module handles:
//! - Action button storage (120 slots with spell/macro/item bindings)
//! - Macro system (up to 18 macros per character)
//! - Account data blobs (8 types: config, bindings, macros, layout, chat)
//! - Tutorial flags (256 bitflags across 8 u32 words)
//! - Database persistence and login synchronization

pub mod account_data;
pub mod state;
pub mod system;

pub use account_data::{compress_account_data, decompress_account_data, AccountDataType};
pub use state::{
    AccountDataEntry, ActionButton, MacroEntry, SettingsState, ACTION_BUTTON_ITEM,
    ACTION_BUTTON_MACRO, ACTION_BUTTON_SPELL, MAX_ACTION_BUTTONS, MAX_MACROS,
    NUM_ACCOUNT_DATA_TYPES, TUTORIAL_FLAG_COUNT,
};
pub use system::SettingsSystem;
