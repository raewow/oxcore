//! Battleground messages.
//!
//! Vanilla reports every queue state with one `SMSG_BATTLEFIELD_STATUS`; 1.14 has a packet per
//! state, each opening with a "ride ticket" (requester, queue slot, type, join time) that the client
//! echoes back when it accepts or leaves. Vanilla names a battleground by map; 1.14 by its
//! `BattlemasterList` id, which is the battleground type id.

use crate::game::battleground::{BattleGroundStatus, BattleGroundTypeId, BattleGroundWinner};
use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::{ObjectGuid, Opcode, WorldPacket};

/// High bits 1.14 adds to a `BattlemasterList` id to form a queue id.
const MODERN_QUEUE_ID_TYPE: u64 = 0x1F10_0000_0000_0000;
/// Ride ticket type for battleground queues.
const RIDE_TYPE_BATTLEGROUNDS: u32 = 1;
/// The failure reason 1.14 shows for a ticket that simply ended (left, expired, removed).
const MODERN_STATUS_FAILED_REMOVED: i32 = 30;

/// What one queue slot shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BattlefieldState {
    /// Nothing in this slot. `bg_type` is what the slot last held, which 1.14 needs to clear it.
    None { bg_type: BattleGroundTypeId },
    /// Waiting in the queue.
    Queued {
        average_wait_ms: u32,
        waited_ms: u32,
        as_group: bool,
    },
    /// Invited; `timeout_ms` left to accept.
    NeedConfirmation { timeout_ms: u32 },
    /// Inside the match. `shutdown_ms` counts down to removal once the match is over (0 while it
    /// runs); `elapsed_ms` is the time since the match started.
    Active { shutdown_ms: u32, elapsed_ms: u32 },
}

/// SMSG_BATTLEFIELD_STATUS (vanilla) / the `SMSG_BATTLEFIELD_STATUS_*` family (1.14).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmsgBattlefieldStatus {
    /// The player the ticket belongs to.
    pub player_guid: ObjectGuid,
    /// Queue slot, 0..3.
    pub queue_slot: u32,
    /// Unix time the player joined this queue (the 1.14 ticket time).
    pub ticket_time: i64,
    pub bg_type: BattleGroundTypeId,
    pub bracket: u8,
    /// Level range of the bracket, shown by 1.14.
    pub min_level: u8,
    pub max_level: u8,
    /// The number players see for this copy of the battleground.
    pub client_instance_id: u32,
    pub state: BattlefieldState,
}

impl SmsgBattlefieldStatus {
    fn status(&self) -> BattleGroundStatus {
        match self.state {
            BattlefieldState::None { .. } => BattleGroundStatus::None,
            BattlefieldState::Queued { .. } => BattleGroundStatus::WaitQueue,
            BattlefieldState::NeedConfirmation { .. } => BattleGroundStatus::WaitJoin,
            BattlefieldState::Active { .. } => BattleGroundStatus::InProgress,
        }
    }

    fn write_ticket(&self, writer: &mut BitWriter) {
        let (high, low) = self.player_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(self.queue_slot + 1);
        writer.write_u32(RIDE_TYPE_BATTLEGROUNDS);
        writer.write_i64(self.ticket_time);
    }

    fn write_header(&self, writer: &mut BitWriter) {
        self.write_ticket(writer);
        writer.write_i32(1); // one battleground in the ticket
        writer.write_u8(self.min_level);
        writer.write_u8(self.max_level);
        writer.write_u8(0); // arena team size
        writer.write_u32(self.client_instance_id);
        writer.write_u64(u64::from(self.bg_type.battlemaster_list_id()) | MODERN_QUEUE_ID_TYPE);
        writer.write_bit(false); // arena
        writer.write_bit(false); // tournament rules
        writer.flush_bits();
    }
}

