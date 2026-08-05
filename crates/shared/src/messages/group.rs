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

use crate::game::group::group_update_flags;
use crate::game::group::{CachedGroup, GroupMember, LootMethod};
use crate::messages::loot::write_modern_item_instance;
use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::loot_guid128;
use crate::protocol::{ObjectGuid, Opcode, WorldPacket};
use std::sync::atomic::{AtomicU32, Ordering};

// =========================================================================
// SHARED 1.14 GROUP ENCODING PIECES
// =========================================================================

/// `HighGuidType703::Party` — the object-type field of a 1.14 party GUID.
const HIGH_GUID_TYPE_PARTY: u64 = 27;

/// The 1.14 GUID that names the party a message is about.
///
/// 1.12 has no party object and sends no party GUID, so there is nothing to translate — the value
/// is synthesised here. Every message that names a party must synthesise the **same** one: the
/// client keys its party frame, ready checks and raid marks off this GUID, and a ready check
/// carrying a party GUID the party update never introduced is silently discarded. That is why this
/// is one shared constant rather than something derived per group.
///
/// A single global value is safe because a client is only ever in one party, so its own party is
/// the only one it can ever see named. The counter is arbitrary — it only has to be non-zero, since
/// an all-zero GUID reads as "no party".
const fn modern_party_guid() -> (u64, u64) {
    (HIGH_GUID_TYPE_PARTY << 58, 1000)
}

/// Monotonic source for `PartyUpdate::SequenceNum`.
///
/// 1.14 stamps every party update with a sequence number and the client keeps the highest it has
/// seen, so a body that repeats or lowers the number is dropped: the party frame would freeze at
/// whatever the first update said and never show a join, a leave or a leader change again. Vanilla
/// has no such field and [`CachedGroup`] carries no counter, so the value is generated here.
///
/// Global rather than per-group on purpose. The client only compares numbers within its own party,
/// so any strictly increasing sequence works, and a process-wide counter is strictly increasing by
/// construction without needing state we do not have.
static PARTY_UPDATE_SEQUENCE: AtomicU32 = AtomicU32::new(1);

fn next_party_update_sequence() -> i32 {
    PARTY_UPDATE_SEQUENCE.fetch_add(1, Ordering::Relaxed) as i32
}

// `GroupFlags`, per the 1.14 wire format. Unrelated to anything vanilla sends: 1.12 signals "raid"
// with a leading bool and has no flags word at all.
const GROUP_FLAG_RAID: u16 = 0x002;
const GROUP_FLAG_DESTROYED: u16 = 0x010;
const GROUP_FLAG_EVERYONE_ASSISTANT: u16 = 0x040;

// `GroupType`, per the 1.14 wire format.
const GROUP_TYPE_NONE: u8 = 0;
const GROUP_TYPE_NORMAL: u8 = 1;

// `GroupMemberFlags`, per the 1.14 wire format.
//
// **Not the same encoding as vanilla's flags byte.** 1.12 packs the subgroup into the low nibble of
// one byte and sets 0x80 for assistant; 1.14 sends the subgroup as its own byte and uses 0x01 for
// assistant in a separate one. Copying vanilla's packed byte across would mark nobody an assistant
// and set the main-tank/main-assist bits from the subgroup index instead.
const GROUP_MEMBER_FLAG_ASSISTANT: u8 = 0x01;
const GROUP_MEMBER_FLAG_MAIN_TANK: u8 = 0x02;
const GROUP_MEMBER_FLAG_MAIN_ASSIST: u8 = 0x04;

// `DifficultyModern`. Classic Era has exactly one dungeon difficulty and one raid size, so these are
// the only two values a 1.12 group can be in.
const DIFFICULTY_NORMAL: u32 = 1;
const DIFFICULTY_RAID40: u32 = 9;

// `AuraFlagsModern`. 1.12 separates positive and negative auras into two masks; 1.14 sends one aura
// list and distinguishes them with these bits, so the split has to be re-encoded rather than copied.
const AURA_FLAG_NEGATIVE: u16 = 0x010;
const AURA_FLAG_POSITIVE: u16 = 0x100;

/// Translate a vanilla party-command result to its 1.14 number.
///
/// **The two enums are renumbered, not extended.** 1.14 inserts `TargetNotInInstance` at 3, which
/// pushes `GroupFull` and everything after it up by one, and reorders `NotLeader` ahead of
/// `PlayerWrongFaction`. Passing the number through unchanged does not fail visibly — it prints a
/// confidently wrong line: "Your party is full." where the server meant "%s is already in a group.",
/// and the player retries an invite that can never succeed.
///
/// Translated by name from the [`crate::game::group`] `ERR_*` constants, which are what every caller
/// passes. Note that those constants are themselves numbered differently from the 1.12 table (this
/// server has no `NOT_IN_GROUP`, and puts wrong-faction/ignoring-you/not-leader at
/// 5/6/7); the mapping below follows the constant *names*, so it stays correct regardless.
///
/// Returns `None` for a value that is not one of the constants. Dropping the packet costs the player
/// a feedback line; guessing a number spends the same packet asserting an error that never happened.
fn to_modern_party_result(result: u32) -> Option<u8> {
    use crate::game::group::{
        ERR_ALREADY_IN_GROUP_S, ERR_BAD_PLAYER_NAME_S, ERR_GROUP_FULL, ERR_IGNORING_YOU_S,
        ERR_NOT_LEADER, ERR_PARTY_RESULT_OK, ERR_PLAYER_WRONG_FACTION, ERR_TARGET_NOT_IN_GROUP_S,
    };

    Some(match result {
        ERR_PARTY_RESULT_OK => 0,
        ERR_BAD_PLAYER_NAME_S => 1,
        ERR_TARGET_NOT_IN_GROUP_S => 2,
        // 3 is `TargetNotInInstance`, added in 1.14; everything below shifts up past it.
        ERR_GROUP_FULL => 4,
        ERR_ALREADY_IN_GROUP_S => 5,
        // 6 is `NotInGroup`, which this server's constant set does not have.
        ERR_NOT_LEADER => 7,
        ERR_PLAYER_WRONG_FACTION => 8,
        ERR_IGNORING_YOU_S => 9,
        _ => return None,
    })
}

