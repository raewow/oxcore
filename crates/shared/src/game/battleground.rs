//! Battleground identifiers and states shared by the server and the packet layer.

/// A battleground type. The number is also the 1.14 `BattlemasterList` id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum BattleGroundTypeId {
    None = 0,
    AlteracValley = 1,
    WarsongGulch = 2,
    ArathiBasin = 3,
}

impl BattleGroundTypeId {
    /// Every playable type, in id order.
    pub const ALL: [BattleGroundTypeId; 3] = [
        BattleGroundTypeId::AlteracValley,
        BattleGroundTypeId::WarsongGulch,
        BattleGroundTypeId::ArathiBasin,
    ];

    pub fn as_u32(self) -> u32 {
        self as u32
    }

    pub fn from_u32(value: u32) -> Self {
        match value {
            1 => BattleGroundTypeId::AlteracValley,
            2 => BattleGroundTypeId::WarsongGulch,
            3 => BattleGroundTypeId::ArathiBasin,
            _ => BattleGroundTypeId::None,
        }
    }

    pub fn map_id(self) -> u32 {
        match self {
            BattleGroundTypeId::AlteracValley => 30,
            BattleGroundTypeId::WarsongGulch => 489,
            BattleGroundTypeId::ArathiBasin => 529,
            BattleGroundTypeId::None => 0,
        }
    }

    pub fn from_map_id(map_id: u32) -> Self {
        match map_id {
            30 => BattleGroundTypeId::AlteracValley,
            489 => BattleGroundTypeId::WarsongGulch,
            529 => BattleGroundTypeId::ArathiBasin,
            _ => BattleGroundTypeId::None,
        }
    }

    /// The 1.14 `BattlemasterList` id, which 1.14 uses where vanilla names a map.
    pub fn battlemaster_list_id(self) -> u32 {
        self as u32
    }
}

/// Where a player stands with a battleground, as the status packet reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BattleGroundStatus {
    /// Not queued; clears the queue slot.
    None = 0,
    /// Waiting in the queue.
    WaitQueue = 1,
    /// Invited and asked to accept.
    WaitJoin = 2,
    /// Inside a running (or starting) match.
    InProgress = 3,
    /// The match is over and players are about to be sent home.
    WaitLeave = 4,
}

impl BattleGroundStatus {
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

/// The winner byte of the scoreboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BattleGroundWinner {
    Horde = 0,
    Alliance = 1,
    None = 2,
}

impl BattleGroundWinner {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Group-join results reported with `SMSG_GROUP_JOINED_BATTLEGROUND`. Positive values are the map
/// id the group was queued for.
pub mod group_join {
    /// A member has the Deserter debuff.
    pub const DESERTERS: i32 = -2;
    /// The member could not be queued.
    pub const FAILED: i32 = -1;
}

/// The Deserter debuff that keeps a player out of battlegrounds.
pub const SPELL_DESERTER: u32 = 26013;
/// The aura a ghost carries while queued for the next resurrection wave.
pub const SPELL_WAITING_TO_RESURRECT: u32 = 2584;
/// A player may be in this many battleground queues at once.
pub const MAX_QUEUE_SLOTS: usize = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_ids_round_trip_through_maps() {
        for bg in BattleGroundTypeId::ALL {
            assert_eq!(BattleGroundTypeId::from_map_id(bg.map_id()), bg);
            assert_eq!(BattleGroundTypeId::from_u32(bg.as_u32()), bg);
        }
        assert_eq!(BattleGroundTypeId::from_map_id(0), BattleGroundTypeId::None);
    }

    #[test]
    fn statuses_use_the_wire_numbers() {
        assert_eq!(BattleGroundStatus::WaitQueue.as_u32(), 1);
        assert_eq!(BattleGroundStatus::WaitJoin.as_u32(), 2);
        assert_eq!(BattleGroundStatus::InProgress.as_u32(), 3);
        assert_eq!(BattleGroundStatus::WaitLeave.as_u32(), 4);
    }
}
