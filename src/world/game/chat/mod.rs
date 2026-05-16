//! Chat System - manages channels, messages, and flood protection
//!
//! This system handles all chat operations in world including:
//! - Channel management (join/leave/moderate)
//! - Message sending (say/yell/whisper/channel)
//! - Flood protection with automatic muting
//! - Built-in and custom channel support
//! - Command system for GM and player commands

pub mod commands;
pub mod system;
pub mod types;
pub mod validation;

#[cfg(test)]
mod tests;

pub use crate::shared::game::chat::{ChatMsg, ChatNotify, ChatTag, Language, Team};
pub use system::ChatSystem;
pub use types::*;
pub use validation::*;