/// `PartyPlayerInfo::Write`, per the 1.14 wire format.
///
/// Two differences bite here. The name's length is a 6-bit field written *before* the GUID while the
/// bytes go last, so the whole entry shifts if the width is wrong; and the recipient is a normal
/// entry in this list, where vanilla omits them and lets the client insert itself.
fn write_modern_party_member(writer: &mut BitWriter, group: &CachedGroup, member: &GroupMember) {
    let name = member.name.as_bytes();
    writer.write_bits(name.len() as u32, 6);
    // VoiceStateID length, biased by one — the client reads `len - 1` bytes, so 1 means "empty".
    // Classic Era has no voice chat, so there is never a string to follow.
    writer.write_bits(1, 6);
    writer.write_bit(false); // FromSocialQueue
    writer.write_bit(false); // VoiceChatSilenced
                             // Byte-sized writes flush the 14 bits above; the name bytes land after every fixed field.

    let (high, low) = member.guid.to_guid128(DEFAULT_REALM_ID);
    writer.write_packed_guid_128(high, low);

    // `GroupMemberOnlineStatus` keeps vanilla's bit values (online 0x01, dead 0x04, AFK 0x40, DND
    // 0x80), so the status carries over unchanged. Only the *width* grew, and this field stayed a
    // byte, so the truncation is the same one vanilla does.
    writer.write_u8(member.status.as_u16() as u8);
    writer.write_u8(member.subgroup);

    let mut flags = 0u8;
    if member.assistant {
        flags |= GROUP_MEMBER_FLAG_ASSISTANT;
    }
    if !member.guid.is_empty() && group.main_tank_guid == member.guid {
        flags |= GROUP_MEMBER_FLAG_MAIN_TANK;
    }
    if !member.guid.is_empty() && group.main_assistant_guid == member.guid {
        flags |= GROUP_MEMBER_FLAG_MAIN_ASSIST;
    }
    writer.write_u8(flags);

    writer.write_u8(0); // RolesAssigned -- tank/healer/dps roles arrive with LFG, after Classic Era
                        // ClassId: 1.12's group list carries no class, and the client fills it from the name query it
                        // already sends for each member. Sending a guess here paints the wrong class icon and, worse,
                        // conflicts with the name-query answer that follows.
    writer.write_u8(0);

    writer.write_bytes(name);
}

/// SMSG_LOOT_START_ROLL - Start a loot roll for an item
///
/// Sent when an item in loot is rolled on by group members.
/// Client displays a roll UI to eligible players.
#[derive(Debug, Clone)]
pub struct SmsgLootRollStarted {
    pub loot_guid: ObjectGuid,
    pub map_id: u32,
    pub item_slot: u32,
    pub item_id: u32,
    pub item_random_prop_id: i32,
    pub item_suffix_factor: u32,
    pub item_count: u8,
    pub roll_timeout: u32,
    pub roll_type: u8,
}

impl ToWorldPacket for SmsgLootRollStarted {
    fn to_vanilla(&self) -> WorldPacket {
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

    /// `StartLootRoll.Write`: the loot object guid128, then map id, then the roll method and item.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = loot_guid128(&self.loot_guid);
        writer.write_packed_guid_128(high, low); // LootObj
        writer.write_u32(self.map_id); // MapID
        writer.write_u32(self.roll_timeout); // RollTime
        writer.write_u8(0x07); // ValidRolls: need|greed|pass
        writer.write_u8(self.roll_type); // Method
        write_modern_item_instance(
            &mut writer,
            self.item_id,
            self.item_random_prop_id.max(0) as u32,
        );
        Some(writer.finish(Opcode::SMSG_LOOT_START_ROLL))
    }
}

/// SMSG_LOOT_ROLL - Player's roll result
///
/// Sent when a player rolls on an item (need, greed, or pass).
/// Broadcast to all group members.
#[derive(Debug, Clone)]
pub struct SmsgLootRoll {
    pub loot_guid: ObjectGuid,
    pub player_guid: ObjectGuid,
    pub item_slot: u32,
    pub item_id: u32,
    pub roll_number: u8,
    pub roll_type: u8,
}

