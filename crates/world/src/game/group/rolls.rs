//! Group loot roll flow: group loot, need before greed, master loot, the roll lifecycle, and
//! the round-robin looter rotation.

use crate::game::broadcast_mgr::BroadcastManagerExt;
use crate::game::group::reward::is_at_group_reward_distance;
use crate::game::items::manager::ItemTemplate;
use crate::game::loot::{Loot, Roll, RollVote};
use crate::World;
use oxcore_shared::game::group::LootMethod;
use oxcore_shared::messages::group::{
    roll_bytes, SmsgLootAllPassed, SmsgLootMasterList, SmsgLootRoll, SmsgLootRollStarted,
    SmsgLootRollWon, SmsgLootRollsComplete,
};
use oxcore_shared::protocol::{ObjectGuid, Position, Protocol};
use rand::Rng;
use std::time::Duration;

/// Creature location + elite rank needed to judge reward distance for rolls.
#[derive(Debug, Clone)]
pub struct LootContext {
    pub position: Position,
    pub map_id: u32,
    pub instance_id: u32,
    pub rank: u8,
}

impl crate::game::group::GroupSystem {
    /// Resolve a looted creature's position/map/instance and template rank.
    pub fn loot_context_for(world: &World, guid: ObjectGuid) -> Option<LootContext> {
        let (position, map_id, instance_id, entry) = world
            .managers
            .creature_mgr
            .with_creature(guid, |c| (c.position, c.map_id, c.instance_id, c.entry))?;
        let rank = world
            .managers
            .creature_mgr
            .get_template(entry)
            .map(|t| t.rank)
            .unwrap_or(0);
        Some(LootContext {
            position,
            map_id,
            instance_id,
            rank,
        })
    }

    /// Group loot: roll over-threshold items, mark the rest under-threshold.
    pub async fn group_loot(
        &self,
        world: &World,
        group_id: u32,
        looted_target: ObjectGuid,
        context: LootContext,
    ) {
        self.classify_and_start_rolls(world, group_id, looted_target, &context, false)
            .await;
    }

    /// Need before greed: like group loot, but only players who can use the item are offered
    /// the roll.
    pub async fn need_before_greed(
        &self,
        world: &World,
        group_id: u32,
        looted_target: ObjectGuid,
        context: LootContext,
    ) {
        self.classify_and_start_rolls(world, group_id, looted_target, &context, true)
            .await;
    }

    /// Master loot: mark under-threshold items and send the eligible-looter list to the
    /// member who opened the corpse.
    pub async fn master_loot(
        &self,
        world: &World,
        group_id: u32,
        looted_target: ObjectGuid,
        opener: ObjectGuid,
        context: LootContext,
    ) {
        let threshold = self
            .groups
            .get(&group_id)
            .map(|g| g.loot_threshold)
            .unwrap_or(2);

        let loot_manager = &*world.systems.loot_manager;
        let Some(loot) = loot_manager.get_loot(looted_target) else {
            return;
        };
        let under: Vec<u8> = loot
            .items
            .iter()
            .filter(|item| !item.is_looted)
            .filter(|item| {
                world
                    .managers
                    .item_mgr
                    .get_template(item.item_id)
                    .map(|t| t.quality)
                    .unwrap_or(0)
                    < threshold
            })
            .map(|item| item.slot)
            .collect();
        drop(loot);
        loot_manager.with_loot_mut(looted_target, |loot| {
            for item in loot.items.iter_mut() {
                if under.contains(&item.slot) {
                    item.is_underthreshold = true;
                }
            }
        });

        // Eligible candidates: in world, within reward distance, allowed looter.
        let mut candidates = Vec::new();
        if let Some(group) = self.groups.get(&group_id) {
            for member in &group.members {
                if is_at_group_reward_distance(
                    world,
                    member.guid,
                    &context.position,
                    context.map_id,
                    context.instance_id,
                    context.rank,
                ) && self.is_allowed_looter(world, looted_target, member.guid)
                {
                    candidates.push(member.guid);
                }
            }
        }

        let msg = SmsgLootMasterList {
            loot_guid: looted_target,
            candidates,
        };
        self.broadcast_mgr.send_msg_to_player(opener, msg);
    }

