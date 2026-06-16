//! Group system message structs
//!
//! This module contains type-safe message structures for all group-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgGroupInvite`] - Send group invitation to player
//! - [`SmsgGroupList`] - Send complete group roster information
//! - [`SmsgGroupSetLeader`] - Notify group members of leader change
//! - [`SmsgGroupDestroyed`] - Notify player that group was disbanded
//! - [`SmsgGroupUninvite`] - Notify player they were kicked from group
//! - [`SmsgPartyCommandResult`] - Result of party/group operations
//! - [`SmsgPartyMemberStats`] - Delta updates for group member stats
//! - [`SmsgLootRollStarted`] - Start a loot roll for an item
//! - [`SmsgLootRoll`] - Player's roll result
//! - [`SmsgLootRollWon`] - Winner of a loot roll
//! - [`SmsgLootAllPassed`] - All players passed on an item
//!
//! ## Bidirectional Messages (MSG)
//! - [`MsgRaidTargetUpdate`] - Update/request raid target icons
//! - [`MsgRaidReadyCheck`] - Initiate or respond to ready check
//! - [`MsgMinimapPing`] - Send minimap ping to group
//! - [`MsgRandomRoll`] - Broadcast random roll result

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};
use crate::world::game::common::group_update_flags;
use crate::world::game::group::{CachedGroup, GroupMember, LootMethod, MemberStatus};

/// SMSG_LOOT_START_ROLL - Start a loot roll for an item
///
/// Sent when an item in loot is rolled on by group members.
/// Client displays a roll UI to eligible players.
#[derive(Debug, Clone)]
pub struct SmsgLootRollStarted {
    pub loot_guid: ObjectGuid,
    pub item_slot: u32,
    pub item_id: u32,
    pub item_random_prop_id: i32,
    pub item_suffix_factor: u32,
    pub item_count: u8,
    pub roll_timeout: u32,
    pub roll_type: u8,
}

impl ToWorldPacket for SmsgLootRollStarted {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOOT_START_ROLL);
        packet.write_u64(self.loot_guid.raw());
        packet.write_u32(self.item_slot);
        packet.write_u32(self.item_id);
        packet.write_u32(self.item_random_prop_id as u32);
        packet.write_u32(self.item_suffix_factor);
        packet.write_u8(self.item_count);
        packet.write_u32(self.roll_timeout);
        packet.write_u8(self.roll_type);
        packet
    }
}

/// SMSG_LOOT_ROLL - Player's roll result
///
/// Sent when a player rolls on an item (need, greed, or pass).
/// Broadcast to all group members.
#[derive(Debug, Clone)]
pub struct SmsgLootRoll {
    pub player_guid: ObjectGuid,
    pub item_slot: u32,
    pub roll_number: u8,
    pub roll_type: u8,
}

impl ToWorldPacket for SmsgLootRoll {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOOT_ROLL);
        packet.write_u64(self.player_guid.raw());
        packet.write_u32(self.item_slot);
        packet.write_u8(self.roll_number);
        packet.write_u8(self.roll_type);
        packet
    }
}

/// SMSG_LOOT_ROLL_WON - Winner of a loot roll
///
/// Sent when the roll period ends and a winner is determined.
/// Broadcast to all group members.
#[derive(Debug, Clone)]
pub struct SmsgLootRollWon {
    pub player_guid: ObjectGuid,
    pub item_slot: u32,
    pub roll_number: u8,
    pub roll_type: u8,
}

impl ToWorldPacket for SmsgLootRollWon {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOOT_ROLL_WON);
        packet.write_u64(self.player_guid.raw());
        packet.write_u32(self.item_slot);
        packet.write_u8(self.roll_number);
        packet.write_u8(self.roll_type);
        packet
    }
}

/// SMSG_LOOT_ALL_PASSED - All players passed on an item
///
/// Sent when all eligible players passed on an item roll.
/// Broadcast to all group members.
#[derive(Debug, Clone)]
pub struct SmsgLootAllPassed {
    pub loot_guid: ObjectGuid,
    pub item_slot: u32,
}

impl ToWorldPacket for SmsgLootAllPassed {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOOT_ALL_PASSED);
        packet.write_u64(self.loot_guid.raw());
        packet.write_u32(self.item_slot);
        packet
    }
}

/// SMSG_GROUP_INVITE - Send group invitation to player
///
/// Sent when a player invites another player to a group.
/// Client displays a popup with the invitation.
#[derive(Debug, Clone)]
pub struct SmsgGroupInvite<'a> {
    /// Name of the player who sent the invitation
    pub inviter_name: &'a str,
}

