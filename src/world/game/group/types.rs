//! Group system types, constants, and structures

use crate::shared::protocol::ObjectGuid;
use std::time::Instant;

// ========== CONSTANTS ==========

/// Maximum group size (normal party)
pub const MAX_GROUP_SIZE: usize = 5;
/// Maximum raid size
pub const MAX_RAID_SIZE: usize = 40;
/// Maximum raid subgroups
pub const MAX_RAID_SUBGROUPS: u8 = 8;
/// Number of raid target icons
pub const TARGET_ICON_COUNT: usize = 8;

// Party operation codes (for SMSG_PARTY_COMMAND_RESULT)
pub const PARTY_OP_INVITE: u32 = 0;
pub const PARTY_OP_LEAVE: u32 = 2;

// Error codes (for SMSG_PARTY_COMMAND_RESULT)
pub const ERR_PARTY_RESULT_OK: u32 = 0;
pub const ERR_BAD_PLAYER_NAME_S: u32 = 1;
pub const ERR_TARGET_NOT_IN_GROUP_S: u32 = 2;
pub const ERR_GROUP_FULL: u32 = 3;
pub const ERR_ALREADY_IN_GROUP_S: u32 = 4;
pub const ERR_PLAYER_WRONG_FACTION: u32 = 5;
pub const ERR_IGNORING_YOU_S: u32 = 6;
pub const ERR_NOT_LEADER: u32 = 7;

// ========== ENUMS ==========

/// Loot method types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LootMethod {
    FreeForAll = 0,
    RoundRobin = 1,
    MasterLooter = 2,
    #[default]
    GroupLoot = 3,
    NeedBeforeGreed = 4,
}

impl From<u8> for LootMethod {
    fn from(value: u8) -> Self {
        match value {
            0 => LootMethod::FreeForAll,
            1 => LootMethod::RoundRobin,
            2 => LootMethod::MasterLooter,
            3 => LootMethod::GroupLoot,
            4 => LootMethod::NeedBeforeGreed,
            _ => LootMethod::GroupLoot,
        }
    }
}

impl From<LootMethod> for u8 {
    fn from(value: LootMethod) -> Self {
        value as u8
    }
}

/// Group member status flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MemberStatus(u16);

impl MemberStatus {
    pub const OFFLINE: MemberStatus = MemberStatus(0x0000);
    pub const ONLINE: MemberStatus = MemberStatus(0x0001);
    pub const PVP: MemberStatus = MemberStatus(0x0002);
    pub const DEAD: MemberStatus = MemberStatus(0x0004);
    pub const GHOST: MemberStatus = MemberStatus(0x0008);
    pub const PVP_FFA: MemberStatus = MemberStatus(0x0010);
    pub const AFK: MemberStatus = MemberStatus(0x0040);
    pub const DND: MemberStatus = MemberStatus(0x0080);

    pub fn new() -> Self {
        Self::ONLINE
    }

    pub fn offline() -> Self {
        Self::OFFLINE
    }

    pub fn with_flag(mut self, flag: MemberStatus) -> Self {
        self.0 |= flag.0;
        self
    }

    pub fn without_flag(mut self, flag: MemberStatus) -> Self {
        self.0 &= !flag.0;
        self
    }

    pub fn has_flag(&self, flag: MemberStatus) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn is_online(&self) -> bool {
        self.has_flag(Self::ONLINE)
    }

    pub fn as_u16(&self) -> u16 {
        self.0
    }

    pub fn as_u8(&self) -> u8 {
        self.0 as u8
    }
}

