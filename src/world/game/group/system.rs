//! Group System - party/raid management for world

use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use rand::Rng;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::shared::database::characters::models::group::{GroupMemberRow, GroupRow};
use crate::shared::database::characters::repositories::GroupRepositoryTrait;
use crate::shared::protocol::ObjectGuid;
use crate::world::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::world::game::player::PlayerManager;

use super::types::*;

/// Group System - manages player parties and raids
pub struct GroupSystem {
    /// Group data indexed by group ID
    groups: DashMap<u32, GroupData>,

    /// Player group membership (player_guid -> group_id)
    player_groups: DashMap<ObjectGuid, u32>,

    /// Pending invites (invitee_guid -> GroupInvite)
    pending_invites: DashMap<ObjectGuid, GroupInvite>,

    /// Next group ID for auto-assignment
    next_group_id: AtomicU32,

    /// Repository for database access
    repository: Arc<dyn GroupRepositoryTrait>,

    /// Broadcast manager for sending packets
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,

    /// Player manager for player lookups
    player_mgr: Arc<PlayerManager>,

    /// Whether cross-faction grouping is allowed
    allow_cross_faction_group: bool,
}

impl GroupSystem {
    /// Create a new group system
    pub fn new(
        repository: Arc<dyn GroupRepositoryTrait>,
        broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
        player_mgr: Arc<PlayerManager>,
        allow_cross_faction_group: bool,
    ) -> Self {
        Self {
            groups: DashMap::new(),
            player_groups: DashMap::new(),
            pending_invites: DashMap::new(),
            next_group_id: AtomicU32::new(1),
            repository,
            broadcast_mgr,
            player_mgr,
            allow_cross_faction_group,
        }
    }

    /// Initialize system (load next group ID from database)
    pub async fn initialize(&self) -> Result<()> {
        let max_id = self.repository.get_max_group_id().await?;
        let next_id = max_id.unwrap_or(0) + 1;
        self.next_group_id.store(next_id, Ordering::Relaxed);


        Ok(())
    }