impl ToWorldPacket for SmsgGroupInvite<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GROUP_INVITE);
        packet.write_string(self.inviter_name);
        packet
    }
}

/// SMSG_GROUP_LIST - Send complete group roster information
///
/// Sent when a player opens their group/raid UI or when group composition changes.
/// Contains all group members with their roles, subgroups, and loot settings.
#[derive(Debug, Clone)]
pub struct SmsgGroupList<'a> {
    /// Reference to the group being listed
    pub group: &'a CachedGroup,
    /// GUID of the player receiving this list (affects own_flags calculation)
    pub member_guid: ObjectGuid,
}

impl ToWorldPacket for SmsgGroupList<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GROUP_LIST);

        // Group type (0 = normal, 1 = raid)
        packet.write_u8(if self.group.is_raid { 1 } else { 0 });

        // Own flags (subgroup | (assistant ? 0x80 : 0))
        let own_flags = {
            let member = self.group.get_member(self.member_guid);
            if let Some(m) = member {
                let mut flags = m.subgroup;
                if m.assistant {
                    flags |= 0x80;
                }
                flags
            } else {
                0
            }
        };
        packet.write_u8(own_flags);

        // Member count (exclude recipient - client adds itself)
        let member_count = self
            .group
            .members
            .iter()
            .filter(|m| m.guid != self.member_guid)
            .count();
        packet.write_u32(member_count as u32);

        // Member list (exclude recipient - client adds itself)
        for member in &self.group.members {
            if member.guid == self.member_guid {
                continue;
            }
            packet.write_string(&member.name);
            packet.write_u64(member.guid.raw());
            packet.write_u8(member.status.as_u16() as u8);
            let mut flags = member.subgroup;
            if member.assistant {
                flags |= 0x80;
            }
            packet.write_u8(flags);
        }

        // Leader GUID
        packet.write_u64(self.group.leader_guid.raw());

        // Loot settings - ALWAYS sent (client expects these even for empty member lists)
        packet.write_u8(self.group.loot_method as u8);
        // Looter GUID: only send actual GUID for master loot, else 0
        if self.group.loot_method == LootMethod::MasterLooter {
            packet.write_u64(self.group.looter_guid.raw());
        } else {
            packet.write_u64(0);
        }
        packet.write_u8(self.group.loot_threshold);

        // Dungeon difficulty (Client 1.10.2+) - only when there are other members
        if member_count > 0 {
            packet.write_u8(0);
        }

        packet
    }
}

/// SMSG_GROUP_SET_LEADER - Notify group members of leader change
///
/// Sent when a new leader is assigned to the group.
/// Client updates the group UI to show the new leader.
#[derive(Debug, Clone)]
pub struct SmsgGroupSetLeader<'a> {
    /// Name of the new group leader
    pub leader_name: &'a str,
}

impl ToWorldPacket for SmsgGroupSetLeader<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GROUP_SET_LEADER);
        packet.write_string(self.leader_name);
        packet
    }
}

/// SMSG_PARTY_COMMAND_RESULT - Result of party/group operations
///
/// Sent in response to group operations like invite, promote, demote, etc.
/// Indicates success or failure of the operation.
#[derive(Debug, Clone)]
pub struct SmsgPartyCommandResult<'a> {
    /// Operation type (GUILD_INVITE_S, PARTY_OP_LEAVE, etc.)
    pub operation: u32,
    /// Name of the target member (or empty string)
    pub member_name: &'a str,
    /// Result code (0 = success, other values = error codes)
    pub result: u32,
}

impl ToWorldPacket for SmsgPartyCommandResult<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_PARTY_COMMAND_RESULT);
        packet.write_u32(self.operation);
        packet.write_string(self.member_name);
        packet.write_u32(self.result);
        packet
    }
}

