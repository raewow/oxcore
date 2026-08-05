//! Group loot roll flow.
//!
//! Ports `Group::GroupLoot`, `NeedBeforeGreed`, `MasterLoot`, `StartLootRoll`,
//! `CountRollVote`, `CountTheRoll`, `CountSingleLooterRoll`, `EndRoll`, and
//! `UpdateLooterGuid` (reference `game/Group/Group.cpp:868-1255` and `:2485-2555`).

use crate::game::group::reward::is_at_group_reward_distance;
use crate::game::broadcast_mgr::BroadcastManagerExt;
use crate::game::items::manager::ItemTemplate;
use crate::game::loot::{Roll, RollVote};
use crate::World;
use oxcore_shared::game::group::LootMethod;
use oxcore_shared::messages::group::{
    SmsgLootAllPassed, SmsgLootMasterList, SmsgLootRoll, SmsgLootRollStarted, SmsgLootRollWon,
    SmsgLootRollsComplete,
};
use oxcore_shared::protocol::{ObjectGuid, Position};
use rand::Rng;
use std::time::Duration;

/// Creature location + elite rank needed to judge reward distance for rolls.
pub struct LootContext {
    pub position: Position,
    pub map_id: u32,
    pub instance_id: u32,
    pub rank: u8,
}

/// Roll vote labels sent to the client (mirrors `SendLootRoll`'s roll numbers).
const ROLL_NUMBER_PASS: u8 = 128;
const ROLL_NUMBER_NEED: u8 = 0;
const ROLL_NUMBER_GREED: u8 = 128;

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

    /// `Group::GroupLoot`: roll over-threshold items, mark the rest under-threshold.
    pub async fn group_loot(
        &self,
        world: &World,
        group_id: u32,
        looted_target: ObjectGuid,
        context: LootContext,
    ) {
        self.classify_and_start_rolls(world, group_id, looted_target, &context, false).await;
    }

    /// `Group::NeedBeforeGreed`: like group loot, but only classes that can use the
    /// item are offered the roll.
    pub async fn need_before_greed(
        &self,
        world: &World,
        group_id: u32,
        looted_target: ObjectGuid,
        context: LootContext,
    ) {
        self.classify_and_start_rolls(world, group_id, looted_target, &context, true).await;
    }

    /// `Group::MasterLoot`: mark under-threshold items and send the eligible-looter list
    /// to the master looter.
    pub async fn master_loot(
        &self,
        world: &World,
        group_id: u32,
        looted_target: ObjectGuid,
        master_guid: ObjectGuid,
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
        for item in loot.items.iter() {
            if item.is_looted {
                continue;
            }
            let quality = world
                .managers
                .item_mgr
                .get_template(item.item_id)
                .map(|t| t.quality)
                .unwrap_or(0);
            if quality < threshold {
                loot_manager.with_loot_mut(looted_target, |loot| {
                    if let Some(it) = loot.items.iter_mut().find(|i| i.slot == item.slot) {
                        it.is_underthreshold = true;
                    }
                });
            }
        }
        drop(loot);

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
        self.broadcast_mgr.send_msg_to_player(master_guid, msg);
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
                world.systems.loot_manager.with_loot_mut(looted_target, |loot| {
                    if let Some(it) = loot.items.iter_mut().find(|i| i.slot == slot) {
                        it.is_underthreshold = true;
                    }
                });
            }
        }
    }

    /// `Group::StartLootRoll`: build the eligible-voter set, then either auto-award
    /// (single voter) or broadcast the roll and block the item.
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
        let Some(item) = loot.items.iter().find(|i| i.slot == item_slot) else {
            return;
        };
        let item_template = world.managers.item_mgr.get_template(item.item_id);
        let item_id = item.item_id;
        let count = item.count;
        let random_property_id = 0;
        drop(loot);

        let Some(group) = self.groups.get(&group_id) else {
            return;
        };
        let members: Vec<ObjectGuid> = group.members.iter().map(|m| m.guid).collect();
        drop(group);

        let mut roll = Roll::new(
            looted_target,
            item_slot,
            item_id,
            count,
            random_property_id,
        );
        for guid in members {
            let Some(player) = self.player_mgr.get_player(guid) else {
                continue;
            };
            let can_use = if method == LootMethod::NeedBeforeGreed {
                item_template
                    .as_ref()
                    .map(|t| can_use_item(player.class, u32::from(player.level), t))
                    .unwrap_or(true)
            } else {
                true
            };
            if can_use
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
            // Single looter: award immediately (CountSingleLooterRoll).
            let winner = *roll.votes.keys().next().unwrap();
            roll.votes.insert(winner, RollVote::Need);
            roll.total_need = 1;
            self.resolve_roll(group_id, roll, world).await;
            return;
        }

        // Broadcast the start to every eligible member and block the item.
        let start_msg = SmsgLootRollStarted {
            loot_guid: looted_target,
            map_id: context.map_id,
            item_slot: u32::from(item_slot),
            item_id,
            item_random_prop_id: random_property_id as i32,
            item_suffix_factor: 0,
            item_count: count as u8,
            roll_timeout: crate::game::loot::LOOT_ROLL_TIMEOUT as u32,
            roll_type: u8::from(method),
        };
        for guid in roll.votes.keys() {
            self.broadcast_mgr.send_msg_to_player(*guid, start_msg.clone());
        }
        world
            .systems
            .loot_manager
            .set_item_blocked(looted_target, item_slot, true);

        self.loot_rolls.entry(group_id).or_default().push(roll);
    }

    /// `Group::CountRollVote`: register a vote and resolve when everyone has voted.
    pub async fn count_roll_vote(
        &self,
        world: &World,
        group_id: u32,
        player_guid: ObjectGuid,
        looted_target: ObjectGuid,
        item_slot: u8,
        vote: RollVote,
    ) {
        let Some(mut group_rolls) = self.loot_rolls.get_mut(&group_id) else {
            return;
        };
        let Some(roll_index) = group_rolls
            .iter()
            .position(|r| r.looted_target == looted_target && r.item_slot == item_slot)
        else {
            return;
        };
        let roll = &mut group_rolls[roll_index];
        if !roll.votes.contains_key(&player_guid) {
            return;
        }
        let (number, label) = match vote {
            RollVote::Pass => (ROLL_NUMBER_PASS, RollVote::Pass),
            RollVote::Need => (ROLL_NUMBER_NEED, RollVote::Need),
            RollVote::Greed => (ROLL_NUMBER_GREED, RollVote::Greed),
            _ => return,
        };
        let msg = SmsgLootRoll {
            loot_guid: looted_target,
            player_guid,
            item_slot: u32::from(item_slot),
            item_id: roll.item_id,
            roll_number: number,
            roll_type: vote_label(label),
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

        let complete = roll.add_vote(player_guid, vote);
        let all_voted = roll.all_voted();
        drop(group_rolls);

        if complete && all_voted {
            self.count_the_roll(group_id, looted_target, item_slot, world)
                .await;
        }
    }

    /// `Group::CountTheRoll` / `Group::EndRoll`: resolve the roll and award the winner.
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
        self.resolve_roll(group_id, roll, world).await;
    }

    /// End every in-flight roll on a corpse. Called on corpse removal and group disband.
    pub fn end_roll(&self, world: &World, looted_target: ObjectGuid) {
        let group_ids: Vec<u32> = self.loot_rolls.iter().map(|e| *e.key()).collect();
        for group_id in group_ids {
            let pending: Vec<(u8, u32)> = self
                .loot_rolls
                .get(&group_id)
                .map(|rolls| {
                    rolls
                        .iter()
                        .filter(|r| r.looted_target == looted_target)
                        .map(|r| (r.item_slot, r.item_id))
                        .collect()
                })
                .unwrap_or_default();
            for (slot, _) in pending {
                let Some(roll) = self.take_roll(group_id, looted_target, slot) else {
                    continue;
                };
                self.spawn_resolution(group_id, roll, world);
            }
        }
    }

    /// `Group::UpdateLooterGuid`: advance the round-robin looter for non-FFA/master
    /// methods, skipping members outside reward distance.
    pub fn update_looter_guid(
        &self,
        world: &World,
        group_id: u32,
        looted_object: ObjectGuid,
        context: &LootContext,
        ifneed: bool,
    ) {
        let method = self
            .groups
            .get(&group_id)
            .map(|g| g.loot_method)
            .unwrap_or(LootMethod::FreeForAll);
        if matches!(method, LootMethod::MasterLooter | LootMethod::FreeForAll) {
            return;
        }

        let members: Vec<ObjectGuid> = self
            .groups
            .get(&group_id)
            .map(|g| g.members.iter().map(|m| m.guid).collect())
            .unwrap_or_default();

        let old = self.round_robin_looters.get(&group_id).map(|g| *g);
        let start = match old {
            Some(old_guid) => members
                .iter()
                .position(|g| *g == old_guid)
                .unwrap_or(members.len()),
            None => 0,
        };

        if let Some(old_guid) = old {
            if ifneed
                && self
                    .player_mgr
                    .get_player(old_guid)
                    .is_some()
                && is_at_group_reward_distance(
                    world,
                    old_guid,
                    &context.position,
                    context.map_id,
                    context.instance_id,
                    context.rank,
                )
            {
                return;
            }
        }

        let mut new_looter = None;
        for i in (start + 1)..members.len() {
            let guid = members[i];
            if self.player_mgr.get_player(guid).is_some()
                && is_at_group_reward_distance(
                    world,
                    guid,
                    &context.position,
                    context.map_id,
                    context.instance_id,
                    context.rank,
                )
            {
                new_looter = Some(guid);
                break;
            }
        }
        if new_looter.is_none() {
            for i in 0..(start + 1).min(members.len()) {
                let guid = members[i];
                if self.player_mgr.get_player(guid).is_some()
                    && is_at_group_reward_distance(
                        world,
                        guid,
                        &context.position,
                        context.map_id,
                        context.instance_id,
                        context.rank,
                    )
                {
                    new_looter = Some(guid);
                    break;
                }
            }
        }

        match new_looter {
            Some(guid) => {
                self.round_robin_looters.insert(group_id, guid);
                self.broadcast_group_list(group_id);
            }
            None => {
                self.round_robin_looters.insert(group_id, ObjectGuid::empty());
                self.broadcast_group_list(group_id);
            }
        }
    }

    /// Round-robin looter for a group, if one is set.
    pub fn current_looter(&self, group_id: u32) -> Option<ObjectGuid> {
        self.round_robin_looters.get(&group_id).map(|g| *g)
    }

    /// Whether a player is on the loot's allowed-looter set (or it is open to all).
    pub fn is_allowed_looter(&self, world: &World, looted_target: ObjectGuid, player: ObjectGuid) -> bool {
        world
            .systems
            .loot_manager
            .is_allowed_looter(looted_target, player)
            .map(|allowed| allowed)
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
                self.spawn_resolution(group_id, roll, world);
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

    fn spawn_resolution(&self, group_id: u32, roll: Roll, world: &World) {
        let world = world.clone();
        if let Some(arc) = self.self_arc.get().cloned() {
            tokio::spawn(async move {
                arc.resolve_roll(group_id, roll, &world).await;
            });
        }
    }

    pub(super) async fn resolve_roll(&self, group_id: u32, roll: Roll, world: &World) {
        let all_passed = roll.total_need == 0 && roll.total_greed == 0;
        if all_passed {
            // SendLootAllPassed to every voter; unblock the item.
            let msg = SmsgLootAllPassed {
                loot_guid: roll.looted_target,
                item_slot: u32::from(roll.item_slot),
                item_id: roll.item_id,
            };
            let voters: Vec<ObjectGuid> = roll.votes.keys().cloned().collect();
            self.broadcast_mgr
                .broadcast_msg_to_players(&voters, &msg);
            world
                .systems
                .loot_manager
                .set_item_blocked(roll.looted_target, roll.item_slot, false);
            self.send_rolls_complete(&roll, &voters);
            return;
        }

        // Vote for the winning category (need beats greed).
        let (candidates, roll_type): (Vec<&ObjectGuid>, u8) = if roll.total_need > 0 {
            (
                roll.votes
                    .iter()
                    .filter(|(_, v)| **v == RollVote::Need)
                    .map(|(g, _)| g)
                    .collect(),
                vote_label(RollVote::Need),
            )
        } else {
            (
                roll.votes
                    .iter()
                    .filter(|(_, v)| **v == RollVote::Greed)
                    .map(|(g, _)| g)
                    .collect(),
                vote_label(RollVote::Greed),
            )
        };
        if candidates.is_empty() {
            return;
        }

        // Random rolls; highest wins (C++ urand(1,100) per voter).
        let mut winner = ObjectGuid::empty();
        let mut best = 0u8;
        for guid in &candidates {
            let number = rand::thread_rng().gen_range(1..=100);
            let msg = SmsgLootRoll {
                loot_guid: roll.looted_target,
                player_guid: **guid,
                item_slot: u32::from(roll.item_slot),
                item_id: roll.item_id,
                roll_number: number,
                roll_type,
            };
            self.broadcast_mgr.send_msg_to_player(**guid, msg);
            if number > best {
                best = number;
                winner = **guid;
            }
        }

        let won_msg = SmsgLootRollWon {
            loot_guid: roll.looted_target,
            player_guid: winner,
            item_slot: u32::from(roll.item_slot),
            item_id: roll.item_id,
            roll_number: best,
            roll_type,
        };
        let voters: Vec<ObjectGuid> = roll.votes.keys().cloned().collect();
        self.broadcast_mgr
            .broadcast_msg_to_players(&voters, &won_msg);
        self.send_rolls_complete(&roll, &voters);

        // Award through inventory, keeping the "full bag leaves the item in the loot"
        // convention. Mark the item looted only after a successful store.
        let award = self.award_item(world, winner, &roll).await;
        if award {
            self.mark_looted(world, &roll);
        } else {
            world
                .systems
                .loot_manager
                .set_item_blocked(roll.looted_target, roll.item_slot, false);
            if let Some(mut loot) = world
                .systems
                .loot_manager
                .get_loot_mut(roll.looted_target)
            {
                if let Some(item) = loot.items.iter_mut().find(|i| i.slot == roll.item_slot) {
                    item.loot_owner = Some(winner);
                }
            }
        }
    }

    /// Store the rolled item in the winner's bags. Returns true on success.
    async fn award_item(&self, world: &World, winner: ObjectGuid, roll: &Roll) -> bool {
        use crate::game::inventory::types::AddItemResult;
        match world
            .systems
            .inventory
            .add_item(winner, roll.item_id, roll.count)
            .await
        {
            AddItemResult::Success { .. } => true,
            _ => false,
        }
    }

    /// Dismiss the client's roll UI for this item. Only rolls that were actually broadcast
    /// (more than one voter) have a UI entry to dismiss.
    fn send_rolls_complete(&self, roll: &Roll, voters: &[ObjectGuid]) {
        if roll.total_players_rolling <= 1 {
            return;
        }
        let msg = SmsgLootRollsComplete {
            loot_guid: roll.looted_target,
            loot_list_id: roll.item_slot + 1,
        };
        self.broadcast_mgr.broadcast_msg_to_players(voters, &msg);
    }

    fn mark_looted(&self, world: &World, roll: &Roll) {
        world
            .systems
            .loot_manager
            .with_loot_mut(roll.looted_target, |loot| {
                if let Some(item) = loot.items.iter_mut().find(|i| i.slot == roll.item_slot) {
                    if !item.is_looted {
                        item.is_looted = true;
                        loot.unlooted_count = loot.unlooted_count.saturating_sub(1);
                    }
                }
            });
    }
}

/// Roll-type byte sent to the client for a vote label.
fn vote_label(vote: RollVote) -> u8 {
    match vote {
        RollVote::Pass => 0,
        RollVote::Need => 1,
        RollVote::Greed => 2,
        _ => 0,
    }
}

/// Simplified `Player::CanUseItem` (`EQUIP_ERR_OK` only). Enforces the required level and
/// the classic armor proficiency table; weapon restrictions are not modeled.
pub fn can_use_item(player_class: u8, player_level: u32, item: &ItemTemplate) -> bool {
    if u32::from(player_level) < item.required_level {
        return false;
    }
    if item.item_class == 4 {
        let proficient = match item.item_subclass {
            1 => true, // cloth: all classes
            2 => !matches!(player_class, 1 | 2),       // leather: not warrior/paladin
            3 => matches!(player_class, 1 | 2 | 3 | 7), // mail: warrior/paladin/hunter/shaman
            4 => matches!(player_class, 1 | 2),        // plate: warrior/paladin
            _ => true,
        };
        if !proficient {
            return false;
        }
    }
    true
}
