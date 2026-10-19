//! A battleground queue: who waits, in which bracket, and how waiting groups
//! are picked for a new match or for free places in a running one.
//!
//! Groups queue as one unit and are invited together. Premade groups (large
//! enough groups queued together) are first matched against each other; after
//! waiting too long they fall back into the normal queue.

use std::collections::HashMap;

use oxcore_shared::game::chat::Team;
use oxcore_shared::protocol::ObjectGuid;

pub type GroupId = u64;

/// How many recent waits the average queue time is taken over.
const WAIT_SAMPLES: usize = 10;
/// An offline queued player is dropped after this long.
pub const OFFLINE_QUEUE_TIME_MS: u64 = 5 * 60 * 1000;

/// Which list of a bracket a group waits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum List {
    PremadeAlliance,
    PremadeHorde,
    NormalAlliance,
    NormalHorde,
}

impl List {
    fn of(team: Team, premade: bool) -> Self {
        match (premade, team == Team::Horde) {
            (true, false) => List::PremadeAlliance,
            (true, true) => List::PremadeHorde,
            (false, false) => List::NormalAlliance,
            (false, true) => List::NormalHorde,
        }
    }

    fn normal(team_index: usize) -> Self {
        if team_index == 0 {
            List::NormalAlliance
        } else {
            List::NormalHorde
        }
    }

    fn premade(team_index: usize) -> Self {
        if team_index == 0 {
            List::PremadeAlliance
        } else {
            List::PremadeHorde
        }
    }
}

/// 0 for Alliance, 1 for Horde.
pub fn team_index(team: Team) -> usize {
    usize::from(team == Team::Horde)
}

/// One queued group (a solo player is a group of one).
#[derive(Debug, Clone)]
pub struct QueuedGroup {
    pub id: GroupId,
    pub team: Team,
    pub players: Vec<ObjectGuid>,
    pub bracket: u8,
    pub join_ms: u64,
    /// Unix time of joining, shown by 1.14.
    pub join_unix: i64,
    pub premade: bool,
    /// The copy the group asked for (client instance id), or 0 for any.
    pub desired_instance: u32,
    /// The match the group is invited to (its copy's instance id), or 0.
    pub invited_to: u32,
    /// When an unanswered invitation lapses.
    pub remove_invite_at_ms: u64,
}

#[derive(Debug, Clone, Copy)]
struct QueuedPlayer {
    group: GroupId,
    online: bool,
    last_online_ms: u64,
}

/// Groups picked for one side of a match.
#[derive(Debug, Default, Clone)]
pub struct SelectionPool {
    pub groups: Vec<GroupId>,
    pub player_count: u32,
}

impl SelectionPool {
    /// Add a group if it fits in `desired`. Returns whether more could be
    /// added (or the group was added).
    fn add_group(&mut self, group: &QueuedGroup, desired: u32, instance: u32) -> bool {
        let size = group.players.len() as u32;
        if group.invited_to == 0
            && (group.desired_instance == 0 || group.desired_instance == instance)
            && desired >= self.player_count + size
        {
            self.groups.push(group.id);
            self.player_count += size;
            return true;
        }
        self.player_count < desired
    }

    /// Drop the group closest to `size` (or the largest). Returns whether a
    /// replacement should be looked for.
    fn kick_group(&mut self, size: u32, groups: &HashMap<GroupId, QueuedGroup>) -> bool {
        if self.player_count == 0 || self.groups.is_empty() {
            return true;
        }
        let size_of = |id: &GroupId| groups.get(id).map(|g| g.players.len() as u32).unwrap_or(0);
        let mut kick = 0;
        let mut found = false;
        for (i, id) in self.groups.iter().enumerate() {
            let len = size_of(id);
            if len.abs_diff(size) <= 1 {
                kick = i;
                found = true;
            } else if !found && len >= size_of(&self.groups[kick]) {
                kick = i;
            }
        }
        let removed = self.groups.remove(kick);
        let len = size_of(&removed);
        self.player_count -= len;
        len > size + 1
    }
}

