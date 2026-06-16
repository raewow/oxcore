//! Group system types, constants, and structures

use crate::protocol::ObjectGuid;
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
    #[error("Player not found")]
    PlayerNotFound,
    #[error("Target not found")]
    TargetNotFound,
    #[error("Player already in group")]
    PlayerAlreadyInGroup,
    #[error("Target already in group")]
    TargetAlreadyInGroup,
    #[error("Target has pending invite")]
    TargetHasPendingInvite,
    #[error("Group is full")]
    GroupFull,
    #[error("Not in group")]
    NotInGroup,
    #[error("Not leader")]
    NotLeader,
    #[error("Not leader or assistant")]
    NotLeaderOrAssistant,
    #[error("Not a raid")]
    NotRaid,
    #[error("Invalid subgroup")]
    InvalidSubgroup,
    #[error("Wrong faction")]
    WrongFaction,
    #[error("Target ignores player")]
    TargetIgnoresPlayer,
    #[error("Member not found")]
    MemberNotFound,
    #[error("Cannot target self")]
    CannotTargetSelf,
    #[error("Internal error: {0}")]
    Internal(String),
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
    pub fn new(id: u32, leader_guid: ObjectGuid, leader_name: String) -> Self {
        let leader = GroupMember::new(leader_guid, leader_name.clone());

        let mut group = Self {
            id,
            leader_guid,
            leader_name,
            members: vec![leader],
            is_raid: false,
            loot_method: LootMethod::default(),
            loot_threshold: 2,
            looter_guid: leader_guid,
            main_tank_guid: ObjectGuid::empty(),
            main_assistant_guid: ObjectGuid::empty(),
            target_icons: [ObjectGuid::empty(); TARGET_ICON_COUNT],
            subgroup_counts: [0; MAX_RAID_SUBGROUPS as usize],
        };

        group.subgroup_counts[0] = 1;
        group
    }

    pub fn is_full(&self) -> bool {
        if self.is_raid {
            self.members.len() >= MAX_RAID_SIZE
        } else {
            self.members.len() >= MAX_GROUP_SIZE
        }
    }

    pub fn max_size(&self) -> usize {
        if self.is_raid {
            MAX_RAID_SIZE
        } else {
            MAX_GROUP_SIZE
        }
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    pub fn has_member(&self, guid: ObjectGuid) -> bool {
        self.members.iter().any(|m| m.guid == guid)
    }

    pub fn get_member(&self, guid: ObjectGuid) -> Option<&GroupMember> {
        self.members.iter().find(|m| m.guid == guid)
    }

    pub fn get_member_mut(&mut self, guid: ObjectGuid) -> Option<&mut GroupMember> {
        self.members.iter_mut().find(|m| m.guid == guid)
    }

    pub fn get_member_by_name(&self, name: &str) -> Option<&GroupMember> {
        let name_lower = name.to_lowercase();
        self.members
            .iter()
            .find(|m| m.name.to_lowercase() == name_lower)
    }

    pub fn is_leader(&self, guid: ObjectGuid) -> bool {
        self.leader_guid == guid
    }

    pub fn is_assistant(&self, guid: ObjectGuid) -> bool {
        self.get_member(guid).map(|m| m.assistant).unwrap_or(false)
    }

    pub fn is_leader_or_assistant(&self, guid: ObjectGuid) -> bool {
        self.is_leader(guid) || self.is_assistant(guid)
    }

    pub fn find_available_subgroup(&self) -> u8 {
        if !self.is_raid {
            return 0;
        }
        for (i, &count) in self.subgroup_counts.iter().enumerate() {
            if count < 5 {
                return i as u8;
            }
        }
        0
    }

    pub fn add_member(&mut self, guid: ObjectGuid, name: String) -> Result<(), GroupError> {
        if self.is_full() {
            return Err(GroupError::GroupFull);
        }
        if self.has_member(guid) {
            return Err(GroupError::PlayerAlreadyInGroup);
        }
        let subgroup = self.find_available_subgroup();
        let member = GroupMember::new(guid, name).with_subgroup(subgroup);
        if (subgroup as usize) < self.subgroup_counts.len() {
            self.subgroup_counts[subgroup as usize] += 1;
        }
        self.members.push(member);
        Ok(())
    }

    pub fn remove_member(&mut self, guid: ObjectGuid) -> Option<GroupMember> {
        if let Some(pos) = self.members.iter().position(|m| m.guid == guid) {
            let member = self.members.remove(pos);
            if (member.subgroup as usize) < self.subgroup_counts.len() {
                self.subgroup_counts[member.subgroup as usize] =
                    self.subgroup_counts[member.subgroup as usize].saturating_sub(1);
            }
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

    pub fn convert_to_raid(&mut self) {
        if !self.is_raid {
            self.is_raid = true;
        }
    }

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
        if self.subgroup_counts[new_subgroup as usize] >= 5 {
            return Err(GroupError::GroupFull);
        }
        if (old_subgroup as usize) < self.subgroup_counts.len() {
            self.subgroup_counts[old_subgroup as usize] =
                self.subgroup_counts[old_subgroup as usize].saturating_sub(1);
        }
        self.subgroup_counts[new_subgroup as usize] += 1;
        if let Some(member) = self.get_member_mut(guid) {
            member.subgroup = new_subgroup;
        }
        Ok(())
    }

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
        if let Some(member1) = self.get_member_mut(guid1) {
            member1.subgroup = subgroup2;
        }
        if let Some(member2) = self.get_member_mut(guid2) {
            member2.subgroup = subgroup1;
        }
        Ok(())
    }

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

    pub fn set_main_tank(&mut self, guid: ObjectGuid) -> Result<(), GroupError> {
        if !self.is_raid {
            return Err(GroupError::NotRaid);
        }
        if !guid.is_empty() && !self.has_member(guid) {
            return Err(GroupError::MemberNotFound);
        }
        if self.main_assistant_guid == guid && !guid.is_empty() {
            self.main_assistant_guid = ObjectGuid::empty();
        }
        self.main_tank_guid = guid;
        Ok(())
    }

    pub fn set_main_assistant(&mut self, guid: ObjectGuid) -> Result<(), GroupError> {
        if !self.is_raid {
            return Err(GroupError::NotRaid);
        }
        if !guid.is_empty() && !self.has_member(guid) {
            return Err(GroupError::MemberNotFound);
        }
        if self.main_tank_guid == guid && !guid.is_empty() {
            self.main_tank_guid = ObjectGuid::empty();
        }
        self.main_assistant_guid = guid;
        Ok(())
    }

    pub fn set_target_icon(
        &mut self,
        icon_id: u8,
        target_guid: ObjectGuid,
    ) -> Result<(), GroupError> {
        if icon_id as usize >= TARGET_ICON_COUNT {
            return Err(GroupError::Internal("Invalid icon ID".into()));
        }
        for icon in &mut self.target_icons {
            if *icon == target_guid && !target_guid.is_empty() {
                *icon = ObjectGuid::empty();
            }
        }
        self.target_icons[icon_id as usize] = target_guid;
        Ok(())
    }

    pub fn get_member_guids(&self) -> Vec<ObjectGuid> {
        self.members.iter().map(|m| m.guid).collect()
    }

    pub fn get_online_member_guids(&self) -> Vec<ObjectGuid> {
        self.members
            .iter()
            .filter(|m| m.status.is_online())
            .map(|m| m.guid)
            .collect()
    }

    pub fn set_member_status(&mut self, guid: ObjectGuid, status: MemberStatus) {
        if let Some(member) = self.get_member_mut(guid) {
            member.status = status;
            member.last_online = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
    }

    pub fn promote_new_leader(&mut self, new_leader_guid: ObjectGuid) -> Result<(), GroupError> {
        let (guid, name) = {
            let new_leader = self
                .get_member(new_leader_guid)
                .ok_or(GroupError::MemberNotFound)?;
            (new_leader.guid, new_leader.name.clone())
        };
        self.leader_guid = guid;
        self.leader_name = name;
        if let Some(member) = self.get_member_mut(new_leader_guid) {
            member.assistant = false;
        }
        Ok(())
    }

    pub fn select_new_leader(&mut self) -> Option<ObjectGuid> {
        let new_leader_guid = self
            .members
            .iter()
            .filter(|m| m.guid != self.leader_guid && m.status.is_online())
            .map(|m| m.guid)
            .next()
            .or_else(|| {
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

/// Cached snapshot of group data for packet building
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
    pub target_icons: [ObjectGuid; TARGET_ICON_COUNT],
    pub members: Vec<GroupMember>,
    pub subgroup_counts: [u8; MAX_RAID_SUBGROUPS as usize],
}

impl CachedGroup {
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

    pub fn get_member(&self, guid: ObjectGuid) -> Option<&GroupMember> {
        self.members.iter().find(|m| m.guid == guid)
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