    async fn classify_and_start_rolls(
        &self,
        world: &World,
        group_id: u32,
        looted_target: ObjectGuid,
        context: &LootContext,
        need_before_greed: bool,
    ) {
        let threshold = self
            .groups
            .get(&group_id)
            .map(|g| g.loot_threshold)
            .unwrap_or(2);

        let Some(loot) = world.systems.loot_manager.get_loot(looted_target) else {
            return;
        };
        let slots: Vec<(u8, u32)> = loot
            .items
            .iter()
            .filter(|i| !i.is_looted && !i.freeforall)
            .map(|i| (i.slot, i.item_id))
            .collect();
        drop(loot);

        for (slot, item_id) in slots {
            let quality = world
                .managers
                .item_mgr
                .get_template(item_id)
                .map(|t| t.quality)
                .unwrap_or(0);
            if quality >= threshold {
                let method = if need_before_greed {
                    LootMethod::NeedBeforeGreed
                } else {
                    LootMethod::GroupLoot
                };
                self.start_loot_roll(world, group_id, looted_target, context, method, slot)
                    .await;
            } else {
                world
                    .systems
                    .loot_manager
                    .with_loot_mut(looted_target, |loot| {
                        if let Some(it) = loot.items.iter_mut().find(|i| i.slot == slot) {
                            it.is_underthreshold = true;
                        }
                    });
            }
        }
    }

    /// Build the eligible-voter set, then either award straight away (a single voter) or
    /// announce the roll and block the item.
    pub async fn start_loot_roll(
        &self,
        world: &World,
        group_id: u32,
        looted_target: ObjectGuid,
        context: &LootContext,
        method: LootMethod,
        item_slot: u8,
    ) {
        let Some(loot) = world.systems.loot_manager.get_loot(looted_target) else {
            return;
        };
        let Some(item) = loot.items.iter().find(|i| i.slot == item_slot).cloned() else {
            return;
        };
        drop(loot);
        let item_template = world.managers.item_mgr.get_template(item.item_id);
        let random_property_id = 0;

        let Some(group) = self.groups.get(&group_id) else {
            return;
        };
        let members: Vec<ObjectGuid> = group.members.iter().map(|m| m.guid).collect();
        drop(group);

        let mut roll = Roll::new(
            looted_target,
            item_slot,
            item.item_id,
            item.count,
            random_property_id,
        );
        for guid in members {
            let Some(player) = self.player_mgr.get_player(guid) else {
                continue;
            };
            let can_use = if method == LootMethod::NeedBeforeGreed {
                item_template
                    .as_ref()
                    .map(|t| can_use_item(player.class, player.race, u32::from(player.level), t))
                    .unwrap_or(true)
            } else {
                true
            };
            drop(player);
            // A player passing on all loot is never offered the roll.
            if can_use
                && !self.passing_on_loot.contains(&guid)
                && Loot::item_allowed_for(&item, guid)
                && is_at_group_reward_distance(
                    world,
                    guid,
                    &context.position,
                    context.map_id,
                    context.instance_id,
                    context.rank,
                )
                && self.is_allowed_looter(world, looted_target, guid)
            {
                roll.votes.insert(guid, RollVote::NotEmittedYet);
                roll.total_players_rolling += 1;
            }
        }

        if roll.total_players_rolling == 0 {
            return;
        }

        if roll.total_players_rolling == 1 {
            self.award_single_looter(world, roll).await;
            return;
        }

        // Announce the roll to every eligible member and block the item.
        let start_msg = SmsgLootRollStarted {
            loot_guid: looted_target,
            map_id: context.map_id,
            item_slot: u32::from(item_slot),
            item_id: item.item_id,
            item_random_prop_id: random_property_id as i32,
            item_suffix_factor: 0,
            item_count: item.count,
            roll_timeout: crate::game::loot::LOOT_ROLL_TIMEOUT as u32,
            roll_type: u8::from(method),
        };
        let voters: Vec<ObjectGuid> = roll.votes.keys().copied().collect();
        self.broadcast_mgr
            .broadcast_msg_to_players(&voters, &start_msg);
        world
            .systems
            .loot_manager
            .set_item_blocked(looted_target, item_slot, true);

        self.loot_rolls.entry(group_id).or_default().push(roll);
    }