impl ToWorldPacket for SmsgBattlefieldStatus {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_BATTLEFIELD_STATUS);
        packet.write_u32(self.queue_slot);
        if matches!(self.state, BattlefieldState::None { .. }) {
            packet.write_u32(0);
            return packet;
        }
        packet.write_u32(self.bg_type.map_id());
        packet.write_u8(self.bracket);
        packet.write_u32(self.client_instance_id);
        packet.write_u32(self.status().as_u32());
        match self.state {
            BattlefieldState::Queued {
                average_wait_ms,
                waited_ms,
                ..
            } => {
                packet.write_u32(average_wait_ms);
                packet.write_u32(waited_ms);
            }
            // The invitation carries only its timeout.
            BattlefieldState::NeedConfirmation { timeout_ms } => packet.write_u32(timeout_ms),
            BattlefieldState::Active {
                shutdown_ms,
                elapsed_ms,
            } => {
                packet.write_u32(shutdown_ms);
                packet.write_u32(elapsed_ms);
            }
            BattlefieldState::None { .. } => {}
        }
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let opcode = match self.state {
            BattlefieldState::None { bg_type } => {
                self.write_ticket(&mut writer);
                writer.write_u64(u64::from(bg_type.battlemaster_list_id()) | MODERN_QUEUE_ID_TYPE);
                writer.write_i32(MODERN_STATUS_FAILED_REMOVED);
                writer.write_packed_guid_128(0, 0); // client id
                Opcode::SMSG_BATTLEFIELD_STATUS_FAILED
            }
            BattlefieldState::Queued {
                average_wait_ms,
                waited_ms,
                as_group,
            } => {
                self.write_header(&mut writer);
                writer.write_u32(average_wait_ms);
                writer.write_u32(waited_ms);
                writer.write_bit(as_group);
                writer.write_bit(true); // eligible for matchmaking
                writer.write_bit(false); // suspended
                writer.flush_bits();
                Opcode::SMSG_BATTLEFIELD_STATUS_QUEUED
            }
            BattlefieldState::NeedConfirmation { timeout_ms } => {
                self.write_header(&mut writer);
                writer.write_u32(self.bg_type.map_id());
                writer.write_u32(timeout_ms);
                writer.write_u8(0); // role
                Opcode::SMSG_BATTLEFIELD_STATUS_NEED_CONFIRMATION
            }
            BattlefieldState::Active {
                shutdown_ms,
                elapsed_ms,
            } => {
                self.write_header(&mut writer);
                writer.write_u32(self.bg_type.map_id());
                writer.write_u32(shutdown_ms);
                writer.write_u32(elapsed_ms);
                writer.write_bit(false); // arena faction
                writer.write_bit(false); // left early
                writer.flush_bits();
                Opcode::SMSG_BATTLEFIELD_STATUS_ACTIVE
            }
        };
        Some(writer.finish(opcode))
    }
}

/// SMSG_BATTLEFIELD_LIST — the copies a battlemaster (or entrance portal) offers.
#[derive(Debug, Clone)]
pub struct SmsgBattlefieldList {
    /// The battlemaster, or the player when queueing at a portal. The client sends it back with
    /// its join request.
    pub battlemaster_guid: ObjectGuid,
    pub bg_type: BattleGroundTypeId,
    pub bracket: u8,
    pub min_level: u8,
    pub max_level: u8,
    pub client_instance_ids: Vec<u32>,
}

impl ToWorldPacket for SmsgBattlefieldList {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_BATTLEFIELD_LIST);
        packet.write_u64(self.battlemaster_guid.raw());
        packet.write_u32(self.bg_type.map_id());
        packet.write_u8(self.bracket);
        packet.write_u32(self.client_instance_ids.len() as u32);
        for &id in &self.client_instance_ids {
            packet.write_u32(id);
        }
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        /// Constant the 1.14 client checks and echoes back in its join request.
        const VERIFICATION: i32 = 121_761_856;
        let mut writer = BitWriter::new();
        let (high, low) = self.battlemaster_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_i32(VERIFICATION);
        writer.write_u32(self.bg_type.battlemaster_list_id());
        writer.write_u8(self.min_level);
        writer.write_u8(self.max_level);
        writer.write_i32(self.client_instance_ids.len() as i32);
        for &id in &self.client_instance_ids {
            writer.write_i32(id as i32);
        }
        writer.write_bit(false); // PvP anywhere
        writer.write_bit(false); // random win today
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_BATTLEFIELD_LIST))
    }
}

/// SMSG_UPDATE_WORLD_STATE — one world state changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmsgUpdateWorldState {
    pub state_id: u32,
    pub value: i32,
}

impl ToWorldPacket for SmsgUpdateWorldState {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_WORLD_STATE);
        packet.write_u32(self.state_id);
        packet.write_u32(self.value as u32);
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.state_id);
        writer.write_i32(self.value);
        writer.write_bit(false); // hidden
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_UPDATE_WORLD_STATE))
    }
}

