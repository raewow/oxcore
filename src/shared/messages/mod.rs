//! Message system for type-safe packet construction
//!
//! This module provides the `ToWorldPacket` trait and message structs
//! for building server packets in a type-safe, self-documenting way.
//!
//! # Example
//! ```rust,no_run
//! use wow_server::shared::messages::{ToWorldPacket, SmsgGuildInvite};
//! # async fn example(session: wow_server::world::core::session::world_session::WorldSession) -> anyhow::Result<()> {
//! let msg = SmsgGuildInvite {
//!     inviter_name: "Alice",
//!     guild_name: "MyGuild",
//! };
//! session.send_msg(msg)?;
//! # Ok(())
//! # }
//! ```

use crate::shared::protocol::WorldPacket;

/// Trait for types that can be serialized to a WorldPacket
///
/// This trait enables clean struct-based packet construction.
/// Types implementing this trait can be sent via `WorldSession::send_msg()`
/// or `BroadcastManager::send_msg_to_player()`.
pub trait ToWorldPacket {
    fn to_world_packet(&self) -> WorldPacket;
}

impl ToWorldPacket for WorldPacket {
    fn to_world_packet(&self) -> WorldPacket {
        self.clone()
    }
}

// Module declarations
pub mod auction;
pub mod auras;
pub mod battleground;
pub mod channel;
pub mod character;
pub mod chat;
pub mod combat;
pub mod create;
pub mod death;
pub mod duel;
pub mod environment;
pub mod errors;
pub mod experience;
pub mod gossip;
pub mod group;
pub mod guild;
pub mod instance;
pub mod inventory;
pub mod inventory_update;
pub mod lfg;
pub mod login;
pub mod loot;
pub mod mail;
pub mod movement;
pub mod petition;
pub mod player;
pub mod query;
pub mod quest;
pub mod reputation;
pub mod settings;
pub mod social;
pub mod spells;
pub mod taxi;
pub mod ticket;
pub mod trade;
pub mod trainer;
pub mod update;
pub mod vendor;

// Re-exports for convenience
pub use auction::*;
pub use auras::*;
pub use battleground::*;
pub use channel::*;
pub use character::*;
pub use chat::*;
pub use combat::*;
pub use create::*;
pub use death::*;
pub use duel::*;
pub use environment::*;
pub use errors::*;
pub use experience::*;
pub use gossip::*;
pub use group::*;
pub use guild::*;
pub use instance::*;
pub use inventory::*;
pub use inventory_update::*;
pub use lfg::*;
pub use login::*;
pub use loot::*;
pub use mail::*;
pub use movement::*;
pub use petition::*;
pub use player::*;
pub use query::*;
pub use quest::*;
pub use reputation::*;
pub use settings::*;
pub use social::*;
pub use spells::*;
pub use taxi::*;
pub use ticket::*;
pub use trade::*;
pub use trainer::*;
pub use update::*;
pub use vendor::*;