/// The queue for one battleground type.
#[derive(Debug, Default)]
pub struct BgQueue {
    next_id: GroupId,
    groups: HashMap<GroupId, QueuedGroup>,
    lists: HashMap<(u8, List), Vec<GroupId>>,
    players: HashMap<ObjectGuid, QueuedPlayer>,
    wait_times: HashMap<(usize, u8), Vec<u64>>,
}

impl BgQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a group. Returns its id.
    pub fn add_group(
        &mut self,
        team: Team,
        players: Vec<ObjectGuid>,
        bracket: u8,
        premade: bool,
        desired_instance: u32,
        now_ms: u64,
        now_unix: i64,
    ) -> GroupId {
        self.next_id += 1;
        let id = self.next_id;
        for &player in &players {
            self.players.insert(
                player,
                QueuedPlayer {
                    group: id,
                    online: true,
                    last_online_ms: 0,
                },
            );
        }
        self.lists
            .entry((bracket, List::of(team, premade)))
            .or_default()
            .push(id);
        self.groups.insert(
            id,
            QueuedGroup {
                id,
                team,
                players,
                bracket,
                join_ms: now_ms,
                join_unix: now_unix,
                premade,
                desired_instance,
                invited_to: 0,
                remove_invite_at_ms: 0,
            },
        );
        id
    }

    pub fn group(&self, id: GroupId) -> Option<&QueuedGroup> {
        self.groups.get(&id)
    }

    /// The group a player waits in.
    pub fn group_of(&self, player: ObjectGuid) -> Option<&QueuedGroup> {
        self.players
            .get(&player)
            .and_then(|p| self.groups.get(&p.group))
    }

    pub fn contains(&self, player: ObjectGuid) -> bool {
        self.players.contains_key(&player)
    }

    /// Take a player out of the queue. Returns the group they were in (as it
    /// was), so the caller can release an invitation slot.
    pub fn remove_player(&mut self, player: ObjectGuid) -> Option<QueuedGroup> {
        let queued = self.players.remove(&player)?;
        let group = self.groups.get_mut(&queued.group)?;
        let before = group.clone();
        group.players.retain(|&p| p != player);
        if group.players.is_empty() {
            let list = List::of(group.team, group.premade);
            let key = (group.bracket, list);
            self.groups.remove(&queued.group);
            for list in [list, List::of(before.team, !before.premade)] {
                if let Some(ids) = self.lists.get_mut(&(key.0, list)) {
                    ids.retain(|&id| id != queued.group);
                }
            }
        }
        Some(before)
    }

    pub fn set_online(&mut self, player: ObjectGuid, online: bool, now_ms: u64) {
        if let Some(p) = self.players.get_mut(&player) {
            p.online = online;
            p.last_online_ms = now_ms;
        }
    }

    /// Players whose offline grace ran out.
    pub fn expired_offline(&self, now_ms: u64) -> Vec<ObjectGuid> {
        self.players
            .iter()
            .filter(|(_, p)| !p.online && now_ms.saturating_sub(p.last_online_ms) > OFFLINE_QUEUE_TIME_MS)
            .map(|(&guid, _)| guid)
            .collect()
    }

    /// Players whose invitation lapsed unanswered.
    pub fn expired_invites(&self, now_ms: u64) -> Vec<(ObjectGuid, GroupId)> {
        self.groups
            .values()
            .filter(|g| g.invited_to != 0 && now_ms >= g.remove_invite_at_ms)
            .flat_map(|g| g.players.iter().map(move |&p| (p, g.id)))
            .collect()
    }

    /// Players invited to a match.
    pub fn invited_to(&self, instance_id: u32) -> Vec<ObjectGuid> {
        self.groups
            .values()
            .filter(|g| g.invited_to == instance_id)
            .flat_map(|g| g.players.iter().copied())
            .collect()
    }

    pub fn has_players(&self, bracket: u8) -> bool {
        self.lists
            .iter()
            .any(|((b, _), ids)| *b == bracket && !ids.is_empty())
    }

    /// Mark a group invited to a match. Returns its players and team.
    pub fn invite(
        &mut self,
        id: GroupId,
        instance_id: u32,
        invite_ms: u64,
        now_ms: u64,
    ) -> Option<(Vec<ObjectGuid>, Team)> {
        let group = self.groups.get_mut(&id)?;
        if group.invited_to != 0 {
            return None;
        }
        group.invited_to = instance_id;
        group.remove_invite_at_ms = now_ms + invite_ms;
        let waited = now_ms.saturating_sub(group.join_ms);
        let (players, team, bracket) = (group.players.clone(), group.team, group.bracket);
        let samples = self
            .wait_times
            .entry((team_index(team), bracket))
            .or_default();
        for _ in 0..players.len() {
            samples.push(waited);
            if samples.len() > WAIT_SAMPLES {
                samples.remove(0);
            }
        }
        Some((players, team))
    }

    /// Average wait over the last few invitations, or 0 until there are enough.
    pub fn average_wait(&self, team: Team, bracket: u8) -> u32 {
        match self.wait_times.get(&(team_index(team), bracket)) {
            Some(samples) if samples.len() >= WAIT_SAMPLES => {
                (samples.iter().sum::<u64>() / WAIT_SAMPLES as u64) as u32
            }
            _ => 0,
        }
    }

    fn list(&self, bracket: u8, list: List) -> Vec<GroupId> {
        self.lists.get(&(bracket, list)).cloned().unwrap_or_default()
    }

    /// Pick waiting groups for the free places of a running match.
    /// With `balance`, large groups are dropped until both sides are even.
    pub fn fill_free_slots(
        &self,
        bracket: u8,
        free: [u32; 2],
        client_instance_id: u32,
        balance: bool,
    ) -> [SelectionPool; 2] {
        let mut pools = [SelectionPool::default(), SelectionPool::default()];
        let lists = [self.list(bracket, List::normal(0)), self.list(bracket, List::normal(1))];
        let mut cursors = [0usize; 2];
        for side in 0..2 {
            while cursors[side] < lists[side].len() {
                let Some(group) = self.groups.get(&lists[side][cursors[side]]) else {
                    cursors[side] += 1;
                    continue;
                };
                if !pools[side].add_group(group, free[side], client_instance_id) {
                    break;
                }
                cursors[side] += 1;
            }
        }
        if !balance {
            return pools;
        }
        let diff = |pools: &[SelectionPool; 2], side: usize| free[side] as i64 - pools[side].player_count as i64;
        while (diff(&pools, 0) - diff(&pools, 1)).abs() > 1
            && (pools[0].player_count > 0 || pools[1].player_count > 0)
        {
            let (low, high) = if diff(&pools, 0) < diff(&pools, 1) { (0, 1) } else { (1, 0) };
            let gap = (diff(&pools, high) - diff(&pools, low)) as u32;
            if pools[low].kick_group(gap, &self.groups) {
                let room = free[low].saturating_sub(diff(&pools, high).max(0) as u32);
                while cursors[low] < lists[low].len() {
                    let Some(group) = self.groups.get(&lists[low][cursors[low]]) else {
                        cursors[low] += 1;
                        continue;
                    };
                    if !pools[low].add_group(group, room, client_instance_id) {
                        break;
                    }
                    cursors[low] += 1;
                }
            }
            if pools[low].player_count == 0 {
                if free[low] as i64 <= diff(&pools, high) + 1 {
                    break;
                }
                pools[high].kick_group(gap, &self.groups);
            }
        }
        pools
    }

    /// Try to match premade groups against each other. Premades that waited
    /// longer than `premade_wait_ms` (or are too small for a side) move to the
    /// normal queue when no match is found.
    pub fn check_premade_match(
        &mut self,
        bracket: u8,
        min_per_team: u32,
        max_per_team: u32,
        premade_min_size: u32,
        premade_wait_ms: u64,
        now_ms: u64,
        testing: bool,
    ) -> Option<[SelectionPool; 2]> {
        let mut pools = [SelectionPool::default(), SelectionPool::default()];
        for side in 0..2 {
            for list in [List::premade(side), List::normal(side)] {
                if pools[side].player_count >= min_per_team {
                    break;
                }
                for id in self.list(bracket, list) {
                    let Some(group) = self.groups.get(&id) else {
                        continue;
                    };
                    if group.invited_to == 0 && group.players.len() as u32 >= premade_min_size {
                        pools[side].add_group(group, max_per_team, 0);
                        if pools[side].player_count >= min_per_team {
                            break;
                        }
                    }
                }
            }
        }
        if testing && (pools[0].player_count > 0 || pools[1].player_count > 0) {
            return Some(pools);
        }
        if pools[0].player_count >= min_per_team && pools[1].player_count >= min_per_team {
            return Some(pools);
        }
        // No match: the oldest premade of each side gives up waiting.
        for side in 0..2 {
            let Some(&first) = self.lists.get(&(bracket, List::premade(side))).and_then(|ids| ids.first())
            else {
                continue;
            };
            let Some(group) = self.groups.get_mut(&first) else {
                continue;
            };
            if group.invited_to == 0
                && (group.join_ms + premade_wait_ms < now_ms || (group.players.len() as u32) < min_per_team)
            {
                group.premade = false;
                if let Some(ids) = self.lists.get_mut(&(bracket, List::premade(side))) {
                    ids.retain(|&id| id != first);
                }
                self.lists
                    .entry((bracket, List::normal(side)))
                    .or_default()
                    .push(first);
            }
        }
        None
    }

    /// Try to fill both sides of a new match from the normal queue. With
    /// `balance`, the larger side is trimmed to within two players.
    pub fn check_normal_match(
        &self,
        bracket: u8,
        min_per_team: u32,
        max_per_team: u32,
        balance: bool,
        testing: bool,
    ) -> Option<[SelectionPool; 2]> {
        let mut pools = [SelectionPool::default(), SelectionPool::default()];
        let lists = [self.list(bracket, List::normal(0)), self.list(bracket, List::normal(1))];
        let mut stopped_at = [lists[0].len(), lists[1].len()];
        for side in 0..2 {
            for (i, id) in lists[side].iter().enumerate() {
                let Some(group) = self.groups.get(id) else {
                    continue;
                };
                if group.invited_to == 0 {
                    pools[side].add_group(group, max_per_team, 0);
                    if pools[side].player_count >= min_per_team {
                        stopped_at[side] = i;
                        break;
                    }
                }
            }
        }
        if balance && pools[0].player_count >= min_per_team && pools[1].player_count >= min_per_team {
            // The smaller side tops up to the size of the larger.
            let small = if pools[1].player_count < pools[0].player_count { 1 } else { 0 };
            let target = pools[1 - small].player_count;
            for id in lists[small].iter().skip(stopped_at[small] + 1) {
                let Some(group) = self.groups.get(id) else {
                    continue;
                };
                if group.invited_to == 0 && !pools[small].add_group(group, target, 0) {
                    break;
                }
            }
            if pools[0].player_count.abs_diff(pools[1].player_count) > 2 {
                return None;
            }
        }
        if testing && (pools[0].player_count > 0 || pools[1].player_count > 0) {
            return Some(pools);
        }
        (pools[0].player_count >= min_per_team && pools[1].player_count >= min_per_team).then_some(pools)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcore_shared::protocol::HighGuid;

    fn guid(n: u32) -> ObjectGuid {
        ObjectGuid::new_without_entry(HighGuid::Player, n)
    }

    fn solo(queue: &mut BgQueue, n: u32, team: Team) -> GroupId {
        queue.add_group(team, vec![guid(n)], 0, false, 0, u64::from(n), 0)
    }

    #[test]
    fn a_normal_match_needs_the_minimum_on_both_sides() {
        let mut queue = BgQueue::new();
        solo(&mut queue, 1, Team::Alliance);
        solo(&mut queue, 2, Team::Alliance);
        solo(&mut queue, 3, Team::Horde);
        assert!(queue.check_normal_match(0, 2, 10, false, false).is_none());
        // Testing mode starts a match with anyone queued.
        assert!(queue.check_normal_match(0, 2, 10, false, true).is_some());
        solo(&mut queue, 4, Team::Horde);
        let pools = queue.check_normal_match(0, 2, 10, false, false).unwrap();
        assert_eq!(pools[0].player_count, 2);
        assert_eq!(pools[1].player_count, 2);
    }

    #[test]
    fn invited_groups_are_not_picked_again() {
        let mut queue = BgQueue::new();
        let a = solo(&mut queue, 1, Team::Alliance);
        let h = solo(&mut queue, 2, Team::Horde);
        queue.invite(a, 100, 80_000, 10).unwrap();
        queue.invite(h, 100, 80_000, 10).unwrap();
        assert!(queue.check_normal_match(0, 1, 10, false, false).is_none());
        assert_eq!(queue.invited_to(100).len(), 2);
        assert_eq!(queue.expired_invites(80_009).len(), 0);
        assert_eq!(queue.expired_invites(80_010).len(), 2);
    }

    #[test]
    fn free_slots_take_groups_that_fit() {
        let mut queue = BgQueue::new();
        queue.add_group(Team::Alliance, vec![guid(1), guid(2), guid(3)], 0, false, 0, 0, 0);
        solo(&mut queue, 4, Team::Alliance);
        let pools = queue.fill_free_slots(0, [2, 0], 1, false);
        // The group of three does not fit two places; the solo player does.
        assert_eq!(pools[0].player_count, 1);
        assert_eq!(pools[1].player_count, 0);
    }

    #[test]
    fn a_group_asking_for_another_copy_is_skipped() {
        let mut queue = BgQueue::new();
        queue.add_group(Team::Alliance, vec![guid(1)], 0, false, 7, 0, 0);
        assert_eq!(queue.fill_free_slots(0, [5, 5], 3, false)[0].player_count, 0);
        assert_eq!(queue.fill_free_slots(0, [5, 5], 7, false)[0].player_count, 1);
    }

    #[test]
    fn stale_premades_fall_back_to_the_normal_queue() {
        let mut queue = BgQueue::new();
        queue.add_group(Team::Alliance, vec![guid(1), guid(2)], 0, true, 0, 0, 0);
        solo(&mut queue, 3, Team::Horde);
        assert!(queue
            .check_premade_match(0, 2, 10, 2, 1_000, 500, false)
            .is_none());
        assert!(queue.check_normal_match(0, 1, 10, false, false).is_none());
        assert!(queue
            .check_premade_match(0, 2, 10, 2, 1_000, 2_000, false)
            .is_none());
        // Now in the normal queue, it can be matched with the solo Horde player.
        assert!(queue.check_normal_match(0, 1, 10, false, false).is_some());
    }

    #[test]
    fn leaving_removes_an_emptied_group() {
        let mut queue = BgQueue::new();
        let id = solo(&mut queue, 1, Team::Horde);
        assert!(queue.has_players(0));
        assert_eq!(queue.remove_player(guid(1)).map(|g| g.id), Some(id));
        assert!(!queue.has_players(0));
        assert!(queue.remove_player(guid(1)).is_none());
    }

    #[test]
    fn average_wait_needs_enough_samples() {
        let mut queue = BgQueue::new();
        for n in 0..WAIT_SAMPLES as u32 {
            let id = solo(&mut queue, n + 1, Team::Alliance);
            assert_eq!(queue.average_wait(Team::Alliance, 0), 0);
            queue.invite(id, 1, 80_000, u64::from(n + 1) + 100).unwrap();
        }
        assert_eq!(queue.average_wait(Team::Alliance, 0), 100);
    }
}
