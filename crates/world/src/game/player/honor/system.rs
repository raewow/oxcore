//! Honour: contribution points from kills and objectives, the honour tab's
//! counters, ranks, and the weekly ranking that turns contribution points
//! into rank points.

use std::collections::HashMap;

use oxcore_shared::game::chat::Team;
use oxcore_shared::messages::battleground::SmsgPvpCredit;
use oxcore_shared::messages::update::{ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock};
use oxcore_shared::protocol::ObjectGuid;
use oxcore_shared::protocol::update_fields::{
    PLAYER_BYTES_3, PLAYER_FIELD_BYTES, PLAYER_FIELD_BYTES2, PLAYER_FIELD_LAST_WEEK_CONTRIBUTION,
    PLAYER_FIELD_LAST_WEEK_KILLS, PLAYER_FIELD_LAST_WEEK_RANK, PLAYER_FIELD_LIFETIME_DISHONORBALE_KILLS,
    PLAYER_FIELD_LIFETIME_HONORBALE_KILLS, PLAYER_FIELD_SESSION_KILLS, PLAYER_FIELD_THIS_WEEK_CONTRIBUTION,
    PLAYER_FIELD_THIS_WEEK_KILLS, PLAYER_FIELD_YESTERDAY_CONTRIBUTION, PLAYER_FIELD_YESTERDAY_KILLS,
};

use super::rank::{self, HonorRankInfo};
use crate::World;

/// Contribution point sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HonorType {
    Honorable = 1,
    Dishonorable = 2,
    Bonus = 3,
    Quest = 4,
    Other = 5,
}

/// Players within this range of a kill share its honour.
const REWARD_DISTANCE: f32 = 74.0;
/// Honour for killing a faction leader.
const RACIAL_LEADER_HONOR: f32 = 488.0;
/// The "Honorless Target" debuff: dying with it gives no honour.
const SPELL_HONORLESS_TARGET: u32 = 2479;
/// Victim type stored for a player victim (`TYPEID_PLAYER`).
const VICTIM_PLAYER: u8 = 4;
/// Victim type stored for a creature victim (`TYPEID_UNIT`).
const VICTIM_UNIT: u8 = 3;

/// One contribution point record.
#[derive(Debug, Clone, PartialEq)]
pub struct HonorCp {
    pub victim_type: u8,
    pub victim_id: u32,
    pub cp: f32,
    pub date: u32,
    pub kind: u8,
    /// Already in the database.
    pub saved: bool,
}

/// A player's honour.
#[derive(Debug, Clone, Default)]
pub struct HonorState {
    /// This and last week's records (older ones are folded into the stored counts).
    pub entries: Vec<HonorCp>,
    pub rank_points: f32,
    pub standing: u32,
    pub rank: HonorRankInfo,
    pub highest_rank: HonorRankInfo,
    pub last_week_hk: u32,
    pub last_week_cp: f32,
    pub stored_hk: u32,
    pub stored_dk: u32,
    pub total_hk: u32,
    pub total_dk: u32,
}

/// Days since the epoch: the honour calendar's day.
pub fn game_day() -> u32 {
    (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86_400) as u32
}

/// The most recent `weekday` (0 = Sunday) on or before `day`.
pub fn last_weekday(day: u32, weekday: u32) -> u32 {
    // Day 0 (1970-01-01) was a Thursday.
    let today = (day + 4) % 7;
    day - (today + 7 - weekday % 7) % 7
}

/// The counters the honour tab shows, from a player's records.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HonorCounters {
    pub today_hk: u32,
    pub today_dk: u32,
    pub yesterday_hk: u32,
    pub yesterday_cp: f32,
    pub this_week_hk: u32,
    pub this_week_cp: f32,
    pub total_hk: u32,
    pub total_dk: u32,
}