/// SMSG_PARTY_MEMBER_STATS - Delta updates for group member stats
///
/// This packet sends delta updates for group member stats (health, power, auras, etc.).
/// Only includes fields that have changed since the last update.
/// Uses a bitmask to indicate which fields are included.
///
/// # Status byte flags
/// - 0x01: ONLINE
/// - 0x40: AFK
/// - 0x80: DND
#[derive(Debug, Clone)]
pub struct SmsgPartyMemberStats<'a> {
    /// GUID of the player whose stats are being updated
    pub player_guid: ObjectGuid,
    /// Bitmask indicating which fields are included (from group_update_flags)
    pub update_mask: u32,
    /// Status flags (online/afk/dnd) - only if STATUS flag set
    pub status: Option<u8>,
    /// Current health - only if CUR_HP flag set
    pub health: Option<u32>,
    /// Maximum health - only if MAX_HP flag set
    pub max_health: Option<u32>,
    /// Power type (mana, rage, etc.) - only if POWER_TYPE flag set
    pub power_type: Option<u8>,
    /// Current power - only if CUR_POWER flag set
    pub cur_power: Option<u32>,
    /// Maximum power - only if MAX_POWER flag set
    pub max_power: Option<u32>,
    /// Player level - only if LEVEL flag set
    pub level: Option<u8>,
    /// Zone ID - only if ZONE flag set
    pub zone_id: Option<u32>,
    /// X position - only if POSITION flag set
    pub position_x: Option<f32>,
    /// Y position - only if POSITION flag set
    pub position_y: Option<f32>,
    /// Positive auras (spell IDs) - only if AURAS flag set
    pub auras: Option<&'a [u32]>,
    /// Negative auras (spell IDs) - only if AURAS_NEGATIVE flag set
    pub negative_auras: Option<&'a [u32]>,
    /// Pet GUID - only if PET_GUID flag set
    pub pet_guid: Option<ObjectGuid>,
    /// Pet name - only if PET_NAME flag set
    pub pet_name: Option<&'a str>,
    /// Pet model ID - only if PET_MODEL_ID flag set
    pub pet_model_id: Option<u16>,
    /// Pet current health - only if PET_CUR_HP flag set
    pub pet_cur_hp: Option<u16>,
    /// Pet maximum health - only if PET_MAX_HP flag set
    pub pet_max_hp: Option<u16>,
    /// Pet power type - only if PET_POWER_TYPE flag set
    pub pet_power_type: Option<u8>,
    /// Pet current power - only if PET_CUR_POWER flag set
    pub pet_cur_power: Option<u16>,
    /// Pet maximum power - only if PET_MAX_POWER flag set
    pub pet_max_power: Option<u16>,
    /// Pet positive auras - only if PET_AURAS flag set
    pub pet_auras: Option<&'a [u32]>,
    /// Pet negative auras - only if PET_AURAS_NEGATIVE flag set
    pub pet_negative_auras: Option<&'a [u32]>,
}

