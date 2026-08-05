//! Group loot roll state.
//!
//! Ported from `Group.h:88-98` (`RollVote`) and the per-group `RollId` map in `Group.cpp`.
//! A [`Roll`] tracks one loot item being rolled on by a group; votes are accumulated and
//! resolved once the timer expires or every eligible member has voted.

use oxcore_shared::protocol::ObjectGuid;
use std::collections::HashMap;

/// How long a roll stays open before it resolves, in milliseconds (`Group.cpp:66`).
pub const LOOT_ROLL_TIMEOUT: u64 = 60_000;

/// A player's vote on a loot roll (`Group.h:88-98`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RollVote {
    Pass = 0,
    Need = 1,
    Greed = 2,
    #[default]
    NotEmittedYet = 3,
    NotValid = 4,
}

/// Roll mask: which vote types the client may send.
pub const ROLL_MASK_PASS: u8 = 0x01;
pub const ROLL_MASK_GREED: u8 = 0x02;
pub const ROLL_MASK_NEED: u8 = 0x04;

/// One in-progress roll for a single loot item.
#[derive(Debug, Clone)]
pub struct Roll {
    /// The creature/corpse the loot belongs to.
    pub looted_target: ObjectGuid,
    /// 0-based slot of the item inside the loot window.
    pub item_slot: u8,
    pub item_id: u32,
    pub count: u32,
    pub random_property_id: u32,
    pub votes: HashMap<ObjectGuid, RollVote>,
    pub total_need: u32,
    pub total_greed: u32,
    pub total_pass: u32,
    /// Members still allowed to roll (set when the roll starts).
    pub total_players_rolling: u32,
    /// Remaining time in milliseconds.
    pub timer_ms: u64,
}

impl Roll {
    pub fn new(
        looted_target: ObjectGuid,
        item_slot: u8,
        item_id: u32,
        count: u32,
        random_property_id: u32,
    ) -> Self {
        Self {
            looted_target,
            item_slot,
            item_id,
            count,
            random_property_id,
            votes: HashMap::new(),
            total_need: 0,
            total_greed: 0,
            total_pass: 0,
            total_players_rolling: 0,
            timer_ms: LOOT_ROLL_TIMEOUT,
        }
    }

    /// Record a vote for a member, returning true if it was accepted.
    /// A `NotEmittedYet` placeholder (set when the roll starts) may be overwritten by a real
    /// vote; only an already-cast vote is rejected.
    pub fn add_vote(&mut self, player: ObjectGuid, vote: RollVote) -> bool {
        if let Some(existing) = self.votes.get(&player) {
            if *existing != RollVote::NotEmittedYet {
                return false;
            }
        }
        self.votes.insert(player, vote);
        match vote {
            RollVote::Need => self.total_need += 1,
            RollVote::Greed => self.total_greed += 1,
            RollVote::Pass => self.total_pass += 1,
            _ => {}
        }
        true
    }

    /// Whether every eligible member has cast a real vote (the `NotEmittedYet`
    /// placeholders seeded at roll start do not count).
    pub fn all_voted(&self) -> bool {
        let cast = self
            .votes
            .values()
            .filter(|v| **v != RollVote::NotEmittedYet)
            .count() as u32;
        cast >= self.total_players_rolling
    }

    /// True while no vote beats it and a higher vote (need) has not been cast.
    /// Need beats greed; greed beats pass.
    pub fn has_need(&self) -> bool {
        self.total_need > 0
    }

    pub fn has_greed(&self) -> bool {
        self.total_greed > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guid(n: u32) -> ObjectGuid {
        ObjectGuid::new_player(n)
    }

    #[test]
    fn votes_tally_and_track_members() {
        let mut roll = Roll::new(guid(9), 0, 12345, 1, 0);
        roll.total_players_rolling = 3;

        assert!(roll.add_vote(guid(1), RollVote::Need));
        assert!(roll.add_vote(guid(2), RollVote::Greed));
        assert!(!roll.all_voted());
        assert!(roll.has_need());

        assert!(!roll.add_vote(guid(1), RollVote::Pass), "double vote rejected");
        assert!(roll.add_vote(guid(3), RollVote::Pass));
        assert!(roll.all_voted());

        assert_eq!(roll.total_need, 1);
        assert_eq!(roll.total_greed, 1);
        assert_eq!(roll.total_pass, 1);
    }

    #[test]
    fn need_beats_greed_beats_pass() {
        let mut roll = Roll::new(guid(9), 0, 12345, 1, 0);
        roll.total_players_rolling = 2;
        roll.add_vote(guid(1), RollVote::Greed);
        roll.add_vote(guid(2), RollVote::Pass);
        assert!(roll.has_greed());
        assert!(!roll.has_need());

        let mut need_roll = Roll::new(guid(9), 0, 12345, 1, 0);
        need_roll.total_players_rolling = 2;
        need_roll.add_vote(guid(1), RollVote::Greed);
        need_roll.add_vote(guid(2), RollVote::Need);
        assert!(need_roll.has_need());
    }
}