impl HonorState {
    /// Recount the tab's counters for `today`, the week beginning `week_begin`.
    pub fn counters(&self, today: u32, week_begin: u32) -> HonorCounters {
        let mut c = HonorCounters {
            total_hk: self.stored_hk,
            total_dk: self.stored_dk,
            ..HonorCounters::default()
        };
        let yesterday = today.saturating_sub(1);
        for entry in &self.entries {
            match entry.kind {
                k if k == HonorType::Honorable as u8 => {
                    if entry.date == today {
                        c.today_hk += 1;
                    }
                    if entry.date == yesterday {
                        c.yesterday_hk += 1;
                    }
                    if entry.date >= week_begin {
                        c.this_week_hk += 1;
                        c.total_hk += 1;
                    }
                }
                k if k == HonorType::Dishonorable as u8 => {
                    if entry.date == today {
                        c.today_dk += 1;
                    }
                    c.total_dk += 1;
                }
                _ => {}
            }
            if entry.kind != HonorType::Dishonorable as u8 {
                if entry.date == yesterday {
                    c.yesterday_cp += entry.cp;
                }
                if entry.date >= week_begin {
                    c.this_week_cp += entry.cp;
                }
            }
        }
        c
    }

    /// Kills of one victim today (each repeat is worth less).
    pub fn kills_of_today(&self, victim_type: u8, victim_id: u32, today: u32) -> u32 {
        self.entries
            .iter()
            .filter(|e| e.victim_type == victim_type && e.victim_id == victim_id && e.date == today)
            .count() as u32
    }

    /// Refresh rank and highest rank from rank points.
    pub fn refresh_rank(&mut self) {
        self.rank = rank::calculate_rank(self.rank_points);
        if self.rank.visual_rank > 0 && self.rank.visual_rank > self.highest_rank.visual_rank {
            self.highest_rank = self.rank;
        }
    }
}

/// The week begins on the last honour maintenance day.
fn week_begin(world: &World) -> u32 {
    last_weekday(game_day(), world.config.honor_maintenance_day)
}

/// Add contribution points to a player. Returns whether anything was added.
pub fn add_honor(
    world: &World,
    player: ObjectGuid,
    cp: f32,
    kind: HonorType,
    source: Option<(ObjectGuid, u8, u32)>,
) -> bool {
    if cp == 0.0 {
        return false;
    }
    let (victim_guid, victim_type, victim_id, victim_rank) = match source {
        Some((guid, victim_type, victim_id)) => {
            let rank = if guid.is_player() {
                world
                    .managers
                    .player_mgr
                    .with_player(guid, |p| p.honor.rank.rank as u32)
                    .unwrap_or(0)
            } else {
                0
            };
            (guid, victim_type, victim_id, rank)
        }
        None => (ObjectGuid::empty(), 0, player.counter(), 0),
    };
    let today = game_day();
    let added = world.managers.player_mgr.with_player_mut(player, |p| {
        if kind == HonorType::Dishonorable {
            // Dishonourable kills cost rank points at once.
            p.honor.rank_points = (p.honor.rank_points - cp).max(0.0);
        }
        p.honor.entries.push(HonorCp {
            victim_type,
            victim_id,
            cp,
            date: today,
            kind: kind as u8,
            saved: false,
        });
        p.honor.refresh_rank();
    });
    if added.is_none() {
        return false;
    }
    // Never show a bare "HK:": an unranked victim shows the first rank.
    let shown_rank = if victim_guid.is_player() && victim_rank == 0 {
        u32::from(rank::RANK_COUNT - 15 + 1)
    } else {
        victim_rank
    };
    let honor = if kind == HonorType::Dishonorable { -cp } else { cp };
    world.managers.broadcast_mgr.send_msg_to_player(
        player,
        SmsgPvpCredit {
            honor: honor as i32 as u32,
            victim_guid,
            victim_rank: shown_rank,
        },
    );
    send_fields(world, player);
    true
}