    /// Register a vote, announce it to everyone in the roll, and resolve once everyone has
    /// voted.
    pub async fn count_roll_vote(
        &self,
        world: &World,
        group_id: u32,
        player_guid: ObjectGuid,
        looted_target: ObjectGuid,
        item_slot: u8,
        vote: RollVote,
    ) {
        let (number, kind) = match vote {
            RollVote::Pass => (roll_bytes::PASS_NUMBER, roll_bytes::PASS_TYPE),
            RollVote::Need => (
                roll_bytes::NEED_SELECTED_NUMBER,
                roll_bytes::NEED_SELECTED_TYPE,
            ),
            RollVote::Greed => (
                roll_bytes::GREED_SELECTED_NUMBER,
                roll_bytes::GREED_ROLL_TYPE,
            ),
            _ => return,
        };

        let (msg, voters, all_voted) = {
            let Some(mut group_rolls) = self.loot_rolls.get_mut(&group_id) else {
                return;
            };
            let Some(roll) = group_rolls
                .iter_mut()
                .find(|r| r.looted_target == looted_target && r.item_slot == item_slot)
            else {
                return;
            };
            // Players who joined after the roll started, and second votes, are ignored.
            if !roll.votes.contains_key(&player_guid) || !roll.add_vote(player_guid, vote) {
                return;
            }
            let msg = SmsgLootRoll {
                loot_guid: looted_target,
                player_guid,
                item_slot: u32::from(item_slot),
                item_id: roll.item_id,
                item_random_prop_id: roll.random_property_id as i32,
                item_count: roll.count,
                roll_number: number,
                roll_type: kind,
            };
            let voters: Vec<ObjectGuid> = roll.votes.keys().copied().collect();
            (msg, voters, roll.all_voted())
        };

        self.broadcast_mgr.broadcast_msg_to_players(&voters, &msg);

        if all_voted {
            self.count_the_roll(group_id, looted_target, item_slot, world)
                .await;
        }
    }

    /// Resolve the roll and award the winner.
    pub async fn count_the_roll(
        &self,
        group_id: u32,
        looted_target: ObjectGuid,
        item_slot: u8,
        world: &World,
    ) {
        let Some(roll) = self.take_roll(group_id, looted_target, item_slot) else {
            return;
        };
        self.resolve_roll(roll, world).await;
    }

    /// End every in-flight roll on a corpse. Called on corpse removal and group disband.
    pub fn end_roll(&self, world: &World, looted_target: ObjectGuid) {
        let group_ids: Vec<u32> = self.loot_rolls.iter().map(|e| *e.key()).collect();
        for group_id in group_ids {
            let pending: Vec<u8> = self
                .loot_rolls
                .get(&group_id)
                .map(|rolls| {
                    rolls
                        .iter()
                        .filter(|r| r.looted_target == looted_target)
                        .map(|r| r.item_slot)
                        .collect()
                })
                .unwrap_or_default();
            for slot in pending {
                let Some(roll) = self.take_roll(group_id, looted_target, slot) else {
                    continue;
                };
                self.spawn_resolution(roll, world);
            }
        }
    }