/// One row of the scoreboard.
#[derive(Debug, Clone, PartialEq)]
pub struct PvpLogPlayer {
    pub guid: ObjectGuid,
    /// Honour rank shown in the scoreboard; 0 when the player has none.
    pub rank: u32,
    pub killing_blows: u32,
    pub honorable_kills: u32,
    pub deaths: u32,
    pub bonus_honor: u32,
    /// The battleground's own columns (flag captures, bases assaulted, ...).
    pub extra: Vec<u32>,
    /// Shown by 1.14, which draws the race and class icons from the scoreboard itself.
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub is_alliance: bool,
}

/// MSG_PVP_LOG_DATA (vanilla) / SMSG_PVP_MATCH_STATISTICS (1.14) — the scoreboard.
#[derive(Debug, Clone, PartialEq)]
pub struct SmsgPvpLogData {
    /// Set once the match is over.
    pub winner: Option<BattleGroundWinner>,
    pub players: Vec<PvpLogPlayer>,
}

impl ToWorldPacket for SmsgPvpLogData {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_PVP_LOG_DATA);
        match self.winner {
            Some(winner) => {
                packet.write_u8(1);
                packet.write_u8(winner.as_u8());
            }
            None => packet.write_u8(0),
        }
        packet.write_u32(self.players.len() as u32);
        for player in &self.players {
            packet.write_u64(player.guid.raw());
            packet.write_u32(player.rank);
            packet.write_u32(player.killing_blows);
            packet.write_u32(player.honorable_kills);
            packet.write_u32(player.deaths);
            packet.write_u32(player.bonus_honor);
            packet.write_u32(player.extra.len() as u32);
            for &value in &player.extra {
                packet.write_u32(value);
            }
        }
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_bit(false); // ratings
        writer.write_bit(false); // arena teams
        writer.write_bit(self.winner.is_some());
        writer.write_i32(self.players.len() as i32);
        let alliance = self.players.iter().filter(|p| p.is_alliance).count();
        let horde = self.players.len() - alliance;
        writer.write_u8(horde.min(127) as u8); // player count per team: horde, alliance
        writer.write_u8(alliance.min(127) as u8);
        if let Some(winner) = self.winner {
            writer.write_u8(winner.as_u8());
        }
        for player in &self.players {
            let (high, low) = player.guid.to_guid128(DEFAULT_REALM_ID);
            writer.write_packed_guid_128(high, low);
            writer.write_u32(player.killing_blows);
            writer.write_u32(0); // damage done
            writer.write_u32(0); // healing done
            writer.write_i32(player.extra.len() as i32);
            writer.write_i32(0); // primary talent tree
            writer.write_u32(u32::from(player.gender));
            writer.write_u32(u32::from(player.race));
            writer.write_u32(u32::from(player.class));
            writer.write_i32(0); // creature id
            writer.write_i32(1); // honor level
            writer.write_i32(player.rank as i32);
            for &value in &player.extra {
                writer.write_u32(value);
            }
            writer.write_bit(player.is_alliance);
            writer.write_bit(true); // in world
            writer.write_bit(true); // honor data follows
            writer.write_bit(false); // pre-match rating
            writer.write_bit(false); // rating change
            writer.write_bit(false); // pre-match MMR
            writer.write_bit(false); // MMR change
            writer.flush_bits();
            writer.write_u32(player.honorable_kills);
            writer.write_u32(player.deaths);
            writer.write_u32(player.bonus_honor);
        }
        Some(writer.finish(Opcode::SMSG_PVP_MATCH_STATISTICS))
    }
}

/// A position on the battleground minimap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BattlegroundPosition {
    pub guid: ObjectGuid,
    pub x: f32,
    pub y: f32,
    pub is_alliance: bool,
}

/// MSG_BATTLEGROUND_PLAYER_POSITIONS (vanilla) / SMSG_BATTLEGROUND_PLAYER_POSITIONS (1.14).
///
/// Vanilla lists the asker's teammates outside their raid and one flag carrier; 1.14 shows only
/// flag carriers (it tracks raid members itself).
#[derive(Debug, Clone, PartialEq)]
pub struct SmsgBattlegroundPlayerPositions {
    pub teammates: Vec<BattlegroundPosition>,
    pub flag_carriers: Vec<BattlegroundPosition>,
}