impl ToWorldPacket for SmsgLootRoll {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOOT_ROLL);
        packet.write_u64(self.player_guid.raw());
        packet.write_u32(self.item_slot);
        packet.write_u8(self.roll_number);
        packet.write_u8(self.roll_type);
        packet
    }

    /// `LootRollBroadcast.Write`.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (loot_high, loot_low) = loot_guid128(&self.loot_guid);
        writer.write_packed_guid_128(loot_high, loot_low); // LootObj
        let (high, low) = self.player_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Player
        writer.write_i32(i32::from(self.roll_number)); // Roll
        writer.write_u8(self.roll_type); // RollType
        write_modern_item_instance(&mut writer, self.item_id, 0);
        writer.write_bit(false); // Autopassed
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_LOOT_ROLL))
    }
}

/// SMSG_LOOT_ROLL_WON - Winner of a loot roll
///
/// Sent when the roll period ends and a winner is determined.
/// Broadcast to all group members.
#[derive(Debug, Clone)]
pub struct SmsgLootRollWon {
    pub loot_guid: ObjectGuid,
    pub player_guid: ObjectGuid,
    pub item_slot: u32,
    pub item_id: u32,
    pub roll_number: u8,
    pub roll_type: u8,
}

impl ToWorldPacket for SmsgLootRollWon {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOOT_ROLL_WON);
        packet.write_u64(self.player_guid.raw());
        packet.write_u32(self.item_slot);
        packet.write_u8(self.roll_number);
        packet.write_u8(self.roll_type);
        packet
    }

    /// `LootRollWon.Write`.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (loot_high, loot_low) = loot_guid128(&self.loot_guid);
        writer.write_packed_guid_128(loot_high, loot_low); // LootObj
        let (high, low) = self.player_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Winner
        writer.write_i32(i32::from(self.roll_number)); // Roll
        writer.write_u8(self.roll_type); // RollType
        write_modern_item_instance(&mut writer, self.item_id, 0);
        writer.write_u8(0); // MainSpec
        Some(writer.finish(Opcode::SMSG_LOOT_ROLL_WON))
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
    pub item_id: u32,
}

impl ToWorldPacket for SmsgLootAllPassed {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOOT_ALL_PASSED);
        packet.write_u64(self.loot_guid.raw());
        packet.write_u32(self.item_slot);
        packet
    }

    /// `LootAllPassed.Write`.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = loot_guid128(&self.loot_guid);
        writer.write_packed_guid_128(high, low); // LootObj
        write_modern_item_instance(&mut writer, self.item_id, 0);
        Some(writer.finish(Opcode::SMSG_LOOT_ALL_PASSED))
    }
}

/// SMSG_LOOT_ROLLS_COMPLETE - Close a loot roll frame (modern-only; no vanilla counterpart)
///
/// Sent once all in-flight rolls for one loot list finish, dismissing the client roll UI.
#[derive(Debug, Clone)]
pub struct SmsgLootRollsComplete {
    pub loot_guid: ObjectGuid,
    pub loot_list_id: u8,
}

impl ToWorldPacket for SmsgLootRollsComplete {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_LOOT_ROLLS_COMPLETE)
    }

    /// `LootRollsComplete.Write`.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = loot_guid128(&self.loot_guid);
        writer.write_packed_guid_128(high, low); // LootObj
        writer.write_u8(self.loot_list_id);
        Some(writer.finish(Opcode::SMSG_LOOT_ROLLS_COMPLETE))
    }
}

/// SMSG_LOOT_MASTER_LIST - Master-looter candidate/assignment list
///
/// Tells the master looter who may receive the looted items.
#[derive(Debug, Clone)]
pub struct SmsgLootMasterList {
    pub loot_guid: ObjectGuid,
    pub candidates: Vec<ObjectGuid>,
}

