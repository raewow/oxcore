//! One running battleground: its copy, players, scores and clock.

use std::collections::HashMap;

use oxcore_shared::game::battleground::{BattleGroundTypeId, BattleGroundWinner};
use oxcore_shared::game::chat::Team;
use oxcore_shared::protocol::ObjectGuid;

use super::queue::team_index;

/// Invited players have this long to accept.
pub const INVITE_ACCEPT_WAIT_MS: u32 = 80_000;
/// The invitation is shown again this long after it was sent.
pub const INVITATION_REMIND_MS: u32 = 60_000;
/// Players are sent home this long after the match ends.
pub const TIME_TO_AUTOREMOVE_MS: u32 = 120_000;
/// Spirit guides resurrect waiting ghosts this often.
pub const RESURRECTION_INTERVAL_MS: u32 = 30_000;
/// The gates are removed this long after the match was created.
pub const DOORS_DESPAWN_AGE_MS: u32 = 180_000;
/// Countdown marks before the gates open: 2 minutes, 1 minute, 30 seconds, now.
pub const START_DELAYS_MS: [i32; 4] = [120_000, 60_000, 30_000, 0];
/// Losers earn a mark only from matches that lasted this long.
pub const LOSER_MARK_MIN_AGE_MS: u32 = 10 * 60 * 1000;

pub const SOUND_BG_START: u32 = 3439;
pub const SOUND_HORDE_WINS: u32 = 8454;
pub const SOUND_ALLIANCE_WINS: u32 = 8455;

/// Where a match stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchStatus {
    /// Created; players are arriving and the start countdown runs.
    WaitJoin,
    /// The gates are open.
    InProgress,
    /// Decided; players are about to be sent home.
    WaitLeave,
}

impl MatchStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            MatchStatus::WaitJoin => "WAIT_JOIN",
            MatchStatus::InProgress => "IN_PROGRESS",
            MatchStatus::WaitLeave => "WAIT_LEAVE",
        }
    }
}

/// The start countdown's steps, as scripts hear them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartStage {
    /// The first player arrived: two minutes to go.
    Preparation,
    OneMinute,
    HalfMinute,
    /// The gates open.
    Begin,
    /// A minute after the gates opened: the gates disappear.
    DoorsDespawn,
}

impl StartStage {
    pub fn as_str(self) -> &'static str {
        match self {
            StartStage::Preparation => "PREPARATION",
            StartStage::OneMinute => "ONE_MINUTE",
            StartStage::HalfMinute => "HALF_MINUTE",
            StartStage::Begin => "BEGIN",
            StartStage::DoorsDespawn => "DOORS_DESPAWN",
        }
    }
}

/// One player's line on the scoreboard.
#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    pub team: Team,
    pub killing_blows: u32,
    pub honorable_kills: u32,
    pub deaths: u32,
    pub bonus_honor: u32,
    /// The battleground's own columns.
    pub extra: Vec<u32>,
}

impl Score {
    pub fn new(team: Team, columns: usize) -> Self {
        Self {
            team,
            killing_blows: 0,
            honorable_kills: 0,
            deaths: 0,
            bonus_honor: 0,
            extra: vec![0; columns],
        }
    }
}

/// A battleground match.
#[derive(Debug, Clone)]
pub struct Match {
    /// The copy's instance id (unique among all copies).
    pub instance_id: u32,
    pub bg_type: BattleGroundTypeId,
    pub map_id: u32,
    pub bracket: u8,
    /// The number players see for this copy.
    pub client_instance_id: u32,
    pub min_level: u8,
    pub max_level: u8,
    pub status: MatchStatus,
    /// Time since the match was created.
    pub age_ms: u32,
    /// Age at which the gates opened (0 until then).
    pub opened_at_ms: u32,
    /// Countdown to the gates; starts when the first player arrives.
    pub start_delay_ms: i32,
    /// Countdown steps already announced (bit per step).
    pub start_steps: u8,
    pub doors_despawned: bool,
    /// Countdown to sending everyone home, once decided.
    pub end_timer_ms: i32,
    pub winner: Option<Team>,
    /// Players inside, and the side each fights for.
    pub players: HashMap<ObjectGuid, Team>,
    /// Scoreboard lines, kept until the match closes.
    pub scores: HashMap<ObjectGuid, Score>,
    /// Places promised by invitation (or taken), per side.
    pub invited: [u32; 2],
    pub max_per_team: u32,
    pub min_per_team: u32,
    /// Extra scoreboard columns this battleground shows.
    pub score_columns: usize,
    /// Who carries each side's flag.
    pub flag_carriers: [Option<ObjectGuid>; 2],
    pub spirit_timer_ms: u32,
    /// Running short-handed: time left before the match closes.
    pub premature_ms: Option<u32>,
    /// Still offered to queued players.
    pub accepting: bool,
    /// Each side's raid, once someone has arrived.
    pub raids: [Option<u32>; 2],
}

