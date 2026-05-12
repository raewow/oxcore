//! Social system types for world

use crate::shared::protocol::ObjectGuid;
use std::collections::HashMap;

// Re-export types from shared that are used by messages and system
pub use crate::shared::game::social::{
    FriendInfo, FriendStatus, FriendsResult, SocialFlag, SOCIALMGR_FRIEND_LIMIT,
    SOCIALMGR_IGNORE_LIMIT,
};

// ========== CACHE TYPES (world specific) ==========

/// Friend entry in cache
#[derive(Debug, Clone)]
pub struct FriendEntry {
    pub friend_guid: ObjectGuid,
    pub flags: u8,
}

impl FriendEntry {
    pub fn new(friend_guid: ObjectGuid, flags: u8) -> Self {
        Self {
            friend_guid,
            flags,
        }
    }
}

/// Ignore entry in cache
#[derive(Debug, Clone)]
pub struct IgnoreEntry {
    pub ignored_guid: ObjectGuid,
    pub flags: u8,
}

impl IgnoreEntry {
    pub fn new(ignored_guid: ObjectGuid, flags: u8) -> Self {
        Self {
            ignored_guid,
            flags,
        }
    }
}

/// Per-player social state
#[derive(Debug, Clone, Default)]
pub struct SocialState {
    pub friends: HashMap<ObjectGuid, FriendEntry>,
    pub ignores: HashMap<ObjectGuid, IgnoreEntry>,
}

impl SocialState {
    pub fn new() -> Self {
        Self {
            friends: HashMap::new(),
            ignores: HashMap::new(),
        }
    }

    pub fn has_friend(&self, friend_guid: ObjectGuid) -> bool {
        self.friends.contains_key(&friend_guid)
    }

    pub fn has_ignore(&self, ignored_guid: ObjectGuid) -> bool {
        self.ignores.contains_key(&ignored_guid)
    }

    pub fn friend_count(&self) -> usize {
        self.friends.len()
    }

    pub fn ignore_count(&self) -> usize {
        self.ignores.len()
    }

    pub fn get_friend_entry(&self, friend_guid: ObjectGuid) -> Option<&FriendEntry> {
        self.friends.get(&friend_guid)
    }

    pub fn get_ignore_entry(&self, ignored_guid: ObjectGuid) -> Option<&IgnoreEntry> {
        self.ignores.get(&ignored_guid)
    }
}

// ========== WHO COMMAND ==========

/// Reason why a whisper was blocked
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhisperBlockReason {
    /// Target has sender on their ignore list
    TargetIgnoresSender,
    /// Cross-faction whispers are disabled and players are different factions
    CrossFactionDisabled,
}

/// Result of whisper validation
pub type WhisperValidationResult = Result<(), WhisperBlockReason>;

/// WHO command search criteria
#[derive(Debug)]
pub struct WhoRequest {
    pub requester_guid: ObjectGuid,
    pub requester_team: u8,
    pub requester_security: u8,
    pub min_level: u32,
    pub max_level: u32,
    pub player_name: String,
    pub guild_name: String,
    pub race_mask: u32,
    pub class_mask: u32,
    pub zone_ids: Vec<u32>,
    pub search_strings: Vec<String>,
}