impl ToWorldPacket for SmsgLootMasterList {
    /// `MasterLootCandidateList.Write`.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = loot_guid128(&self.loot_guid);
        writer.write_packed_guid_128(high, low); // LootObj
        writer.write_i32(self.candidates.len() as i32); // Players.Count
        for guid in &self.candidates {
            let (gh, gl) = guid.to_guid128(DEFAULT_REALM_ID);
            writer.write_packed_guid_128(gh, gl);
        }
        Some(writer.finish(Opcode::SMSG_LOOT_MASTER_LIST))
    }

    /// Vanilla master-loot list carries one `{ slot, looter, item_id }` triple per pending item.
    /// The candidate list alone cannot fill the item id, so it is sent as a best-effort slot list.
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOOT_MASTER_LIST);
        packet.write_u8(self.candidates.len() as u8);
        for (i, guid) in self.candidates.iter().enumerate() {
            packet.write_u8(i as u8);
            packet.write_u64(guid.raw());
            packet.write_u32(0);
        }
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GROUP_INVITE);
        packet.write_string(self.inviter_name);
        packet
    }

    /// `PartyInvite::Write`. Vanilla's whole body is the inviter's name; 1.14 wraps that name in six
    /// leading bits, a realm block, two GUIDs and four LFG fields.
    ///
    /// The name's 6-bit length is written *before* the realm block but the bytes go after every
    /// fixed field, so the string is not where a vanilla reader would look for it. `CanAccept` must
    /// be set: with it clear the popup appears with no accept button and the invite can only be
    /// declined.
    ///
    /// The inviter GUID is sent empty. Vanilla names the inviter only by name, and the client uses
    /// the name for the popup text; the GUID feeds cross-realm and Battle.net paths that Classic Era
    /// has none of.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let name = self.inviter_name.as_bytes();

        writer.write_bit(true); // CanAccept
        writer.write_bit(false); // MightCRZYou -- no cross-realm zones
        writer.write_bit(false); // IsXRealm
        writer.write_bit(false); // MustBeBNetFriend
        writer.write_bit(false); // AllowMultipleRoles
        writer.write_bit(false); // QuestSessionActive
        writer.write_bits(name.len() as u32, 6);

        // VirtualRealmInfo: the address, then a name block of two bits and two 8-bit lengths. Both
        // realm names are sent empty -- the client only renders them for a cross-realm invite, and
        // `IsLocal` says this is not one.
        writer.write_u32(VIRTUAL_REALM_ADDRESS);
        writer.write_bit(true); // IsLocal
        writer.write_bit(false); // IsInternalRealm
        writer.write_bits(0, 8); // RealmNameActual length
        writer.write_bits(0, 8); // RealmNameNormalized length
        writer.flush_bits();

        writer.write_packed_guid_128(0, 0); // InviterGUID -- see above
        writer.write_packed_guid_128(0, 0); // InviterBNetAccountId
        writer.write_u16(4904); // Unk1
        writer.write_u32(0); // ProposedRoles
        writer.write_i32(0); // LfgSlots count
        writer.write_i32(0); // LfgCompletedMask

        writer.write_bytes(name);

        Some(writer.finish(Opcode::SMSG_GROUP_INVITE))
    }
}