/// Group operation errors
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GroupError {
    /// Player not found or not online
    #[error("Player not found")]
    PlayerNotFound,
    /// Target player not found or not online
    #[error("Target not found")]
    TargetNotFound,
    /// Player already in a group
    #[error("Player already in group")]
    PlayerAlreadyInGroup,
    /// Target already in a group
    #[error("Target already in group")]
    TargetAlreadyInGroup,
    /// Target has a pending invite
    #[error("Target has pending invite")]
    TargetHasPendingInvite,
    /// Group is full
    #[error("Group is full")]
    GroupFull,
    /// Player not in a group
    #[error("Not in group")]
    NotInGroup,
    /// Player is not the group leader
    #[error("Not leader")]
    NotLeader,
    /// Player is not the leader or an assistant
    #[error("Not leader or assistant")]
    NotLeaderOrAssistant,
    /// Operation requires raid group
    #[error("Not a raid")]
    NotRaid,
    /// Invalid subgroup number
    #[error("Invalid subgroup")]
    InvalidSubgroup,
    /// Cross-faction grouping not allowed
    #[error("Wrong faction")]
    WrongFaction,
    /// Target is ignoring the player
    #[error("Target ignores player")]
    TargetIgnoresPlayer,
    /// Player not found in group
    #[error("Member not found")]
    MemberNotFound,
    /// Cannot perform action on self
    #[error("Cannot target self")]
    CannotTargetSelf,
    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Party operation codes (for SMSG_PARTY_COMMAND_RESULT)
pub mod party_op {
    pub const INVITE: u32 = 0;
    pub const LEAVE: u32 = 2;
}

/// Party command result codes (for SMSG_PARTY_COMMAND_RESULT)
pub mod result_codes {
    pub const OK: u32 = 0;
    pub const BAD_PLAYER_NAME: u32 = 1;
    pub const TARGET_NOT_IN_GROUP: u32 = 2;
    pub const GROUP_FULL: u32 = 3;
    pub const ALREADY_IN_GROUP: u32 = 4;
    pub const NOT_IN_GROUP: u32 = 5;
    pub const NOT_LEADER: u32 = 6;
    pub const WRONG_FACTION: u32 = 7;
    pub const IGNORING_YOU: u32 = 8;
}

// ========== DATA STRUCTURES ==========

/// Group member information
#[derive(Debug, Clone)]
pub struct GroupMember {
    pub guid: ObjectGuid,
    pub name: String,
    pub subgroup: u8,
    pub assistant: bool,
    pub status: MemberStatus,
    pub last_online: u64,
}

impl GroupMember {
    pub fn new(guid: ObjectGuid, name: String) -> Self {
        Self {
            guid,
            name,
            subgroup: 0,
            assistant: false,
            status: MemberStatus::new(),
            last_online: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    pub fn with_subgroup(mut self, subgroup: u8) -> Self {
        self.subgroup = subgroup;
        self
    }
}

/// Pending group invite
#[derive(Debug, Clone)]
pub struct GroupInvite {
    pub inviter_guid: ObjectGuid,
    pub inviter_name: String,
    /// Group ID (0 if inviter not in a group yet - will be created on accept)
    pub group_id: u32,
    pub timestamp: Instant,
}

impl GroupInvite {
    pub fn new(inviter_guid: ObjectGuid, inviter_name: String, group_id: u32) -> Self {
        Self {
            inviter_guid,
            inviter_name,
            group_id,
            timestamp: Instant::now(),
        }
    }
}

/// Complete group data (stored in system cache)
#[derive(Debug, Clone)]
pub struct GroupData {
    pub id: u32,
    pub leader_guid: ObjectGuid,
    pub leader_name: String,
    pub members: Vec<GroupMember>,
    pub is_raid: bool,
    pub loot_method: LootMethod,
    pub loot_threshold: u8,
    pub looter_guid: ObjectGuid,
    pub main_tank_guid: ObjectGuid,
    pub main_assistant_guid: ObjectGuid,
    pub target_icons: [ObjectGuid; TARGET_ICON_COUNT],
    pub subgroup_counts: [u8; MAX_RAID_SUBGROUPS as usize],
}

impl GroupData {
    /// Create a new group with the given leader
    pub fn new(id: u32, leader_guid: ObjectGuid, leader_name: String) -> Self {
        let leader = GroupMember::new(leader_guid, leader_name.clone());

        let mut group = Self {
            id,
            leader_guid,
            leader_name,
            members: vec![leader],
            is_raid: false,
            loot_method: LootMethod::default(),
            loot_threshold: 2, // Uncommon quality
            looter_guid: leader_guid,
            main_tank_guid: ObjectGuid::empty(),
            main_assistant_guid: ObjectGuid::empty(),
            target_icons: [ObjectGuid::empty(); TARGET_ICON_COUNT],
            subgroup_counts: [0; MAX_RAID_SUBGROUPS as usize],
        };

        // Count leader in subgroup 0
        group.subgroup_counts[0] = 1;

        group
    }

    /// Check if group is full
    pub fn is_full(&self) -> bool {
        if self.is_raid {
            self.members.len() >= MAX_RAID_SIZE
        } else {
            self.members.len() >= MAX_GROUP_SIZE
        }
    }

    /// Get maximum group size
    pub fn max_size(&self) -> usize {
        if self.is_raid {
            MAX_RAID_SIZE
        } else {
            MAX_GROUP_SIZE
        }
    }

    /// Get member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Check if a player is in the group
    pub fn has_member(&self, guid: ObjectGuid) -> bool {
        self.members.iter().any(|m| m.guid == guid)
    }

    /// Get member by GUID
    pub fn get_member(&self, guid: ObjectGuid) -> Option<&GroupMember> {
        self.members.iter().find(|m| m.guid == guid)
    }

    /// Get mutable member by GUID
    pub fn get_member_mut(&mut self, guid: ObjectGuid) -> Option<&mut GroupMember> {
        self.members.iter_mut().find(|m| m.guid == guid)
    }

    /// Get member by name (case-insensitive)
    pub fn get_member_by_name(&self, name: &str) -> Option<&GroupMember> {
        let name_lower = name.to_lowercase();
        self.members
            .iter()
            .find(|m| m.name.to_lowercase() == name_lower)
    }

    /// Check if member is the leader
    pub fn is_leader(&self, guid: ObjectGuid) -> bool {
        self.leader_guid == guid
    }

    /// Check if member is an assistant
    pub fn is_assistant(&self, guid: ObjectGuid) -> bool {
        self.get_member(guid).map(|m| m.assistant).unwrap_or(false)
    }

    /// Check if member is leader or assistant
    pub fn is_leader_or_assistant(&self, guid: ObjectGuid) -> bool {
        self.is_leader(guid) || self.is_assistant(guid)
    }

    /// Find available subgroup with space
    pub fn find_available_subgroup(&self) -> u8 {
        if !self.is_raid {
            return 0;
        }

        // Find subgroup with least members that isn't full (5 per subgroup)
        for (i, &count) in self.subgroup_counts.iter().enumerate() {
            if count < 5 {
                return i as u8;
            }
        }

        0 // Default to subgroup 0 if all full
    }

    /// Add a member to the group
    pub fn add_member(&mut self, guid: ObjectGuid, name: String) -> Result<(), GroupError> {
        if self.is_full() {
            return Err(GroupError::GroupFull);
        }

        if self.has_member(guid) {
            return Err(GroupError::PlayerAlreadyInGroup);
        }

        let subgroup = self.find_available_subgroup();
        let member = GroupMember::new(guid, name).with_subgroup(subgroup);

        // Update subgroup count
        if (subgroup as usize) < self.subgroup_counts.len() {
            self.subgroup_counts[subgroup as usize] += 1;
        }

        self.members.push(member);
        Ok(())
    }

    /// Remove a member from the group
    pub fn remove_member(&mut self, guid: ObjectGuid) -> Option<GroupMember> {
        if let Some(pos) = self.members.iter().position(|m| m.guid == guid) {
            let member = self.members.remove(pos);

            // Update subgroup count
            if (member.subgroup as usize) < self.subgroup_counts.len() {
                self.subgroup_counts[member.subgroup as usize] =
                    self.subgroup_counts[member.subgroup as usize].saturating_sub(1);
            }

            // Clear main tank/assistant if removed
            if self.main_tank_guid == guid {
                self.main_tank_guid = ObjectGuid::empty();
            }
            if self.main_assistant_guid == guid {
                self.main_assistant_guid = ObjectGuid::empty();
            }

            Some(member)
        } else {
            None
        }
    }

    /// Convert party to raid
    pub fn convert_to_raid(&mut self) {
        if self.is_raid {
            return;
        }
        self.is_raid = true;
        // Subgroup counts already maintained
    }

    /// Change a member's subgroup
    pub fn change_subgroup(
        &mut self,
        guid: ObjectGuid,
        new_subgroup: u8,
    ) -> Result<(), GroupError> {
        if new_subgroup >= MAX_RAID_SUBGROUPS {
            return Err(GroupError::InvalidSubgroup);
        }

        if !self.is_raid {
            return Err(GroupError::NotRaid);
        }

        let old_subgroup = self
            .get_member(guid)
            .map(|m| m.subgroup)
            .ok_or(GroupError::MemberNotFound)?;

        // Check new subgroup has space
        if self.subgroup_counts[new_subgroup as usize] >= 5 {
            return Err(GroupError::GroupFull);
        }

        // Update subgroup counts
        if (old_subgroup as usize) < self.subgroup_counts.len() {
            self.subgroup_counts[old_subgroup as usize] =
                self.subgroup_counts[old_subgroup as usize].saturating_sub(1);
        }
        self.subgroup_counts[new_subgroup as usize] += 1;

        // Update member
        if let Some(member) = self.get_member_mut(guid) {
            member.subgroup = new_subgroup;
        }

        Ok(())
    }

    /// Swap two members' subgroups
    pub fn swap_subgroups(
        &mut self,
        guid1: ObjectGuid,
        guid2: ObjectGuid,
    ) -> Result<(), GroupError> {
        if !self.is_raid {
            return Err(GroupError::NotRaid);
        }

        let subgroup1 = self
            .get_member(guid1)
            .map(|m| m.subgroup)
            .ok_or(GroupError::MemberNotFound)?;
        let subgroup2 = self
            .get_member(guid2)
            .map(|m| m.subgroup)
            .ok_or(GroupError::MemberNotFound)?;

        // Swap subgroups (no count changes needed since it's a swap)
        if let Some(member1) = self.get_member_mut(guid1) {
            member1.subgroup = subgroup2;
        }
        if let Some(member2) = self.get_member_mut(guid2) {
            member2.subgroup = subgroup1;
        }

        Ok(())
    }

    /// Set assistant flag for a member
    pub fn set_assistant(&mut self, guid: ObjectGuid, assistant: bool) -> Result<(), GroupError> {
        if !self.is_raid {
            return Err(GroupError::NotRaid);
        }

        if let Some(member) = self.get_member_mut(guid) {
            member.assistant = assistant;
            Ok(())
        } else {
            Err(GroupError::MemberNotFound)
        }
    }

    /// Set main tank (raid only)
    pub fn set_main_tank(&mut self, guid: ObjectGuid) -> Result<(), GroupError> {
        if !self.is_raid {
            return Err(GroupError::NotRaid);
        }
        if !guid.is_empty() && !self.has_member(guid) {
            return Err(GroupError::MemberNotFound);
        }
        // If setting as main tank, clear main assistant if same player
        if self.main_assistant_guid == guid && !guid.is_empty() {
            self.main_assistant_guid = ObjectGuid::empty();
        }
        self.main_tank_guid = guid;
        Ok(())
    }

    /// Set main assistant (raid only)
    pub fn set_main_assistant(&mut self, guid: ObjectGuid) -> Result<(), GroupError> {
        if !self.is_raid {
            return Err(GroupError::NotRaid);
        }
        if !guid.is_empty() && !self.has_member(guid) {
            return Err(GroupError::MemberNotFound);
        }
        // If setting as main assistant, clear main tank if same player
        if self.main_tank_guid == guid && !guid.is_empty() {
            self.main_tank_guid = ObjectGuid::empty();
        }
        self.main_assistant_guid = guid;
        Ok(())
    }

    /// Set raid target icon
    pub fn set_target_icon(
        &mut self,
        icon_id: u8,
        target_guid: ObjectGuid,
    ) -> Result<(), GroupError> {
        if icon_id as usize >= TARGET_ICON_COUNT {
            return Err(GroupError::Internal("Invalid icon ID".into()));
        }

        // Clear this target from any other icon first
        for icon in &mut self.target_icons {
            if *icon == target_guid && !target_guid.is_empty() {
                *icon = ObjectGuid::empty();
            }
        }

        self.target_icons[icon_id as usize] = target_guid;
        Ok(())
    }

    /// Get all member GUIDs
    pub fn get_member_guids(&self) -> Vec<ObjectGuid> {
        self.members.iter().map(|m| m.guid).collect()
    }

    /// Get online member GUIDs
    pub fn get_online_member_guids(&self) -> Vec<ObjectGuid> {
        self.members
            .iter()
            .filter(|m| m.status.is_online())
            .map(|m| m.guid)
            .collect()
    }

    /// Update member status
    pub fn set_member_status(&mut self, guid: ObjectGuid, status: MemberStatus) {
        if let Some(member) = self.get_member_mut(guid) {
            member.status = status;
            member.last_online = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
    }

    /// Promote a new leader (called when leader leaves/changes)
    pub fn promote_new_leader(&mut self, new_leader_guid: ObjectGuid) -> Result<(), GroupError> {
        // Copy values first to avoid borrow issues
        let (guid, name) = {
            let new_leader = self
                .get_member(new_leader_guid)
                .ok_or(GroupError::MemberNotFound)?;
            (new_leader.guid, new_leader.name.clone())
        };

        self.leader_guid = guid;
        self.leader_name = name;

        // Remove assistant flag from new leader if they had it
        if let Some(member) = self.get_member_mut(new_leader_guid) {
            member.assistant = false;
        }

        Ok(())
    }

    /// Select a new leader automatically (first online member, or first member)
    pub fn select_new_leader(&mut self) -> Option<ObjectGuid> {
        // Prefer online members
        let new_leader_guid = self
            .members
            .iter()
            .filter(|m| m.guid != self.leader_guid && m.status.is_online())
            .map(|m| m.guid)
            .next()
            .or_else(|| {
                // Fall back to any member
                self.members
                    .iter()
                    .filter(|m| m.guid != self.leader_guid)
                    .map(|m| m.guid)
                    .next()
            });

        if let Some(guid) = new_leader_guid {
            let _ = self.promote_new_leader(guid);
        }

        new_leader_guid
    }
}

/// Group update flags for SMSG_PARTY_MEMBER_STATS
pub mod group_update_flags {
    pub const NONE: u32 = 0x00000000;
    pub const STATUS: u32 = 0x00000001;
    pub const CUR_HP: u32 = 0x00000002;
    pub const MAX_HP: u32 = 0x00000004;
    pub const POWER_TYPE: u32 = 0x00000008;
    pub const CUR_POWER: u32 = 0x00000010;
    pub const MAX_POWER: u32 = 0x00000020;
    pub const LEVEL: u32 = 0x00000040;
    pub const ZONE: u32 = 0x00000080;
    pub const POSITION: u32 = 0x00000100;
    pub const AURAS: u32 = 0x00000200;
    pub const AURAS_NEGATIVE: u32 = 0x00000400;
    pub const PET_GUID: u32 = 0x00000800;
    pub const PET_NAME: u32 = 0x00001000;
    pub const PET_MODEL_ID: u32 = 0x00002000;
    pub const PET_CUR_HP: u32 = 0x00004000;
    pub const PET_MAX_HP: u32 = 0x00008000;
    pub const PET_POWER_TYPE: u32 = 0x00010000;
    pub const PET_CUR_POWER: u32 = 0x00020000;
    pub const PET_MAX_POWER: u32 = 0x00040000;
    pub const PET_AURAS: u32 = 0x00080000;
    pub const PET_AURAS_NEGATIVE: u32 = 0x00100000;

    pub const PET: u32 = 0x001FF800;
    pub const FULL: u32 = 0x001FFFFF;
}