    /// Load all groups from database on server startup
    pub async fn load_all_groups(&self) -> Result<()> {
        let group_rows = self.repository.find_all().await?;

        for group_row in group_rows {
            // Load members for this group
            let member_rows = self
                .repository
                .find_members_with_character_data(group_row.group_id)
                .await?;

            // Filter out ghost members (those with no valid character data) and duplicates
            let mut invalid_guids = Vec::new();
            let mut seen_guids = std::collections::HashSet::new();
            let valid_member_rows: Vec<_> = member_rows
                .into_iter()
                .filter(|m| {
                    // Check for ghost member (no character data)
                    if m.name.is_none() || m.name.as_ref().map(|n| n.is_empty()).unwrap_or(true) {
            tracing::warn!(
                "Removing ghost member {} from group {} (no character data)",
                m.member_guid,
                group_row.group_id
            );
                        invalid_guids.push(m.member_guid);
                        return false;
                    }
                    // Check for duplicate member
                    if !seen_guids.insert(m.member_guid) {
                    tracing::warn!(
                        "Removing duplicate member {} from group {}",
                        m.member_guid,
                        group_row.group_id
                    );
                        invalid_guids.push(m.member_guid);
                        return false;
                    }
                    true
                })
                .collect();

            // Clean up invalid members from database
            for invalid_guid in invalid_guids {
                if let Err(e) = self
                    .repository
                    .remove_member(group_row.group_id, invalid_guid)
                    .await
                {
                tracing::error!(
                    "Failed to remove invalid member {} from group {}: {}",
                    invalid_guid,
                    group_row.group_id,
                    e
                );
                }
            }

            // Skip groups with less than 2 valid members (invalid/stale)
            if valid_member_rows.len() < 2 {
                tracing::info!(
                    "Disbanding group {} with only {} valid members",
                    group_row.group_id,
                    valid_member_rows.len()
                );
                // Clean up the group from database
                if let Err(e) = self.repository.delete_group(group_row.group_id).await {
                    tracing::error!(
                        "Failed to delete invalid group {}: {}",
                        group_row.group_id,
                        e
                    );
                }
                continue;
            }

            // Check if leader is valid (exists in valid members)
            let leader_guid = ObjectGuid::new_player(group_row.leader_guid);
            let leader_valid = valid_member_rows
                .iter()
                .any(|m| m.member_guid == group_row.leader_guid);

            if !leader_valid {
                tracing::warn!(
                    "Group {} has invalid leader {}, promoting first member",
                    group_row.group_id,
                    group_row.leader_guid
                );
            }

            // Get leader name (or first member if leader is invalid)
            let (actual_leader_guid, leader_name) = if leader_valid {
                let name = valid_member_rows
                    .iter()
                    .find(|m| m.member_guid == group_row.leader_guid)
                    .and_then(|m| m.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                (leader_guid, name)
            } else {
                // Promote first valid member to leader
                let first = &valid_member_rows[0];
                let new_leader_guid = ObjectGuid::new_player(first.member_guid);
                let name = first.name.clone().unwrap_or_else(|| "Unknown".to_string());
                (new_leader_guid, name)
            };

            // Build members list from valid members only
            let mut members = Vec::new();
            for member_row in &valid_member_rows {
                let member = GroupMember {
                    guid: ObjectGuid::new_player(member_row.member_guid),
                    name: member_row.name.clone().unwrap_or_default(),
                    subgroup: member_row.subgroup as u8,
                    assistant: member_row.assistant != 0,
                    status: MemberStatus::offline(),
                    last_online: 0,
                };
                members.push(member);
            }

            // Build target icons
            let target_icons = [
                ObjectGuid::from_raw(group_row.icon1 as u64),
                ObjectGuid::from_raw(group_row.icon2 as u64),
                ObjectGuid::from_raw(group_row.icon3 as u64),
                ObjectGuid::from_raw(group_row.icon4 as u64),
                ObjectGuid::from_raw(group_row.icon5 as u64),
                ObjectGuid::from_raw(group_row.icon6 as u64),
                ObjectGuid::from_raw(group_row.icon7 as u64),
                ObjectGuid::from_raw(group_row.icon8 as u64),
            ];

            // Calculate subgroup counts
            let mut subgroup_counts = [0u8; MAX_RAID_SUBGROUPS as usize];
            for member in &members {
                if (member.subgroup as usize) < subgroup_counts.len() {
                    subgroup_counts[member.subgroup as usize] += 1;
                }
            }

            let group = GroupData {
                id: group_row.group_id,
                leader_guid: actual_leader_guid,
                leader_name,
                members,
                is_raid: group_row.is_raid != 0,
                loot_method: LootMethod::from(group_row.loot_method),
                loot_threshold: group_row.loot_threshold,
                looter_guid: ObjectGuid::new_player(group_row.looter_guid),
                main_tank_guid: ObjectGuid::new_player(group_row.main_tank_guid),
                main_assistant_guid: ObjectGuid::new_player(group_row.main_assistant_guid),
                target_icons,
                subgroup_counts,
            };

            // Save group if leader was changed
            if actual_leader_guid != leader_guid {
                let group_row_updated = self.group_to_row(&group);
                if let Err(e) = self.repository.save_group(&group_row_updated).await {
                    tracing::error!(
                        "Failed to update group {} with new leader: {}",
                        group_row.group_id,
                        e
                    );
                }
            }

            // Log loaded group details
            tracing::debug!(
                "Loaded group {} with {} members",
                group.id,
                group.members.len()
            );

            // Cache group
            self.groups.insert(group_row.group_id, group);

            // Cache player->group mappings for valid members only
            for member_row in &valid_member_rows {
                let member_guid = ObjectGuid::new_player(member_row.member_guid);
                self.player_groups.insert(member_guid, group_row.group_id);
            }
        }

        tracing::info!("Loaded {} groups", self.groups.len());
        Ok(())
    }

    // ========== QUERY METHODS ==========

    /// Get group by ID
    pub fn get_group(&self, group_id: u32) -> Option<GroupData> {
        self.groups.get(&group_id).map(|r| r.clone())
    }

    /// Get player's group
    pub fn get_player_group(&self, player_guid: ObjectGuid) -> Option<GroupData> {
        self.player_groups
            .get(&player_guid)
            .and_then(|group_id| self.groups.get(&*group_id).map(|g| g.clone()))
    }

    /// Get player's group ID
    pub fn get_player_group_id(&self, player_guid: ObjectGuid) -> Option<u32> {
        self.player_groups.get(&player_guid).map(|r| *r)
    }

    /// Check if player is in a group
    pub fn is_in_group(&self, player_guid: ObjectGuid) -> bool {
        self.player_groups.contains_key(&player_guid)
    }

    /// Check if player has a pending invite
    pub fn has_pending_invite(&self, player_guid: ObjectGuid) -> bool {
        self.pending_invites.contains_key(&player_guid)
    }

    /// Get pending invite for a player
    pub fn get_pending_invite(&self, player_guid: ObjectGuid) -> Option<GroupInvite> {
        self.pending_invites.get(&player_guid).map(|r| r.clone())
    }

    /// Check if player is the leader of their group
    pub fn is_leader(&self, player_guid: ObjectGuid) -> bool {
        self.get_player_group(player_guid)
            .map(|g| g.is_leader(player_guid))
            .unwrap_or(false)
    }

    /// Check if player is an assistant in their group
    pub fn is_assistant(&self, player_guid: ObjectGuid) -> bool {
        self.get_player_group(player_guid)
            .map(|g| g.is_assistant(player_guid))
            .unwrap_or(false)
    }

    /// Check if player is leader or assistant
    pub fn is_leader_or_assistant(&self, player_guid: ObjectGuid) -> bool {
        self.get_player_group(player_guid)
            .map(|g| g.is_leader_or_assistant(player_guid))
            .unwrap_or(false)
    }

    // ========== GROUP LIFECYCLE ==========

    /// Create a new group with the given leader
    async fn create_group(
        &self,
        leader_guid: ObjectGuid,
        leader_name: String,
    ) -> Result<u32, GroupError> {
        // Allocate group ID
        let group_id = self.next_group_id.fetch_add(1, Ordering::Relaxed);

        // Create group data
        let group = GroupData::new(group_id, leader_guid, leader_name);

        // Save to database
        let group_row = self.group_to_row(&group);
        self.repository
            .save_group(&group_row)
            .await
            .map_err(|e| GroupError::Internal(e.to_string()))?;

        // Add leader as member in database
        self.repository
            .add_member(group_id, leader_guid.counter(), 0)
            .await
            .map_err(|e| GroupError::Internal(e.to_string()))?;

        // Add to cache
        self.groups.insert(group_id, group);
        self.player_groups.insert(leader_guid, group_id);

        tracing::info!(
            "Created group {} with leader {:?}",
            group_id,
            leader_guid
        );

        Ok(group_id)
    }

    /// Disband a group
    pub async fn disband_group(&self, group_id: u32) -> Result<(), GroupError> {
        let group = self
            .groups
            .remove(&group_id)
            .map(|(_, g)| g)
            .ok_or(GroupError::NotInGroup)?;

        // Remove all player mappings
        for member in &group.members {
            self.player_groups.remove(&member.guid);
        }

        // Delete from database
        self.repository
            .delete_group(group_id)
            .await
            .map_err(|e| GroupError::Internal(e.to_string()))?;

        // Notify all members
        use crate::shared::messages::group::SmsgGroupDestroyed;
        for member in &group.members {
            self.broadcast_mgr
                .send_msg_to_player(member.guid, SmsgGroupDestroyed)
                ;

            tracing::debug!("Sent GROUP_DESTROYED to {:?}", member.guid);
        }

        tracing::info!("Disbanded group {}", group_id);
        Ok(())
    }

    // ========== INVITE/JOIN/LEAVE ==========

    /// Invite a player to the group (CMSG_GROUP_INVITE)
    pub async fn invite_player(
        &self,
        inviter_guid: ObjectGuid,
        target_name: String,
    ) -> Result<(), GroupError> {
        // Find target by name
        let target_guid = self
            .player_mgr
            .find_player_by_name(&target_name)
            .ok_or(GroupError::TargetNotFound)?;

        // Cannot invite self
        if inviter_guid == target_guid {
            return Err(GroupError::CannotTargetSelf);
        }

        // Check target not already in group
        if self.is_in_group(target_guid) {
            return Err(GroupError::TargetAlreadyInGroup);
        }

        // Check target doesn't have pending invite
        if self.has_pending_invite(target_guid) {
            return Err(GroupError::TargetHasPendingInvite);
        }

        // Get inviter's group (or 0 if not in one)
        let group_id = self.get_player_group_id(inviter_guid).unwrap_or(0);

        // If inviter in group, check permissions and capacity
        if group_id > 0 {
            let group = self.get_group(group_id).ok_or(GroupError::NotInGroup)?;

            // Check permissions (leader or assistant)
            if !group.is_leader_or_assistant(inviter_guid) {
                return Err(GroupError::NotLeaderOrAssistant);
            }

            // Check not full
            if group.is_full() {
                return Err(GroupError::GroupFull);
            }
        }

        // TODO: Check faction if cross-faction disabled
        // TODO: Check ignore list

        // Get inviter name
        let inviter_name = self
            .player_mgr
            .get_player_name(inviter_guid)
            .unwrap_or_else(|| "Unknown".to_string());

        // Store pending invite
        let invite = GroupInvite::new(inviter_guid, inviter_name.clone(), group_id);
        self.pending_invites.insert(target_guid, invite);

        // Send SMSG_GROUP_INVITE to target
        use crate::shared::messages::group::SmsgGroupInvite;
        let msg = SmsgGroupInvite {
            inviter_name: &inviter_name,
        };
        self.broadcast_mgr
            .send_msg_to_player(target_guid, msg)
            ;

        tracing::info!(
            "{} invited {} to group",
            inviter_name,
            target_name
        );

        Ok(())
    }

    /// Accept a group invite (CMSG_GROUP_ACCEPT)
    pub async fn accept_invite(&self, player_guid: ObjectGuid) -> Result<(), GroupError> {
        // Get and remove pending invite
        let invite = self
            .pending_invites
            .remove(&player_guid)
            .map(|(_, v)| v)
            .ok_or(GroupError::NotInGroup)?;

        let player_name = self
            .player_mgr
            .get_player_name(player_guid)
            .unwrap_or_else(|| "Unknown".to_string());

        // Create group if inviter wasn't in one
        let group_id = if invite.group_id == 0 {
            // Create new group with inviter as leader
            self.create_group(invite.inviter_guid, invite.inviter_name.clone())
                .await?
        } else {
            invite.group_id
        };

        // Add player to group
        self.add_member_to_group(group_id, player_guid, player_name.clone())
            .await?;

        // Broadcast updated group list to all members
        self.broadcast_group_list(group_id);

        tracing::info!("{} joined group {}", player_name, group_id);

        Ok(())
    }

    /// Decline a group invite (CMSG_GROUP_DECLINE)
    pub async fn decline_invite(&self, player_guid: ObjectGuid) -> Result<(), GroupError> {
        // Remove pending invite
        let invite = self.pending_invites.remove(&player_guid).map(|(_, v)| v);

        if let Some(invite) = invite {
            // Send notification to inviter that player declined
            use crate::shared::messages::group::SmsgPartyCommandResult;
            let player_name = self
                .player_mgr
                .get_player_name(player_guid)
                .unwrap_or_else(|| "Unknown".to_string());

            let msg = SmsgPartyCommandResult {
                operation: PARTY_OP_INVITE,
                member_name: &player_name,
                result: ERR_PARTY_RESULT_OK, // Declined is not an error
            };
            self.broadcast_mgr
                .send_msg_to_player(invite.inviter_guid, msg)
                ;

            tracing::info!(
                "{:?} declined invite from {:?}",
                player_guid,
                invite.inviter_guid
            );
        }

        Ok(())
    }

    /// Leave the current group (MSG_PARTY_LEAVE or CMSG_GROUP_DISBAND for leader)
    pub async fn leave_group(&self, player_guid: ObjectGuid) -> Result<(), GroupError> {
        let group_id = self
            .get_player_group_id(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        // Remove player from group
        self.remove_member_from_group(group_id, player_guid).await?;

        // Check if group should be disbanded (less than 2 members)
        if let Some(group) = self.get_group(group_id) {
            if group.member_count() < 2 {
                self.disband_group(group_id).await?;
            } else {
                // Broadcast updated group list
                self.broadcast_group_list(group_id);
            }
        }

        // Send SMSG_GROUP_DESTROYED to leaving player
        use crate::shared::messages::group::SmsgGroupDestroyed;
        self.broadcast_mgr
            .send_msg_to_player(player_guid, SmsgGroupDestroyed)
            ;

        Ok(())
    }

    /// Uninvite (kick) a player from the group
    pub async fn uninvite_player(
        &self,
        remover_guid: ObjectGuid,
        target_name: String,
    ) -> Result<(), GroupError> {
        let group_id = self
            .get_player_group_id(remover_guid)
            .ok_or(GroupError::NotInGroup)?;

        let group = self.get_group(group_id).ok_or(GroupError::NotInGroup)?;

        // Check permissions (leader or assistant)
        if !group.is_leader_or_assistant(remover_guid) {
            return Err(GroupError::NotLeaderOrAssistant);
        }

        // Find target in group
        let target = group
            .get_member_by_name(&target_name)
            .ok_or(GroupError::MemberNotFound)?;

        let target_guid = target.guid;

        // Cannot kick self
        if target_guid == remover_guid {
            return Err(GroupError::CannotTargetSelf);
        }

        // Cannot kick leader
        if group.is_leader(target_guid) {
            return Err(GroupError::NotLeader);
        }

        // Remove from group
        self.remove_member_from_group(group_id, target_guid).await?;

        // Check if group should be disbanded
        if let Some(group) = self.get_group(group_id) {
            if group.member_count() < 2 {
                self.disband_group(group_id).await?;
            } else {
                self.broadcast_group_list(group_id);
            }
        }

        // Send SMSG_GROUP_UNINVITE to kicked player
        use crate::shared::messages::group::SmsgGroupUninvite;
        self.broadcast_mgr
            .send_msg_to_player(target_guid, SmsgGroupUninvite)
            ;

        tracing::info!(
            "{:?} kicked {} from group {}",
            remover_guid,
            target_name,
            group_id
        );

        Ok(())
    }

    /// Add a member to an existing group
    async fn add_member_to_group(
        &self,
        group_id: u32,
        player_guid: ObjectGuid,
        player_name: String,
    ) -> Result<(), GroupError> {
        // Update in-memory cache
        {
            let mut group = self
                .groups
                .get_mut(&group_id)
                .ok_or(GroupError::NotInGroup)?;
            group.add_member(player_guid, player_name)?;
        }

        // Get subgroup for database
        let subgroup = self
            .get_group(group_id)
            .and_then(|g| g.get_member(player_guid).map(|m| m.subgroup as u16))
            .unwrap_or(0);

        // Add to player mappings
        self.player_groups.insert(player_guid, group_id);

        // Save to database
        self.repository
            .add_member(group_id, player_guid.counter(), subgroup)
            .await
            .map_err(|e| GroupError::Internal(e.to_string()))?;

        Ok(())
    }

    /// Remove a member from a group
    async fn remove_member_from_group(
        &self,
        group_id: u32,
        player_guid: ObjectGuid,
    ) -> Result<(), GroupError> {
        let was_leader;

        // Update in-memory cache
        {
            let mut group = self
                .groups
                .get_mut(&group_id)
                .ok_or(GroupError::NotInGroup)?;

            was_leader = group.is_leader(player_guid);
            group
                .remove_member(player_guid)
                .ok_or(GroupError::MemberNotFound)?;

            // If leader left, promote new leader
            if was_leader && group.member_count() > 0 {
                group.select_new_leader();

                // Update database with new leader
                let group_row = self.group_to_row(&group);
                drop(group); // Release lock before async
                let _ = self.repository.save_group(&group_row);
            }
        }

        // Remove from player mappings
        self.player_groups.remove(&player_guid);

        // Remove from database
        self.repository
            .remove_member(group_id, player_guid.counter())
            .await
            .map_err(|e| GroupError::Internal(e.to_string()))?;

        Ok(())
    }

    // ========== LEADERSHIP ==========

    /// Set the group leader (CMSG_GROUP_SET_LEADER)
    pub async fn set_leader(
        &self,
        player_guid: ObjectGuid,
        new_leader_guid: ObjectGuid,
    ) -> Result<(), GroupError> {
        let group_id = self
            .get_player_group_id(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        // Verify current player is leader
        {
            let group = self.get_group(group_id).ok_or(GroupError::NotInGroup)?;

            if !group.is_leader(player_guid) {
                return Err(GroupError::NotLeader);
            }

            if !group.has_member(new_leader_guid) {
                return Err(GroupError::MemberNotFound);
            }
        }

        // Update group
        {
            let mut group = self
                .groups
                .get_mut(&group_id)
                .ok_or(GroupError::NotInGroup)?;

            group.promote_new_leader(new_leader_guid)?;

            // Save to database
            let group_row = self.group_to_row(&group);
            drop(group);
            self.repository
                .save_group(&group_row)
                .await
                .map_err(|e| GroupError::Internal(e.to_string()))?;
        }

        // Broadcast SMSG_GROUP_SET_LEADER and updated group list
        let new_leader_name = self
            .player_mgr
            .get_player_name(new_leader_guid)
            .unwrap_or_else(|| "Unknown".to_string());

        use crate::shared::messages::group::SmsgGroupSetLeader;
        let msg = SmsgGroupSetLeader {
            leader_name: &new_leader_name,
        };
        self.broadcast_to_group(group_id, msg);

        self.broadcast_group_list(group_id);

        tracing::info!(
            "Group {} leader changed to {:?}",
            group_id,
            new_leader_guid
        );

        Ok(())
    }

    /// Set assistant flag for a member (CMSG_GROUP_ASSISTANT_LEADER)
    pub async fn set_assistant(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        assistant: bool,
    ) -> Result<(), GroupError> {
        let group_id = self
            .get_player_group_id(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        // Verify current player is leader (only leader can set assistants)
        {
            let group = self.get_group(group_id).ok_or(GroupError::NotInGroup)?;

            if !group.is_leader(player_guid) {
                return Err(GroupError::NotLeader);
            }

            if !group.is_raid {
                return Err(GroupError::NotRaid);
            }
        }

        // Update group
        {
            let mut group = self
                .groups
                .get_mut(&group_id)
                .ok_or(GroupError::NotInGroup)?;

            group.set_assistant(target_guid, assistant)?;
        }

        // Update database
        if let Some(group) = self.get_group(group_id) {
            if let Some(member) = group.get_member(target_guid) {
                self.repository
                    .update_member(
                        group_id,
                        target_guid.counter(),
                        assistant,
                        member.subgroup as u16,
                    )
                    .await
                    .map_err(|e| GroupError::Internal(e.to_string()))?;
            }
        }

        self.broadcast_group_list(group_id);

        Ok(())
    }

    /// Set main tank (CMSG_GROUP_SET_MAIN_TANK)
    pub async fn set_main_tank(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
    ) -> Result<(), GroupError> {
        let group_id = self
            .get_player_group_id(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        // Verify permissions (leader only)
        {
            let group = self.get_group(group_id).ok_or(GroupError::NotInGroup)?;

            if !group.is_leader(player_guid) {
                return Err(GroupError::NotLeader);
            }
        }

        // Update group
        {
            let mut group = self
                .groups
                .get_mut(&group_id)
                .ok_or(GroupError::NotInGroup)?;

            group.set_main_tank(target_guid)?;

            // Save to database
            let group_row = self.group_to_row(&group);
            drop(group);
            self.repository
                .save_group(&group_row)
                .await
                .map_err(|e| GroupError::Internal(e.to_string()))?;
        }

        self.broadcast_group_list(group_id);

        Ok(())
    }

    /// Set main assistant (CMSG_GROUP_SET_MAIN_ASSISTANT)
    pub async fn set_main_assistant(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
    ) -> Result<(), GroupError> {
        let group_id = self
            .get_player_group_id(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        // Verify permissions (leader only)
        {
            let group = self.get_group(group_id).ok_or(GroupError::NotInGroup)?;

            if !group.is_leader(player_guid) {
                return Err(GroupError::NotLeader);
            }
        }

        // Update group
        {
            let mut group = self
                .groups
                .get_mut(&group_id)
                .ok_or(GroupError::NotInGroup)?;

            group.set_main_assistant(target_guid)?;

            // Save to database
            let group_row = self.group_to_row(&group);
            drop(group);
            self.repository
                .save_group(&group_row)
                .await
                .map_err(|e| GroupError::Internal(e.to_string()))?;
        }

        self.broadcast_group_list(group_id);

        Ok(())
    }

    // ========== ORGANIZATION ==========

    /// Convert party to raid (CMSG_GROUP_RAID_CONVERT)
    pub async fn convert_to_raid(&self, player_guid: ObjectGuid) -> Result<(), GroupError> {
        let group_id = self
            .get_player_group_id(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        // Verify permissions (leader only)
        {
            let group = self.get_group(group_id).ok_or(GroupError::NotInGroup)?;

            if !group.is_leader(player_guid) {
                return Err(GroupError::NotLeader);
            }

            if group.is_raid {
                return Ok(()); // Already a raid
            }
        }

        // Send SMSG_PARTY_COMMAND_RESULT before converting
        use crate::shared::messages::group::SmsgPartyCommandResult;
        let cmd_result = SmsgPartyCommandResult {
            operation: PARTY_OP_INVITE,
            member_name: "",
            result: ERR_PARTY_RESULT_OK,
        };
        self.broadcast_mgr
            .send_msg_to_player(player_guid, cmd_result)
            ;

        // Update group
        {
            let mut group = self
                .groups
                .get_mut(&group_id)
                .ok_or(GroupError::NotInGroup)?;

            group.convert_to_raid();

            // Save to database
            let group_row = self.group_to_row(&group);
            drop(group);
            self.repository
                .save_group(&group_row)
                .await
                .map_err(|e| GroupError::Internal(e.to_string()))?;
        }

        self.broadcast_group_list(group_id);

        tracing::info!("Group {} converted to raid", group_id);

        Ok(())
    }

    /// Change a member's subgroup (CMSG_GROUP_CHANGE_SUB_GROUP)
    pub async fn change_subgroup(
        &self,
        player_guid: ObjectGuid,
        target_name: String,
        new_subgroup: u8,
    ) -> Result<(), GroupError> {
        let group_id = self
            .get_player_group_id(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        let target_guid;

        // Verify permissions (leader or assistant)
        {
            let group = self.get_group(group_id).ok_or(GroupError::NotInGroup)?;

            if !group.is_leader_or_assistant(player_guid) {
                return Err(GroupError::NotLeaderOrAssistant);
            }

            let target = group
                .get_member_by_name(&target_name)
                .ok_or(GroupError::MemberNotFound)?;

            target_guid = target.guid;
        }

        // Update group
        {
            let mut group = self
                .groups
                .get_mut(&group_id)
                .ok_or(GroupError::NotInGroup)?;

            group.change_subgroup(target_guid, new_subgroup)?;
        }

        // Update database
        if let Some(group) = self.get_group(group_id) {
            if let Some(member) = group.get_member(target_guid) {
                self.repository
                    .update_member(
                        group_id,
                        target_guid.counter(),
                        member.assistant,
                        new_subgroup as u16,
                    )
                    .await
                    .map_err(|e| GroupError::Internal(e.to_string()))?;
            }
        }

        self.broadcast_group_list(group_id);

        Ok(())
    }

    /// Swap two members' subgroups (CMSG_GROUP_SWAP_SUB_GROUP)
    pub async fn swap_subgroups(
        &self,
        player_guid: ObjectGuid,
        name1: String,
        name2: String,
    ) -> Result<(), GroupError> {
        let group_id = self
            .get_player_group_id(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        let guid1;
        let guid2;
        let subgroup1;
        let subgroup2;

        // Verify permissions (leader or assistant)
        {
            let group = self.get_group(group_id).ok_or(GroupError::NotInGroup)?;

            if !group.is_leader_or_assistant(player_guid) {
                return Err(GroupError::NotLeaderOrAssistant);
            }

            let member1 = group
                .get_member_by_name(&name1)
                .ok_or(GroupError::MemberNotFound)?;
            let member2 = group
                .get_member_by_name(&name2)
                .ok_or(GroupError::MemberNotFound)?;

            guid1 = member1.guid;
            guid2 = member2.guid;
            subgroup1 = member1.subgroup;
            subgroup2 = member2.subgroup;
        }

        // Update group
        {
            let mut group = self
                .groups
                .get_mut(&group_id)
                .ok_or(GroupError::NotInGroup)?;

            group.swap_subgroups(guid1, guid2)?;
        }

        // Update database for both members
        if let Some(group) = self.get_group(group_id) {
            if let Some(m1) = group.get_member(guid1) {
                let _ = self
                    .repository
                    .update_member(group_id, guid1.counter(), m1.assistant, subgroup2 as u16)
                    ;
            }
            if let Some(m2) = group.get_member(guid2) {
                let _ = self
                    .repository
                    .update_member(group_id, guid2.counter(), m2.assistant, subgroup1 as u16)
                    ;
            }
        }

        self.broadcast_group_list(group_id);

        Ok(())
    }

    // ========== LOOT SETTINGS ==========

    /// Set loot method (CMSG_SET_LOOT_METHOD)
    pub async fn set_loot_method(
        &self,
        player_guid: ObjectGuid,
        method: LootMethod,
        threshold: u8,
        looter_guid: ObjectGuid,
    ) -> Result<(), GroupError> {
        let group_id = self
            .get_player_group_id(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        // Verify permissions (leader only)
        {
            let group = self.get_group(group_id).ok_or(GroupError::NotInGroup)?;

            if !group.is_leader(player_guid) {
                return Err(GroupError::NotLeader);
            }
        }

        // Update group
        {
            let mut group = self
                .groups
                .get_mut(&group_id)
                .ok_or(GroupError::NotInGroup)?;

            group.loot_method = method;
            group.loot_threshold = threshold;
            group.looter_guid = looter_guid;

            // Save to database
            let group_row = self.group_to_row(&group);
            drop(group);
            self.repository
                .save_group(&group_row)
                .await
                .map_err(|e| GroupError::Internal(e.to_string()))?;
        }

        self.broadcast_group_list(group_id);

        Ok(())
    }

    // ========== RAID FEATURES ==========

    /// Set raid target icon (MSG_RAID_TARGET_UPDATE)
    pub async fn set_target_icon(
        &self,
        player_guid: ObjectGuid,
        icon_id: u8,
        target_guid: ObjectGuid,
    ) -> Result<(), GroupError> {
        let group_id = self
            .get_player_group_id(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        // Verify permissions (leader or assistant)
        {
            let group = self.get_group(group_id).ok_or(GroupError::NotInGroup)?;

            if !group.is_leader_or_assistant(player_guid) {
                return Err(GroupError::NotLeaderOrAssistant);
            }
        }

        // Update group
        {
            let mut group = self
                .groups
                .get_mut(&group_id)
                .ok_or(GroupError::NotInGroup)?;

            group.set_target_icon(icon_id, target_guid)?;

            // Save to database
            let group_row = self.group_to_row(&group);
            drop(group);
            self.repository
                .save_group(&group_row)
                .await
                .map_err(|e| GroupError::Internal(e.to_string()))?;
        }

        // Broadcast target icon update to group
        use crate::shared::messages::group::MsgRaidTargetUpdate;
        let Some(group) = self.get_group(group_id) else {
            return Ok(());
        };

        let msg = MsgRaidTargetUpdate {
            mode: 0, // set icons
            target_icons: group.target_icons,
        };

        for member in &group.members {
            self.broadcast_mgr
                .send_msg_to_player(member.guid, msg.clone())
                ;
        }

        Ok(())
    }

    /// Get target icons (MSG_RAID_TARGET_UPDATE with icon_id=0xFF)
    pub async fn get_target_icons(
        &self,
        player_guid: ObjectGuid,
    ) -> Result<[ObjectGuid; 8], GroupError> {
        let group = self
            .get_player_group(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        Ok(group.target_icons)
    }

    /// Send current target icons to a player (response to icon request)
    pub fn send_target_icons(&self, player_guid: ObjectGuid) {
        let Some(group) = self.get_player_group(player_guid) else {
            return;
        };

        use crate::shared::messages::group::MsgRaidTargetUpdate;
        let msg = MsgRaidTargetUpdate {
            mode: 0, // set icons
            target_icons: group.target_icons,
        };
        self.broadcast_mgr
            .send_msg_to_player(player_guid, msg)
            ;
    }

    /// Initiate ready check (MSG_RAID_READY_CHECK)
    pub async fn initiate_ready_check(&self, player_guid: ObjectGuid) -> Result<(), GroupError> {
        let group_id = self
            .get_player_group_id(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        // Verify permissions (leader or assistant)
        {
            let group = self.get_group(group_id).ok_or(GroupError::NotInGroup)?;

            if !group.is_leader_or_assistant(player_guid) {
                return Err(GroupError::NotLeaderOrAssistant);
            }
        }

        // Broadcast ready check to all members
        use crate::shared::messages::group::MsgRaidReadyCheck;
        if let Some(group) = self.get_group(group_id) {
            let msg = MsgRaidReadyCheck {
                player_guid,
                ready: None, // None = initiate
            };

            for member in &group.members {
                self.broadcast_mgr
                    .send_msg_to_player(member.guid, msg.clone())
                    ;

                tracing::debug!("Sent READY_CHECK to {:?}", member.guid);
            }
        }

        tracing::info!("Ready check initiated by {:?}", player_guid);

        Ok(())
    }

    /// Respond to ready check (MSG_RAID_READY_CHECK with state)
    pub async fn respond_ready_check(
        &self,
        player_guid: ObjectGuid,
        ready: bool,
    ) -> Result<(), GroupError> {
        let group = self
            .get_player_group(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        // Send response to all group members
        use crate::shared::messages::group::MsgRaidReadyCheck;
        let msg = MsgRaidReadyCheck {
            player_guid,
            ready: Some(ready),
        };

        for member in &group.members {
            self.broadcast_mgr
                .send_msg_to_player(member.guid, msg.clone())
                ;
        }

        tracing::debug!(
            "{:?} responded to ready check: {}",
            player_guid,
            ready
        );

        Ok(())
    }

    // ========== COMMUNICATION ==========

    /// Send minimap ping to group (MSG_MINIMAP_PING)
    pub fn send_minimap_ping(
        &self,
        player_guid: ObjectGuid,
        x: f32,
        y: f32,
    ) -> Result<(), GroupError> {
        let group = self
            .get_player_group(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        // Broadcast to all group members except sender
        use crate::shared::messages::group::MsgMinimapPing;
        let msg = MsgMinimapPing { player_guid, x, y };

        for member in &group.members {
            if member.guid != player_guid {
                self.broadcast_mgr
                    .send_msg_to_player(member.guid, msg)
                    ;

                tracing::debug!("Sent MINIMAP_PING to {:?}", member.guid);
            }
        }

        Ok(())
    }

    /// Random roll (MSG_RANDOM_ROLL)
    pub async fn random_roll(
        &self,
        player_guid: ObjectGuid,
        min: u32,
        max: u32,
    ) -> Result<(), GroupError> {
        // Validate range
        if min > max || max > 10000 {
            return Ok(());
        }

        // Generate roll
        let roll = if min == max {
            min
        } else {
            rand::thread_rng().gen_range(min..=max)
        };

        // Broadcast to group (or just player if not in group)
        use crate::shared::messages::group::MsgRandomRoll;
        if let Some(group) = self.get_player_group(player_guid) {
            let msg = MsgRandomRoll {
                min,
                max,
                roll,
                player_guid,
            };

            for member in &group.members {
                self.broadcast_mgr
                    .send_msg_to_player(member.guid, msg)
                    ;

                tracing::debug!(
                    "Sent RANDOM_ROLL to {:?}: {} ({}-{})",
                    member.guid,
                    roll,
                    min,
                    max
                );
            }
        }

        tracing::debug!(
            "{:?} rolled {} ({}-{})",
            player_guid,
            roll,
            min,
            max
        );

        Ok(())
    }

    // ========== MEMBER STATS ==========

    /// Request party member stats (CMSG_REQUEST_PARTY_MEMBER_STATS)
    pub async fn request_member_stats(
        &self,
        requester_guid: ObjectGuid,
        target_guid: ObjectGuid,
    ) -> Result<(), GroupError> {
        // Verify both in same group
        let group = self
            .get_player_group(requester_guid)
            .ok_or(GroupError::NotInGroup)?;

        if !group.has_member(target_guid) {
            return Err(GroupError::MemberNotFound);
        }

        // TODO: Build and send SMSG_PARTY_MEMBER_STATS_FULL

        Ok(())
    }

    // ========== BROADCASTING ==========

    /// Broadcast SMSG_GROUP_LIST to all group members
    fn broadcast_group_list(&self, group_id: u32) {
        let Some(group) = self.get_group(group_id) else {
            return;
        };

        // Log what we're about to send
        tracing::debug!(
            "Broadcasting group {} with {} members",
            group_id,
            group.members.len()
        );

        // Convert to CachedGroup for packet building
        use crate::world::game::group::cache::CachedGroup;
        let cached_group = CachedGroup::from_group_data(&group);

        // Send SMSG_GROUP_LIST to each member (each needs different own_flags)
        use crate::shared::messages::group::SmsgGroupList;
        for member in &group.members {
            let msg = SmsgGroupList {
                group: &cached_group,
                member_guid: member.guid,
            };
            self.broadcast_mgr
                .send_msg_to_player(member.guid, msg)
                ;

            tracing::debug!("Sent GROUP_LIST to {:?}", member.guid);
        }

        // Send MSG_RAID_TARGET_UPDATE after group list
        // Mode 1 = full icon list - only includes non-empty icons
        use crate::shared::messages::group::MsgRaidTargetUpdate;
        let raid_target_msg = MsgRaidTargetUpdate {
            mode: 1, // Full icon list
            target_icons: group.target_icons,
        };
        for member in &group.members {
            self.broadcast_mgr
                .send_msg_to_player(member.guid, raid_target_msg.clone())
                ;
        }
    }

    /// Broadcast a message to all group members
    #[allow(dead_code)]
    fn broadcast_to_group<M>(&self, group_id: u32, msg: M)
    where
        M: crate::shared::messages::ToWorldPacket + Clone + Send,
    {
        let Some(group) = self.get_group(group_id) else {
            return;
        };

        for member in &group.members {
            self.broadcast_mgr
                .send_msg_to_player(member.guid, msg.clone())
                ;
        }
    }

    // ========== HELPER METHODS ==========

    /// Convert GroupData to database row
    fn group_to_row(&self, group: &GroupData) -> GroupRow {
        GroupRow {
            group_id: group.id,
            leader_guid: group.leader_guid.counter(),
            main_tank_guid: group.main_tank_guid.counter(),
            main_assistant_guid: group.main_assistant_guid.counter(),
            loot_method: group.loot_method as u8,
            loot_threshold: group.loot_threshold,
            looter_guid: group.looter_guid.counter(),
            icon1: group.target_icons[0].raw() as u32,
            icon2: group.target_icons[1].raw() as u32,
            icon3: group.target_icons[2].raw() as u32,
            icon4: group.target_icons[3].raw() as u32,
            icon5: group.target_icons[4].raw() as u32,
            icon6: group.target_icons[5].raw() as u32,
            icon7: group.target_icons[6].raw() as u32,
            icon8: group.target_icons[7].raw() as u32,
            is_raid: if group.is_raid { 1 } else { 0 },
        }
    }

    /// Update member status when they login
    pub fn update_member_online(&self, player_guid: ObjectGuid) {
        if let Some(group_id) = self.get_player_group_id(player_guid) {
            if let Some(mut group) = self.groups.get_mut(&group_id) {
                group.set_member_status(player_guid, MemberStatus::new());
            }
        }
    }

    /// Update member status when they logout
    pub fn update_member_offline(&self, player_guid: ObjectGuid) {
        if let Some(group_id) = self.get_player_group_id(player_guid) {
            if let Some(mut group) = self.groups.get_mut(&group_id) {
                group.set_member_status(player_guid, MemberStatus::offline());
            }
        }
    }
}

// ========== SYSTEM TRAIT IMPLEMENTATION ==========

impl GroupSystem {
    pub async fn init(&self) -> Result<()> {
        self.initialize().await?;
        self.load_all_groups().await?;
        Ok(())
    }

    pub fn update(&self, _diff: std::time::Duration) -> Result<()> {
        // No periodic updates needed
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.groups.clear();
        self.player_groups.clear();
        self.pending_invites.clear();

        Ok(())
    }

    pub async fn on_player_login(&self, guid: ObjectGuid) -> Result<()> {
        // Update member status to online
        self.update_member_online(guid);

        // Send group list to player on login if in group
        if let Some(group_id) = self.get_player_group_id(guid) {
            tracing::info!(
                "[GROUP] Player {:?} logged in, in group {}, broadcasting group list",
                guid,
                group_id
            );
            // Broadcast updated group list to all members
            self.broadcast_group_list(group_id);
        } else {
            tracing::debug!("[GROUP] Player {:?} logged in, not in any group", guid);
        }

        Ok(())
    }

    pub async fn on_player_logout(&self, guid: ObjectGuid) -> Result<()> {
        // Clear pending invites
        self.pending_invites.remove(&guid);

        // Get group_id before updating status
        let group_id = self.get_player_group_id(guid);

        // Update member status to offline
        self.update_member_offline(guid);

        // Broadcast updated group list to remaining members
        if let Some(group_id) = group_id {
            tracing::info!(
                "[GROUP] Player {:?} logged out from group {}",
                guid,
                group_id
            );
            self.broadcast_group_list(group_id);
        }

        Ok(())
    }
}