/// `region << 24 | site << 16 | realm id`, the same scheme the bnet realm list advertises.
const VIRTUAL_REALM_ADDRESS: u32 = 0x0101_0000 | DEFAULT_REALM_ID as u32;

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
    fn to_vanilla(&self) -> WorldPacket {
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

    /// `PartyUpdate::Write`. The most heavily restructured body in this module.
    ///
    /// Four things differ from vanilla and each is silently wrong if missed:
    ///
    /// * **The recipient is in the list.** Vanilla omits them and the client inserts itself from its
    ///   own state; 1.14 sends every member and reads `MyIndex` to find which one is the viewer.
    ///   Keeping vanilla's filter would leave the player absent from their own party frame.
    /// * **Subgroup and assistant split apart.** Vanilla packs both into one byte (`subgroup |
    ///   0x80`); 1.14 sends a subgroup byte and a `GroupMemberFlags` byte where assistant is `0x01`.
    /// * **Loot settings are optional, not unconditional.** They follow the member list behind a
    ///   presence bit, and the three presence bits are written *before* the members.
    /// * **A sequence number gates the whole update** — see `PARTY_UPDATE_SEQUENCE`.
    ///
    /// An empty member list is the disband path: 1.14 expresses it as a party update carrying the
    /// `Destroyed` flag and `MyIndex` of -1, with none of the optional blocks.
    fn to_modern(&self) -> Option<WorldPacket> {
        let members = &self.group.members;
        let destroyed = members.is_empty();

        let mut party_flags = 0u16;
        if destroyed {
            party_flags |= GROUP_FLAG_DESTROYED;
        } else {
            if self.group.is_raid {
                party_flags |= GROUP_FLAG_RAID;
            }
            if members.iter().all(|m| m.assistant) {
                party_flags |= GROUP_FLAG_EVERYONE_ASSISTANT;
            }
        }

        let mut writer = BitWriter::new();
        writer.write_u16(party_flags);
        // PartyIndex 0 is the real party; 1 is the battleground raid, which this server does not
        // form here. It also selects which party GUID the client matches, so it must stay 0
        // everywhere -- see `modern_party_guid`.
        writer.write_u8(0);
        writer.write_u8(if destroyed {
            GROUP_TYPE_NONE
        } else {
            GROUP_TYPE_NORMAL
        });

        let my_index = members
            .iter()
            .position(|m| m.guid == self.member_guid)
            .map_or(-1, |index| index as i32);
        writer.write_i32(my_index);

        let (party_high, party_low) = if destroyed {
            (0, 0)
        } else {
            modern_party_guid()
        };
        writer.write_packed_guid_128(party_high, party_low);

        writer.write_i32(next_party_update_sequence());

        let (leader_high, leader_low) = if destroyed {
            (0, 0)
        } else {
            self.group.leader_guid.to_guid128(DEFAULT_REALM_ID)
        };
        writer.write_packed_guid_128(leader_high, leader_low);

        writer.write_i32(members.len() as i32);
        writer.write_bit(false); // HasLfgInfos -- no dungeon finder in Classic Era
        writer.write_bit(!destroyed); // HasLootSettings
        writer.write_bit(!destroyed); // HasDifficultySettings
        writer.flush_bits();

        for member in members {
            write_modern_party_member(&mut writer, self.group, member);
        }

        if !destroyed {
            // PartyLootSettings. As in vanilla, the looter GUID is only meaningful under master
            // loot; the client draws the crown from it and would put one on a stale player
            // otherwise.
            writer.write_u8(self.group.loot_method as u8);
            let (looter_high, looter_low) = if self.group.loot_method == LootMethod::MasterLooter {
                self.group.looter_guid.to_guid128(DEFAULT_REALM_ID)
            } else {
                (0, 0)
            };
            writer.write_packed_guid_128(looter_high, looter_low);
            writer.write_u8(self.group.loot_threshold);

            // PartyDifficultySettings. Vanilla sends a single difficulty byte and only when the
            // group is non-empty; 1.14 always reads three ints here.
            writer.write_u32(DIFFICULTY_NORMAL); // DungeonDifficultyID
            writer.write_u32(DIFFICULTY_RAID40); // RaidDifficultyID
            writer.write_u32(0); // LegacyRaidDifficultyID -- no legacy raids exist yet
        }

        Some(writer.finish(Opcode::SMSG_GROUP_LIST))
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GROUP_SET_LEADER);
        packet.write_string(self.leader_name);
        packet
    }

    /// `GroupNewLeader::Write`. The name loses its null terminator and gains a 9-bit length, and a
    /// party index byte now leads the body.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u8(0); // PartyIndex -- the real party; see `SmsgGroupList::to_modern`
        let name = self.leader_name.as_bytes();
        writer.write_bits(name.len() as u32, 9);
        writer.write_bytes(name);
        Some(writer.finish(Opcode::SMSG_GROUP_SET_LEADER))
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_PARTY_COMMAND_RESULT);
        packet.write_u32(self.operation);
        packet.write_string(self.member_name);
        packet.write_u32(self.result);
        packet
    }

    /// `PartyCommandResult::Write`. Vanilla's two 32-bit enums become a 4-bit command and a 6-bit
    /// result packed alongside the name's 9-bit length, and the **result enum is renumbered** — see
    /// `to_modern_party_result`, which is where the real hazard in this message lives.
    ///
    /// The command enum is *not* renumbered (invite is still 0, leave still 2), so it is cast. It
    /// only has 4 bits now, so a command above 15 would wrap into the result field.
    fn to_modern(&self) -> Option<WorldPacket> {
        let result = to_modern_party_result(self.result)?;
        let name = self.member_name.as_bytes();

        let mut writer = BitWriter::new();
        writer.write_bits(name.len() as u32, 9);
        writer.write_bits(self.operation & 0xF, 4);
        writer.write_bits(u32::from(result), 6);
        // 19 bits; the u32 below flushes them out to three bytes.

        writer.write_u32(0); // ResultData -- only LFG results populate it
        writer.write_packed_guid_128(0, 0); // ResultGUID -- vanilla names the target by name only
        writer.write_bytes(name);

        Some(writer.finish(Opcode::SMSG_PARTY_COMMAND_RESULT))
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
    fn to_vanilla(&self) -> WorldPacket {
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

    /// `PartyMemberPartialState::Write`. Vanilla's 32-bit update mask becomes a run of presence
    /// bits, one per optional field, in an order that is **not** the mask's bit order.
    ///
    /// Three traps:
    ///
    /// * **The pet block comes before the affected GUID**, not after it. Every other field follows
    ///   the GUID. Writing the pet where vanilla's ordering suggests puts the whole tail one block
    ///   out of place, and the client attributes the stats to the wrong unit.
    /// * **The positive and negative aura masks merge into one list.** Vanilla sends two bitmasks
    ///   with the spell ids implied by set bits; 1.14 sends a counted list of entries that each
    ///   carry a `Negative`/`Positive` flag, so the split has to be re-encoded rather than copied.
    /// * **Health and power widen unevenly.** Health becomes 32-bit while power stays 16-bit, so a
    ///   uniform widening desynchronises everything after the health fields.
    fn to_modern(&self) -> Option<WorldPacket> {
        use group_update_flags::*;

        let has = |flag: u32| (self.update_mask & flag) != 0;

        let positive: &[u32] = if has(AURAS) {
            self.auras.unwrap_or(&[])
        } else {
            &[]
        };
        let negative: &[u32] = if has(AURAS_NEGATIVE) {
            self.negative_auras.unwrap_or(&[])
        } else {
            &[]
        };
        let has_auras = has(AURAS) || has(AURAS_NEGATIVE);

        let pet_positive: &[u32] = if has(PET_AURAS) {
            self.pet_auras.unwrap_or(&[])
        } else {
            &[]
        };
        let pet_negative: &[u32] = if has(PET_AURAS_NEGATIVE) {
            self.pet_negative_auras.unwrap_or(&[])
        } else {
            &[]
        };
        let has_pet_auras = has(PET_AURAS) || has(PET_AURAS_NEGATIVE);
        let has_pet = has(PET_GUID)
            || has(PET_NAME)
            || has(PET_MODEL_ID)
            || has(PET_CUR_HP)
            || has(PET_MAX_HP)
            || has_pet_auras;

        let mut writer = BitWriter::new();

        writer.write_bit(false); // ForEnemyChanged
        writer.write_bit(false); // SetPvPInactive
        writer.write_bit(false); // Unk901_1
        writer.write_bit(false); // HasPartyType
        writer.write_bit(has(STATUS));
        writer.write_bit(has(POWER_TYPE));
        writer.write_bit(false); // HasOverrideDisplayPower
        writer.write_bit(has(CUR_HP));
        writer.write_bit(has(MAX_HP));
        writer.write_bit(has(CUR_POWER));
        writer.write_bit(has(MAX_POWER));
        writer.write_bit(has(LEVEL));
        writer.write_bit(false); // HasSpec -- talent specs are a later expansion's concept
        writer.write_bit(has(ZONE));
        writer.write_bit(false); // HasWmoGroupID
        writer.write_bit(false); // HasWmoDoodadPlacementID
        writer.write_bit(has(POSITION));
        writer.write_bit(false); // HasVehicleSeatRecID
        writer.write_bit(has_auras);
        writer.write_bit(has_pet);
        writer.write_bit(false); // HasPhase
        writer.write_bit(false); // HasUnk901_2
        writer.flush_bits();

        if has_pet {
            // PartyMemberPetStats::WritePartial -- its own presence-bit run, and the pet's name is
            // written straight after that run, ahead of the pet GUID.
            writer.write_bit(has(PET_GUID));
            writer.write_bit(has(PET_NAME));
            writer.write_bit(has(PET_MODEL_ID));
            writer.write_bit(has(PET_MAX_HP));
            writer.write_bit(has(PET_CUR_HP));
            writer.write_bit(has_pet_auras);
            writer.flush_bits();

            if has(PET_NAME) {
                let pet_name = self.pet_name.unwrap_or("").as_bytes();
                writer.write_bits(pet_name.len() as u32, 8);
                writer.write_bytes(pet_name);
            }
            if has(PET_GUID) {
                let (high, low) = self
                    .pet_guid
                    .unwrap_or_else(ObjectGuid::empty)
                    .to_guid128(DEFAULT_REALM_ID);
                writer.write_packed_guid_128(high, low);
            }
            if has(PET_MODEL_ID) {
                writer.write_u32(u32::from(self.pet_model_id.unwrap_or(0)));
            }
            if has(PET_MAX_HP) {
                writer.write_u32(u32::from(self.pet_max_hp.unwrap_or(0)));
            }
            if has(PET_CUR_HP) {
                writer.write_u32(u32::from(self.pet_cur_hp.unwrap_or(0)));
            }
            if has_pet_auras {
                writer.write_i32((pet_positive.len() + pet_negative.len()) as i32);
                for &spell_id in pet_positive {
                    write_modern_party_aura(&mut writer, spell_id, AURA_FLAG_POSITIVE);
                }
                for &spell_id in pet_negative {
                    write_modern_party_aura(&mut writer, spell_id, AURA_FLAG_NEGATIVE);
                }
            }
            // The pet's power type and power have no home in the 1.14 body; the client reads pet
            // power off the pet's own object update instead. Dropping them is not a data loss.
        }

        let (high, low) = self.player_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // AffectedGUID -- after the pet block, see above

        if has(STATUS) {
            // `GroupMemberOnlineStatus` widened to 16 bits but kept vanilla's bit values.
            writer.write_u16(u16::from(self.status.unwrap_or(0)));
        }
        if has(POWER_TYPE) {
            writer.write_u8(self.power_type.unwrap_or(0));
        }
        if has(CUR_HP) {
            writer.write_u32(self.health.unwrap_or(0));
        }
        if has(MAX_HP) {
            writer.write_u32(self.max_health.unwrap_or(0));
        }
        if has(CUR_POWER) {
            writer.write_u16(self.cur_power.unwrap_or(0).min(u16::MAX as u32) as u16);
        }
        if has(MAX_POWER) {
            writer.write_u16(self.max_power.unwrap_or(0).min(u16::MAX as u32) as u16);
        }
        if has(LEVEL) {
            writer.write_u16(u16::from(self.level.unwrap_or(0)));
        }
        if has(ZONE) {
            writer.write_u16(self.zone_id.unwrap_or(0).min(u16::MAX as u32) as u16);
        }
        if has(POSITION) {
            // 1.14 reads three signed shorts holding the world coordinate truncated to an integer --
            // the party frame only needs enough precision to place a dot on the zone map. Vanilla
            // sends X and Y; Z has no vanilla source, and 0 is what the client treats as "unknown".
            writer.write_i16(self.position_x.unwrap_or(0.0) as i16);
            writer.write_i16(self.position_y.unwrap_or(0.0) as i16);
            writer.write_i16(0);
        }
        if has_auras {
            writer.write_i32((positive.len() + negative.len()) as i32);
            for &spell_id in positive {
                write_modern_party_aura(&mut writer, spell_id, AURA_FLAG_POSITIVE);
            }
            for &spell_id in negative {
                write_modern_party_aura(&mut writer, spell_id, AURA_FLAG_NEGATIVE);
            }
        }

        Some(writer.finish(Opcode::SMSG_PARTY_MEMBER_STATS))
    }
}

/// `PartyMemberAuraStates::Write`, per the 1.14 wire format.
///
/// Shared by the member's own auras and the pet's, which send the same entry shape.
///
/// `ActiveFlags` is a per-effect-index bitmask, not a boolean; vanilla's group aura masks say only
/// that the aura is present, so bit 0 is what "it is applied" translates to.
fn write_modern_party_aura(writer: &mut BitWriter, spell_id: u32, aura_flags: u16) {
    writer.write_u32(spell_id);
    writer.write_u16(aura_flags);
    writer.write_u32(1); // ActiveFlags
    writer.write_i32(0); // Points count -- vanilla sends no aura effect values in this message
}

/// SMSG_GROUP_DESTROYED - Notify player that group was disbanded
///
/// Sent when a group is disbanded (empty packet).
#[derive(Debug, Clone, Copy)]
pub struct SmsgGroupDestroyed;

impl ToWorldPacket for SmsgGroupDestroyed {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_GROUP_DESTROYED)
    }

    /// Empty in both protocols, so the vanilla body is byte-identical.
    ///
    /// 1.14 does not clear the party frame from this alone — it acts on the party update carrying
    /// the `Destroyed` flag (see [`SmsgGroupList::to_modern`]). Sending this too is harmless and
    /// matches what the disband path already does for vanilla.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
    }
}