impl Match {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instance_id: u32,
        bg_type: BattleGroundTypeId,
        bracket: u8,
        client_instance_id: u32,
        levels: (u8, u8),
        min_per_team: u32,
        max_per_team: u32,
        score_columns: usize,
    ) -> Self {
        Self {
            instance_id,
            bg_type,
            map_id: bg_type.map_id(),
            bracket,
            client_instance_id,
            min_level: levels.0,
            max_level: levels.1,
            status: MatchStatus::WaitJoin,
            age_ms: 0,
            opened_at_ms: 0,
            start_delay_ms: START_DELAYS_MS[0],
            start_steps: 0,
            doors_despawned: false,
            end_timer_ms: 0,
            winner: None,
            players: HashMap::new(),
            scores: HashMap::new(),
            invited: [0; 2],
            max_per_team,
            min_per_team,
            score_columns,
            flag_carriers: [None; 2],
            spirit_timer_ms: RESURRECTION_INTERVAL_MS,
            premature_ms: None,
            accepting: true,
            raids: [None; 2],
        }
    }

    pub fn players_of(&self, team: Team) -> u32 {
        self.players.values().filter(|&&t| t == team).count() as u32
    }

    /// Places still free for a side.
    pub fn free_slots(&self, team: Team) -> u32 {
        if self.status == MatchStatus::WaitLeave {
            return 0;
        }
        self.max_per_team.saturating_sub(self.invited[team_index(team)])
    }

    pub fn has_free_slots(&self) -> bool {
        (self.players.len() as u32) < self.max_per_team * 2
    }

    pub fn team_of(&self, player: ObjectGuid) -> Option<Team> {
        self.players.get(&player).copied()
    }

    /// Time since the gates opened.
    pub fn elapsed_since_open_ms(&self) -> u32 {
        if self.opened_at_ms == 0 {
            0
        } else {
            self.age_ms.saturating_sub(self.opened_at_ms)
        }
    }

    pub fn winner_byte(&self) -> Option<BattleGroundWinner> {
        (self.status == MatchStatus::WaitLeave).then(|| match self.winner {
            Some(Team::Horde) => BattleGroundWinner::Horde,
            Some(Team::Alliance) => BattleGroundWinner::Alliance,
            _ => BattleGroundWinner::None,
        })
    }

    /// Advance the start countdown by `diff_ms`, returning the steps reached.
    /// The first step fires as soon as a player is inside.
    pub fn advance_start(&mut self, diff_ms: u32) -> Vec<StartStage> {
        let mut steps = Vec::new();
        if self.status != MatchStatus::WaitJoin || self.players.is_empty() {
            return steps;
        }
        if self.start_steps & 1 == 0 {
            self.start_steps |= 1;
            self.start_delay_ms = START_DELAYS_MS[0];
            steps.push(StartStage::Preparation);
            return steps;
        }
        self.start_delay_ms -= diff_ms as i32;
        let marks = [
            (2u8, START_DELAYS_MS[1], StartStage::OneMinute),
            (4u8, START_DELAYS_MS[2], StartStage::HalfMinute),
            (8u8, START_DELAYS_MS[3], StartStage::Begin),
        ];
        for (bit, at, stage) in marks {
            if self.start_steps & bit == 0 && self.start_delay_ms <= at {
                self.start_steps |= bit;
                steps.push(stage);
                if stage == StartStage::Begin {
                    self.status = MatchStatus::InProgress;
                    self.opened_at_ms = self.age_ms.max(1);
                }
                // One announcement per tick, as the countdown would show it.
                break;
            }
        }
        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcore_shared::protocol::HighGuid;

    fn sample() -> Match {
        Match::new(10, BattleGroundTypeId::WarsongGulch, 0, 1, (10, 19), 1, 10, 2)
    }

    #[test]
    fn the_countdown_waits_for_a_player_then_steps_to_the_start() {
        let mut m = sample();
        assert!(m.advance_start(1_000).is_empty());
        m.players.insert(ObjectGuid::new_without_entry(HighGuid::Player, 1), Team::Alliance);
        assert_eq!(m.advance_start(50), vec![StartStage::Preparation]);
        assert!(m.advance_start(59_000).is_empty());
        assert_eq!(m.advance_start(1_000), vec![StartStage::OneMinute]);
        assert_eq!(m.advance_start(30_000), vec![StartStage::HalfMinute]);
        m.age_ms = 125_000;
        assert_eq!(m.advance_start(30_000), vec![StartStage::Begin]);
        assert_eq!(m.status, MatchStatus::InProgress);
        assert_eq!(m.opened_at_ms, 125_000);
        assert!(m.advance_start(1_000).is_empty());
    }

    #[test]
    fn free_slots_count_invitations() {
        let mut m = sample();
        m.invited = [3, 10];
        assert_eq!(m.free_slots(Team::Alliance), 7);
        assert_eq!(m.free_slots(Team::Horde), 0);
        m.status = MatchStatus::WaitLeave;
        assert_eq!(m.free_slots(Team::Alliance), 0);
    }

    #[test]
    fn the_winner_byte_appears_only_once_decided() {
        let mut m = sample();
        m.winner = Some(Team::Horde);
        assert_eq!(m.winner_byte(), None);
        m.status = MatchStatus::WaitLeave;
        assert_eq!(m.winner_byte(), Some(BattleGroundWinner::Horde));
        m.winner = None;
        assert_eq!(m.winner_byte(), Some(BattleGroundWinner::None));
    }
}