/// Show a player their honour tab counters and rank.
pub fn send_fields(world: &World, player: ObjectGuid) {
    let today = game_day();
    let begin = week_begin(world);
    let Some((c, state, gender, toggles, bytes2_flags)) = world.managers.player_mgr.with_player(player, |p| {
        (
            p.honor.counters(today, begin),
            p.honor.clone(),
            p.gender,
            p.settings.action_bar_toggles,
            p.player_bytes2_flags,
        )
    }) else {
        return;
    };
    let rp = state.rank_points.abs();
    let span = (state.rank.max_rp - state.rank.min_rp).max(1.0);
    let bar = (((rp - state.rank.min_rp) / span).clamp(0.0, 1.0) * 255.0) as u8;
    let block = ValuesUpdateBlock::new(player, ObjectType::Player)
        .set_field(PLAYER_FIELD_SESSION_KILLS, c.today_hk | (c.today_dk << 16))
        .set_field(PLAYER_FIELD_YESTERDAY_KILLS, c.yesterday_hk)
        .set_field(PLAYER_FIELD_YESTERDAY_CONTRIBUTION, c.yesterday_cp.max(0.0) as u32)
        .set_field(PLAYER_FIELD_THIS_WEEK_KILLS, c.this_week_hk)
        .set_field(PLAYER_FIELD_THIS_WEEK_CONTRIBUTION, c.this_week_cp.max(0.0) as u32)
        .set_field(PLAYER_FIELD_LAST_WEEK_KILLS, state.last_week_hk)
        .set_field(PLAYER_FIELD_LAST_WEEK_CONTRIBUTION, state.last_week_cp.max(0.0) as u32)
        .set_field(PLAYER_FIELD_LAST_WEEK_RANK, state.standing)
        .set_field(PLAYER_FIELD_LIFETIME_HONORBALE_KILLS, c.total_hk)
        .set_field(PLAYER_FIELD_LIFETIME_DISHONORBALE_KILLS, c.total_dk)
        .set_field(PLAYER_FIELD_BYTES, u32::from_le_bytes([0, 0, toggles, state.highest_rank.rank]))
        .set_field(PLAYER_FIELD_BYTES2, u32::from_le_bytes([bar, bytes2_flags, 0, 0]))
        .set_field(PLAYER_BYTES_3, u32::from_le_bytes([gender, 0, 0, state.rank.rank]));
    world.managers.broadcast_mgr.send_msg_to_player(
        player,
        SmsgUpdateObject::new().add_block(UpdateBlockData::Values(block)),
    );
    world.managers.player_mgr.with_player_mut(player, |p| {
        p.honor.total_hk = c.total_hk;
        p.honor.total_dk = c.total_dk;
    });
}

/// The share of honour a group of `count` members earns together.
fn group_rate(count: usize) -> f32 {
    match count {
        0..=2 => 1.0,
        3 => 1.166,
        4 => 1.3,
        _ => 1.4,
    }
}

/// Honour for an honourable kill by `killer` of `victim`, before sharing.
fn kill_value(world: &World, killer: ObjectGuid, victim: ObjectGuid) -> f32 {
    let today = game_day();
    let Some((killer_level, kills_today)) = world.managers.player_mgr.with_player(killer, |p| {
        (
            u32::from(p.level),
            p.honor.kills_of_today(VICTIM_PLAYER, victim.counter(), today),
        )
    }) else {
        return 0.0;
    };
    let Some((victim_level, victim_rank)) = world
        .managers
        .player_mgr
        .with_player(victim, |p| (u32::from(p.level), i32::from(p.honor.rank.visual_rank)))
    else {
        return 0.0;
    };
    rank::honor_gain(killer_level, victim_level, victim_rank, kills_today, 1)
}

fn distance(a: oxcore_shared::protocol::Position, b: oxcore_shared::protocol::Position) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// Tracks who hurt whom, and hands out honour on PvP deaths.
pub struct HonorSystem;

impl Default for HonorSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl HonorSystem {
    pub fn new() -> Self {
        Self
    }

    /// Record a player hurting an enemy player.
    pub fn record_damage(&self, attacker: ObjectGuid, victim: ObjectGuid, damage: u32, world: &World) {
        if damage == 0 || !attacker.is_player() || !victim.is_player() || attacker == victim {
            return;
        }
        let mgr = &world.managers.player_mgr;
        let a = mgr.with_player(attacker, |p| p.get_team());
        let v = mgr.with_player(victim, |p| p.get_team());
        if a.is_none() || a == v {
            return;
        }
        let now = now_secs();
        mgr.with_player_mut(victim, |p| p.combat.honor.record_damage(attacker, now, damage));
    }

