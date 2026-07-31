//! Message system for type-safe packet construction
//!
//! This module provides the `ToWorldPacket` trait and message structs
//! for building server packets in a type-safe, self-documenting way.
//!
//! # Example
//! ```rust,ignore
//! use oxcore_shared::messages::{ToWorldPacket, SmsgGuildInvite};
//! # async fn example(session: oxcore_world::core::session::world_session::WorldSession) -> anyhow::Result<()> {
//! let msg = SmsgGuildInvite {
//!     inviter_name: "Alice",
//!     guild_name: "MyGuild",
//! };
//! session.send_msg(msg)?;
//! # Ok(())
//! # }
//! ```

use crate::protocol::{ObjectGuid, WorldPacket};

/// Serializes a message into the body its recipient's protocol expects.
///
/// The same logical message has a different byte layout on the vanilla 1.12 wire and the modern
/// 1.14 wire, so a type provides one encoding per protocol. `WorldSession::send_msg()` picks
/// between them from the session's protocol; the socket layer then frames whichever it gets.
///
/// Both return a [`WorldPacket`] — opcode plus body bytes — because that is the currency every
/// outbound path already speaks (session queue, broadcaster, per-player handler). Only the socket
/// knows about framing, and it reads the protocol-appropriate number off the [`Opcode`] the packet
/// carries. Modern bodies are built with a bit-packing writer rather than the byte-oriented
/// `WorldPacket` writers, but they finish as the same type.
///
/// [`Opcode`]: crate::protocol::Opcode
pub trait ToWorldPacket {
    /// The vanilla 1.12 body. Every message must provide this — it is the protocol the server was
    /// built for and the one the 1.12 client still uses.
    fn to_vanilla(&self) -> WorldPacket;

    /// The modern 1.14 body, or `None` if this message has not been ported yet.
    ///
    /// Defaulted so the ~200 existing messages need no change: modern encodings are added one at a
    /// time as a 1.14 client turns out to need them. Sending an unported message to a modern
    /// session logs once and drops the packet rather than erroring — a missing cosmetic packet
    /// should not tear down a connection that is otherwise working.
    fn to_modern(&self) -> Option<WorldPacket> {
        None
    }

    /// The modern body for a specific recipient.
    ///
    /// Some modern bodies depend on *who* is receiving them, in a way vanilla bodies never do. An
    /// object update marks the recipient's own object with `ThisIsYou` and encodes it under a
    /// larger field table, and it carries the recipient's map id in its header — so one broadcast
    /// to ten nearby players is ten different bodies. Baking the recipient into the message before
    /// broadcasting cannot express that; only the send path still knows who each copy is for.
    ///
    /// Defaulted to ignore the recipient, since almost every message does.
    fn to_modern_for(&self, _recipient: Recipient) -> Option<WorldPacket> {
        self.to_modern()
    }
}

/// Who a message is being encoded for.
///
/// Only the modern protocol reads this; see [`ToWorldPacket::to_modern_for`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recipient {
    /// The receiving player's own object.
    pub guid: ObjectGuid,
    /// The map that player is on.
    pub map_id: u16,
    /// The realm serving this player, used to qualify every 128-bit GUID in the body.
    ///
    /// Must match what the character list and name queries used, or the client treats the same
    /// object as two different ones. Carried per-recipient rather than as a constant so a realm
    /// with an id other than 1 is not silently wrong.
    pub realm_id: u16,
}

impl ToWorldPacket for WorldPacket {
    fn to_vanilla(&self) -> WorldPacket {
        self.clone()
    }

    /// Deliberately not implemented. An already-encoded packet carries no record of which
    /// protocol's layout its bytes are in, so re-labelling it as modern would put vanilla bytes on
    /// a modern wire. Callers that need both must send the message type, not a built packet.
    fn to_modern(&self) -> Option<WorldPacket> {
        None
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
pub mod hotfix;
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
pub mod weather;

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
pub use weather::*;