impl ToWorldPacket for SmsgPartyMemberStats<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        use group_update_flags::*;

        let mut packet = WorldPacket::new(Opcode::SMSG_PARTY_MEMBER_STATS);

        // Write packed GUID (Client 1.8.4+)
        packet.write_packed_guid_raw(self.player_guid.raw());

        // Write update mask
        packet.write_u32(self.update_mask);

        // Calculate byte count for variable-length fields
        let mut byte_count = 0;

        // Status flags (uint8)
        if (self.update_mask & STATUS) != 0 {
            byte_count += 1;
        }

        // HP (uint16)
        if (self.update_mask & CUR_HP) != 0 {
            byte_count += 2;
        }
        if (self.update_mask & MAX_HP) != 0 {
            byte_count += 2;
        }

        // Power type (uint8)
        if (self.update_mask & POWER_TYPE) != 0 {
            byte_count += 1;
        }

        // Power (uint16)
        if (self.update_mask & CUR_POWER) != 0 {
            byte_count += 2;
        }
        if (self.update_mask & MAX_POWER) != 0 {
            byte_count += 2;
        }

        // Level (uint16)
        if (self.update_mask & LEVEL) != 0 {
            byte_count += 2;
        }

        // Zone (uint16)
        if (self.update_mask & ZONE) != 0 {
            byte_count += 2;
        }

        // Position (uint16, uint16)
        if (self.update_mask & POSITION) != 0 {
            byte_count += 4;
        }

        // Auras (uint32 mask + spellids)
        if (self.update_mask & AURAS) != 0 {
            byte_count += 4; // mask
            if let Some(auras) = self.auras {
                byte_count += auras.len() * 2; // spell IDs
            }
        }

        // Negative auras (uint16 mask + spellids)
        if (self.update_mask & AURAS_NEGATIVE) != 0 {
            byte_count += 2; // mask
            if let Some(negative_auras) = self.negative_auras {
                byte_count += negative_auras.len() * 2; // spell IDs
            }
        }

        // Pet GUID (uint64)
        if (self.update_mask & PET_GUID) != 0 {
            byte_count += 8;
        }

        // Pet name (string)
        if (self.update_mask & PET_NAME) != 0 {
            byte_count += 1; // null terminator
            if let Some(name) = self.pet_name {
                byte_count += name.len();
            }
        }

        // Pet model ID (uint16)
        if (self.update_mask & PET_MODEL_ID) != 0 {
            byte_count += 2;
        }

        // Pet HP (uint16)
        if (self.update_mask & PET_CUR_HP) != 0 {
            byte_count += 2;
        }
        if (self.update_mask & PET_MAX_HP) != 0 {
            byte_count += 2;
        }

        // Pet power type (uint8)
        if (self.update_mask & PET_POWER_TYPE) != 0 {
            byte_count += 1;
        }

        // Pet power (uint16)
        if (self.update_mask & PET_CUR_POWER) != 0 {
            byte_count += 2;
        }
        if (self.update_mask & PET_MAX_POWER) != 0 {
            byte_count += 2;
        }

        // Pet auras (uint32 mask + spellids)
        if (self.update_mask & PET_AURAS) != 0 {
            byte_count += 4; // mask
            if let Some(pet_auras) = self.pet_auras {
                byte_count += pet_auras.len() * 2; // spell IDs
            }
        }

        // Pet negative auras (uint16 mask + spellids)
        if (self.update_mask & PET_AURAS_NEGATIVE) != 0 {
            byte_count += 2; // mask
            if let Some(pet_negative_auras) = self.pet_negative_auras {
                byte_count += pet_negative_auras.len() * 2; // spell IDs
            }
        }

        // Write byte count
        packet.write_u8(byte_count as u8);

        // Write fields based on mask
        if (self.update_mask & STATUS) != 0 {
            // Status flags: 0x01=ONLINE, 0x40=AFK, 0x80=DND
            packet.write_u8(self.status.unwrap_or(0));
        }

        if (self.update_mask & CUR_HP) != 0 {
            packet.write_u16(self.health.unwrap_or(0).min(65535) as u16);
        }

        if (self.update_mask & MAX_HP) != 0 {
            packet.write_u16(self.max_health.unwrap_or(0).min(65535) as u16);
        }

        if (self.update_mask & POWER_TYPE) != 0 {
            packet.write_u8(self.power_type.unwrap_or(0));
        }

        if (self.update_mask & CUR_POWER) != 0 {
            packet.write_u16(self.cur_power.unwrap_or(0).min(65535) as u16);
        }

        if (self.update_mask & MAX_POWER) != 0 {
            packet.write_u16(self.max_power.unwrap_or(0).min(65535) as u16);
        }

        if (self.update_mask & LEVEL) != 0 {
            packet.write_u16(self.level.unwrap_or(0) as u16);
        }

        if (self.update_mask & ZONE) != 0 {
            packet.write_u16(self.zone_id.unwrap_or(0).min(65535) as u16);
        }

        if (self.update_mask & POSITION) != 0 {
            // Convert float position to uint16 (0-65535 maps to world coordinates)
            // For simplicity, we'll use a simple conversion
            // TODO: Use proper coordinate conversion
            let x = ((self.position_x.unwrap_or(0.0) + 17066.0) / 0.5) as u16;
            let y = ((self.position_y.unwrap_or(0.0) + 17066.0) / 0.5) as u16;
            packet.write_u16(x);
            packet.write_u16(y);
        }

        if (self.update_mask & AURAS) != 0 {
            // Write aura mask (32 bits, one per aura slot)
            let auras = self.auras.unwrap_or(&[]);
            let mask = if auras.len() > 0 {
                (1u32 << auras.len().min(32)) - 1
            } else {
                0
            };
            packet.write_u32(mask);
            for &spell_id in auras.iter().take(32) {
                packet.write_u16(spell_id.min(65535) as u16);
            }
        }

        if (self.update_mask & AURAS_NEGATIVE) != 0 {
            // Write negative aura mask (16 bits)
            let negative_auras = self.negative_auras.unwrap_or(&[]);
            let mask = if negative_auras.len() > 0 {
                (1u16 << negative_auras.len().min(16)) - 1
            } else {
                0
            };
            packet.write_u16(mask);
            for &spell_id in negative_auras.iter().take(16) {
                packet.write_u16(spell_id.min(65535) as u16);
            }
        }

        if (self.update_mask & PET_GUID) != 0 {
            if let Some(guid) = self.pet_guid {
                packet.write_u64(guid.raw());
            } else {
                packet.write_u64(0);
            }
        }

        if (self.update_mask & PET_NAME) != 0 {
            if let Some(name) = self.pet_name {
                packet.write_string(name);
            } else {
                packet.write_string("");
            }
        }

        if (self.update_mask & PET_MODEL_ID) != 0 {
            packet.write_u16(self.pet_model_id.unwrap_or(0));
        }

        if (self.update_mask & PET_CUR_HP) != 0 {
            packet.write_u16(self.pet_cur_hp.unwrap_or(0));
        }

        if (self.update_mask & PET_MAX_HP) != 0 {
            packet.write_u16(self.pet_max_hp.unwrap_or(0));
        }

        if (self.update_mask & PET_POWER_TYPE) != 0 {
            packet.write_u8(self.pet_power_type.unwrap_or(0));
        }

        if (self.update_mask & PET_CUR_POWER) != 0 {
            packet.write_u16(self.pet_cur_power.unwrap_or(0));
        }

        if (self.update_mask & PET_MAX_POWER) != 0 {
            packet.write_u16(self.pet_max_power.unwrap_or(0));
        }

        if (self.update_mask & PET_AURAS) != 0 {
            let pet_auras = self.pet_auras.unwrap_or(&[]);
            let mask = if pet_auras.len() > 0 {
                (1u32 << pet_auras.len().min(32)) - 1
            } else {
                0
            };
            packet.write_u32(mask);
            for &spell_id in pet_auras.iter().take(32) {
                packet.write_u16(spell_id.min(65535) as u16);
            }
        }

        if (self.update_mask & PET_AURAS_NEGATIVE) != 0 {
            let pet_negative_auras = self.pet_negative_auras.unwrap_or(&[]);
            let mask = if pet_negative_auras.len() > 0 {
                (1u16 << pet_negative_auras.len().min(16)) - 1
            } else {
                0
            };
            packet.write_u16(mask);
            for &spell_id in pet_negative_auras.iter().take(16) {
                packet.write_u16(spell_id.min(65535) as u16);
            }
        }

        packet
    }
}