    /// A member left the group: drop their votes from every open roll and resolve any roll
    /// that is now complete.
    pub fn remove_member_rolls(&self, world: &World, group_id: u32, member: ObjectGuid) {
        let completed: Vec<(ObjectGuid, u8)> = self
            .loot_rolls
            .get_mut(&group_id)
            .map(|mut rolls| {
                let mut done = Vec::new();
                for roll in rolls.iter_mut() {
                    let Some(vote) = roll.votes.remove(&member) else {
                        continue;
                    };
                    match vote {
                        RollVote::Need => roll.total_need = roll.total_need.saturating_sub(1),
                        RollVote::Greed => roll.total_greed = roll.total_greed.saturating_sub(1),
                        RollVote::Pass => roll.total_pass = roll.total_pass.saturating_sub(1),
                        _ => {}
                    }
                    if vote != RollVote::NotValid {
                        roll.total_players_rolling = roll.total_players_rolling.saturating_sub(1);
                    }
                    if roll.all_voted() {
                        done.push((roll.looted_target, roll.item_slot));
                    }
                }
                done
            })
            .unwrap_or_default();

        for (looted, slot) in completed {
            if let Some(roll) = self.take_roll(group_id, looted, slot) {
                self.spawn_resolution(roll, world);
            }
        }
    }

    /// Advance the group's round-robin cursor for every method except free-for-all and master
    /// loot, skipping members who are offline or out of range, and return the new looter.
    ///
    /// An unset cursor starts at the first member. With `ifneed`, a current looter who is still
    /// in range keeps the turn.
    pub fn update_looter_guid(
        &self,
        world: &World,
        group_id: u32,
        _looted_object: ObjectGuid,
        context: &LootContext,
        ifneed: bool,
    ) -> Option<ObjectGuid> {
        let method = self
            .groups
            .get(&group_id)
            .map(|g| g.loot_method)
            .unwrap_or(LootMethod::FreeForAll);
        if matches!(method, LootMethod::MasterLooter | LootMethod::FreeForAll) {
            return None;
        }

        let members: Vec<ObjectGuid> = self
            .groups
            .get(&group_id)
            .map(|g| g.members.iter().map(|m| m.guid).collect())
            .unwrap_or_default();

        let in_range = |guid: ObjectGuid| {
            self.player_mgr.get_player(guid).is_some()
                && is_at_group_reward_distance(
                    world,
                    guid,
                    &context.position,
                    context.map_id,
                    context.instance_id,
                    context.rank,
                )
        };

        let old = self
            .round_robin_looters
            .get(&group_id)
            .map(|g| *g)
            .filter(|g| !g.is_empty());
        let old_index = old.and_then(|old_guid| members.iter().position(|g| *g == old_guid));

        if let (Some(old_guid), Some(_)) = (old, old_index) {
            if ifneed && in_range(old_guid) {
                return Some(old_guid);
            }
        }

        // Search after the current looter, then wrap around from the start (which includes the
        // current looter). With no current looter, the search starts at the first member.
        let start = old_index.map(|i| i + 1).unwrap_or(0);
        let new_looter = members[start.min(members.len())..]
            .iter()
            .chain(members[..start.min(members.len())].iter())
            .copied()
            .find(|guid| in_range(*guid));

        self.round_robin_looters
            .insert(group_id, new_looter.unwrap_or_else(ObjectGuid::empty));
        new_looter
    }