impl ToWorldPacket for SmsgBattlegroundPlayerPositions {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_BATTLEGROUND_PLAYER_POSITIONS);
        packet.write_u32(self.teammates.len() as u32);
        for position in &self.teammates {
            packet.write_u64(position.guid.raw());
            packet.write_f32(position.x);
            packet.write_f32(position.y);
        }
        match self.flag_carriers.first() {
            Some(carrier) => {
                packet.write_u8(1);
                packet.write_u64(carrier.guid.raw());
                packet.write_f32(carrier.x);
                packet.write_f32(carrier.y);
            }
            None => packet.write_u8(0),
        }
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_i32(self.flag_carriers.len() as i32);
        for carrier in &self.flag_carriers {
            let (high, low) = carrier.guid.to_guid128(DEFAULT_REALM_ID);
            writer.write_packed_guid_128(high, low);
            writer.write_f32(carrier.x);
            writer.write_f32(carrier.y);
            // Icon and arena slot select the flag drawn: Alliance carriers hold the Horde flag.
            let (icon, slot) = if carrier.is_alliance { (1, 3) } else { (2, 2) };
            writer.write_u8(icon);
            writer.write_u8(slot);
        }
        Some(writer.finish(Opcode::SMSG_BATTLEGROUND_PLAYER_POSITIONS))
    }
}

/// SMSG_BATTLEGROUND_PLAYER_JOINED / _LEFT — a teammate entered or left the match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmsgBattlegroundPlayerJoinedOrLeft {
    pub player_guid: ObjectGuid,
    pub joined: bool,
}

impl SmsgBattlegroundPlayerJoinedOrLeft {
    fn opcode(&self) -> Opcode {
        if self.joined {
            Opcode::SMSG_BATTLEGROUND_PLAYER_JOINED
        } else {
            Opcode::SMSG_BATTLEGROUND_PLAYER_LEFT
        }
    }
}

impl ToWorldPacket for SmsgBattlegroundPlayerJoinedOrLeft {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(self.opcode());
        packet.write_u64(self.player_guid.raw());
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.player_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        Some(writer.finish(self.opcode()))
    }
}

/// SMSG_GROUP_JOINED_BATTLEGROUND — vanilla only: the map a group was queued for, or a
/// [`group_join`](crate::game::battleground::group_join) failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmsgGroupJoinedBattleground {
    pub result: i32,
}

impl ToWorldPacket for SmsgGroupJoinedBattleground {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GROUP_JOINED_BATTLEGROUND);
        packet.write_u32(self.result as u32);
        packet
    }
}

/// SMSG_BATTLEFIELD_WIN / SMSG_BATTLEFIELD_LOSE — vanilla only, empty: plays the result fanfare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmsgBattlefieldResult {
    pub won: bool,
}

impl ToWorldPacket for SmsgBattlefieldResult {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(if self.won {
            Opcode::SMSG_BATTLEFIELD_WIN
        } else {
            Opcode::SMSG_BATTLEFIELD_LOSE
        })
    }
}

/// SMSG_PVP_CREDIT — honour from a kill, shown in the combat log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmsgPvpCredit {
    pub honor: u32,
    pub victim_guid: ObjectGuid,
    pub victim_rank: u32,
}

impl ToWorldPacket for SmsgPvpCredit {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_PVP_CREDIT);
        packet.write_u32(self.honor);
        packet.write_u64(self.victim_guid.raw());
        packet.write_u32(self.victim_rank);
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_i32(self.honor as i32); // original honor
        writer.write_i32(self.honor as i32);
        let (high, low) = self.victim_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(self.victim_rank);
        Some(writer.finish(Opcode::SMSG_PVP_CREDIT))
    }
}

/// SMSG_PLAY_SOUND — play a sound for the receiving client (not positioned).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmsgPlaySound {
    pub sound_id: u32,
}