    /// A player died to players: share honour among the enemies who hurt
    /// them in the last minute and are alive and close, by group.
    pub async fn reward_honor_on_death(&self, victim: ObjectGuid, killer: Option<ObjectGuid>, world: &World) {
        let now = now_secs();
        let mgr = &world.managers.player_mgr;
        let Some((mut contributors, victim_team, victim_pos, honorless)) = mgr.with_player_mut(victim, |p| {
            (
                p.combat.honor.take_contributors(now),
                p.get_team(),
                p.movement.position,
                p.auras.container.has_aura(SPELL_HONORLESS_TARGET),
            )
        }) else {
            return;
        };
        if honorless {
            return;
        }
        if contributors.is_empty() {
            if let Some(k) = killer.filter(|k| k.is_player() && *k != victim) {
                contributors.push(super::Contributor {
                    guid: k,
                    last_damage_time: now,
                    total_damage: 1,
                });
            }
        }
        let total: u64 = contributors.iter().map(|c| c.total_damage).sum();
        if total == 0 {
            return;
        }

        let eligible = |guid: ObjectGuid| -> bool {
            mgr.with_player(guid, |p| {
                p.is_alive()
                    && p.get_team() != victim_team
                    && distance(p.movement.position, victim_pos) <= REWARD_DISTANCE
            })
            .unwrap_or(false)
        };

        let mut by_group: HashMap<u32, u64> = HashMap::new();
        let mut alone: Vec<(ObjectGuid, u64)> = Vec::new();
        for c in &contributors {
            match world.systems.group.get_player_group_id(c.guid) {
                Some(group) => *by_group.entry(group).or_default() += c.total_damage,
                None => {
                    if eligible(c.guid) {
                        alone.push((c.guid, c.total_damage));
                    }
                }
            }
        }

        let mut rewards: Vec<(ObjectGuid, f32)> = Vec::new();
        for (group_id, damage) in by_group {
            let Some(group) = world.systems.group.get_group(group_id) else {
                continue;
            };
            let rewarded: Vec<ObjectGuid> = group
                .members
                .iter()
                .map(|m| m.guid)
                .filter(|&g| eligible(g))
                .collect();
            if rewarded.is_empty() {
                continue;
            }
            let rate = damage as f32 * group_rate(rewarded.len()) / total as f32 / rewarded.len() as f32;
            for member in rewarded {
                rewards.push((member, kill_value(world, member, victim) * rate));
            }
        }
        for (player, damage) in alone {
            rewards.push((player, kill_value(world, player, victim) * damage as f32 / total as f32));
        }
        for (player, points) in rewards {
            let points = points.floor();
            if points > 0.0 {
                add_honor(
                    world,
                    player,
                    points,
                    HonorType::Honorable,
                    Some((victim, VICTIM_PLAYER, victim.counter())),
                );
            }
        }
    }
}

/// A player killed a creature: racial leaders are honourable kills, gray
/// civilians dishonourable ones.
pub fn on_creature_killed(world: &World, killer: ObjectGuid, creature: ObjectGuid, entry: u32, creature_level: u32) {
    if !killer.is_player() {
        return;
    }
    let Some(template) = world.managers.creature_mgr.get_template(entry) else {
        return;
    };
    let killer_level = world
        .managers
        .player_mgr
        .with_player(killer, |p| u32::from(p.level))
        .unwrap_or(0);
    if template.ai.civilian {
        if world.config.honor_dishonorable_kills && rank::level_factor(killer_level, creature_level) == 0.0 {
            add_honor(
                world,
                killer,
                rank::dishonorable_kill_points(killer_level),
                HonorType::Dishonorable,
                Some((creature, VICTIM_UNIT, entry)),
            );
        }
        return;
    }
    if template.ai.racial_leader {
        add_honor(
            world,
            killer,
            RACIAL_LEADER_HONOR,
            HonorType::Honorable,
            Some((creature, VICTIM_UNIT, entry)),
        );
    }
}