    /// Make sure a master-loot group has a master who can hand this corpse out.
    ///
    /// An offline master is replaced by an online leader or assistant. With nobody to take over,
    /// the group falls back to group loot at the uncommon threshold.
    pub fn resolve_master_looter(
        &self,
        _world: &World,
        group_id: u32,
        _looted_object: ObjectGuid,
        _context: &LootContext,
    ) -> Option<ObjectGuid> {
        let group = self.get_group(group_id)?;
        if !group.looter_guid.is_empty() && self.player_mgr.get_player(group.looter_guid).is_some()
        {
            return Some(group.looter_guid);
        }

        let replacement = group
            .members
            .iter()
            .filter(|m| m.guid != group.looter_guid)
            .filter(|m| m.guid == group.leader_guid || m.assistant)
            .map(|m| m.guid)
            .find(|guid| self.player_mgr.get_player(*guid).is_some());

        match replacement {
            Some(master) => {
                if let Some(mut group) = self.groups.get_mut(&group_id) {
                    group.looter_guid = master;
                }
                self.broadcast_group_list(group_id);
                Some(master)
            }
            None => {
                if let Some(mut group) = self.groups.get_mut(&group_id) {
                    group.loot_method = LootMethod::GroupLoot;
                    group.loot_threshold = 2;
                }
                self.broadcast_group_list(group_id);
                // The caller now sees group loot and picks the round-robin looter itself.
                None
            }
        }
    }

    /// Current position of the group's round-robin cursor, if one is set.
    pub fn current_looter(&self, group_id: u32) -> Option<ObjectGuid> {
        self.round_robin_looters
            .get(&group_id)
            .map(|g| *g)
            .filter(|g| !g.is_empty())
    }

    /// Whether a player is on the loot's allowed-looter set (or it is open to all).
    pub fn is_allowed_looter(
        &self,
        world: &World,
        looted_target: ObjectGuid,
        player: ObjectGuid,
    ) -> bool {
        world
            .systems
            .loot_manager
            .is_allowed_looter(looted_target, player)
            .unwrap_or(true)
    }

    /// Decrement roll timers; resolve any that have expired. Runs from the sync tick.
    pub fn update_roll_timers(&self, diff: Duration, world: &World) {
        let diff_ms = diff.as_millis() as u64;
        if diff_ms == 0 {
            return;
        }
        let group_ids: Vec<u32> = self.loot_rolls.iter().map(|e| *e.key()).collect();
        for group_id in group_ids {
            let timed_out: Vec<(ObjectGuid, u8)> = self
                .loot_rolls
                .get_mut(&group_id)
                .map(|mut rolls| {
                    let mut out = Vec::new();
                    for roll in rolls.iter_mut() {
                        roll.timer_ms = roll.timer_ms.saturating_sub(diff_ms);
                        if roll.timer_ms == 0 {
                            out.push((roll.looted_target, roll.item_slot));
                        }
                    }
                    out
                })
                .unwrap_or_default();
            for (looted, slot) in timed_out {
                let Some(roll) = self.take_roll(group_id, looted, slot) else {
                    continue;
                };
                self.spawn_resolution(roll, world);
            }
        }
    }

    /// Remove the roll from the group's queue.
    fn take_roll(&self, group_id: u32, looted_target: ObjectGuid, item_slot: u8) -> Option<Roll> {
        let mut removed = None;
        if let Some(mut rolls) = self.loot_rolls.get_mut(&group_id) {
            if let Some(idx) = rolls
                .iter()
                .position(|r| r.looted_target == looted_target && r.item_slot == item_slot)
            {
                removed = Some(rolls.remove(idx));
            }
        }
        // Drop the read guard before removing the (now empty) entry, or `remove` will try to
        // write-lock the same shard the guard still holds and deadlock.
        let empty = self.loot_rolls.get(&group_id).is_some_and(|r| r.is_empty());
        if empty {
            self.loot_rolls.remove(&group_id);
        }
        removed
    }

    fn spawn_resolution(&self, roll: Roll, world: &World) {
        let world = world.clone();
        if let Some(arc) = self.self_arc.get().cloned() {
            tokio::spawn(async move {
                arc.resolve_roll(roll, &world).await;
            });
        }
    }

