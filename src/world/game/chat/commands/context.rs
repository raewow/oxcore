//! Chat command context and types for world
//!
//! Defines the context and metadata types for in-game chat commands.

use crate::shared::common::AccountType;
use crate::shared::protocol::ObjectGuid;
use crate::world::core::session::WorldSession;
use crate::world::World;

/// Context passed to chat command handlers
pub struct ChatCommandContext<'a> {
    /// Reference to the player's session
    pub session: &'a WorldSession,
    /// GUID of the player executing the command
    pub player_guid: ObjectGuid,
    /// Current target (if any) - can be enhanced later with targeting system
    pub target: Option<ObjectGuid>,
    /// Reference to World for accessing game systems and managers
    pub world: &'a World,
    /// Security level from session
    pub security: AccountType,
}

/// Metadata about a chat command
pub struct ChatCommandInfo {
    /// Command name (without the dot prefix)
    pub name: &'static str,
    /// Help text describing the command and usage
    pub help: &'static str,
    /// Minimum security level required
    pub min_security: AccountType,
}