impl ToWorldPacket for SmsgPlaySound {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_PLAY_SOUND);
        packet.write_u32(self.sound_id);
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.sound_id);
        writer.write_packed_guid_128(0, 0); // no source object
        writer.write_i32(0); // broadcast text
        Some(writer.finish(Opcode::SMSG_PLAY_SOUND))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(state: BattlefieldState) -> SmsgBattlefieldStatus {
        SmsgBattlefieldStatus {
            player_guid: ObjectGuid::from_raw(5),
            queue_slot: 1,
            ticket_time: 1_000,
            bg_type: BattleGroundTypeId::WarsongGulch,
            bracket: 2,
            min_level: 30,
            max_level: 39,
            client_instance_id: 7,
            state,
        }
    }

    #[test]
    fn vanilla_status_layouts_follow_the_state() {
        let empty = status(BattlefieldState::None {
            bg_type: BattleGroundTypeId::WarsongGulch,
        })
        .to_vanilla();
        assert_eq!(empty.contents(), &[1u8, 0, 0, 0, 0, 0, 0, 0][..]);

        let queued = status(BattlefieldState::Queued {
            average_wait_ms: 10,
            waited_ms: 20,
            as_group: false,
        })
        .to_vanilla();
        // slot, map, bracket, instance, status, average, waited
        assert_eq!(queued.contents().len(), 4 + 4 + 1 + 4 + 4 + 4 + 4);
        assert_eq!(&queued.contents()[4..8], &489u32.to_le_bytes());
        assert_eq!(queued.contents()[8], 2);
        assert_eq!(&queued.contents()[13..17], &1u32.to_le_bytes());

        // The invitation writes only its timeout.
        let invite = status(BattlefieldState::NeedConfirmation { timeout_ms: 80_000 }).to_vanilla();
        assert_eq!(invite.contents().len(), 4 + 4 + 1 + 4 + 4 + 4);
        assert_eq!(&invite.contents()[13..17], &2u32.to_le_bytes());
    }

    #[test]
    fn modern_status_picks_the_packet_for_the_state() {
        let cases = [
            (
                BattlefieldState::None {
                    bg_type: BattleGroundTypeId::ArathiBasin,
                },
                Opcode::SMSG_BATTLEFIELD_STATUS_FAILED,
            ),
            (
                BattlefieldState::Queued {
                    average_wait_ms: 0,
                    waited_ms: 0,
                    as_group: false,
                },
                Opcode::SMSG_BATTLEFIELD_STATUS_QUEUED,
            ),
            (
                BattlefieldState::NeedConfirmation { timeout_ms: 1 },
                Opcode::SMSG_BATTLEFIELD_STATUS_NEED_CONFIRMATION,
            ),
            (
                BattlefieldState::Active {
                    shutdown_ms: 0,
                    elapsed_ms: 0,
                },
                Opcode::SMSG_BATTLEFIELD_STATUS_ACTIVE,
            ),
        ];
        for (state, opcode) in cases {
            assert_eq!(status(state).to_modern().unwrap().opcode(), opcode);
        }
    }

    #[test]
    fn vanilla_list_names_the_battlemaster_and_bracket() {
        let list = SmsgBattlefieldList {
            battlemaster_guid: ObjectGuid::from_raw(0x1234),
            bg_type: BattleGroundTypeId::WarsongGulch,
            bracket: 3,
            min_level: 40,
            max_level: 49,
            client_instance_ids: vec![1, 2],
        }
        .to_vanilla();
        let data = list.contents();
        assert_eq!(&data[0..8], &0x1234u64.to_le_bytes());
        assert_eq!(&data[8..12], &489u32.to_le_bytes());
        assert_eq!(data[12], 3);
        assert_eq!(&data[13..17], &2u32.to_le_bytes());
        assert_eq!(data.len(), 17 + 8);
    }

    #[test]
    fn vanilla_scoreboard_writes_the_winner_only_when_ended() {
        let running = SmsgPvpLogData {
            winner: None,
            players: vec![],
        }
        .to_vanilla();
        assert_eq!(running.contents(), &[0u8, 0, 0, 0, 0][..]);
        let ended = SmsgPvpLogData {
            winner: Some(BattleGroundWinner::Horde),
            players: vec![],
        }
        .to_vanilla();
        assert_eq!(ended.contents(), &[1u8, 0, 0, 0, 0, 0][..]);
    }

    #[test]
    fn world_state_update_is_two_words_on_vanilla() {
        let packet = SmsgUpdateWorldState {
            state_id: 1581,
            value: -1,
        }
        .to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_UPDATE_WORLD_STATE);
        assert_eq!(&packet.contents()[0..4], &1581u32.to_le_bytes());
        assert_eq!(&packet.contents()[4..8], &u32::MAX.to_le_bytes());
    }
}
