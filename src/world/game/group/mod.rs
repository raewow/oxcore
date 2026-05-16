//! Group System for world
//!
//! This module implements party (5-man) and raid (40-man) group functionality
//! following the world patterns (thin handlers, system-owned state, DashMap caching).
//!
//! ## Architecture
//!
//! - **types.rs**: Constants, enums (LootMethod, MemberStatus, GroupError), GroupData/GroupMember structs
//! - **system.rs**: GroupSystem implementing the System trait
//! - **tests.rs**: Unit tests
//!
//! ## Permission Model
//!
//! - Leader OR Assistant: invite, uninvite, change subgroup, swap subgroups, set target icons, initiate ready check
//! - Leader only: set leader, set assistant, set main tank/assistant, set loot method, convert to raid
//!
//! ## Group Flow
//!
//! 1. Player A invites Player B (CMSG_GROUP_INVITE)
//! 2. Player B receives SMSG_GROUP_INVITE
//! 3. Player B accepts (CMSG_GROUP_ACCEPT) or declines (CMSG_GROUP_DECLINE)
//! 4. On accept, group is created (if needed) and SMSG_GROUP_LIST sent to all members
//! 5. Members can leave, leader can promote/demote, convert to raid, etc.

pub mod cache;
pub mod system;
pub mod types;

#[cfg(test)]
mod tests;

pub use cache::CachedGroup;
pub use system::GroupSystem;
pub use types::{
    GroupData, GroupError, GroupInvite, GroupMember, LootMethod, MemberStatus,
    ERR_ALREADY_IN_GROUP_S, ERR_BAD_PLAYER_NAME_S, ERR_GROUP_FULL, ERR_IGNORING_YOU_S,
    ERR_NOT_LEADER, ERR_PARTY_RESULT_OK, ERR_PLAYER_WRONG_FACTION, ERR_TARGET_NOT_IN_GROUP_S,
    MAX_GROUP_SIZE, MAX_RAID_SIZE, MAX_RAID_SUBGROUPS, PARTY_OP_INVITE, PARTY_OP_LEAVE,
};
