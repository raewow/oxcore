//! Battlegrounds: queues, matches and the players in them.
//!
//! This is the part shared by every battleground — who queues, who is
//! invited, the copy a match runs in, its clock, scoreboard, resurrection
//! waves and leaving. What happens on the field (flags, bases, towers) is the
//! battleground's Lua script, the same objective scripts open-world PvP uses.

pub mod data;
pub mod insignia;
pub mod matches;
pub mod player;
pub mod queue;
pub mod system;

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use dashmap::DashMap;
use oxcore_shared::game::battleground::BattleGroundTypeId;
use parking_lot::{Mutex, RwLock};

pub use data::{BattlegroundData, BattlegroundTemplate, EntranceTrigger};
pub use matches::{Match, MatchStatus, Score, StartStage};
pub use player::{EntryPoint, PlayerBattleground};
pub use queue::{BgQueue, GroupId};

/// Owns battleground definitions, queues and running matches.
pub struct BattlegroundManager {
    data: RwLock<BattlegroundData>,
    pub(crate) queues: Mutex<HashMap<BattleGroundTypeId, BgQueue>>,
    pub(crate) matches: DashMap<u32, Match>,
    client_ids: Mutex<HashMap<(BattleGroundTypeId, u8), BTreeSet<u32>>>,
    scheduled: Mutex<HashSet<(BattleGroundTypeId, u8)>>,
    /// Invitations to remind about: (when, player, match).
    pub(crate) reminders: Mutex<Vec<(u64, oxcore_shared::protocol::ObjectGuid, u32)>>,
    testing: AtomicBool,
    epoch: Instant,
}

impl Default for BattlegroundManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BattlegroundManager {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(BattlegroundData::default()),
            queues: Mutex::new(HashMap::new()),
            matches: DashMap::new(),
            client_ids: Mutex::new(HashMap::new()),
            scheduled: Mutex::new(HashSet::new()),
            reminders: Mutex::new(Vec::new()),
            testing: AtomicBool::new(false),
            epoch: Instant::now(),
        }
    }

    pub fn set_data(&self, data: BattlegroundData) {
        *self.data.write() = data;
    }

    /// Milliseconds on the manager's own clock.
    pub fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    pub fn template(&self, bg_type: BattleGroundTypeId) -> Option<BattlegroundTemplate> {
        self.data.read().templates.get(&bg_type).cloned()
    }

    pub fn battlemaster_bg(&self, entry: u32) -> Option<BattleGroundTypeId> {
        self.data.read().battlemasters.get(&entry).copied()
    }

    pub fn entrance(&self, trigger_id: u32) -> Option<EntranceTrigger> {
        self.data.read().entrances.get(&trigger_id).cloned()
    }

    pub fn template_count(&self) -> usize {
        self.data.read().templates.len()
    }

    /// Testing mode starts matches with a single player on either side.
    pub fn toggle_testing(&self) -> bool {
        !self.testing.fetch_xor(true, Ordering::Relaxed)
    }

    pub fn is_testing(&self) -> bool {
        self.testing.load(Ordering::Relaxed)
    }

    /// A copy of a match.
    pub fn get_match(&self, instance_id: u32) -> Option<Match> {
        self.matches.get(&instance_id).map(|m| m.clone())
    }

    /// Change a match. The closure must not call back into the manager.
    pub fn with_match<R>(&self, instance_id: u32, f: impl FnOnce(&mut Match) -> R) -> Option<R> {
        self.matches.get_mut(&instance_id).map(|mut m| f(&mut m))
    }

    pub fn is_match_copy(&self, map_id: u32, instance_id: u32) -> bool {
        self.matches
            .get(&instance_id)
            .is_some_and(|m| m.map_id == map_id)
    }

    pub fn match_ids(&self) -> Vec<u32> {
        self.matches.iter().map(|m| *m.key()).collect()
    }

    /// Matches of a battleground and bracket.
    pub fn matches_of(&self, bg_type: BattleGroundTypeId, bracket: u8) -> Vec<u32> {
        let mut ids: Vec<u32> = self
            .matches
            .iter()
            .filter(|m| m.bg_type == bg_type && m.bracket == bracket)
            .map(|m| *m.key())
            .collect();
        ids.sort_unstable();
        ids
    }

    /// The match whose client-visible number is `client_id`.
    pub fn match_by_client_id(&self, bg_type: BattleGroundTypeId, bracket: u8, client_id: u32) -> Option<u32> {
        self.matches
            .iter()
            .find(|m| m.bg_type == bg_type && m.bracket == bracket && m.client_instance_id == client_id)
            .map(|m| *m.key())
    }

    /// The lowest unused client-visible number for a bracket.
    pub fn take_client_id(&self, bg_type: BattleGroundTypeId, bracket: u8) -> u32 {
        let mut ids = self.client_ids.lock();
        let used = ids.entry((bg_type, bracket)).or_default();
        let mut id = 1;
        while used.contains(&id) {
            id += 1;
        }
        used.insert(id);
        id
    }

    pub fn release_client_id(&self, bg_type: BattleGroundTypeId, bracket: u8, id: u32) {
        if let Some(used) = self.client_ids.lock().get_mut(&(bg_type, bracket)) {
            used.remove(&id);
        }
    }

    pub fn client_ids(&self, bg_type: BattleGroundTypeId, bracket: u8) -> Vec<u32> {
        self.client_ids
            .lock()
            .get(&(bg_type, bracket))
            .map(|ids| ids.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Ask for a queue pass on the next tick.
    pub fn schedule(&self, bg_type: BattleGroundTypeId, bracket: u8) {
        self.scheduled.lock().insert((bg_type, bracket));
    }

    pub(crate) fn take_scheduled(&self) -> Vec<(BattleGroundTypeId, u8)> {
        self.scheduled.lock().drain().collect()
    }

    pub(crate) fn with_queue<R>(&self, bg_type: BattleGroundTypeId, f: impl FnOnce(&mut BgQueue) -> R) -> R {
        let mut queues = self.queues.lock();
        f(queues.entry(bg_type).or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_ids_fill_the_lowest_gap() {
        let mgr = BattlegroundManager::new();
        let wsg = BattleGroundTypeId::WarsongGulch;
        assert_eq!(mgr.take_client_id(wsg, 0), 1);
        assert_eq!(mgr.take_client_id(wsg, 0), 2);
        assert_eq!(mgr.take_client_id(wsg, 1), 1);
        mgr.release_client_id(wsg, 0, 1);
        assert_eq!(mgr.take_client_id(wsg, 0), 1);
        assert_eq!(mgr.client_ids(wsg, 0), vec![1, 2]);
    }

    #[test]
    fn testing_mode_toggles() {
        let mgr = BattlegroundManager::new();
        assert!(mgr.toggle_testing());
        assert!(mgr.is_testing());
        assert!(!mgr.toggle_testing());
    }
}