/// SMSG_GROUP_DESTROYED - Notify player that group was disbanded
///
/// Sent when a group is disbanded (empty packet).
#[derive(Debug, Clone, Copy)]
pub struct SmsgGroupDestroyed;

impl ToWorldPacket for SmsgGroupDestroyed {
    fn to_world_packet(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_GROUP_DESTROYED)
    }
}

/// SMSG_GROUP_UNINVITE - Notify player they were kicked from group
///
/// Sent when a player is removed from a group by the leader (empty packet).
#[derive(Debug, Clone, Copy)]
pub struct SmsgGroupUninvite;

impl ToWorldPacket for SmsgGroupUninvite {
    fn to_world_packet(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_GROUP_UNINVITE)
    }
}

/// MSG_RAID_TARGET_UPDATE - Update/request raid target icons
///
/// Bidirectional message for setting or querying raid target icons.
/// Mode 1 = full icon list, Mode 0 = delta update (single icon change)
#[derive(Debug, Clone)]
pub struct MsgRaidTargetUpdate {
    /// Mode: 0 = delta update (single icon), 1 = full icon list
    pub mode: u8,
    /// Target icons (8 icons) - only non-empty icons are sent
    pub target_icons: [ObjectGuid; 8],
}

impl ToWorldPacket for MsgRaidTargetUpdate {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_RAID_TARGET_UPDATE);
        packet.write_u8(self.mode);

        // Only write icons that are set (non-empty GUIDs)
        // Format per icon: icon_index (u8) + target_guid (u64)
        for (index, &icon_target) in self.target_icons.iter().enumerate() {
            if !icon_target.is_empty() {
                packet.write_u8(index as u8);
                packet.write_u64(icon_target.raw());
            }
        }
        packet
    }
}

/// MSG_RAID_READY_CHECK - Initiate or respond to ready check
///
/// Initiator sends player GUID, responder sends GUID + ready state.
#[derive(Debug, Clone)]
pub struct MsgRaidReadyCheck {
    /// Player GUID (initiator or responder)
    pub player_guid: ObjectGuid,
    /// Ready state (Some(true/false) for response, None for initiate)
    pub ready: Option<bool>,
}

impl ToWorldPacket for MsgRaidReadyCheck {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_RAID_READY_CHECK);
        packet.write_u64(self.player_guid.raw());
        if let Some(ready) = self.ready {
            packet.write_u8(if ready { 1 } else { 0 });
        }
        packet
    }
}

