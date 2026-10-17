//! Group System - party/raid management for world

use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use rand::Rng;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::game::player::death::DeathState;
use crate::game::player::{
    PlayerManager, PLAYER_FLAGS_AFK, PLAYER_FLAGS_DND, PLAYER_FLAGS_GHOST,
    PLAYER_FLAGS_GROUP_LEADER,
};
use crate::game::social::SocialSystem;
use crate::World;
use oxcore_db::database::characters::models::group::{GroupMemberRow, GroupRow};
use oxcore_db::database::characters::repositories::GroupRepositoryTrait;
use oxcore_shared::game::chat::Team;
use oxcore_shared::protocol::ObjectGuid;

use oxcore_shared::game::group::{
    group_update_flags, CachedGroup, GroupData, GroupError, GroupInvite, GroupMember, LootMethod,
    MemberStatus, ERR_ALREADY_IN_GROUP_S, ERR_BAD_PLAYER_NAME_S, ERR_GROUP_FULL,
    ERR_IGNORING_YOU_S, ERR_NOT_LEADER, ERR_PARTY_RESULT_OK, ERR_PLAYER_WRONG_FACTION,
    ERR_TARGET_NOT_IN_GROUP_S, MAX_GROUP_SIZE, MAX_RAID_SIZE, MAX_RAID_SUBGROUPS, PARTY_OP_INVITE,
    PARTY_OP_LEAVE,
};
use oxcore_shared::messages::group::SmsgPartyMemberStats;
/// Current Unix time in seconds.
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Non-pet subset of `group_update_flags::FULL` pushed to a member on join and login.
const MEMBER_STATS_FULL: u32 = group_update_flags::STATUS
    | group_update_flags::CUR_HP
    | group_update_flags::MAX_HP
    | group_update_flags::POWER_TYPE
    | group_update_flags::CUR_POWER
    | group_update_flags::MAX_POWER
    | group_update_flags::LEVEL
    | group_update_flags::ZONE;

/// How long a ready check stays open: the client's own countdown.
const READY_CHECK_DURATION_MS: u64 = 35_000;

/// An open ready check: who has answered, and how long is left.
#[derive(Debug, Clone)]
struct ReadyCheck {
    answered: std::collections::HashSet<ObjectGuid>,
    remaining_ms: u64,
}

/// How often grouped players' party-frame values are diffed for stat pushes.
const MEMBER_SNAPSHOT_INTERVAL_MS: u64 = 1_000;

/// The party-frame values tracked for out-of-range stat pushes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MemberSnapshot {
    status: u8,
    health: u32,
    max_health: u32,
    power_type: u8,
    power: u32,
    max_power: u32,
    level: u8,
    zone: u32,
}

impl MemberSnapshot {
    /// Update flags for every field that differs from `previous`.
    fn changed_flags(&self, previous: &MemberSnapshot) -> u32 {
        let mut flags = group_update_flags::NONE;
        if self.status != previous.status {
            flags |= group_update_flags::STATUS;
        }
        if self.health != previous.health {
            flags |= group_update_flags::CUR_HP;
        }
        if self.max_health != previous.max_health {
            flags |= group_update_flags::MAX_HP;
        }
        if self.power_type != previous.power_type {
            flags |= group_update_flags::POWER_TYPE
                | group_update_flags::CUR_POWER
                | group_update_flags::MAX_POWER;
        }
        if self.power != previous.power {
            flags |= group_update_flags::CUR_POWER;
        }
        if self.max_power != previous.max_power {
            flags |= group_update_flags::MAX_POWER;
        }
        if self.level != previous.level {
            flags |= group_update_flags::LEVEL;
        }
        if self.zone != previous.zone {
            flags |= group_update_flags::ZONE;
        }
        flags
    }
}

/// Group System - manages player parties and raids
pub struct GroupSystem {
    /// Group data indexed by group ID
    pub(super) groups: DashMap<u32, GroupData>,

    /// Player group membership (player_guid -> group_id)
    player_groups: DashMap<ObjectGuid, u32>,

    /// Pending invites (invitee_guid -> GroupInvite)
    pending_invites: DashMap<ObjectGuid, GroupInvite>,

    /// Next group ID for auto-assignment
    next_group_id: AtomicU32,

    /// Repository for database access
    repository: Arc<dyn GroupRepositoryTrait>,

    /// Broadcast manager for sending packets
    pub(super) broadcast_mgr: Arc<dyn BroadcastManagerTrait>,

    /// Player manager for player lookups
    pub(super) player_mgr: Arc<PlayerManager>,

    /// Social system for ignore-list checks on invites
    social_system: Arc<SocialSystem>,

    /// Whether cross-faction grouping is allowed
    allow_cross_faction_group: bool,

    /// Pending SMSG_PARTY_MEMBER_STATS update masks, keyed by member GUID. Set by systems
    /// when a grouped stat changes; drained each tick in `update`.
    member_update_flags: DashMap<ObjectGuid, u32>,

    /// Per-group in-flight loot rolls (`roll.rs`), keyed by group ID.
    pub(super) loot_rolls: DashMap<u32, Vec<crate::game::loot::Roll>>,

    /// Per-group round-robin cursor (`group_id -> last looter`), for non-FFA/master methods.
    /// Each corpse records its own looter; this only decides who gets the next one.
    pub(super) round_robin_looters: DashMap<u32, ObjectGuid>,

    /// Groups with an offline-leader reassignment task in flight.
    leader_reassignments: dashmap::DashSet<u32>,

    /// Last party-frame values seen per grouped player, diffed once a second to queue
    /// SMSG_PARTY_MEMBER_STATS updates for whatever changed.
    member_snapshots: DashMap<ObjectGuid, MemberSnapshot>,

    /// Milliseconds accumulated towards the next snapshot diff.
    snapshot_elapsed_ms: std::sync::atomic::AtomicU64,

    /// Open ready checks by group ID.
    ready_checks: DashMap<u32, ReadyCheck>,

    /// Players who chose to pass on all loot (1.14 `CMSG_OPT_OUT_OF_LOOT`); they are left out
    /// of new rolls.
    pub(super) passing_on_loot: dashmap::DashSet<ObjectGuid>,

    /// Arc to self, set once after construction, so periodic ticks can spawn
    /// long-running tasks (leader reassignment) that need `&GroupSystem`.
    pub(super) self_arc: std::sync::OnceLock<Arc<GroupSystem>>,
}

