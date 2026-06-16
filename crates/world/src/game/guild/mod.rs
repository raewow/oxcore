//! Guild system module
//!
//! This module provides a centralized guild system following three-layer architecture
//! (System -> State). No bank-related code in this migration.
//!
//! ## Architecture
//! - `GuildSystem`: Business logic for all guild operations (excluding bank)
//! - `GuildData`: In-memory storage for guilds (owned by system)
//! - `PlayerGuildState`: Per-player guild membership tracking
//!
//! ## Features
//! - Guild creation from petition (charter system)
//! - Member management (invite/accept/decline/leave/promote/demote/remove)
//! - Rank management (create/update/delete/set public/officer notes)
//! - Emblem management
//! - MOTD and info management
//! - Roster broadcasting to all members
//! - Guild info queries

mod system;
pub mod utils;

#[cfg(test)]
pub mod tests;

pub use oxcore_shared::game::guild::{
    CachedGuild, Guild, GuildData, GuildEmblem, GuildEvent, GuildId, GuildMember, GuildPermissions,
    GuildRank, PlayerGuildState, ERR_GUILD_NAME_EXISTS, ERR_GUILD_NAME_INVALID,
    ERR_GUILD_PERMISSIONS, ERR_GUILD_SUCCESS, GRF_ONLINE, GRIGHT_OFFCHATLISTEN,
    GUILD_NAME_MAX_LENGTH, GUILD_RANKS_MAX_COUNT,
};
pub use system::GuildSystem;
pub use utils::*;