    /// Only one member could roll: they win outright with a 100, no roll is shown.
    async fn award_single_looter(&self, world: &World, roll: Roll) {
        let Some(winner) = roll.votes.keys().next().copied() else {
            return;
        };
        let won_msg = SmsgLootRollWon {
            loot_guid: roll.looted_target,
            player_guid: winner,
            item_slot: u32::from(roll.item_slot),
            item_id: roll.item_id,
            item_random_prop_id: roll.random_property_id as i32,
            item_count: roll.count,
            roll_number: 100,
            roll_type: roll_bytes::NEED_ROLL_TYPE,
        };
        self.broadcast_mgr.send_msg_to_player(winner, won_msg);
        self.award_to_winner(world, winner, &roll).await;
    }

    pub(super) async fn resolve_roll(&self, roll: Roll, world: &World) {
        let voters: Vec<ObjectGuid> = roll.votes.keys().copied().collect();

        if roll.total_need == 0 && roll.total_greed == 0 {
            // Everybody passed (or nobody voted in time): announce it and free the item.
            let msg = SmsgLootAllPassed {
                loot_guid: roll.looted_target,
                item_slot: u32::from(roll.item_slot),
                item_id: roll.item_id,
                item_random_prop_id: roll.random_property_id as i32,
                item_count: roll.count,
            };
            self.broadcast_mgr.broadcast_msg_to_players(&voters, &msg);
            world
                .systems
                .loot_manager
                .set_item_blocked(roll.looted_target, roll.item_slot, false);
            self.send_rolls_complete(&roll, &voters);
            return;
        }

        // Need beats greed; only the winning category rolls.
        let (category, roll_type) = if roll.total_need > 0 {
            (RollVote::Need, roll_bytes::NEED_ROLL_TYPE)
        } else {
            (RollVote::Greed, roll_bytes::GREED_ROLL_TYPE)
        };
        let candidates: Vec<ObjectGuid> = roll
            .votes
            .iter()
            .filter(|(_, v)| **v == category)
            .map(|(g, _)| *g)
            .collect();
        if candidates.is_empty() {
            return;
        }

        // Each candidate rolls 1-100, announced to the whole roll; the first highest wins.
        let mut winner = candidates[0];
        let mut best = 0u8;
        for guid in &candidates {
            let number: u8 = rand::thread_rng().gen_range(1..=100);
            let msg = SmsgLootRoll {
                loot_guid: roll.looted_target,
                player_guid: *guid,
                item_slot: u32::from(roll.item_slot),
                item_id: roll.item_id,
                item_random_prop_id: roll.random_property_id as i32,
                item_count: roll.count,
                roll_number: number,
                roll_type,
            };
            self.broadcast_mgr.broadcast_msg_to_players(&voters, &msg);
            if number > best {
                best = number;
                winner = *guid;
            }
        }

        let won_msg = SmsgLootRollWon {
            loot_guid: roll.looted_target,
            player_guid: winner,
            item_slot: u32::from(roll.item_slot),
            item_id: roll.item_id,
            item_random_prop_id: roll.random_property_id as i32,
            item_count: roll.count,
            roll_number: best,
            roll_type,
        };
        self.broadcast_mgr
            .broadcast_msg_to_players(&voters, &won_msg);
        self.send_rolls_complete(&roll, &voters);

        self.award_to_winner(world, winner, &roll).await;
    }

    /// Store the item in the winner's bags. When that fails (full bags, offline) the item stays
    /// in the loot, unblocked and reserved for the winner so they can take it themselves.
    async fn award_to_winner(&self, world: &World, winner: ObjectGuid, roll: &Roll) {
        use crate::game::inventory::types::AddItemResult;
        let stored = matches!(
            world
                .systems
                .inventory
                .add_item(winner, roll.item_id, roll.count)
                .await,
            AddItemResult::Success { .. }
        );

        if stored {
            self.mark_looted(world, roll);
            world
                .systems
                .loot
                .notify_item_removed(roll.looted_target, false, roll.item_slot, winner);
            world
                .systems
                .quest
                .handle_item_added(winner, roll.item_id, roll.count);
            crate::game::gameobject::quest_activation::refresh_quest_gameobjects(winner, world);
        } else {
            world
                .systems
                .loot_manager
                .with_loot_mut(roll.looted_target, |loot| {
                    if let Some(item) = loot.items.iter_mut().find(|i| i.slot == roll.item_slot) {
                        item.is_blocked = false;
                        item.loot_owner = Some(winner);
                    }
                });
        }
    }