/// Load a player's honour at login.
pub async fn load(world: &World, player: ObjectGuid) {
    let pool = &world.databases.character;
    let guid = i64::from(player.counter());
    let stored = sqlx::query_as::<_, (f32, i64, i64, i64, f32, i64, i64)>(
        "SELECT honor_rank_points, honor_standing::bigint, honor_highest_rank::bigint,
                honor_last_week_hk::bigint, honor_last_week_cp, honor_stored_hk::bigint,
                honor_stored_dk::bigint
         FROM characters.characters WHERE guid = $1",
    )
    .bind(guid)
    .fetch_optional(pool)
    .await;
    let entries = sqlx::query_as::<_, (i64, i64, f32, i64, i64)>(
        "SELECT victim_type::bigint, victim_id::bigint, cp, date::bigint, type::bigint
         FROM characters.character_honor_cp WHERE guid = $1",
    )
    .bind(guid)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    let stored = match stored {
        Ok(stored) => stored,
        Err(e) => {
            tracing::warn!("Failed to load honour of {:?}: {}", player, e);
            None
        }
    };
    world.managers.player_mgr.with_player_mut(player, |p| {
        if let Some((rp, standing, highest, lw_hk, lw_cp, hk, dk)) = stored {
            p.honor.rank_points = rp;
            p.honor.standing = standing as u32;
            p.honor.highest_rank = rank::rank_info(highest as u8, true);
            p.honor.last_week_hk = lw_hk as u32;
            p.honor.last_week_cp = lw_cp;
            p.honor.stored_hk = hk as u32;
            p.honor.stored_dk = dk as u32;
        }
        p.honor.entries = entries
            .into_iter()
            .map(|(victim_type, victim_id, cp, date, kind)| HonorCp {
                victim_type: victim_type as u8,
                victim_id: victim_id as u32,
                cp,
                date: date as u32,
                kind: kind as u8,
                saved: true,
            })
            .collect();
        p.honor.refresh_rank();
    });
    send_fields(world, player);
}

/// Save a player's new contribution points and stored honour.
pub async fn save(world: &World, player: ObjectGuid) {
    let Some(state) = world.managers.player_mgr.with_player_mut(player, |p| {
        let state = p.honor.clone();
        for entry in &mut p.honor.entries {
            entry.saved = true;
        }
        state
    }) else {
        return;
    };
    let pool = &world.databases.character;
    let guid = i64::from(player.counter());
    for entry in state.entries.iter().filter(|e| !e.saved) {
        if let Err(e) = sqlx::query(
            "INSERT INTO characters.character_honor_cp (guid, victim_type, victim_id, cp, date, type)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(guid)
        .bind(i16::from(entry.victim_type))
        .bind(i64::from(entry.victim_id))
        .bind(entry.cp)
        .bind(i64::from(entry.date))
        .bind(i16::from(entry.kind))
        .execute(pool)
        .await
        {
            tracing::warn!("Failed to save honour of {:?}: {}", player, e);
        }
    }
    if let Err(e) = sqlx::query(
        "UPDATE characters.characters SET honor_rank_points = $1, honor_highest_rank = $2
         WHERE guid = $3",
    )
    .bind(state.rank_points)
    .bind(i64::from(state.highest_rank.rank))
    .bind(guid)
    .execute(pool)
    .await
    {
        tracing::warn!("Failed to save honour of {:?}: {}", player, e);
    }
}

/// One week's standing: who earned what.
#[derive(Debug, Clone, Default)]
struct WeeklyScore {
    level: u32,
    team: Option<Team>,
    hk: u32,
    dk: u32,
    cp: f32,
    old_rp: f32,
    new_rp: f32,
    standing: u32,
    highest_rank: u8,
}

/// Run the weekly ranking if a maintenance day has passed. Returns whether it ran.
///
/// Every character's contribution points of the week rank them within their
/// faction; the standing earns rank points along that week's curve, minus
/// decay, capped by level. Characters below the kill minimum only decay.
pub async fn weekly_maintenance(world: &World) -> anyhow::Result<bool> {
    let pool = &world.databases.character;
    let today = game_day();
    let (last, next) = sqlx::query_as::<_, (i64, i64)>(
        "SELECT honor_last_maintenance_day::bigint, honor_next_maintenance_day::bigint
         FROM characters.saved_variables WHERE key = 0",
    )
    .fetch_optional(pool)
    .await?
    .unwrap_or((0, 0));
    let mut last = last as u32;
    let mut next = next as u32;
    if last == 0 {
        last = last_weekday(today, world.config.honor_maintenance_day);
        next = last + 7;
        save_maintenance_days(world, last, next).await?;
        return Ok(false);
    }
    if today < next {
        return Ok(false);
    }
    while today >= next {
        run_week(world, last, last + 6).await?;
        last = next;
        next = last + 7;
        save_maintenance_days(world, last, next).await?;
    }
    // Online players see their new rank at once.
    for player in world.managers.player_mgr.collect_online_guids() {
        load(world, player).await;
    }
    Ok(true)
}

async fn save_maintenance_days(world: &World, last: u32, next: u32) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO characters.saved_variables (key, honor_last_maintenance_day, honor_next_maintenance_day)
         VALUES (0, $1, $2)
         ON CONFLICT (key) DO UPDATE SET honor_last_maintenance_day = EXCLUDED.honor_last_maintenance_day,
             honor_next_maintenance_day = EXCLUDED.honor_next_maintenance_day",
    )
    .bind(i64::from(last))
    .bind(i64::from(next))
    .execute(&world.databases.character)
    .await?;
    Ok(())
}

