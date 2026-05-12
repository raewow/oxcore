//! Cached group data for packet building
//!
//! This module provides a snapshot view of group data that's compatible with
//! the shared message types. It converts from the internal GroupData to a format
//! suitable for packet serialization.

use crate::shared::protocol::ObjectGuid;
use crate::world::game::group::types::{GroupData, GroupMember, LootMethod};

/// Cached snapshot of group data for packet building
///
/// This type provides a read-only view of group data that's compatible
/// with the shared message packet builders.
#[derive(Debug, Clone)]
pub struct CachedGroup {
    pub id: u32,
    pub leader_guid: ObjectGuid,
    pub leader_name: String,
    pub is_raid: bool,
    pub loot_method: LootMethod,
    pub loot_threshold: u8,
    pub looter_guid: ObjectGuid,
    pub main_tank_guid: ObjectGuid,
    pub main_assistant_guid: ObjectGuid,
    pub target_icons: [ObjectGuid; 8],
    pub members: Vec<GroupMember>,
    pub subgroup_counts: [u8; 8],
}

impl CachedGroup {
    /// Create a cached group from GroupData
    pub fn from_group_data(group: &GroupData) -> Self {
        Self {
            id: group.id,
            leader_guid: group.leader_guid,
            leader_name: group.leader_name.clone(),
            is_raid: group.is_raid,
            loot_method: group.loot_method,
            loot_threshold: group.loot_threshold,
            looter_guid: group.looter_guid,
            main_tank_guid: group.main_tank_guid,
            main_assistant_guid: group.main_assistant_guid,
            target_icons: group.target_icons,
            members: group.members.clone(),
            subgroup_counts: group.subgroup_counts,
        }
    }

    /// Get a member by GUID
    pub fn get_member(&self, guid: ObjectGuid) -> Option<&GroupMember> {
        self.members.iter().find(|m| m.guid == guid)
    }
}