    /// Dismiss the 1.14 client's roll frame for this item. Only rolls that were actually
    /// announced (more than one voter) have a frame, and 1.12 has no such message.
    fn send_rolls_complete(&self, roll: &Roll, voters: &[ObjectGuid]) {
        if roll.total_players_rolling <= 1 {
            return;
        }
        let modern: Vec<ObjectGuid> = voters
            .iter()
            .copied()
            .filter(|guid| {
                self.player_mgr
                    .get_broadcaster(*guid)
                    .is_some_and(|b| b.protocol() == Protocol::Modern)
            })
            .collect();
        if modern.is_empty() {
            return;
        }
        let msg = SmsgLootRollsComplete {
            loot_guid: roll.looted_target,
            loot_list_id: roll.item_slot + 1,
        };
        self.broadcast_mgr.broadcast_msg_to_players(&modern, &msg);
    }

    fn mark_looted(&self, world: &World, roll: &Roll) {
        world
            .systems
            .loot_manager
            .with_loot_mut(roll.looted_target, |loot| {
                if let Some(item) = loot.items.iter_mut().find(|i| i.slot == roll.item_slot) {
                    if !item.is_looted {
                        item.is_looted = true;
                        item.is_blocked = false;
                        loot.unlooted_count = loot.unlooted_count.saturating_sub(1);
                    }
                }
            });
    }
}

/// Classes (1-based class ids) as `AllowableClass` bits.
fn class_mask(class: u8) -> i32 {
    if class == 0 {
        0
    } else {
        1 << (class - 1)
    }
}

/// Simplified usability check for need-before-greed: whether a player could ever equip or use
/// the item. Covers the class/race restriction masks, required level, and armor and shield
/// proficiency. Weapon skills, profession requirements and reputation are not modeled.
pub fn can_use_item(player_class: u8, player_race: u8, player_level: u32, item: &ItemTemplate) -> bool {
    const WARRIOR: u8 = 1;
    const PALADIN: u8 = 2;
    const HUNTER: u8 = 3;
    const PRIEST: u8 = 5;
    const SHAMAN: u8 = 7;
    const MAGE: u8 = 8;
    const WARLOCK: u8 = 9;
    const DRUID: u8 = 11;

    if player_level < item.required_level {
        return false;
    }
    // A mask of 0 or -1 means every class / race.
    let class_ok = item.allowable_class <= 0 || item.allowable_class & class_mask(player_class) != 0;
    let race_ok = item.allowable_race <= 0
        || player_race == 0
        || item.allowable_race & (1 << (player_race - 1)) != 0;
    if !class_ok || !race_ok {
        return false;
    }

    if item.item_class == 4 {
        return match item.item_subclass {
            // Cloth: everyone.
            1 => true,
            // Leather: everyone but the pure casters.
            2 => !matches!(player_class, PRIEST | MAGE | WARLOCK),
            // Mail: warriors and paladins; hunters and shamans from level 40.
            3 => {
                matches!(player_class, WARRIOR | PALADIN)
                    || (matches!(player_class, HUNTER | SHAMAN) && player_level >= 40)
            }
            // Plate: warriors and paladins.
            4 => matches!(player_class, WARRIOR | PALADIN),
            // Shields: warriors, paladins, shamans.
            6 => matches!(player_class, WARRIOR | PALADIN | SHAMAN),
            // Librams, idols, totems.
            7 => player_class == PALADIN,
            8 => player_class == DRUID,
            9 => player_class == SHAMAN,
            _ => true,
        };
    }
    true
}