/// MSG_MINIMAP_PING - Send minimap ping to group
///
/// Broadcast when a player pings their minimap.
#[derive(Debug, Clone, Copy)]
pub struct MsgMinimapPing {
    /// Player who sent the ping
    pub player_guid: ObjectGuid,
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
}

impl ToWorldPacket for MsgMinimapPing {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_MINIMAP_PING);
        packet.write_u64(self.player_guid.raw());
        packet.write_f32(self.x);
        packet.write_f32(self.y);
        packet
    }
}

/// MSG_RANDOM_ROLL - Broadcast random roll result
///
/// Sent when a player performs a /random roll.
#[derive(Debug, Clone, Copy)]
pub struct MsgRandomRoll {
    /// Minimum value
    pub min: u32,
    /// Maximum value
    pub max: u32,
    /// Roll result
    pub roll: u32,
    /// Player who rolled
    pub player_guid: ObjectGuid,
}

impl ToWorldPacket for MsgRandomRoll {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_RANDOM_ROLL);
        packet.write_u32(self.min);
        packet.write_u32(self.max);
        packet.write_u32(self.roll);
        packet.write_u64(self.player_guid.raw());
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::{ObjectGuid, Opcode};
    use crate::world::game::group::types::group_update_flags;
    use crate::world::game::group::{CachedGroup, GroupMember, LootMethod, MemberStatus};

    #[test]
    fn test_smsg_group_list() {
        let leader_guid = ObjectGuid::new_player(123);
        let member_guid = ObjectGuid::new_player(456);

        let group = CachedGroup {
            id: 1,
            leader_guid,
            leader_name: "Leader".to_string(),
            is_raid: false,
            loot_method: LootMethod::GroupLoot,
            loot_threshold: 2,
            looter_guid: ObjectGuid::empty(),
            main_tank_guid: ObjectGuid::empty(),
            main_assistant_guid: ObjectGuid::empty(),
            target_icons: [ObjectGuid::empty(); 8],
            members: vec![
                GroupMember {
                    guid: leader_guid,
                    name: "Leader".to_string(),
                    subgroup: 0,
                    assistant: false,
                    status: MemberStatus::new(),
                    last_online: 0,
                },
                GroupMember {
                    guid: member_guid,
                    name: "Member".to_string(),
                    subgroup: 0,
                    assistant: false,
                    status: MemberStatus::new(),
                    last_online: 0,
                },
            ],
            subgroup_counts: [2, 0, 0, 0, 0, 0, 0, 0],
        };

        let msg = SmsgGroupList {
            group: &group,
            member_guid,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_GROUP_LIST);
    }

    #[test]
    fn test_smsg_group_set_leader() {
        let msg = SmsgGroupSetLeader {
            leader_name: "NewLeader",
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_GROUP_SET_LEADER);
    }

    #[test]
    fn test_smsg_party_command_result() {
        let msg = SmsgPartyCommandResult {
            operation: 1,
            member_name: "TargetPlayer",
            result: 0,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_PARTY_COMMAND_RESULT);
    }

    #[test]
    fn test_smsg_party_member_stats() {
        let msg = SmsgPartyMemberStats {
            player_guid: ObjectGuid::from_low(123),
            update_mask: group_update_flags::STATUS | group_update_flags::CUR_HP,
            status: Some(0x01), // ONLINE
            health: Some(100),
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
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_PARTY_MEMBER_STATS);
    }

    #[test]
    fn test_smsg_loot_roll_started() {
        let msg = SmsgLootRollStarted {
            loot_guid: ObjectGuid::from_low(456),
            item_slot: 0,
            item_id: 12345,
            item_random_prop_id: 0,
            item_suffix_factor: 0,
            item_count: 1,
            roll_timeout: 60,
            roll_type: 0,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_LOOT_START_ROLL);
    }

    #[test]
    fn test_smsg_loot_roll() {
        let msg = SmsgLootRoll {
            player_guid: ObjectGuid::from_low(789),
            item_slot: 0,
            roll_number: 42,
            roll_type: 0,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_LOOT_ROLL);
    }

    #[test]
    fn test_smsg_loot_roll_won() {
        let msg = SmsgLootRollWon {
            player_guid: ObjectGuid::from_low(789),
            item_slot: 0,
            roll_number: 95,
            roll_type: 0,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_LOOT_ROLL_WON);
    }

    #[test]
    fn test_smsg_loot_all_passed() {
        let msg = SmsgLootAllPassed {
            loot_guid: ObjectGuid::from_low(456),
            item_slot: 0,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_LOOT_ALL_PASSED);
    }
}
