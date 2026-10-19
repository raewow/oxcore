//! Battleground raids.
//!
//! Each side of a battleground match fights as one raid. Joining it sets the
//! player's own group aside rather than leaving it: they stay a member, the
//! rest of that group keeps seeing them listed, and it becomes their group
//! again when they leave the battleground. The raids live in memory only and
//! take their ids from a range the database never sees.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use async_trait::async_trait;
use oxcore_db::database::characters::models::group::*;
use oxcore_db::database::characters::repositories::GroupRepositoryTrait;
use oxcore_shared::game::group::{CachedGroup, GroupData};
use oxcore_shared::protocol::ObjectGuid;

use super::system::GroupSystem;
use crate::game::broadcast_mgr::BroadcastManagerExt;

/// First id handed to a battleground raid.
pub const BATTLEGROUND_RAID_ID_BASE: u32 = 0xF000_0000;

/// Whether a group id names a battleground raid.
pub fn is_battleground_raid(group_id: u32) -> bool {
    group_id >= BATTLEGROUND_RAID_ID_BASE
}

/// The group repository, with every write about a battleground raid dropped.
pub struct PersistentGroupsOnly(pub Arc<dyn GroupRepositoryTrait>);

#[async_trait]
impl GroupRepositoryTrait for PersistentGroupsOnly {
    async fn get_max_group_id(&self) -> anyhow::Result<Option<u32>> {
        self.0.get_max_group_id().await
    }

    async fn find_by_id(&self, group_id: u32) -> anyhow::Result<Option<GroupRow>> {
        self.0.find_by_id(group_id).await
    }

    async fn find_all(&self) -> anyhow::Result<Vec<GroupRow>> {
        self.0.find_all().await
    }

    async fn find_members(&self, group_id: u32) -> anyhow::Result<Vec<GroupMemberRow>> {
        self.0.find_members(group_id).await
    }

    async fn find_group_for_member(&self, member_guid: u32) -> anyhow::Result<Option<u32>> {
        self.0.find_group_for_member(member_guid).await
    }

    async fn find_members_with_character_data(
        &self,
        group_id: u32,
    ) -> anyhow::Result<Vec<GroupMemberWithCharacterDataRow>> {
        self.0.find_members_with_character_data(group_id).await
    }

    async fn save_group(&self, group: &GroupRow) -> anyhow::Result<()> {
        if is_battleground_raid(group.group_id) {
            return Ok(());
        }
        self.0.save_group(group).await
    }

    async fn add_member(&self, group_id: u32, member_guid: u32, subgroup: u16) -> anyhow::Result<()> {
        if is_battleground_raid(group_id) {
            return Ok(());
        }
        self.0.add_member(group_id, member_guid, subgroup).await
    }

    async fn update_member(
        &self,
        group_id: u32,
        member_guid: u32,
        assistant: bool,
        subgroup: u16,
    ) -> anyhow::Result<()> {
        if is_battleground_raid(group_id) {
            return Ok(());
        }
        self.0
            .update_member(group_id, member_guid, assistant, subgroup)
            .await
    }

    async fn remove_member(&self, group_id: u32, member_guid: u32) -> anyhow::Result<()> {
        if is_battleground_raid(group_id) {
            return Ok(());
        }
        self.0.remove_member(group_id, member_guid).await
    }

    async fn delete_group(&self, group_id: u32) -> anyhow::Result<()> {
        if is_battleground_raid(group_id) {
            return Ok(());
        }
        self.0.delete_group(group_id).await
    }
}

impl GroupSystem {
    /// Whether the player's current group is a battleground raid.
    pub fn in_battleground_raid(&self, player: ObjectGuid) -> bool {
        self.get_player_group_id(player)
            .is_some_and(is_battleground_raid)
    }

    /// Put a player into their side's raid (`raid`, or a new one when there
    /// is none yet) and return the raid's id. A player who led their own group
    /// takes the lead of the raid.
    pub fn join_battleground_raid(&self, player: ObjectGuid, raid: Option<u32>) -> Option<u32> {
        let name = self.player_mgr.with_player(player, |p| p.name.clone())?;
        let current = self.get_player_group_id(player);
        if current.is_some_and(|id| Some(id) == raid) {
            return raid;
        }
        // Leaving one battleground raid for another (never expected) drops the old one.
        if current.is_some_and(is_battleground_raid) {
            self.leave_battleground_raid(player);
        }
        let original = self
            .get_player_group_id(player)
            .filter(|&id| !is_battleground_raid(id));
        let led_own_group = original
            .and_then(|id| self.get_group(id))
            .is_some_and(|g| g.is_leader(player));

        let raid_id = match raid.filter(|id| self.groups.contains_key(id)) {
            Some(id) => {
                let mut group = self.groups.get_mut(&id)?;
                if !group.has_member(player) && group.add_member(player, name).is_err() {
                    return None;
                }
                if led_own_group {
                    let _ = group.promote_new_leader(player);
                }
                id
            }
            None => {
                let id = self.next_battleground_raid_id.fetch_add(1, Ordering::Relaxed);
                let mut group = GroupData::new(id, player, name);
                group.convert_to_raid();
                self.groups.insert(id, group);
                id
            }
        };
        if let Some(original) = original {
            self.original_groups.insert(player, original);
        }
        self.player_groups.insert(player, raid_id);
        self.set_member_update(player, super::system::MEMBER_STATS_FULL);
        self.broadcast_group_list(raid_id);
        Some(raid_id)
    }

    /// Take a player out of their battleground raid and give them their own
    /// group back. The raid goes when its last member leaves.
    pub fn leave_battleground_raid(&self, player: ObjectGuid) {
        let Some(raid_id) = self
            .get_player_group_id(player)
            .filter(|&id| is_battleground_raid(id))
        else {
            return;
        };
        let remaining = self.groups.get_mut(&raid_id).map(|mut group| {
            let was_leader = group.is_leader(player);
            group.remove_member(player);
            if was_leader && group.member_count() > 0 {
                let player_mgr = Arc::clone(&self.player_mgr);
                if let Some(next) = group
                    .clone()
                    .select_new_leader(|guid| player_mgr.get_player(guid).is_some(), false)
                    .or_else(|| group.members.first().map(|m| m.guid))
                {
                    let _ = group.promote_new_leader(next);
                }
            }
            group.member_count()
        });
        self.player_groups.remove_if(&player, |_, id| *id == raid_id);
        match remaining {
            Some(0) => {
                self.groups.remove(&raid_id);
            }
            Some(_) => self.broadcast_group_list(raid_id),
            None => {}
        }

        let original = self
            .original_groups
            .remove(&player)
            .map(|(_, id)| id)
            .filter(|id| self.get_group(*id).is_some_and(|g| g.has_member(player)));
        match original {
            Some(original) => {
                self.player_groups.insert(player, original);
                self.set_member_update(player, super::system::MEMBER_STATS_FULL);
                self.send_group_list_to(original, player);
            }
            None => {
                use oxcore_shared::messages::group::SmsgGroupList;
                let empty = CachedGroup::empty();
                self.broadcast_mgr.send_msg_to_player(
                    player,
                    SmsgGroupList {
                        group: &empty,
                        member_guid: player,
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battleground_raid_ids_sit_above_every_stored_group() {
        assert!(!is_battleground_raid(1));
        assert!(!is_battleground_raid(BATTLEGROUND_RAID_ID_BASE - 1));
        assert!(is_battleground_raid(BATTLEGROUND_RAID_ID_BASE));
    }
}