/// SMSG_GROUP_UNINVITE - Notify player they were kicked from group
///
/// Sent when a player is removed from a group by the leader (empty packet).
#[derive(Debug, Clone, Copy)]
pub struct SmsgGroupUninvite;

impl ToWorldPacket for SmsgGroupUninvite {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_GROUP_UNINVITE)
    }

    /// Empty in both protocols, so the vanilla body is byte-identical. This is the packet that makes
    /// the client say it was removed from the party, so it must not be sent on a voluntary leave.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
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
    fn to_vanilla(&self) -> WorldPacket {
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

    /// `SendRaidTargetUpdateAll::Write`.
    ///
    /// 1.14 splits this message in two: a single-mark form that names who placed the mark, and a
    /// whole-table form that does not. Vanilla's delta form carries no setter GUID at all, so the
    /// single form is not encodable from what we have — and it does not need to be. This struct
    /// always carries the full eight-slot table regardless of `mode`, and the whole-table form is a
    /// complete state sync: sending it is correct whether one mark changed or all of them did.
    ///
    /// The per-entry order is **reversed** from vanilla: 1.14 writes the target GUID and *then* the
    /// symbol index, where vanilla leads with the index. Getting that backwards puts every mark on
    /// the wrong unit rather than failing to parse. The list is also explicitly counted instead of
    /// running to the end of the packet.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u8(0); // PartyIndex -- the real party; see `SmsgGroupList::to_modern`

        let count = self
            .target_icons
            .iter()
            .filter(|target| !target.is_empty())
            .count();
        writer.write_i32(count as i32);

        for (index, target) in self.target_icons.iter().enumerate() {
            if target.is_empty() {
                continue;
            }
            let (high, low) = target.to_guid128(DEFAULT_REALM_ID);
            writer.write_packed_guid_128(high, low);
            writer.write_u8(index as u8); // Symbol
        }

        Some(writer.finish(Opcode::MSG_RAID_TARGET_UPDATE))
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_RAID_READY_CHECK);
        packet.write_u64(self.player_guid.raw());
        if let Some(ready) = self.ready {
            packet.write_u8(if ready { 1 } else { 0 });
        }
        packet
    }

    /// `ReadyCheckStarted::Write` and `ReadyCheckResponse::Write`.
    ///
    /// 1.14 **splits this opcode in two.** Vanilla distinguishes "a ready check started" from "a
    /// player answered" by body length alone — the answer has a trailing byte, the start does not —
    /// which is why the vanilla struct makes the answer an `Option`. 1.14 gives each its own opcode,
    /// so the `Option` selects the opcode rather than a trailing field. Sending a response body
    /// under the start opcode does not misparse; it starts a second ready check.
    ///
    /// Both forms name the party, which vanilla never does. See `modern_party_guid` for why that
    /// GUID is synthesised and why it has to be the same one the party update sent.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (party_high, party_low) = modern_party_guid();
        let (high, low) = self.player_guid.to_guid128(DEFAULT_REALM_ID);

        match self.ready {
            None => {
                writer.write_u8(0); // PartyIndex -- see `SmsgGroupList::to_modern`
                writer.write_packed_guid_128(party_high, party_low);
                writer.write_packed_guid_128(high, low); // InitiatorGUID
                                                         // Duration drives the countdown on the ready-check frame and has no vanilla source.
                                                         // 35s is the client's own default window, so this matches what a native server
                                                         // would put here rather than inventing a timeout of our own.
                writer.write_u64(35_000);
                Some(writer.finish(Opcode::MSG_RAID_READY_CHECK))
            }
            Some(ready) => {
                writer.write_packed_guid_128(party_high, party_low);
                writer.write_packed_guid_128(high, low); // Player
                writer.write_bit(ready);
                writer.flush_bits();
                Some(writer.finish(Opcode::SMSG_READY_CHECK_RESPONSE))
            }
        }
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_MINIMAP_PING);
        packet.write_u64(self.player_guid.raw());
        packet.write_f32(self.x);
        packet.write_f32(self.y);
        packet
    }

    /// `MinimapPing::Write`. Same three fields in the same order; only the GUID's encoding changes,
    /// from a raw 64-bit value to a packed 128-bit one.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.player_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // SenderGUID
        writer.write_f32(self.x);
        writer.write_f32(self.y);
        Some(writer.finish(Opcode::MSG_MINIMAP_PING))
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_RANDOM_ROLL);
        packet.write_u32(self.min);
        packet.write_u32(self.max);
        packet.write_u32(self.roll);
        packet.write_u64(self.player_guid.raw());
        packet
    }

    /// `RandomRoll::Write`. The roller's GUID moves from **last to first** and gains a companion
    /// account GUID; the three numbers keep their order behind them. Leaving the GUID where vanilla
    /// puts it would have the client read the low half of the roll range as the roller.
    ///
    /// The account GUID is sent empty: it identifies the roller's game account for Battle.net
    /// features that Classic Era does not have, and vanilla has no account identity on the wire at
    /// all.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.player_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Roller
        writer.write_packed_guid_128(0, 0); // RollerWowAccount -- see above
        writer.write_i32(self.min as i32);
        writer.write_i32(self.max as i32);
        writer.write_i32(self.roll as i32);
        Some(writer.finish(Opcode::MSG_RANDOM_ROLL))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::group::group_update_flags;
    use crate::game::group::{CachedGroup, GroupMember, LootMethod, MemberStatus};
    use crate::protocol::{ObjectGuid, Opcode};

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
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_GROUP_LIST);
    }

    #[test]
    fn test_smsg_group_set_leader() {
        let msg = SmsgGroupSetLeader {
            leader_name: "NewLeader",
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_GROUP_SET_LEADER);
    }

    #[test]
    fn test_smsg_party_command_result() {
        let msg = SmsgPartyCommandResult {
            operation: 1,
            member_name: "TargetPlayer",
            result: 0,
        };
        let packet = msg.to_vanilla();
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
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_PARTY_MEMBER_STATS);
    }

    #[test]
    fn test_smsg_loot_roll_started() {
        let msg = SmsgLootRollStarted {
            loot_guid: ObjectGuid::from_low(456),
            map_id: 1,
            item_slot: 0,
            item_id: 12345,
            item_random_prop_id: 0,
            item_suffix_factor: 0,
            item_count: 1,
            roll_timeout: 60,
            roll_type: 0,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_LOOT_START_ROLL);
        let modern = msg.to_modern().expect("roll started must encode modern");
        assert_eq!(modern.opcode(), Opcode::SMSG_LOOT_START_ROLL);
        assert!(!modern.data().is_empty());
    }

    #[test]
    fn test_smsg_loot_roll() {
        let msg = SmsgLootRoll {
            loot_guid: ObjectGuid::from_low(456),
            player_guid: ObjectGuid::from_low(789),
            item_slot: 0,
            item_id: 12345,
            roll_number: 42,
            roll_type: 0,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_LOOT_ROLL);
        let modern = msg.to_modern().expect("roll broadcast must encode modern");
        assert_eq!(modern.opcode(), Opcode::SMSG_LOOT_ROLL);
        assert!(!modern.data().is_empty());
    }

    #[test]
    fn test_smsg_loot_roll_won() {
        let msg = SmsgLootRollWon {
            loot_guid: ObjectGuid::from_low(456),
            player_guid: ObjectGuid::from_low(789),
            item_slot: 0,
            item_id: 12345,
            roll_number: 95,
            roll_type: 0,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_LOOT_ROLL_WON);
        let modern = msg.to_modern().expect("roll won must encode modern");
        assert_eq!(modern.opcode(), Opcode::SMSG_LOOT_ROLL_WON);
        assert!(!modern.data().is_empty());
    }

    #[test]
    fn test_smsg_loot_all_passed() {
        let msg = SmsgLootAllPassed {
            loot_guid: ObjectGuid::from_low(456),
            item_slot: 0,
            item_id: 12345,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_LOOT_ALL_PASSED);
        let modern = msg.to_modern().expect("all-passed must encode modern");
        assert_eq!(modern.opcode(), Opcode::SMSG_LOOT_ALL_PASSED);
        assert!(!modern.data().is_empty());
    }

    #[test]
    fn test_smsg_loot_rolls_complete_is_modern_only() {
        let msg = SmsgLootRollsComplete {
            loot_guid: ObjectGuid::from_low(456),
            loot_list_id: 1,
        };
        let modern = msg.to_modern().expect("rolls complete must encode modern");
        assert_eq!(modern.opcode(), Opcode::SMSG_LOOT_ROLLS_COMPLETE);
        assert!(!modern.data().is_empty());
    }

    #[test]
    fn test_smsg_loot_master_list() {
        let msg = SmsgLootMasterList {
            loot_guid: ObjectGuid::from_low(456),
            candidates: vec![ObjectGuid::from_low(1), ObjectGuid::from_low(2)],
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_LOOT_MASTER_LIST);
        assert_eq!(packet.data()[0], 2);
        let modern = msg.to_modern().expect("master list must encode modern");
        assert_eq!(modern.opcode(), Opcode::SMSG_LOOT_MASTER_LIST);
        assert!(!modern.data().is_empty());
    }
}