impl GroupSystem {
    /// Create a new group system
    pub fn new(
        repository: Arc<dyn GroupRepositoryTrait>,
        broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
        player_mgr: Arc<PlayerManager>,
        social_system: Arc<SocialSystem>,
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
            social_system,
            allow_cross_faction_group,
            member_update_flags: DashMap::new(),
            loot_rolls: DashMap::new(),
            round_robin_looters: DashMap::new(),
            leader_reassignments: dashmap::DashSet::new(),
            member_snapshots: DashMap::new(),
            snapshot_elapsed_ms: std::sync::atomic::AtomicU64::new(0),
            ready_checks: DashMap::new(),
            passing_on_loot: dashmap::DashSet::new(),
            self_arc: std::sync::OnceLock::new(),
        }
    }

    /// Store the `Arc<Self>` held by the system manager so spawned tasks can use it.
    pub fn set_self_arc(&self, arc: Arc<GroupSystem>) {
        let _ = self.self_arc.set(arc);
    }

    /// Flags pushed to out-of-range group members when the player levels up.
    pub fn level_update_flags() -> u32 {
        group_update_flags::STATUS
            | group_update_flags::CUR_HP
            | group_update_flags::MAX_HP
            | group_update_flags::CUR_POWER
            | group_update_flags::MAX_POWER
            | group_update_flags::LEVEL
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
                leader_last_online: 0,
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
        self.set_player_flag(leader_guid, PLAYER_FLAGS_GROUP_LEADER, true);

        tracing::info!("Created group {} with leader {:?}", group_id, leader_guid);

        Ok(group_id)
    }

    /// Disband a group.
    ///
    /// `initiator` is the player who left by choice; the remaining player still inside a dungeon
    /// inherits the group's non-permanent instance save (2f), then the group's instances are reset.
    pub async fn disband_group(
        &self,
        world: &World,
        group_id: u32,
        initiator: ObjectGuid,
    ) -> Result<(), GroupError> {
        let group = self
            .groups
            .remove(&group_id)
            .map(|(_, g)| g)
            .ok_or(GroupError::NotInGroup)?;

        // A player still inside a dungeon the group is saved to inherits that
        // temporary save personally, so disbanding does not strand them.
        if let Some(remaining) = group.members.iter().find(|m| m.guid != initiator) {
            let map_id = world
                .managers
                .player_mgr
                .get_player(remaining.guid)
                .map(|player| player.map_id);
            if let Some(map_id) = map_id {
                let instance_mgr = &world.managers.instance_mgr;
                if let Err(e) = instance_mgr
                    .ensure_group_bindings(&world.databases, group.leader_guid)
                    .await
                {
                    tracing::warn!("Failed to load saves of group {}: {}", group_id, e);
                }
                if let Err(e) = instance_mgr
                    .ensure_player_bindings(&world.databases, remaining.guid)
                    .await
                {
                    tracing::warn!("Failed to load saves of {:?}: {}", remaining.guid, e);
                }
                if let Some(binding) = instance_mgr.get_group_binding(group.leader_guid, map_id) {
                    let has_personal = instance_mgr
                        .get_player_binding(remaining.guid, map_id)
                        .is_some();
                    if !binding.permanent && !has_personal {
                        match instance_mgr
                            .bind_player_to_instance(
                                &world.databases,
                                remaining.guid,
                                map_id,
                                binding.instance_id,
                                false,
                                binding.reset_time,
                            )
                            .await
                        {
                            Err(e) => tracing::warn!(
                                "Failed to inherit instance {} to {:?} on group {} disband: {}",
                                binding.instance_id,
                                remaining.guid,
                                group_id,
                                e
                            ),
                            Ok(()) => {
                                if let Err(e) = instance_mgr
                                    .unbind_group_from_instance(
                                        &world.databases,
                                        group.leader_guid,
                                        map_id,
                                        binding.instance_id,
                                    )
                                    .await
                                {
                                    tracing::warn!(
                                        "Failed to release inherited group instance binding: {}",
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // The group's remaining saves go with it: resettable copies are reset,
        // copies someone is permanently saved to only lose the group's binding.
        let players_in_instance = |map_id: u32, instance_id: u32| {
            world
                .managers
                .map_mgr
                .get_map(map_id, instance_id)
                .map(|map| map.player_guids())
                .unwrap_or_default()
        };
        let is_raid_map = |map_id: u32| {
            crate::game::instance::lifecycle::map_entry(world, map_id)
                .is_some_and(|entry| entry.is_raid())
        };
        let outcome = world
            .managers
            .instance_mgr
            .reset_group_instances(
                &world.databases,
                group.leader_guid,
                crate::game::instance::manager::ResetScope::GroupDisband,
                false,
                is_raid_map,
                players_in_instance,
            )
            .await;
        crate::game::instance::lifecycle::apply_reset_outcome(world, &outcome);

        // Remove all player mappings
        for member in &group.members {
            self.player_groups.remove(&member.guid);
        }
        // Invites into a group that no longer exists are void.
        self.pending_invites
            .retain(|_, invite| invite.group_id != group_id);
        self.ready_checks.remove(&group_id);

        // Phase 5: resolve any in-flight loot rolls so rolled items are not stranded.
        if let Some((_, rolls)) = self.loot_rolls.remove(&group_id) {
            if let Some(self_arc) = self.self_arc.get().cloned() {
                for roll in rolls {
                    let world = world.clone();
                    let arc = Arc::clone(&self_arc);
                    tokio::spawn(async move {
                        arc.resolve_roll(roll, &world).await;
                    });
                }
            }
        }
        self.round_robin_looters.remove(&group_id);

        // Delete from database
        self.repository
            .delete_group(group_id)
            .await
            .map_err(|e| GroupError::Internal(e.to_string()))?;
        world
            .managers
            .instance_mgr
            .unload_group_bindings(group.leader_guid);

        // A group dissolving because it dropped below two members ends quietly: everyone gets
        // the "not in a group" list rather than a "group disbanded" message.
        self.set_player_flag(group.leader_guid, PLAYER_FLAGS_GROUP_LEADER, false);
        for member in &group.members {
            self.send_empty_group_list(member.guid);
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

        // Check faction if cross-faction disabled
        if !self.allow_cross_faction_group {
            let inviter_team = self
                .player_mgr
                .get_player(inviter_guid)
                .map(|p| p.get_team())
                .unwrap_or(Team::None);
            let target_team = self
                .player_mgr
                .get_player(target_guid)
                .map(|p| p.get_team())
                .unwrap_or(Team::None);
            if inviter_team != target_team {
                return Err(GroupError::WrongFaction);
            }
        }

        // Check target is not ignoring the inviter
        if self.social_system.has_ignore(target_guid, inviter_guid) {
            return Err(GroupError::TargetIgnoresPlayer);
        }

        // Get inviter name
        let inviter_name = self
            .player_mgr
            .get_player_name(inviter_guid)
            .unwrap_or_else(|| "Unknown".to_string());

        // Store pending invite
        let invite = GroupInvite::new(inviter_guid, inviter_name.clone(), group_id);
        self.pending_invites.insert(target_guid, invite);

        // Send SMSG_GROUP_INVITE to target
        use oxcore_shared::messages::group::SmsgGroupInvite;
        let msg = SmsgGroupInvite {
            inviter_name: &inviter_name,
        };
        self.broadcast_mgr.send_msg_to_player(target_guid, msg);

        tracing::info!("{} invited {} to group", inviter_name, target_name);

        Ok(())
    }

    /// Accept a group invite (CMSG_GROUP_ACCEPT)
    pub async fn accept_invite(
        &self,
        world: &World,
        player_guid: ObjectGuid,
    ) -> Result<(), GroupError> {
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

        // Join whatever group the inviter leads *now*. Two invites sent before either was
        // accepted both carry no group; the first accept creates it and the second must join it
        // rather than create another one led by the same player.
        let current_group = self
            .get_player_group_id(invite.inviter_guid)
            .filter(|id| self.groups.contains_key(id));
        let group_id = match current_group {
            Some(group_id) => {
                let group = self.get_group(group_id).ok_or(GroupError::NotInGroup)?;
                if !group.is_leader_or_assistant(invite.inviter_guid) {
                    return Err(GroupError::NotLeaderOrAssistant);
                }
                if group.is_full() {
                    use oxcore_shared::messages::group::SmsgPartyCommandResult;
                    self.broadcast_mgr.send_msg_to_player(
                        player_guid,
                        SmsgPartyCommandResult {
                            operation: PARTY_OP_INVITE,
                            member_name: "",
                            result: ERR_GROUP_FULL,
                        },
                    );
                    return Err(GroupError::GroupFull);
                }
                group_id
            }
            None => {
                // The inviter's group is gone (or never existed): start a new one.
                self.create_group(invite.inviter_guid, invite.inviter_name.clone())
                    .await?
            }
        };

        // Add player to group
        self.add_member_to_group(world, group_id, player_guid, player_name.clone())
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
            // "X declines your group invitation." goes to the inviting group's leader.
            use oxcore_shared::messages::group::SmsgGroupDecline;
            let player_name = self
                .player_mgr
                .get_player_name(player_guid)
                .unwrap_or_else(|| "Unknown".to_string());

            let leader = self
                .get_group(invite.group_id)
                .map(|g| g.leader_guid)
                .unwrap_or(invite.inviter_guid);
            self.broadcast_mgr.send_msg_to_player(
                leader,
                SmsgGroupDecline {
                    name: &player_name,
                },
            );

            tracing::info!(
                "{:?} declined invite from {:?}",
                player_guid,
                invite.inviter_guid
            );
        }

        Ok(())
    }

    /// Leave the current group (CMSG_GROUP_DISBAND; 1.14's CMSG_LEAVE_GROUP)
    pub async fn leave_group(
        &self,
        world: &World,
        player_guid: ObjectGuid,
    ) -> Result<(), GroupError> {
        let group_id = self
            .get_player_group_id(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        // Remove player from group
        self.remove_member_from_group(world, group_id, player_guid)
            .await?;

        // Check if group should be disbanded (less than 2 members)
        if let Some(group) = self.get_group(group_id) {
            if group.member_count() < 2 {
                self.disband_group(world, group_id, player_guid).await?;
            } else {
                // Broadcast updated group list
                self.broadcast_group_list(group_id);
            }
        }

        // The leaver's frames clear on an empty group list.
        self.send_empty_group_list(player_guid);

        Ok(())
    }

    /// Uninvite (kick) a player from the group
    pub async fn uninvite_player(
        &self,
        world: &World,
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
        self.remove_member_from_group(world, group_id, target_guid)
            .await?;

        // Check if group should be disbanded
        if let Some(group) = self.get_group(group_id) {
            if group.member_count() < 2 {
                self.disband_group(world, group_id, remover_guid).await?;
            } else {
                self.broadcast_group_list(group_id);
            }
        }

        // "You have been removed from the group", then clear their frames.
        use oxcore_shared::messages::group::SmsgGroupUninvite;
        self.broadcast_mgr
            .send_msg_to_player(target_guid, SmsgGroupUninvite);
        self.send_empty_group_list(target_guid);

        tracing::info!(
            "{:?} kicked {} from group {}",
            remover_guid,
            target_name,
            group_id
        );

        Ok(())
    }

    /// Uninvite by GUID (1.12 CMSG_GROUP_UNINVITE_GUID, 1.14 CMSG_PARTY_UNINVITE).
    ///
    /// A member is kicked as by name. A player who was only invited, not yet a member, has the
    /// invite withdrawn instead.
    pub async fn uninvite_by_guid(
        &self,
        world: &World,
        remover_guid: ObjectGuid,
        target_guid: ObjectGuid,
    ) -> Result<(), GroupError> {
        if target_guid == remover_guid {
            return Err(GroupError::CannotTargetSelf);
        }

        if let Some(name) = self
            .get_player_group(remover_guid)
            .and_then(|g| g.get_member(target_guid).map(|m| m.name.clone()))
        {
            return self.uninvite_player(world, remover_guid, name).await;
        }

        // Withdraw an outstanding invite sent by this player or their group.
        let group_id = self.get_player_group_id(remover_guid).unwrap_or(0);
        if group_id != 0 {
            let allowed = self
                .get_group(group_id)
                .is_some_and(|g| g.is_leader_or_assistant(remover_guid));
            if !allowed {
                return Err(GroupError::NotLeaderOrAssistant);
            }
        }
        let withdrawn = self
            .pending_invites
            .remove_if(&target_guid, |_, invite| {
                invite.inviter_guid == remover_guid || (group_id != 0 && invite.group_id == group_id)
            })
            .is_some();
        if !withdrawn {
            return Err(GroupError::MemberNotFound);
        }

        use oxcore_shared::messages::group::SmsgGroupUninvite;
        self.broadcast_mgr
            .send_msg_to_player(target_guid, SmsgGroupUninvite);
        Ok(())
    }

    /// Make every other member an assistant, or none (1.14 CMSG_SET_EVERYONE_IS_ASSISTANT).
    pub async fn set_everyone_assistant(
        &self,
        player_guid: ObjectGuid,
        apply: bool,
    ) -> Result<(), GroupError> {
        let group = self
            .get_player_group(player_guid)
            .ok_or(GroupError::NotInGroup)?;
        for member in &group.members {
            if member.guid == player_guid || member.assistant == apply {
                continue;
            }
            self.set_assistant(player_guid, member.guid, apply).await?;
        }
        Ok(())
    }

    /// Record a player's choice to pass on every roll (1.14 CMSG_OPT_OUT_OF_LOOT).
    pub fn set_passing_on_loot(&self, player_guid: ObjectGuid, passing: bool) {
        if passing {
            self.passing_on_loot.insert(player_guid);
        } else {
            self.passing_on_loot.remove(&player_guid);
        }
    }

    /// Add a member to an existing group
    async fn add_member_to_group(
        &self,
        world: &World,
        group_id: u32,
        player_guid: ObjectGuid,
        player_name: String,
    ) -> Result<(), GroupError> {
        // Update in-memory cache
        let is_leader = {
            let mut group = self
                .groups
                .get_mut(&group_id)
                .ok_or(GroupError::NotInGroup)?;
            group.add_member(player_guid, player_name)?;
            group.is_leader(player_guid)
        };

        // A joining member's temporary solo saves give way to the group's, except
        // for the dungeon they are standing in, which is released when they leave.
        if !is_leader {
            let current_map = world
                .managers
                .player_mgr
                .get_player(player_guid)
                .map(|p| p.map_id);
            let players_in_instance = |map_id: u32, instance_id: u32| {
                world
                    .managers
                    .map_mgr
                    .get_map(map_id, instance_id)
                    .map(|map| map.player_guids())
                    .unwrap_or_default()
            };
            let is_raid_map = |map_id: u32| {
                crate::game::instance::lifecycle::map_entry(world, map_id)
                    .is_some_and(|entry| entry.is_raid())
            };
            let outcome = world
                .managers
                .instance_mgr
                .reset_player_instances(
                    &world.databases,
                    player_guid,
                    current_map,
                    crate::game::instance::manager::ResetScope::GroupJoin,
                    is_raid_map,
                    players_in_instance,
                )
                .await;
            crate::game::instance::lifecycle::apply_reset_outcome(world, &outcome);
        }

        // Get subgroup for database
        let subgroup = self
            .get_group(group_id)
            .and_then(|g| g.get_member(player_guid).map(|m| m.subgroup as u16))
            .unwrap_or(0);

        // Add to player mappings
        self.player_groups.insert(player_guid, group_id);

        // Queue a full stat push for the new member (3c).
        self.set_member_update(player_guid, MEMBER_STATS_FULL);

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
        world: &World,
        group_id: u32,
        player_guid: ObjectGuid,
    ) -> Result<(), GroupError> {
        // Update in-memory cache
        let auto_promote = {
            let mut group = self
                .groups
                .get_mut(&group_id)
                .ok_or(GroupError::NotInGroup)?;

            let was_leader = group.is_leader(player_guid);
            group
                .remove_member(player_guid)
                .ok_or(GroupError::MemberNotFound)?;

            // If the leader left and the group survives, pick a replacement online member. A
            // group left with a single member is disbanded by the caller, so it gets no new
            // leader (and no leader announcement) first.
            if was_leader && group.member_count() > 1 {
                let player_mgr = Arc::clone(&self.player_mgr);
                group
                    .clone()
                    .select_new_leader(|guid| player_mgr.get_player(guid).is_some(), false)
            } else {
                None
            }
        };

        // Remove from player mappings
        self.player_groups.remove(&player_guid);

        // Their votes no longer count; a roll they were the last holdout on resolves now.
        self.remove_member_rolls(world, group_id, player_guid);

        // Remove from database
        self.repository
            .remove_member(group_id, player_guid.counter())
            .await
            .map_err(|e| GroupError::Internal(e.to_string()))?;

        // Announce and persist the promoted leader (single leader-change path, 2c).
        if let Some(new_leader) = auto_promote {
            self.apply_new_leader(world, group_id, new_leader, true)
                .await?;
        }

        Ok(())
    }

    // ========== LEADERSHIP ==========

    /// Set the group leader (CMSG_GROUP_SET_LEADER)
    pub async fn set_leader(
        &self,
        world: &World,
        player_guid: ObjectGuid,
        new_leader_guid: ObjectGuid,
    ) -> Result<(), GroupError> {
        let group_id = self
            .get_player_group_id(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        // Verify current player is leader
        let group = self.get_group(group_id).ok_or(GroupError::NotInGroup)?;

        if !group.is_leader(player_guid) {
            return Err(GroupError::NotLeader);
        }

        if !group.has_member(new_leader_guid) {
            return Err(GroupError::MemberNotFound);
        }
        if new_leader_guid == player_guid {
            return Err(GroupError::CannotTargetSelf);
        }
        // Leadership only goes to someone who is online to take it.
        if self.player_mgr.get_player(new_leader_guid).is_none() {
            return Err(GroupError::PlayerNotFound);
        }
        drop(group);

        self.apply_new_leader(world, group_id, new_leader_guid, true)
            .await
    }

    /// Apply a leader change: update the cache, transfer instance bindings (2d), persist the new
    /// leader, and optionally announce. Single path shared by `set_leader`, the auto-promote on
    /// member removal, and the offline-leader sweep (2c).
    async fn apply_new_leader(
        &self,
        world: &World,
        group_id: u32,
        new_leader: ObjectGuid,
        announce: bool,
    ) -> Result<(), GroupError> {
        let old_leader = {
            let group = self.groups.get(&group_id).ok_or(GroupError::NotInGroup)?;
            if !group.has_member(new_leader) {
                return Err(GroupError::MemberNotFound);
            }
            group.leader_guid
        };
        // The manager transfers bindings and their leader foreign key together.
        // Keep the live group on its old leader if that transaction fails.
        world
            .managers
            .instance_mgr
            .transfer_group_bindings(&world.databases, old_leader, new_leader)
            .await
            .map_err(|e| GroupError::Internal(e.to_string()))?;
        {
            let mut group = self
                .groups
                .get_mut(&group_id)
                .ok_or(GroupError::NotInGroup)?;

            group.promote_new_leader(new_leader)?;
            group.leader_last_online = now_secs();
        }
        self.set_player_flag(old_leader, PLAYER_FLAGS_GROUP_LEADER, false);
        self.set_player_flag(new_leader, PLAYER_FLAGS_GROUP_LEADER, true);

        if announce {
            let new_leader_name = self
                .player_mgr
                .get_player_name(new_leader)
                .unwrap_or_else(|| "Unknown".to_string());

            use oxcore_shared::messages::group::SmsgGroupSetLeader;
            let msg = SmsgGroupSetLeader {
                leader_name: &new_leader_name,
            };
            self.broadcast_to_group(group_id, msg);
        }

        self.broadcast_group_list(group_id);

        tracing::info!("Group {} leader changed to {:?}", group_id, new_leader);

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
        use oxcore_shared::messages::group::SmsgPartyCommandResult;
        let cmd_result = SmsgPartyCommandResult {
            operation: PARTY_OP_INVITE,
            member_name: "",
            result: ERR_PARTY_RESULT_OK,
        };
        self.broadcast_mgr
            .send_msg_to_player(player_guid, cmd_result);

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
        {
            let m1 = self
                .get_group(group_id)
                .and_then(|g| g.get_member(guid1).cloned())
                .map(|m| (m.assistant, subgroup2 as u16));
            let m2 = self
                .get_group(group_id)
                .and_then(|g| g.get_member(guid2).cloned())
                .map(|m| (m.assistant, subgroup1 as u16));

            if let Some((assistant, subgroup)) = m1 {
                self.repository
                    .update_member(group_id, guid1.counter(), assistant, subgroup)
                    .await
                    .map_err(|e| GroupError::Internal(e.to_string()))?;
            }
            if let Some((assistant, subgroup)) = m2 {
                self.repository
                    .update_member(group_id, guid2.counter(), assistant, subgroup)
                    .await
                    .map_err(|e| GroupError::Internal(e.to_string()))?;
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

            // The target loses any other icon it carried; each change is announced singly.
            let mut changes: Vec<(u8, ObjectGuid)> = Vec::new();
            if !target_guid.is_empty() {
                for (index, icon) in group.target_icons.iter().enumerate() {
                    if *icon == target_guid && index != icon_id as usize {
                        changes.push((index as u8, ObjectGuid::empty()));
                    }
                }
            }
            group.set_target_icon(icon_id, target_guid)?;
            changes.push((icon_id, target_guid));

            // Save to database
            let group_row = self.group_to_row(&group);
            let target_icons = group.target_icons;
            let members = group.get_member_guids();
            drop(group);
            self.repository
                .save_group(&group_row)
                .await
                .map_err(|e| GroupError::Internal(e.to_string()))?;

            use oxcore_shared::messages::group::MsgRaidTargetUpdate;
            for changed in changes {
                let msg = MsgRaidTargetUpdate {
                    mode: 0,
                    target_icons,
                    changed: Some(changed),
                    changed_by: player_guid,
                };
                self.broadcast_mgr.broadcast_msg_to_players(&members, &msg);
            }
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

        use oxcore_shared::messages::group::MsgRaidTargetUpdate;
        let msg = MsgRaidTargetUpdate {
            mode: 1, // full icon list
            target_icons: group.target_icons,
            changed: None,
            changed_by: ObjectGuid::empty(),
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
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
        use oxcore_shared::messages::group::MsgRaidReadyCheck;
        let Some(group) = self.get_group(group_id) else {
            return Ok(());
        };
        let msg = MsgRaidReadyCheck {
            player_guid,
            ready: None, // None = initiate
        };
        let members = group.get_member_guids();
        self.broadcast_mgr.broadcast_msg_to_players(&members, &msg);

        // The initiator counts as ready; 1.14 frames show that straight away.
        let mut answered = std::collections::HashSet::new();
        answered.insert(player_guid);
        self.ready_checks.insert(
            group_id,
            ReadyCheck {
                answered,
                remaining_ms: READY_CHECK_DURATION_MS,
            },
        );
        let initiator_ready = MsgRaidReadyCheck {
            player_guid,
            ready: Some(true),
        };
        let modern = self.modern_members(&members);
        self.broadcast_mgr
            .broadcast_msg_to_players(&modern, &initiator_ready);

        tracing::info!("Ready check initiated by {:?}", player_guid);

        Ok(())
    }

    /// Respond to ready check (MSG_RAID_READY_CHECK with state)
    ///
    /// 1.12 shows answers on the leader's frame only; 1.14 shows every answer to every member,
    /// and closes the frame on SMSG_READY_CHECK_COMPLETED once all online members answered.
    pub async fn respond_ready_check(
        &self,
        player_guid: ObjectGuid,
        ready: bool,
    ) -> Result<(), GroupError> {
        let group = self
            .get_player_group(player_guid)
            .ok_or(GroupError::NotInGroup)?;

        use oxcore_shared::messages::group::MsgRaidReadyCheck;
        let msg = MsgRaidReadyCheck {
            player_guid,
            ready: Some(ready),
        };
        let members = group.get_member_guids();
        let mut recipients = self.modern_members(&members);
        if !recipients.contains(&group.leader_guid) {
            recipients.push(group.leader_guid);
        }
        self.broadcast_mgr.broadcast_msg_to_players(&recipients, &msg);

        let all_answered = self
            .ready_checks
            .get_mut(&group.id)
            .map(|mut check| {
                check.answered.insert(player_guid);
                members
                    .iter()
                    .filter(|m| self.player_mgr.get_player(**m).is_some())
                    .all(|m| check.answered.contains(m))
            })
            .unwrap_or(false);
        if all_answered {
            self.finish_ready_check(group.id);
        }

        tracing::debug!("{:?} responded to ready check: {}", player_guid, ready);

        Ok(())
    }

    /// Close a group's ready check: 1.14 members get SMSG_READY_CHECK_COMPLETED.
    fn finish_ready_check(&self, group_id: u32) {
        if self.ready_checks.remove(&group_id).is_none() {
            return;
        }
        let Some(group) = self.get_group(group_id) else {
            return;
        };
        let modern = self.modern_members(&group.get_member_guids());
        use oxcore_shared::messages::group::SmsgReadyCheckCompleted;
        self.broadcast_mgr
            .broadcast_msg_to_players(&modern, &SmsgReadyCheckCompleted);
    }

    /// Time out ready checks nobody finished answering.
    fn update_ready_checks(&self, diff_ms: u64) {
        let expired: Vec<u32> = self
            .ready_checks
            .iter_mut()
            .filter_map(|mut entry| {
                entry.remaining_ms = entry.remaining_ms.saturating_sub(diff_ms);
                (entry.remaining_ms == 0).then_some(*entry.key())
            })
            .collect();
        for group_id in expired {
            self.finish_ready_check(group_id);
        }
    }

    /// The subset of `guids` connected with a 1.14 client.
    pub(super) fn modern_members(&self, guids: &[ObjectGuid]) -> Vec<ObjectGuid> {
        use oxcore_shared::protocol::Protocol;
        guids
            .iter()
            .copied()
            .filter(|guid| {
                self.player_mgr
                    .get_broadcaster(*guid)
                    .is_some_and(|b| b.protocol() == Protocol::Modern)
            })
            .collect()
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
        use oxcore_shared::messages::group::MsgMinimapPing;
        let msg = MsgMinimapPing { player_guid, x, y };

        for member in &group.members {
            if member.guid != player_guid {
                self.broadcast_mgr.send_msg_to_player(member.guid, msg);

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
        use oxcore_shared::messages::group::MsgRandomRoll;
        let msg = MsgRandomRoll {
            min,
            max,
            roll,
            player_guid,
        };
        match self.get_player_group(player_guid) {
            Some(group) => {
                for member in &group.members {
                    self.broadcast_mgr.send_msg_to_player(member.guid, msg);
                }
            }
            None => self.broadcast_mgr.send_msg_to_player(player_guid, msg),
        }

        tracing::debug!("{:?} rolled {} ({}-{})", player_guid, roll, min, max);

        Ok(())
    }

    // ========== MEMBER STATS ==========

    /// Request party member stats (CMSG_REQUEST_PARTY_MEMBER_STATS)
    pub async fn request_member_stats(
        &self,
        requester_guid: ObjectGuid,
        target_guid: ObjectGuid,
    ) -> Result<(), GroupError> {
        // Only a member of the requester's group, online, gets real stats; anyone else is
        // answered as offline so the client's frame stops waiting.
        let same_group = self
            .get_player_group(requester_guid)
            .is_some_and(|group| group.has_member(target_guid));
        let stats = if same_group {
            self.build_party_member_stats(target_guid)
        } else {
            None
        };

        let msg = stats.unwrap_or_else(|| {
            let mut offline = Self::empty_party_member_stats(target_guid);
            offline.update_mask = group_update_flags::STATUS;
            offline.status = Some(MemberStatus::OFFLINE.as_u8());
            offline
        });
        self.broadcast_mgr.send_msg_to_player(requester_guid, msg);

        Ok(())
    }

    // ========== BROADCASTING ==========

    /// Broadcast SMSG_GROUP_LIST to all group members
    pub(super) fn broadcast_group_list(&self, group_id: u32) {
        let Some(group) = self.get_group(group_id) else {
            return;
        };

        // Log what we're about to send
        tracing::debug!(
            "Broadcasting group {} with {} members",
            group_id,
            group.members.len()
        );

        // Convert to CachedGroup for packet building, recomputing each member's status live.
        use oxcore_shared::game::group::CachedGroup;
        let cached_group =
            CachedGroup::from_group_data_with_status(&group, |guid| self.member_status(guid));

        // Send SMSG_GROUP_LIST to each member (each needs different own_flags)
        use oxcore_shared::messages::group::SmsgGroupList;
        for member in &group.members {
            let msg = SmsgGroupList {
                group: &cached_group,
                member_guid: member.guid,
            };
            self.broadcast_mgr.send_msg_to_player(member.guid, msg);

            tracing::debug!("Sent GROUP_LIST to {:?}", member.guid);
        }

        // Send MSG_RAID_TARGET_UPDATE after group list
        // Mode 1 = full icon list - only includes non-empty icons
        use oxcore_shared::messages::group::MsgRaidTargetUpdate;
        let raid_target_msg = MsgRaidTargetUpdate {
            mode: 1, // Full icon list
            target_icons: group.target_icons,
            changed: None,
            changed_by: ObjectGuid::empty(),
        };
        for member in &group.members {
            self.broadcast_mgr
                .send_msg_to_player(member.guid, raid_target_msg.clone());
        }
    }

    /// Broadcast a message to all group members
    fn broadcast_to_group<M>(&self, group_id: u32, msg: M)
    where
        M: oxcore_shared::messages::ToWorldPacket + Clone + Send,
    {
        let Some(group) = self.get_group(group_id) else {
            return;
        };

        for member in &group.members {
            self.broadcast_mgr
                .send_msg_to_player(member.guid, msg.clone());
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

    // ========== LIVE MEMBER STATUS ==========

    /// Live member status for `guid` (reference `GetGroupMemberStatus`, Group.cpp:45).
    ///
    /// Offline when there is no live player. Omitted by caller request: the PvP and FFA-PvP
    /// bits (`MEMBER_STATUS_PVP`, `MEMBER_STATUS_PVP_FFA`) depend on a PvP-flag port that does
    /// not yet exist; the remaining bits are computed from the live player state.
    fn member_status(&self, guid: ObjectGuid) -> MemberStatus {
        let Some(player) = self.player_mgr.get_player(guid) else {
            return MemberStatus::OFFLINE;
        };

        let mut status = MemberStatus::ONLINE;
        if !matches!(player.death.death_state, DeathState::Alive) {
            status = status.with_flag(MemberStatus::DEAD);
        }
        if player.player_flags & PLAYER_FLAGS_GHOST != 0 {
            status = status.with_flag(MemberStatus::GHOST);
        }
        if player.player_flags & PLAYER_FLAGS_AFK != 0 {
            status = status.with_flag(MemberStatus::AFK);
        }
        if player.player_flags & PLAYER_FLAGS_DND != 0 {
            status = status.with_flag(MemberStatus::DND);
        }
        status
    }

    /// The "not in a group" list for a player who left, was kicked, or whose group dissolved.
    fn send_empty_group_list(&self, player_guid: ObjectGuid) {
        use oxcore_shared::messages::group::SmsgGroupList;
        let empty = CachedGroup::empty();
        self.broadcast_mgr.send_msg_to_player(
            player_guid,
            SmsgGroupList {
                group: &empty,
                member_guid: player_guid,
            },
        );
    }

    /// Set or clear a `PLAYER_FLAGS` bit on an online player and show it to everyone nearby
    /// (leader crown, AFK/DND tags). A no-op for an offline player or an unchanged flag.
    pub fn set_player_flag(&self, player_guid: ObjectGuid, flag: u32, on: bool) {
        let flags = {
            let Some(mut player) = self.player_mgr.get_player_mut(player_guid) else {
                return;
            };
            let flags = if on {
                player.player_flags | flag
            } else {
                player.player_flags & !flag
            };
            if flags == player.player_flags {
                return;
            }
            player.player_flags = flags;
            flags
        };
        self.broadcast_player_flags(player_guid, flags);
    }

    /// Push a player's current `PLAYER_FLAGS` to them and everyone who can see them.
    pub fn broadcast_player_flags(&self, player_guid: ObjectGuid, flags: u32) {
        use crate::game::common::update_fields::PLAYER_FLAGS;
        use oxcore_shared::messages::update::{
            ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
        };
        let world_guid = crate::core::common::guid::ObjectGuid::from_raw(player_guid.raw());
        let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(world_guid, ObjectType::Player).set_field(PLAYER_FLAGS, flags),
        ));
        self.broadcast_mgr.broadcast_msg_nearby(player_guid, &update, true);
    }

    /// Queue an SMSG_PARTY_MEMBER_STATS push for a grouped member.
    ///
    /// `flags` is a mask of [`group_update_flags`]; the system drains it on the next tick.
    pub fn set_member_update(&self, player_guid: ObjectGuid, flags: u32) {
        use oxcore_shared::game::group::group_update_flags;
        if flags == group_update_flags::NONE {
            return;
        }
        if !self.is_in_group(player_guid) {
            return;
        }
        self.member_update_flags
            .entry(player_guid)
            .and_modify(|f| *f |= flags)
            .or_insert(flags);
    }

    /// An SMSG_PARTY_MEMBER_STATS for `guid` carrying no fields.
    fn empty_party_member_stats(guid: ObjectGuid) -> SmsgPartyMemberStats<'static> {
        SmsgPartyMemberStats {
            player_guid: guid,
            update_mask: group_update_flags::NONE,
            status: None,
            health: None,
            max_health: None,
            power_type: None,
            cur_power: None,
            max_power: None,
            level: None,
            zone_id: None,
            position_x: None,
            position_y: None,
            auras: None,
            negative_auras: None,
            pet_guid: None,
            pet_name: None,
            pet_model_id: None,
            pet_cur_hp: None,
            pet_max_hp: None,
            pet_power_type: None,
            pet_cur_power: None,
            pet_max_power: None,
            pet_auras: None,
            pet_negative_auras: None,
        }
    }

    /// Build a full non-pet SMSG_PARTY_MEMBER_STATS message for a live player.
    fn build_party_member_stats(&self, guid: ObjectGuid) -> Option<SmsgPartyMemberStats<'static>> {
        use oxcore_shared::game::group::group_update_flags::*;

        let player = self.player_mgr.get_player(guid)?;
        let power = player.power.power_type.as_u8() as usize;

        let status = self.member_status(guid);
        Some(SmsgPartyMemberStats {
            player_guid: guid,
            update_mask: STATUS
                | CUR_HP
                | MAX_HP
                | POWER_TYPE
                | CUR_POWER
                | MAX_POWER
                | LEVEL
                | ZONE,
            status: Some(status.as_u8()),
            health: Some(player.stats.health),
            max_health: Some(player.stats.max_health),
            power_type: Some(player.power.power_type.as_u8()),
            cur_power: Some(player.power.current[power]),
            max_power: Some(player.power.max[power]),
            level: Some(player.level),
            zone_id: Some(player.zone_id),
            position_x: None,
            position_y: None,
            auras: None,
            negative_auras: None,
            pet_guid: None,
            pet_name: None,
            pet_model_id: None,
            pet_cur_hp: None,
            pet_max_hp: None,
            pet_power_type: None,
            pet_cur_power: None,
            pet_max_power: None,
            pet_auras: None,
            pet_negative_auras: None,
        })
    }

    /// Current party-frame values for a live player.
    fn member_snapshot(&self, guid: ObjectGuid) -> Option<MemberSnapshot> {
        let status = self.member_status(guid).as_u8();
        let player = self.player_mgr.get_player(guid)?;
        let power = player.power.power_type.as_u8() as usize;
        Some(MemberSnapshot {
            status,
            health: player.stats.health,
            max_health: player.stats.max_health,
            power_type: player.power.power_type.as_u8(),
            power: player.power.current[power],
            max_power: player.power.max[power],
            level: player.level,
            zone: player.zone_id,
        })
    }

    /// Diff every online grouped player's party-frame values against the last snapshot and queue
    /// an update for the fields that changed (health, power, level, zone, death and AFK status).
    fn queue_changed_member_stats(&self) {
        let grouped: Vec<ObjectGuid> = self.player_groups.iter().map(|e| *e.key()).collect();
        for guid in grouped {
            let Some(current) = self.member_snapshot(guid) else {
                self.member_snapshots.remove(&guid);
                continue;
            };
            let flags = match self.member_snapshots.insert(guid, current) {
                Some(previous) => current.changed_flags(&previous),
                None => group_update_flags::NONE,
            };
            self.set_member_update(guid, flags);
        }
    }

    /// Drain queued member stat updates, sending each to group members who cannot see the
    /// target (reference `UpdatePlayerOutOfRange`, Group.cpp:1423).
    fn push_party_member_stats(&self) {
        let dirty: Vec<ObjectGuid> = self
            .member_update_flags
            .iter()
            .filter_map(|entry| (*entry.value() != 0).then_some(*entry.key()))
            .collect();
        if dirty.is_empty() {
            return;
        }

        for guid in dirty {
            self.member_update_flags.remove(&guid);
            let Some(group) = self.get_player_group(guid) else {
                continue;
            };
            let Some(msg) = self.build_party_member_stats(guid) else {
                continue;
            };
            for member in &group.members {
                if member.guid == guid {
                    continue;
                }
                // Members who can already see the player get the values through the normal
                // object-update path.
                let visible = self
                    .player_mgr
                    .get_player(member.guid)
                    .map(|p| p.visibility.visible_objects.contains(&guid))
                    .unwrap_or(false);
                if !visible {
                    self.broadcast_mgr
                        .send_msg_to_player(member.guid, msg.clone());
                }
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

    pub fn update(&self, diff: std::time::Duration, world: &World) -> Result<()> {
        // Phase 2g: reassign leadership from leaders who have been offline too long.
        if let Some(self_arc) = self.self_arc.get().cloned() {
            self.check_offline_leaders(world, self_arc);
        }
        // Queue stat pushes for party-frame values that changed since the last diff, then push
        // queued updates to out-of-range group members.
        let elapsed = self
            .snapshot_elapsed_ms
            .fetch_add(diff.as_millis() as u64, Ordering::Relaxed)
            + diff.as_millis() as u64;
        if elapsed >= MEMBER_SNAPSHOT_INTERVAL_MS {
            self.snapshot_elapsed_ms.store(0, Ordering::Relaxed);
            self.queue_changed_member_stats();
        }
        self.push_party_member_stats();
        // Phase 5: advance loot-roll timers; resolve any that expired.
        self.update_roll_timers(diff, world);
        self.update_ready_checks(diff.as_millis() as u64);
        Ok(())
    }

    /// Sweep groups whose leader has been offline longer than `group_offline_leader_delay`
    /// and reassign leadership to an online member. DB writes and the instance-binding
    /// transfer run in a spawned task because they cannot be awaited from the sync tick.
    fn check_offline_leaders(&self, world: &World, self_arc: Arc<GroupSystem>) {
        let delay = world.config.group_offline_leader_delay;
        if delay == 0 {
            return;
        }

        let now = now_secs();
        // A last-online stamp of 0 means "not seen since startup", which is as overdue as it
        // gets. Groups with nobody online to promote are skipped rather than retried as tasks.
        let overdue: Vec<u32> = self
            .groups
            .iter()
            .filter(|entry| {
                let group = entry.value();
                group.leader_last_online.saturating_add(delay) < now
                    && !self.leader_reassignments.contains(&group.id)
                    && world
                        .managers
                        .player_mgr
                        .get_player(group.leader_guid)
                        .is_none()
                    && group.members.iter().any(|m| {
                        m.guid != group.leader_guid
                            && world.managers.player_mgr.get_player(m.guid).is_some()
                    })
            })
            .map(|entry| entry.id)
            .collect();

        for group_id in overdue {
            // One reassignment per group at a time: the binding transfer awaits the database,
            // and the tick must not queue another while it runs.
            if !self.leader_reassignments.insert(group_id) {
                continue;
            }
            let world = world.clone();
            let self_arc = Arc::clone(&self_arc);
            tokio::spawn(async move {
                if let Err(e) = Self::reassign_offline_leader(&self_arc, &world, group_id).await {
                    tracing::warn!("Failed to reassign leader for group {}: {:?}", group_id, e);
                }
                self_arc.leader_reassignments.remove(&group_id);
            });
        }
    }

    /// Pick an online replacement for a group whose leader has been offline too long and apply
    /// the change. Uses the `offline` variant of `select_new_leader`, so when nobody online is
    /// available the leadership is left with the absentee.
    async fn reassign_offline_leader(
        self_arc: &Arc<GroupSystem>,
        world: &World,
        group_id: u32,
    ) -> Result<(), GroupError> {
        let new_leader = {
            let mut group = self_arc
                .groups
                .get_mut(&group_id)
                .ok_or(GroupError::NotInGroup)?;
            if group.leader_guid.is_empty() || group.member_count() == 0 {
                return Err(GroupError::NotInGroup);
            }
            let player_mgr = Arc::clone(&self_arc.player_mgr);
            match group
                .clone()
                .select_new_leader(|guid| player_mgr.get_player(guid).is_some(), true)
            {
                Some(guid) => guid,
                None => return Ok(()),
            }
        };

        self_arc
            .apply_new_leader(world, group_id, new_leader, true)
            .await
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

        // The leader is back online: clear the offline-leader timer (2g).
        let mut leads = false;
        if let Some(group_id) = self.get_player_group_id(guid) {
            if let Some(mut group) = self.groups.get_mut(&group_id) {
                if group.is_leader(guid) {
                    group.leader_last_online = 0;
                    leads = true;
                }
            }
        }
        // The flag may be stale from a session where leadership changed while offline.
        self.set_player_flag(guid, PLAYER_FLAGS_GROUP_LEADER, leads);

        // Send group list to player on login if in group
        if let Some(group_id) = self.get_player_group_id(guid) {
            tracing::info!(
                "[GROUP] Player {:?} logged in, in group {}, broadcasting group list",
                guid,
                group_id
            );
            // Broadcast updated group list to all members
            self.broadcast_group_list(group_id);

            // Queue a full stat push for the just-logged-in member (3c).
            self.set_member_update(guid, MEMBER_STATS_FULL);
        } else {
            tracing::debug!("[GROUP] Player {:?} logged in, not in any group", guid);
        }

        Ok(())
    }

    pub async fn on_player_logout(&self, guid: ObjectGuid) -> Result<()> {
        // Clear pending invites to this player, and the ones they sent
        self.pending_invites.remove(&guid);
        self.pending_invites
            .retain(|_, invite| invite.inviter_guid != guid);
        self.passing_on_loot.remove(&guid);

        // Get group_id before updating status
        let group_id = self.get_player_group_id(guid);

        // Update member status to offline
        self.update_member_offline(guid);

        // Stamp the leader's last-online time so the offline-leader sweep can fire (2g).
        if let Some(group_id) = group_id {
            if let Some(mut group) = self.groups.get_mut(&group_id) {
                if group.is_leader(guid) {
                    group.leader_last_online = now_secs();
                }
            }
        }

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

    #[cfg(test)]
    pub fn add_group_test(&self, group: GroupData) {
        self.groups.insert(group.id, group);
    }

    #[cfg(test)]
    pub fn add_player_to_group_test(&self, player_guid: ObjectGuid, group_id: u32) {
        self.player_groups.insert(player_guid, group_id);
    }
}