async fn run_week(world: &World, begin: u32, end: u32) -> anyhow::Result<()> {
    let pool = &world.databases.character;
    // Online players' unsaved points belong to the week too.
    for player in world.managers.player_mgr.collect_online_guids() {
        save(world, player).await;
    }
    let rows = sqlx::query_as::<_, (i64, i64, i64, f32, i64, i64, i64, f32)>(
        "SELECT c.guid::bigint, c.level::bigint, c.race::bigint, c.honor_rank_points,
                c.honor_highest_rank::bigint,
                COALESCE(SUM(CASE WHEN h.type = 1 THEN 1 ELSE 0 END), 0)::bigint,
                COALESCE(SUM(CASE WHEN h.type = 2 THEN 1 ELSE 0 END), 0)::bigint,
                COALESCE(SUM(CASE WHEN h.type <> 2 THEN h.cp ELSE 0 END), 0)::real
         FROM characters.characters c
         LEFT JOIN characters.character_honor_cp h
           ON h.guid = c.guid AND h.date BETWEEN $1 AND $2
         WHERE c.honor_rank_points <> 0 OR h.guid IS NOT NULL
         GROUP BY c.guid, c.level, c.race, c.honor_rank_points, c.honor_highest_rank",
    )
    .bind(i64::from(begin))
    .bind(i64::from(end))
    .fetch_all(pool)
    .await?;

    let min_hk = world.config.min_honor_kills.unwrap_or(15);
    let mut scores: HashMap<i64, WeeklyScore> = HashMap::new();
    for (guid, level, race, rp, highest, hk, dk, cp) in rows {
        scores.insert(
            guid,
            WeeklyScore {
                level: level as u32,
                team: match Team::from_race(race as u8) {
                    Team::Alliance => Some(Team::Alliance),
                    Team::Horde => Some(Team::Horde),
                    _ => None,
                },
                hk: hk as u32,
                dk: dk as u32,
                cp,
                old_rp: rp,
                highest_rank: highest as u8,
                ..WeeklyScore::default()
            },
        );
    }

    let decay = world.config.honor_rp_decay;
    let pool_size = world.config.pvp_pool_size_per_faction.unwrap_or(0);
    for team in [Team::Alliance, Team::Horde] {
        let mut standing: Vec<(i64, f32)> = scores
            .iter()
            .filter(|(_, s)| s.team == Some(team) && s.hk >= min_hk)
            .map(|(&g, s)| (g, s.cp))
            .collect();
        standing.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        if standing.is_empty() {
            continue;
        }
        let cps: Vec<f32> = standing.iter().map(|(_, cp)| *cp).collect();
        let curve = rank::generate_scores(&cps, pool_size);
        for (position, (guid, cp)) in standing.iter().enumerate() {
            if let Some(score) = scores.get_mut(guid) {
                let earning = rank::rp_earning(*cp, &curve);
                score.new_rp = rank::rp_after_decay(earning, score.old_rp, decay)
                    .min(rank::max_rp_at_level(score.level));
                score.standing = position as u32 + 1;
            }
        }
    }
    // The inactive only decay.
    for score in scores.values_mut() {
        if score.standing == 0 {
            score.new_rp = rank::rp_after_decay(0.0, score.old_rp, decay);
        }
    }

    sqlx::query("UPDATE characters.characters SET honor_standing = 0 WHERE honor_standing > 0")
        .execute(pool)
        .await?;
    for (guid, score) in &scores {
        let current = rank::calculate_rank(score.new_rp);
        let mut highest = rank::rank_info(score.highest_rank, true);
        if current.visual_rank > 0 && current.visual_rank > highest.visual_rank {
            highest = current;
        }
        sqlx::query(
            "UPDATE characters.characters SET honor_highest_rank = $1, honor_rank_points = $2,
                 honor_standing = $3, honor_last_week_hk = $4,
                 honor_stored_hk = honor_stored_hk + $5, honor_stored_dk = honor_stored_dk + $6,
                 honor_last_week_cp = $7
             WHERE guid = $8",
        )
        .bind(i64::from(highest.rank))
        .bind(score.new_rp)
        .bind(i64::from(score.standing))
        .bind(i64::from(score.hk))
        .bind(score.hk as i32)
        .bind(score.dk as i32)
        .bind(score.cp)
        .bind(guid)
        .execute(pool)
        .await?;
    }
    // Keep only the last day (shown as "yesterday") and newer.
    sqlx::query("DELETE FROM characters.character_honor_cp WHERE date < $1")
        .bind(i64::from(end))
        .execute(pool)
        .await?;
    tracing::info!(
        "Honour week {}-{} ranked {} characters",
        begin,
        end,
        scores.len()
    );
    Ok(())
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cp(kind: HonorType, date: u32, value: f32) -> HonorCp {
        HonorCp {
            victim_type: VICTIM_PLAYER,
            victim_id: 1,
            cp: value,
            date,
            kind: kind as u8,
            saved: true,
        }
    }

    #[test]
    fn counters_split_today_yesterday_and_the_week() {
        let state = HonorState {
            entries: vec![
                cp(HonorType::Honorable, 100, 10.0),
                cp(HonorType::Honorable, 99, 20.0),
                cp(HonorType::Bonus, 99, 5.0),
                cp(HonorType::Dishonorable, 100, 50.0),
                cp(HonorType::Honorable, 90, 40.0),
            ],
            stored_hk: 7,
            ..HonorState::default()
        };
        let c = state.counters(100, 95);
        assert_eq!((c.today_hk, c.today_dk), (1, 1));
        assert_eq!(c.yesterday_hk, 1);
        assert_eq!(c.yesterday_cp, 25.0);
        assert_eq!(c.this_week_hk, 2);
        assert_eq!(c.this_week_cp, 35.0);
        assert_eq!(c.total_hk, 9);
        assert_eq!(c.total_dk, 1);
    }

    #[test]
    fn the_week_starts_on_the_maintenance_weekday() {
        // Day 0 was a Thursday (weekday 4).
        assert_eq!(last_weekday(0, 4), 0);
        assert_eq!(last_weekday(6, 4), 0);
        assert_eq!(last_weekday(7, 4), 7);
        // The Wednesday before day 0 is day -1: day 6 is a Wednesday.
        assert_eq!(last_weekday(13, 3), 13);
        assert_eq!(last_weekday(12, 3), 6);
    }

    #[test]
    fn repeat_kills_are_counted_per_day() {
        let state = HonorState {
            entries: vec![cp(HonorType::Honorable, 5, 1.0), cp(HonorType::Honorable, 5, 1.0), cp(HonorType::Honorable, 4, 1.0)],
            ..HonorState::default()
        };
        assert_eq!(state.kills_of_today(VICTIM_PLAYER, 1, 5), 2);
        assert_eq!(state.kills_of_today(VICTIM_PLAYER, 2, 5), 0);
    }
}
