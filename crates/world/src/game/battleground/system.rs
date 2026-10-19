//! Battleground flows: queueing, invitations, entering and leaving a match,
//! the match clock and scoreboard, resurrection waves, logging in and out.

use oxcore_shared::game::battleground::{
    group_join, BattleGroundTypeId, MAX_QUEUE_SLOTS, SPELL_DESERTER, SPELL_WAITING_TO_RESURRECT,
};
use oxcore_shared::game::chat::{ChatMsg, ChatTag, Language, Team};
use oxcore_shared::messages::battleground::{
    BattlefieldState, BattlegroundPosition, PvpLogPlayer, SmsgBattlefieldList,
    SmsgBattlefieldResult, SmsgBattlefieldStatus, SmsgBattlegroundPlayerJoinedOrLeft,
    SmsgBattlegroundPlayerPositions, SmsgGroupJoinedBattleground, SmsgPlaySound, SmsgPvpLogData,
};
use oxcore_shared::messages::chat::SmsgMessageChat;
use oxcore_shared::messages::movement::SmsgControlUpdate;
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::{ObjectGuid, Position, Protocol};

use super::matches::{
    Match, MatchStatus, Score, StartStage, INVITATION_REMIND_MS, INVITE_ACCEPT_WAIT_MS,
    LOSER_MARK_MIN_AGE_MS, RESURRECTION_INTERVAL_MS, SOUND_ALLIANCE_WINS, SOUND_BG_START,
    SOUND_HORDE_WINS, TIME_TO_AUTOREMOVE_MS,
};
use super::player::EntryPoint;
use super::queue::{team_index, GroupId};
use crate::game::pvp::scope::Scope;
use crate::World;

/// `UNIT_NPC_FLAG_BATTLEMASTER`.
const NPC_FLAG_BATTLEMASTER: u32 = 0x0000_0800;
/// How close a player must stand to talk to a battlemaster.
const INTERACTION_DISTANCE: f32 = 5.0;
/// How close to an entrance portal a portal queue must be made.
const PORTAL_QUEUE_DISTANCE: f32 = 50.0;
/// Teammates within this range of a kill share the honourable kill.
const GROUP_REWARD_DISTANCE: f32 = 74.0;
/// A ghost queued with a spirit guide within this range is raised by its wave.
const SPIRIT_WAVE_RANGE: f32 = 20.0;
/// Players further than this from their start when the gates open go to a graveyard.
const START_AREA_RANGE: f32 = 100.0;
/// Spirit of Redemption: a priest dying in it does not count a death yet.
const SPELL_SPIRIT_OF_REDEMPTION: u32 = 27827;
/// How often 1.14 clients are sent the flag carriers' positions.
const POSITION_PUSH_MS: u32 = 2_000;

/// Failures of a group queue, reported in battleground chat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupJoinError {
    OfflineMember,
    TooMany,
    MixedFaction,
    MixedLevels,
    AlreadyQueued,
    Deserter,
    NoFreeSlot,
}

impl GroupJoinError {
    fn message(self) -> &'static str {
        match self {
            GroupJoinError::OfflineMember => {
                "Your group has an offline member. Please remove them before joining."
            }
            GroupJoinError::TooMany => {
                "Your group is too large for this battleground. Please regroup to join."
            }
            GroupJoinError::MixedFaction => {
                "Your group has players from the opposing faction. You can't join the battleground as a group."
            }
            GroupJoinError::MixedLevels => {
                "Your group has players from different battleground brackets. You can't join as a group."
            }
            GroupJoinError::AlreadyQueued => {
                "Someone in your party is already in this battleground queue. They must leave it before joining as a group."
            }
            GroupJoinError::Deserter => "Someone in your party is a Deserter. You can't join as a group.",
            GroupJoinError::NoFreeSlot => {
                "Someone in your party is already in three battleground queues. You cannot join as a group."
            }
        }
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn send(world: &World, player: ObjectGuid, msg: impl ToWorldPacket + Send) {
    world.managers.broadcast_mgr.send_msg_to_player(player, msg);
}

fn is_modern(world: &World, player: ObjectGuid) -> bool {
    world
        .session_mgr
        .get_session_by_player(player)
        .is_some_and(|s| s.protocol() == Protocol::Modern)
}

fn race_team(world: &World, player: ObjectGuid) -> Option<Team> {
    world
        .managers
        .player_mgr
        .with_player(player, |p| p.get_team())
        .filter(|t| matches!(t, Team::Alliance | Team::Horde))
}

/// The side a player fights for: their match team, else their faction.
pub fn player_team(world: &World, player: ObjectGuid) -> Option<Team> {
    world
        .managers
        .player_mgr
        .with_player(player, |p| p.battleground.team)
        .flatten()
        .or_else(|| race_team(world, player))
}

fn has_aura(world: &World, player: ObjectGuid, spell_id: u32) -> bool {
    world
        .managers
        .player_mgr
        .with_player(player, |p| p.auras.container.has_aura(spell_id))
        .unwrap_or(false)
}

fn level_of(world: &World, player: ObjectGuid) -> u32 {
    world
        .managers
        .player_mgr
        .with_player(player, |p| u32::from(p.level))
        .unwrap_or(0)
}

/// The match a player is in (or being sent to).
pub fn match_of(world: &World, player: ObjectGuid) -> Option<u32> {
    world
        .managers
        .player_mgr
        .with_player(player, |p| p.battleground.instance_id)
        .filter(|&id| id != 0)
}

/// Whether the player is a participant of a running match on their map.
pub fn in_battleground(world: &World, player: ObjectGuid) -> bool {
    let Some((map, instance)) = world
        .managers
        .player_mgr
        .with_player(player, |p| (p.map_id, p.instance_id))
    else {
        return false;
    };
    world
        .managers
        .battleground_mgr
        .with_match(instance, |m| m.map_id == map && m.players.contains_key(&player))
        .unwrap_or(false)
}

/// Announce to a match (or one side), in battleground chat.
pub fn announce(world: &World, instance_id: u32, text: &str, chat: ChatMsg, team: Option<Team>) {
    let Some(players) = world
        .managers
        .battleground_mgr
        .with_match(instance_id, |m| {
            m.players
                .iter()
                .filter(|(_, &t)| team.map_or(true, |team| t == team))
                .map(|(&g, _)| g)
                .collect::<Vec<_>>()
        })
    else {
        return;
    };
    let message = system_chat(text, chat);
    world
        .managers
        .broadcast_mgr
        .broadcast_msg_to_players(&players, &message);
}

pub(crate) fn system_chat(text: &str, chat: ChatMsg) -> SmsgMessageChat<'_> {
    SmsgMessageChat {
        msgtype: chat,
        language: Language::Universal,
        sender_guid: ObjectGuid::empty(),
        sender_name: None,
        target_guid: None,
        channel_name: None,
        player_rank: None,
        message: text,
        chat_tag: ChatTag::None,
    }
}

fn play_sound(world: &World, players: &[ObjectGuid], sound_id: u32) {
    world
        .managers
        .broadcast_mgr
        .broadcast_msg_to_players(players, &SmsgPlaySound { sound_id });
}

// ==================== Status ====================

/// Show a queue slot's state to its player.
fn send_status(
    world: &World,
    player: ObjectGuid,
    slot: usize,
    bg_type: BattleGroundTypeId,
    m: Option<&Match>,
    state: BattlefieldState,
) {
    let bgm = &world.managers.battleground_mgr;
    let (bracket, client_id, min_level, max_level) = match m {
        Some(m) => (m.bracket, m.client_instance_id, m.min_level, m.max_level),
        None => {
            let level = level_of(world, player);
            match bgm.template(bg_type) {
                Some(t) => {
                    let bracket = t.bracket_for_level(level).unwrap_or(0);
                    let (min, max) = t.bracket_levels(bracket);
                    (bracket, 0, min, max)
                }
                None => (0, 0, 0, 0),
            }
        }
    };
    let ticket_time = world
        .managers
        .player_mgr
        .with_player(player, |p| p.battleground.ticket_times[slot])
        .unwrap_or_else(now_unix);
    send(
        world,
        player,
        SmsgBattlefieldStatus {
            player_guid: player,
            queue_slot: slot as u32,
            ticket_time,
            bg_type,
            bracket,
            min_level,
            max_level,
            client_instance_id: client_id,
            state,
        },
    );
}

/// Clear a slot on the client.
fn send_status_none(world: &World, player: ObjectGuid, slot: usize, bg_type: BattleGroundTypeId) {
    send_status(world, player, slot, bg_type, None, BattlefieldState::None { bg_type });
}

/// Show the "in the match" status.
fn send_status_active(world: &World, player: ObjectGuid, m: &Match) {
    let slot = world
        .managers
        .player_mgr
        .with_player(player, |p| p.battleground.slot_of(m.bg_type))
        .flatten()
        .unwrap_or(0);
    let shutdown_ms = if m.status == MatchStatus::WaitLeave {
        m.end_timer_ms.max(0) as u32
    } else {
        0
    };
    // A 1.14 client wants the init packet ahead of a running match's status.
    if shutdown_ms == 0 && is_modern(world, player) {
        send(world, player, oxcore_shared::messages::battleground::SmsgBattlegroundInit::RUNNING);
    }
    send_status(
        world,
        player,
        slot,
        m.bg_type,
        Some(m),
        BattlefieldState::Active {
            shutdown_ms,
            elapsed_ms: m.age_ms,
        },
    );
}

/// `CMSG_BATTLEFIELD_STATUS`: report every queue slot again.
pub fn send_all_statuses(world: &World, player: ObjectGuid) {
    let bgm = &world.managers.battleground_mgr;
    let Some(state) = world
        .managers
        .player_mgr
        .with_player(player, |p| p.battleground.clone())
    else {
        return;
    };
    for slot in 0..MAX_QUEUE_SLOTS {
        let Some(bg_type) = state.queue_slots[slot] else {
            continue;
        };
        if state.instance_id != 0 {
            if let Some(m) = bgm.get_match(state.instance_id).filter(|m| m.bg_type == bg_type) {
                send_status_active(world, player, &m);
                continue;
            }
        }
        let group = bgm.with_queue(bg_type, |q| q.group_of(player).cloned());
        let Some(group) = group else {
            continue;
        };
        if group.invited_to != 0 {
            if let Some(m) = bgm.get_match(group.invited_to) {
                let left = group.remove_invite_at_ms.saturating_sub(bgm.now_ms()) as u32;
                send_status(
                    world,
                    player,
                    slot,
                    bg_type,
                    Some(&m),
                    BattlefieldState::NeedConfirmation { timeout_ms: left },
                );
            }
            continue;
        }
        let average = bgm.with_queue(bg_type, |q| q.average_wait(group.team, group.bracket));
        send_status(
            world,
            player,
            slot,
            bg_type,
            None,
            BattlefieldState::Queued {
                average_wait_ms: average,
                waited_ms: bgm.now_ms().saturating_sub(group.join_ms) as u32,
                as_group: group.players.len() > 1,
            },
        );
    }
}

// ==================== Battlemaster and list ====================

/// The list of copies offered for a battleground, as seen by `player`.
pub fn send_list(world: &World, player: ObjectGuid, battlemaster: ObjectGuid, bg_type: BattleGroundTypeId) {
    let bgm = &world.managers.battleground_mgr;
    let Some(template) = bgm.template(bg_type) else {
        return;
    };
    let bracket = template
        .bracket_for_level(level_of(world, player))
        .unwrap_or(0);
    let (min_level, max_level) = template.bracket_levels(bracket);
    send(
        world,
        player,
        SmsgBattlefieldList {
            battlemaster_guid: battlemaster,
            bg_type,
            bracket,
            min_level,
            max_level,
            client_instance_ids: bgm.client_ids(bg_type, bracket),
        },
    );
}

/// Whether a player's level lets them into a battleground.
fn has_level_access(world: &World, player: ObjectGuid, bg_type: BattleGroundTypeId) -> bool {
    let level = level_of(world, player);
    world
        .managers
        .battleground_mgr
        .template(bg_type)
        .is_some_and(|t| level >= t.min_level && level <= t.max_level)
}

/// A creature a player may talk to as a battlemaster, and its battleground.
fn battlemaster_bg(world: &World, player: ObjectGuid, npc: ObjectGuid) -> Option<BattleGroundTypeId> {
    let (player_pos, map) = world
        .managers
        .player_mgr
        .with_player(player, |p| (p.movement.position, p.map_id))?;
    let (entry, flags, pos, npc_map) = world
        .managers
        .creature_mgr
        .with_creature(npc, |c| (c.entry, c.npc_flags, c.position, c.map_id))?;
    if flags & NPC_FLAG_BATTLEMASTER == 0 || npc_map != map {
        return None;
    }
    if distance(player_pos, pos) > INTERACTION_DISTANCE + 5.0 {
        return None;
    }
    world.managers.battleground_mgr.battlemaster_bg(entry)
}

fn distance(a: Position, b: Position) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// `CMSG_BATTLEMASTER_HELLO` (and the battlemaster gossip option).
pub fn battlemaster_hello(world: &World, player: ObjectGuid, npc: ObjectGuid) {
    let Some(bg_type) = battlemaster_bg(world, player, npc) else {
        return;
    };
    if !has_level_access(world, player, bg_type) {
        let min = world
            .managers
            .battleground_mgr
            .template(bg_type)
            .map(|t| t.min_level)
            .unwrap_or(0);
        send(
            world,
            player,
            system_chat(
                &format!("You must be level {} to enter this battleground.", min),
                ChatMsg::System,
            ),
        );
        return;
    }
    send_list(world, player, npc, bg_type);
}

/// An area trigger at a battleground entrance portal: refuse the wrong side
/// or level, else remember where to send the player back and offer the list.
/// Returns whether the trigger was an entrance.
pub fn entrance_trigger(world: &World, player: ObjectGuid, trigger_id: u32) -> bool {
    let bgm = &world.managers.battleground_mgr;
    let Some(entrance) = bgm.entrance(trigger_id) else {
        return false;
    };
    let Some(template) = bgm.template(entrance.bg_type) else {
        return true;
    };
    let level = level_of(world, player);
    let team = race_team(world, player);
    if level < template.min_level || level > template.max_level || team != Some(entrance.team) {
        let side = if entrance.team == Team::Alliance { "Alliance" } else { "Horde" };
        let suffix = if template.min_level % 2 == 1 { "st" } else { "th" };
        send(
            world,
            player,
            oxcore_shared::messages::instance::SmsgAreaTriggerMessage {
                trigger_id,
                text: format!(
                    "You must be in the {} and at least {}{} level to enter.",
                    side, template.min_level, suffix
                ),
            },
        );
        return true;
    }
    world.managers.player_mgr.with_player_mut(player, |p| {
        if !p.battleground.in_battleground() {
            p.battleground.entry_point = Some(EntryPoint {
                map_id: entrance.exit_map,
                position: entrance.exit,
            });
        }
    });
    send_list(world, player, player, entrance.bg_type);
    true
}

// ==================== Joining a queue ====================

/// Remember where a player came from, before they first enter a battleground.
fn set_entry_point(world: &World, player: ObjectGuid, leader: ObjectGuid, at_portal: bool) {
    let leader_place = world.managers.player_mgr.with_player(leader, |p| {
        (
            p.map_id,
            p.movement.position,
            p.unit_flags & crate::game::common::unit_flags::ON_TAXI != 0,
            p.battleground.entry_point,
        )
    });
    world.managers.player_mgr.with_player_mut(player, |p| {
        if p.battleground.in_battleground() {
            return;
        }
        let homebind = EntryPoint {
            map_id: p.homebind_map,
            position: Position::new(p.homebind_x, p.homebind_y, p.homebind_z, 0.0),
        };
        let Some((map, position, flying, leader_entry)) = leader_place else {
            p.battleground.entry_point = Some(homebind);
            return;
        };
        if at_portal {
            p.battleground.entry_point = leader_entry.or(Some(homebind));
            return;
        }
        let instanced = crate::game::instance::lifecycle::map_entry(world, map)
            .is_some_and(|e| e.is_dungeon() || e.is_battleground());
        p.battleground.entry_point = if flying || instanced {
            Some(homebind)
        } else {
            Some(EntryPoint {
                map_id: map,
                position,
            })
        };
    });
}

/// Why a group may not queue together.
fn check_group(
    world: &World,
    members: &[ObjectGuid],
    bg_type: BattleGroundTypeId,
    bracket: u8,
    team: Team,
    max_per_team: u32,
) -> Result<(), GroupJoinError> {
    if members.len() as u32 > max_per_team {
        return Err(GroupJoinError::TooMany);
    }
    let template = world.managers.battleground_mgr.template(bg_type);
    for &member in members {
        let Some((level, member_team, slots_free, queued)) =
            world.managers.player_mgr.with_player(member, |p| {
                (
                    u32::from(p.level),
                    p.get_team(),
                    p.battleground.has_free_slot(),
                    p.battleground.slot_of(bg_type).is_some(),
                )
            })
        else {
            return Err(GroupJoinError::OfflineMember);
        };
        if member_team != team {
            return Err(GroupJoinError::MixedFaction);
        }
        if template.as_ref().and_then(|t| t.bracket_for_level(level)) != Some(bracket) {
            return Err(GroupJoinError::MixedLevels);
        }
        if queued {
            return Err(GroupJoinError::AlreadyQueued);
        }
        if has_aura(world, member, SPELL_DESERTER) {
            return Err(GroupJoinError::Deserter);
        }
        if !slots_free {
            return Err(GroupJoinError::NoFreeSlot);
        }
    }
    Ok(())
}

/// Join a battleground's queue, alone or with the group.
///
/// `battlemaster` is the NPC talked to, or the player themselves when
/// queueing at an entrance portal; `client_instance` asks for one copy (0: any).
pub fn request_join(
    world: &World,
    player: ObjectGuid,
    battlemaster: ObjectGuid,
    bg_type: BattleGroundTypeId,
    client_instance: u32,
    as_group: bool,
) {
    let bgm = &world.managers.battleground_mgr;
    if bg_type == BattleGroundTypeId::None {
        return;
    }
    // Alterac Valley is solo queue only.
    if bg_type == BattleGroundTypeId::AlteracValley && as_group {
        return;
    }
    let at_portal = battlemaster == player;
    if at_portal {
        let near_portal = world
            .managers
            .player_mgr
            .with_player(player, |p| {
                p.battleground.entry_point.is_some_and(|e| {
                    e.map_id == p.map_id && distance(e.position, p.movement.position) <= PORTAL_QUEUE_DISTANCE
                })
            })
            .unwrap_or(false);
        if !near_portal {
            return;
        }
    } else if battlemaster_bg(world, player, battlemaster) != Some(bg_type) {
        return;
    }
    if world
        .managers
        .player_mgr
        .with_player(player, |p| p.battleground.in_battleground())
        .unwrap_or(true)
    {
        return;
    }
    let Some(template) = bgm.template(bg_type) else {
        return;
    };
    let Some(bracket) = template.bracket_for_level(level_of(world, player)) else {
        return;
    };
    let Some(team) = race_team(world, player) else {
        return;
    };
    let now_ms = bgm.now_ms();
    let now = now_unix();

    if !as_group {
        if has_aura(world, player, SPELL_DESERTER) {
            send(world, player, SmsgGroupJoinedBattleground { result: group_join::DESERTERS });
            return;
        }
        let blocked = world
            .managers
            .player_mgr
            .with_player(player, |p| {
                p.battleground.slot_of(bg_type).is_some() || !p.battleground.has_free_slot()
            })
            .unwrap_or(true);
        if blocked {
            return;
        }
        let group = bgm.with_queue(bg_type, |q| {
            q.add_group(team, vec![player], bracket, false, client_instance, now_ms, now)
        });
        queued(world, player, player, bg_type, group, at_portal, false, now);
        announce_queue(world, player, bg_type, bracket);
    } else {
        let Some(group) = world.systems.group.get_player_group(player) else {
            return;
        };
        if group.leader_guid != player {
            return;
        }
        let members: Vec<ObjectGuid> = group.members.iter().map(|m| m.guid).collect();
        if let Err(err) = check_group(world, &members, bg_type, bracket, team, template.max_players_per_team) {
            if err == GroupJoinError::Deserter {
                send(world, player, SmsgGroupJoinedBattleground { result: group_join::DESERTERS });
            }
            send(world, player, system_chat(err.message(), ChatMsg::BgSystemNeutral));
            return;
        }
        let config = &world.config;
        // Groups above the limit are queued one by one.
        let limit = config.battleground_group_queue_limit;
        if members.len() as u32 > limit {
            let text = format!("Group queue limit is set to {limit}. You have been queued solo.");
            for &member in &members {
                let group_id = bgm.with_queue(bg_type, |q| {
                    q.add_group(team, vec![member], bracket, false, client_instance, now_ms, now)
                });
                queued(world, member, member, bg_type, group_id, at_portal, false, now);
                send(world, member, system_chat(&text, ChatMsg::System));
            }
            announce_queue(world, player, bg_type, bracket);
            bgm.schedule(bg_type, bracket);
            return;
        }
        let premade = config.battleground_premade_wait_ms > 0
            && members.len() as u32 >= config.battleground_premade_min_size;
        let group_id = bgm.with_queue(bg_type, |q| {
            q.add_group(team, members.clone(), bracket, premade, client_instance, now_ms, now)
        });
        for &member in &members {
            queued(world, member, player, bg_type, group_id, at_portal, true, now);
            send(
                world,
                member,
                SmsgGroupJoinedBattleground {
                    result: bg_type.map_id() as i32,
                },
            );
        }
        if !premade {
            announce_queue(world, player, bg_type, bracket);
        }
    }
    bgm.schedule(bg_type, bracket);
}

/// The queue announcer: how many wait on each side and how many more are
/// needed, told to the joiner (setting 1) or to everyone (setting 2).
fn announce_queue(world: &World, joiner: ObjectGuid, bg_type: BattleGroundTypeId, bracket: u8) {
    let mode = world.config.battleground_queue_announcer_join;
    if mode == 0 {
        return;
    }
    let bgm = &world.managers.battleground_mgr;
    let Some(template) = bgm.template(bg_type) else {
        return;
    };
    let (min_level, max_level) = template.bracket_levels(bracket);
    let [alliance, horde] =
        bgm.with_queue(bg_type, |q| [q.waiting_players(bracket, 0), q.waiting_players(bracket, 1)]);
    let need = |n: u32| template.min_players_per_team.saturating_sub(n);
    if mode == 1 {
        let text = format!(
            "Queue status for {} (Lvl: {} to {})\nQueued alliances: {} (Need at least {} more)\nQueued hordes: {} (Need at least {} more)",
            template.name, min_level, max_level, alliance, need(alliance), horde, need(horde)
        );
        send(world, joiner, system_chat(&text, ChatMsg::System));
    } else {
        let text = format!(
            "|cffff0000[BG Queue Announcer]:|r {} -- [{}-{}] A: {} (Need: {}), H: {} (Need: {})|r",
            template.name, min_level, max_level, alliance, need(alliance), horde, need(horde)
        );
        world
            .managers
            .broadcast_mgr
            .broadcast_msg_to_all(&system_chat(&text, ChatMsg::System));
    }
}

/// One player's side of joining: take a slot, remember the way back, show the status.
#[allow(clippy::too_many_arguments)]
fn queued(
    world: &World,
    player: ObjectGuid,
    leader: ObjectGuid,
    bg_type: BattleGroundTypeId,
    group: GroupId,
    at_portal: bool,
    as_group: bool,
    now: i64,
) {
    let slot = world
        .managers
        .player_mgr
        .with_player_mut(player, |p| {
            let slot = p.battleground.add_queue(bg_type)?;
            p.battleground.ticket_times[slot] = now;
            Some(slot)
        })
        .flatten();
    let Some(slot) = slot else {
        return;
    };
    set_entry_point(world, player, leader, at_portal);
    let bgm = &world.managers.battleground_mgr;
    let (team, bracket) = bgm
        .with_queue(bg_type, |q| q.group(group).map(|g| (g.team, g.bracket)))
        .unwrap_or((Team::Alliance, 0));
    let average = bgm.with_queue(bg_type, |q| q.average_wait(team, bracket));
    send_status(
        world,
        player,
        slot,
        bg_type,
        None,
        BattlefieldState::Queued {
            average_wait_ms: average,
            waited_ms: 0,
            as_group,
        },
    );
}

// ==================== Queue passes ====================

/// Invite a queued group to a match.
fn invite_group(world: &World, bg_type: BattleGroundTypeId, group: GroupId, instance_id: u32) {
    let bgm = &world.managers.battleground_mgr;
    let now = bgm.now_ms();
    let Some((players, team)) = bgm.with_queue(bg_type, |q| {
        q.invite(group, instance_id, u64::from(INVITE_ACCEPT_WAIT_MS), now)
    }) else {
        return;
    };
    let Some(m) = bgm.with_match(instance_id, |m| {
        m.invited[team_index(team)] += players.len() as u32;
        m.clone()
    }) else {
        return;
    };
    for player in players {
        let slot = world
            .managers
            .player_mgr
            .with_player_mut(player, |p| {
                let slot = p.battleground.slot_of(bg_type)?;
                p.battleground.invited_to[slot] = instance_id;
                Some(slot)
            })
            .flatten();
        let Some(slot) = slot else {
            continue;
        };
        bgm.reminders
            .lock()
            .push((now + u64::from(INVITATION_REMIND_MS), player, instance_id));
        send_status(
            world,
            player,
            slot,
            bg_type,
            Some(&m),
            BattlefieldState::NeedConfirmation {
                timeout_ms: INVITE_ACCEPT_WAIT_MS,
            },
        );
    }
}

/// Start a new match: a fresh copy of the battleground's map.
fn create_match(world: &World, bg_type: BattleGroundTypeId, bracket: u8) -> Option<u32> {
    let bgm = &world.managers.battleground_mgr;
    let template = bgm.template(bg_type)?;
    let instance_id = world.managers.instance_mgr.reserve_instance_id()?;
    let client_id = bgm.take_client_id(bg_type, bracket);
    let columns = crate::game::pvp::objective::declared_number(
        world,
        crate::core::lua::objective::ScopeScript::Instance(template.map_id),
        "ScoreColumns",
    )
    .unwrap_or(0) as usize;
    let m = Match::new(
        instance_id,
        bg_type,
        bracket,
        client_id,
        template.bracket_levels(bracket),
        template.min_players_per_team,
        template.max_players_per_team,
        columns,
    );
    bgm.matches.insert(instance_id, m);
    world
        .managers
        .map_mgr
        .get_or_create_map(template.map_id, instance_id);
    tracing::info!(
        "Battleground {:?} bracket {} opened as copy {} (#{})",
        bg_type,
        bracket,
        instance_id,
        client_id
    );
    Some(instance_id)
}

/// One pass over a bracket's queue: fill running matches, start new ones.
fn update_queue(world: &World, bg_type: BattleGroundTypeId, bracket: u8) {
    let bgm = &world.managers.battleground_mgr;
    if !bgm.with_queue(bg_type, |q| q.has_players(bracket)) {
        return;
    }
    fill_free_slots(world, bg_type, bracket);
    let Some(template) = bgm.template(bg_type) else {
        return;
    };
    let testing = bgm.is_testing();
    let min = if testing { 1 } else { template.min_players_per_team };
    let mut max = template.max_players_per_team;
    let config = &world.config;
    // Alterac Valley may wait for a crowd before opening, and open smaller than it can grow.
    let mut initial_cap = false;
    if bg_type == BattleGroundTypeId::AlteracValley && !testing {
        let wanted = config.alterac_min_players_in_queue;
        if wanted > 0 {
            let waiting = bgm.with_queue(bg_type, |q| [q.waiting_players(bracket, 0), q.waiting_players(bracket, 1)]);
            if waiting.iter().any(|&n| n < wanted) {
                return;
            }
        }
        let cap = config.alterac_init_max_players;
        if cap > min && cap < max {
            max = cap;
            initial_cap = true;
        }
    }
    let balance = world.config.battleground_invitation_type != 0;
    let now = bgm.now_ms();
    let premade_wait = u64::from(world.config.battleground_premade_wait_ms);
    let premade_size = world.config.battleground_premade_min_size;
    let pools = bgm.with_queue(bg_type, |q| {
        let premade = (premade_wait > 0)
            .then(|| q.check_premade_match(bracket, min, max, premade_size, premade_wait, now, testing))
            .flatten();
        premade.or_else(|| q.check_normal_match(bracket, min, max, balance, testing))
    });
    let Some(pools) = pools else {
        return;
    };
    let Some(instance_id) = create_match(world, bg_type, bracket) else {
        return;
    };
    for pool in pools {
        for group in pool.groups {
            invite_group(world, bg_type, group, instance_id);
        }
    }
    // A capped opening fills up on later passes, as more players queue.
    if !initial_cap {
        fill_free_slots(world, bg_type, bracket);
    }
}

/// Offer the free places of running matches to waiting groups.
fn fill_free_slots(world: &World, bg_type: BattleGroundTypeId, bracket: u8) {
    let bgm = &world.managers.battleground_mgr;
    let balance = world.config.battleground_invitation_type != 0;
    for instance_id in bgm.matches_of(bg_type, bracket) {
        let Some(m) = bgm.get_match(instance_id) else {
            continue;
        };
        if !m.accepting || m.status == MatchStatus::WaitLeave {
            continue;
        }
        if !m.has_free_slots() {
            bgm.with_match(instance_id, |m| m.accepting = false);
            continue;
        }
        let free = [m.free_slots(Team::Alliance), m.free_slots(Team::Horde)];
        let pools = bgm.with_queue(bg_type, |q| {
            q.fill_free_slots(bracket, free, m.client_instance_id, balance)
        });
        for pool in pools {
            for group in pool.groups {
                invite_group(world, bg_type, group, instance_id);
            }
        }
    }
}

/// Take a player off a queue, releasing a promised place.
fn leave_queue(world: &World, player: ObjectGuid, bg_type: BattleGroundTypeId, decrease_invited: bool) {
    let bgm = &world.managers.battleground_mgr;
    let Some(group) = bgm.with_queue(bg_type, |q| q.remove_player(player)) else {
        return;
    };
    if decrease_invited && group.invited_to != 0 {
        bgm.with_match(group.invited_to, |m| {
            let side = &mut m.invited[team_index(group.team)];
            *side = side.saturating_sub(1);
            m.accepting = m.status != MatchStatus::WaitLeave;
        });
    }
    bgm.schedule(bg_type, group.bracket);
}

/// Unanswered invitations lapse; invitation reminders go out.
fn update_invitations(world: &World) {
    let bgm = &world.managers.battleground_mgr;
    let now = bgm.now_ms();
    let due: Vec<(ObjectGuid, u32)> = {
        let mut reminders = bgm.reminders.lock();
        let (due, rest): (Vec<_>, Vec<_>) = reminders.drain(..).partition(|(at, _, _)| *at <= now);
        *reminders = rest;
        due.into_iter().map(|(_, p, m)| (p, m)).collect()
    };
    for (player, instance_id) in due {
        let Some(m) = bgm.get_match(instance_id) else {
            continue;
        };
        let Some(slot) = world
            .managers
            .player_mgr
            .with_player(player, |p| {
                p.battleground
                    .slot_of(m.bg_type)
                    .filter(|&s| p.battleground.invited_to[s] == instance_id && !p.battleground.in_battleground())
            })
            .flatten()
        else {
            continue;
        };
        send_status(
            world,
            player,
            slot,
            m.bg_type,
            Some(&m),
            BattlefieldState::NeedConfirmation {
                timeout_ms: INVITE_ACCEPT_WAIT_MS - INVITATION_REMIND_MS,
            },
        );
    }

    for bg_type in BattleGroundTypeId::ALL {
        let expired = bgm.with_queue(bg_type, |q| {
            let mut gone = q.expired_invites(now);
            gone.extend(q.expired_offline(now).into_iter().map(|p| (p, 0)));
            gone
        });
        for (player, _) in expired {
            leave_queue(world, player, bg_type, true);
            let slot = world
                .managers
                .player_mgr
                .with_player_mut(player, |p| p.battleground.remove_queue(bg_type))
                .flatten();
            if let Some(slot) = slot {
                send_status_none(world, player, slot, bg_type);
            }
        }
    }
}

// ==================== Accepting or declining ====================

/// `CMSG_BATTLEFIELD_PORT`: enter the match invited to, or leave the queue.
pub async fn port(world: &World, player: ObjectGuid, bg_type: BattleGroundTypeId, accept: bool) {
    let bgm = &world.managers.battleground_mgr;
    let Some((slot, invited, gm)) = world
        .managers
        .player_mgr
        .with_player(player, |p| {
            p.battleground
                .slot_of(bg_type)
                .map(|slot| (slot, p.battleground.invited_to[slot], p.gm_mode))
        })
        .flatten()
    else {
        return;
    };
    let Some(group) = bgm.with_queue(bg_type, |q| q.group_of(player).cloned()) else {
        return;
    };
    let mut accept = accept;
    let target = if group.invited_to != 0 {
        bgm.get_match(group.invited_to)
    } else {
        None
    };
    if accept {
        let Some(m) = target.as_ref() else {
            return;
        };
        if invited != m.instance_id {
            return;
        }
        if has_aura(world, player, SPELL_DESERTER) && !gm {
            send(world, player, SmsgGroupJoinedBattleground { result: group_join::DESERTERS });
            accept = false;
        }
        if level_of(world, player) > u32::from(m.max_level) || m.status == MatchStatus::WaitLeave {
            accept = false;
        }
    }

    if !accept {
        world
            .managers
            .player_mgr
            .with_player_mut(player, |p| p.battleground.remove_queue(bg_type));
        leave_queue(world, player, bg_type, true);
        send_status_none(world, player, slot, bg_type);
        return;
    }
    let Some(m) = target else {
        return;
    };

    // Into the match.
    if !world.managers.player_mgr.is_player_alive(player) {
        let _ = world.systems.death.resurrect_in_battleground(player, world);
    }
    crate::game::taxi::flight::interrupt(world, player);
    send_status(
        world,
        player,
        slot,
        bg_type,
        Some(&m),
        BattlefieldState::Active {
            shutdown_ms: 0,
            elapsed_ms: m.age_ms,
        },
    );
    leave_queue(world, player, bg_type, false);
    if let Some(current) = match_of(world, player) {
        if current != m.instance_id {
            remove_player_at_leave(world, current, player, false, true).await;
        }
    }
    world.managers.player_mgr.with_player_mut(player, |p| {
        p.battleground.instance_id = m.instance_id;
        p.battleground.bg_type = Some(bg_type);
        p.battleground.team = Some(group.team);
    });
    save_player(world, player).await;
    send_to_match(world, player, &m, group.team).await;
}

/// Teleport a player to their side's start in a match.
async fn send_to_match(world: &World, player: ObjectGuid, m: &Match, team: Team) {
    let Some(template) = world.managers.battleground_mgr.template(m.bg_type) else {
        return;
    };
    // An away player would not come back by themselves.
    world.managers.player_mgr.with_player_mut(player, |p| {
        p.player_flags &= !crate::game::player::PLAYER_FLAGS_AFK;
    });
    let start = template.start_location(team);
    if let Err(e) = crate::game::player::teleport::teleport_to(world, player, m.map_id, start, 0).await {
        tracing::warn!("Failed to send {:?} into battleground: {}", player, e);
    }
}

/// The copy a player may enter on a battleground map: their match's.
pub fn transfer_instance(world: &World, player: ObjectGuid, map_id: u32) -> Option<u32> {
    let instance_id = match_of(world, player)?;
    world
        .managers
        .battleground_mgr
        .with_match(instance_id, |m| m.map_id == map_id)
        .filter(|&same| same)
        .map(|_| instance_id)
}

// ==================== Inside the match ====================

/// A player finished loading into a copy: join it if it is their match.
pub async fn on_player_arrived(world: &World, player: ObjectGuid, map_id: u32, instance_id: u32) {
    let bgm = &world.managers.battleground_mgr;
    if !bgm.is_match_copy(map_id, instance_id) {
        return;
    }
    if match_of(world, player) != Some(instance_id) {
        return;
    }
    let Some(team) = player_team(world, player) else {
        return;
    };
    let Some(m) = bgm.with_match(instance_id, |m| {
        m.players.insert(player, team);
        let columns = m.score_columns;
        m.scores.entry(player).or_insert_with(|| Score::new(team, columns));
        // Each side fights as one raid; the player's own group waits for them.
        let side = team_index(team);
        m.raids[side] = world.systems.group.join_battleground_raid(player, m.raids[side]);
        m.clone()
    }) else {
        return;
    };
    // Teammates hear about the new arrival.
    let teammates: Vec<ObjectGuid> = m
        .players
        .iter()
        .filter(|(&g, &t)| t == team && g != player)
        .map(|(&g, _)| g)
        .collect();
    world.managers.broadcast_mgr.broadcast_msg_to_players(
        &teammates,
        &SmsgBattlegroundPlayerJoinedOrLeft {
            player_guid: player,
            joined: true,
        },
    );
    if m.status == MatchStatus::WaitLeave {
        send(
            world,
            player,
            SmsgControlUpdate {
                guid: player,
                has_control: false,
            },
        );
        send(world, player, scoreboard(world, &m));
        send_status_active(world, player, &m);
    }
}

/// `CMSG_LEAVE_BATTLEFIELD`: leave unless fighting in a running match.
pub async fn request_leave(world: &World, player: ObjectGuid) {
    let Some(instance_id) = match_of(world, player) else {
        return;
    };
    let in_combat = world
        .managers
        .player_mgr
        .with_player(player, |p| p.unit_flags & crate::game::common::unit_flags::IN_COMBAT != 0)
        .unwrap_or(false);
    let running = world
        .managers
        .battleground_mgr
        .with_match(instance_id, |m| m.status != MatchStatus::WaitLeave)
        .unwrap_or(false);
    if in_combat && running {
        return;
    }
    leave_battleground(world, player, true).await;
}

/// Leave a match, as a deserter if it was still being fought.
pub async fn leave_battleground(world: &World, player: ObjectGuid, teleport: bool) {
    let Some(instance_id) = match_of(world, player) else {
        return;
    };
    let _ = world
        .systems
        .auras
        .remove_spell_auras(player, SPELL_WAITING_TO_RESURRECT, world)
        .await;
    let running = world
        .managers
        .battleground_mgr
        .with_match(instance_id, |m| m.status != MatchStatus::WaitLeave)
        .unwrap_or(false);
    let gm = world
        .managers
        .player_mgr
        .with_player(player, |p| p.gm_mode)
        .unwrap_or(false);
    if running && !gm && world.config.battleground_cast_deserter {
        let _ = world
            .systems
            .spells
            .cast_spell(player, SPELL_DESERTER, Some(player), true, world)
            .await;
    }
    remove_player_at_leave(world, instance_id, player, teleport, true).await;
}

/// Take a player out of a match: scoreboard, places, the queue slot, and
/// (with `teleport`) send them back where they came from.
pub async fn remove_player_at_leave(
    world: &World,
    instance_id: u32,
    player: ObjectGuid,
    teleport: bool,
    send_packet: bool,
) {
    let bgm = &world.managers.battleground_mgr;
    let Some((participant, team, m)) = bgm.with_match(instance_id, |m| {
        let team = m.players.remove(&player);
        m.scores.remove(&player);
        if let Some(team) = team {
            let side = &mut m.invited[team_index(team)];
            *side = side.saturating_sub(1);
            if m.status != MatchStatus::WaitLeave {
                m.accepting = true;
            }
        }
        for carrier in m.flag_carriers.iter_mut() {
            if *carrier == Some(player) {
                *carrier = None;
            }
        }
        (team.is_some(), team, m.clone())
    }) else {
        world
            .managers
            .player_mgr
            .with_player_mut(player, |p| clear_match(&mut p.battleground));
        world.systems.group.leave_battleground_raid(player);
        return;
    };
    world.systems.group.leave_battleground_raid(player);

    if !world.managers.player_mgr.is_player_alive(player) {
        let _ = world.systems.death.resurrect_in_battleground(player, world);
    }

    if participant {
        let slot = world
            .managers
            .player_mgr
            .with_player_mut(player, |p| p.battleground.remove_queue(m.bg_type))
            .flatten();
        if send_packet {
            if let Some(slot) = slot {
                send_status_none(world, player, slot, m.bg_type);
            }
        }
        if m.status != MatchStatus::WaitLeave {
            bgm.schedule(m.bg_type, m.bracket);
            if let Some(team) = team {
                let teammates: Vec<ObjectGuid> = m
                    .players
                    .iter()
                    .filter(|(_, &t)| t == team)
                    .map(|(&g, _)| g)
                    .collect();
                world.managers.broadcast_mgr.broadcast_msg_to_players(
                    &teammates,
                    &SmsgBattlegroundPlayerJoinedOrLeft {
                        player_guid: player,
                        joined: false,
                    },
                );
            }
        }
    }

    let (entry, on_match_map) = world
        .managers
        .player_mgr
        .with_player_mut(player, |p| {
            let entry = p.battleground.entry_point;
            let on_map = p.map_id == m.map_id && p.instance_id == instance_id;
            clear_match(&mut p.battleground);
            p.pvp.is_flag_carrier = false;
            (entry, on_map)
        })
        .unwrap_or((None, false));
    save_player(world, player).await;

    if teleport && on_match_map {
        let entry = entry.or_else(|| {
            world.managers.player_mgr.with_player(player, |p| EntryPoint {
                map_id: p.homebind_map,
                position: Position::new(p.homebind_x, p.homebind_y, p.homebind_z, 0.0),
            })
        });
        if let Some(entry) = entry {
            // Boxed: teleporting can itself take a player out of a match.
            let _ = Box::pin(crate::game::player::teleport::teleport_to(
                world,
                player,
                entry.map_id,
                entry.position,
                0,
            ))
            .await;
        }
    }
}

fn clear_match(state: &mut super::PlayerBattleground) {
    state.instance_id = 0;
    state.bg_type = None;
    state.team = None;
}

/// End a match. Scripts award their own win honour first; this does the
/// part every battleground shares: sounds, marks, the final scoreboard.
pub async fn end_match(world: &World, instance_id: u32, winner: Option<Team>) {
    let bgm = &world.managers.battleground_mgr;
    let Some(m) = bgm.with_match(instance_id, |m| {
        if m.status == MatchStatus::WaitLeave {
            return None;
        }
        m.status = MatchStatus::WaitLeave;
        m.winner = winner;
        m.end_timer_ms = TIME_TO_AUTOREMOVE_MS as i32;
        m.accepting = false;
        m.premature_ms = None;
        Some(m.clone())
    })
    .flatten() else {
        return;
    };
    let template = bgm.template(m.bg_type);
    let players: Vec<(ObjectGuid, Team)> = m.players.iter().map(|(&g, &t)| (g, t)).collect();
    let guids: Vec<ObjectGuid> = players.iter().map(|(g, _)| *g).collect();
    match winner {
        Some(Team::Alliance) => play_sound(world, &guids, SOUND_ALLIANCE_WINS),
        Some(Team::Horde) => play_sound(world, &guids, SOUND_HORDE_WINS),
        _ => {}
    }
    let final_score = scoreboard(world, &m);
    for (player, team) in players {
        if !world.managers.player_mgr.is_player_alive(player) {
            let _ = world.systems.death.resurrect_in_battleground(player, world);
        } else {
            crate::game::player::teleport::stop_combat(world, player, true);
        }
        if let Some(template) = &template {
            let spell = if Some(team) == winner {
                template.win_spell(team)
            } else if m.age_ms > LOSER_MARK_MIN_AGE_MS {
                template.lose_spell(team)
            } else {
                0
            };
            if spell != 0 {
                let _ = world
                    .systems
                    .spells
                    .cast_spell(player, spell, Some(player), true, world)
                    .await;
            }
        }
        send(
            world,
            player,
            SmsgControlUpdate {
                guid: player,
                has_control: false,
            },
        );
        send(
            world,
            player,
            SmsgBattlefieldResult {
                won: Some(team) == winner,
            },
        );
        send(world, player, final_score.clone());
        send_status_active(world, player, &m);
    }
    bgm.schedule(m.bg_type, m.bracket);
    tracing::info!(
        "Battleground copy {} ended, winner {:?}",
        instance_id,
        winner
    );
}

/// The scoreboard of a match.
pub fn scoreboard(world: &World, m: &Match) -> SmsgPvpLogData {
    let mut players: Vec<PvpLogPlayer> = m
        .scores
        .iter()
        .take(80)
        .map(|(&guid, score)| {
            let (race, class, gender, rank) = world
                .managers
                .player_mgr
                .with_player(guid, |p| (p.race, p.class, p.gender, p.honor.rank.rank as u32))
                .unwrap_or((0, 0, 0, 0));
            PvpLogPlayer {
                guid,
                rank,
                killing_blows: score.killing_blows,
                honorable_kills: score.honorable_kills,
                deaths: score.deaths,
                bonus_honor: score.bonus_honor,
                extra: score.extra.clone(),
                race,
                class,
                gender,
                is_alliance: score.team == Team::Alliance,
            }
        })
        .collect();
    players.sort_by_key(|p| p.guid.raw());
    SmsgPvpLogData {
        winner: m.winner_byte(),
        players,
    }
}

/// `MSG_PVP_LOG_DATA`: the scoreboard of the player's match.
pub fn send_scoreboard(world: &World, player: ObjectGuid) {
    let Some(m) = match_of(world, player).and_then(|id| world.managers.battleground_mgr.get_match(id)) else {
        return;
    };
    send(world, player, scoreboard(world, &m));
}

/// Positions for the minimap: teammates and the flag carrier.
fn positions_for(world: &World, m: &Match, viewer: Option<ObjectGuid>) -> SmsgBattlegroundPlayerPositions {
    let pos = |guid: ObjectGuid| {
        world
            .managers
            .player_mgr
            .with_player(guid, |p| (p.movement.position, p.get_team() == Team::Alliance))
    };
    let viewer_team = viewer.and_then(|v| m.team_of(v));
    let teammates = viewer_team
        .map(|team| {
            m.players
                .iter()
                .filter(|(&g, &t)| t == team && Some(g) != viewer)
                .filter_map(|(&g, _)| {
                    pos(g).map(|(p, alliance)| BattlegroundPosition {
                        guid: g,
                        x: p.x,
                        y: p.y,
                        is_alliance: alliance,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    // Vanilla shows the carrier of the enemy flag on the viewer's side; 1.14 shows every carrier.
    let carriers: Vec<ObjectGuid> = match viewer_team {
        Some(team) => {
            let enemy_flag = 1 - team_index(team);
            m.flag_carriers[enemy_flag].into_iter().collect()
        }
        None => m.flag_carriers.iter().flatten().copied().collect(),
    };
    let flag_carriers = carriers
        .into_iter()
        .filter_map(|g| {
            pos(g).map(|(p, alliance)| BattlegroundPosition {
                guid: g,
                x: p.x,
                y: p.y,
                is_alliance: alliance,
            })
        })
        .collect();
    SmsgBattlegroundPlayerPositions {
        teammates,
        flag_carriers,
    }
}

/// `MSG_BATTLEGROUND_PLAYER_POSITIONS` (vanilla asks for them).
pub fn send_positions(world: &World, player: ObjectGuid) {
    let Some(m) = match_of(world, player).and_then(|id| world.managers.battleground_mgr.get_match(id)) else {
        return;
    };
    send(world, player, positions_for(world, &m, Some(player)));
}

/// A player died in a match: kills, honourable kills and deaths on the board.
pub fn on_player_killed(world: &World, victim: ObjectGuid, killer: Option<ObjectGuid>) {
    let Some(instance_id) = match_of(world, victim) else {
        return;
    };
    let victim_pos = world.managers.player_mgr.get_player_position(victim);
    let spirit_of_redemption = has_aura(world, victim, SPELL_SPIRIT_OF_REDEMPTION);
    let positions: Vec<(ObjectGuid, Option<Position>)> = world
        .managers
        .battleground_mgr
        .get_match(instance_id)
        .map(|m| {
            m.players
                .keys()
                .map(|&g| (g, world.managers.player_mgr.get_player_position(g)))
                .collect()
        })
        .unwrap_or_default();
    world.managers.battleground_mgr.with_match(instance_id, |m| {
        if m.status != MatchStatus::InProgress {
            return;
        }
        let victim_team = m.team_of(victim);
        if let Some(killer) = killer.filter(|k| k.is_player()) {
            if let Some(killer_team) = m.team_of(killer).filter(|t| Some(*t) != victim_team) {
                if let Some(score) = m.scores.get_mut(&killer) {
                    score.honorable_kills += 1;
                    score.killing_blows += 1;
                }
                for (guid, pos) in &positions {
                    if *guid == killer || m.team_of(*guid) != Some(killer_team) {
                        continue;
                    }
                    let near = match (pos, victim_pos) {
                        (Some(a), Some(b)) => distance(*a, b) <= GROUP_REWARD_DISTANCE,
                        _ => false,
                    };
                    if near {
                        if let Some(score) = m.scores.get_mut(guid) {
                            score.honorable_kills += 1;
                        }
                    }
                }
            }
        }
        if !spirit_of_redemption {
            if let Some(score) = m.scores.get_mut(&victim) {
                score.deaths += 1;
            }
        }
    });
    if !spirit_of_redemption {
        // Enemies may now take the body's insignia.
        super::insignia::set_skinnable(world, victim, true);
    }
}

/// Add to a scoreboard column (0-based) of a player's line.
pub fn add_score(world: &World, player: ObjectGuid, column: u32, value: u32) {
    let Some(instance_id) = match_of(world, player) else {
        return;
    };
    world.managers.battleground_mgr.with_match(instance_id, |m| {
        if let Some(score) = m.scores.get_mut(&player) {
            if let Some(cell) = score.extra.get_mut(column as usize) {
                *cell += value;
            }
        }
    });
}

/// Bonus honour in a match also shows on the scoreboard.
pub fn add_bonus_honor(world: &World, player: ObjectGuid, honor: u32) {
    if let Some(instance_id) = match_of(world, player) {
        world.managers.battleground_mgr.with_match(instance_id, |m| {
            if let Some(score) = m.scores.get_mut(&player) {
                score.bonus_honor += honor;
            }
        });
    }
}

/// Record who carries a team's flag.
pub fn set_flag_carrier(world: &World, instance_id: u32, team: Team, carrier: Option<ObjectGuid>) {
    let previous = world
        .managers
        .battleground_mgr
        .with_match(instance_id, |m| {
            std::mem::replace(&mut m.flag_carriers[team_index(team)], carrier)
        })
        .flatten();
    if let Some(previous) = previous {
        world
            .managers
            .player_mgr
            .with_player_mut(previous, |p| p.pvp.is_flag_carrier = false);
    }
    if let Some(carrier) = carrier {
        world
            .managers
            .player_mgr
            .with_player_mut(carrier, |p| p.pvp.is_flag_carrier = true);
        crate::game::pvp::flag::update_pvp(world, carrier, true, false);
    }
    push_positions(world, instance_id);
}

/// Send 1.14 players the flag carriers' positions (they do not ask for them).
fn push_positions(world: &World, instance_id: u32) {
    let Some(m) = world.managers.battleground_mgr.get_match(instance_id) else {
        return;
    };
    let message = positions_for(world, &m, None);
    for &player in m.players.keys() {
        if is_modern(world, player) {
            send(world, player, message.clone());
        }
    }
}

// ==================== Spirit guides and graveyards ====================

/// `CMSG_AREA_SPIRIT_HEALER_QUERY`: time to the next resurrection wave.
pub fn spirit_healer_time(world: &World, player: ObjectGuid) -> Option<u32> {
    let instance_id = match_of(world, player)?;
    world
        .managers
        .battleground_mgr
        .with_match(instance_id, |m| m.spirit_timer_ms)
}

/// `CMSG_AREA_SPIRIT_HEALER_QUEUE`: wait at the guide for the next wave.
pub async fn queue_for_wave(world: &World, player: ObjectGuid) {
    if match_of(world, player).is_none() {
        return;
    }
    let _ = world
        .systems
        .spells
        .cast_spell(player, SPELL_WAITING_TO_RESURRECT, Some(player), true, world)
        .await;
}

/// Raise the ghosts waiting at a spirit guide.
async fn resurrection_wave(world: &World, m: &Match) {
    let guides: Vec<Position> = world
        .managers
        .creature_mgr
        .creatures_in_copy(m.map_id, m.instance_id)
        .into_iter()
        .filter_map(|g| {
            world
                .managers
                .creature_mgr
                .with_creature(g, |c| (c.npc_flags & 0x40 != 0).then_some(c.position))
                .flatten()
        })
        .collect();
    for &player in m.players.keys() {
        if world.managers.player_mgr.is_player_alive(player)
            || !has_aura(world, player, SPELL_WAITING_TO_RESURRECT)
        {
            continue;
        }
        let Some(pos) = world.managers.player_mgr.get_player_position(player) else {
            continue;
        };
        if !guides.is_empty() && !guides.iter().any(|g| distance(*g, pos) <= SPIRIT_WAVE_RANGE) {
            continue;
        }
        let _ = world
            .systems
            .auras
            .remove_spell_auras(player, SPELL_WAITING_TO_RESURRECT, world)
            .await;
        let _ = world.systems.death.resurrect_in_battleground(player, world);
    }
}

/// Where a ghost in a match goes: the battleground script decides, if it wants to.
pub fn graveyard_for(world: &World, player: ObjectGuid) -> Option<(u32, Position)> {
    let instance_id = match_of(world, player)?;
    let m = world.managers.battleground_mgr.get_match(instance_id)?;
    crate::game::pvp::objective::select_graveyard(
        world,
        Scope::Copy {
            map_id: m.map_id,
            instance_id,
        },
        player,
    )
}

// ==================== The clock ====================

/// Advance every match and queue.
pub async fn update(world: &World, diff_ms: u32) {
    let bgm = &world.managers.battleground_mgr;
    for instance_id in bgm.match_ids() {
        update_match(world, instance_id, diff_ms).await;
    }
    update_invitations(world);
    for (bg_type, bracket) in bgm.take_scheduled() {
        update_queue(world, bg_type, bracket);
    }
}

async fn update_match(world: &World, instance_id: u32, diff_ms: u32) {
    let bgm = &world.managers.battleground_mgr;
    let premature_limit = world.config.battleground_premature_finish_ms;
    // The clock moves under the lock; what it triggers runs after.
    let Some((m, steps, despawn_doors, wave, push, premature)) = bgm.with_match(instance_id, |m| {
        m.age_ms = m.age_ms.saturating_add(diff_ms);
        let steps = m.advance_start(diff_ms);
        let despawn_doors = m.status == MatchStatus::InProgress
            && !m.doors_despawned
            && m.age_ms > super::matches::DOORS_DESPAWN_AGE_MS;
        if despawn_doors {
            m.doors_despawned = true;
        }
        if m.status == MatchStatus::WaitLeave {
            m.end_timer_ms -= diff_ms as i32;
        }
        let mut wave = false;
        if m.status != MatchStatus::WaitLeave {
            if m.spirit_timer_ms <= diff_ms {
                m.spirit_timer_ms = RESURRECTION_INTERVAL_MS;
                wave = true;
            } else {
                m.spirit_timer_ms -= diff_ms;
            }
        }
        let push = m.flag_carriers.iter().any(Option::is_some)
            && (m.age_ms / position_period()) != (m.age_ms.saturating_sub(diff_ms) / position_period());
        // Short-handed: close the match unless the sides fill up.
        let mut premature = None;
        let short = m.bg_type != BattleGroundTypeId::AlteracValley
            && m.status == MatchStatus::InProgress
            && premature_limit > 0
            && (m.players_of(Team::Alliance) < m.min_per_team || m.players_of(Team::Horde) < m.min_per_team);
        if short {
            let before = m.premature_ms.unwrap_or(premature_limit);
            let after = before.saturating_sub(diff_ms);
            m.premature_ms = Some(after);
            premature = Some((before, after));
        } else {
            m.premature_ms = None;
        }
        (m.clone(), steps, despawn_doors, wave, push, premature)
    }) else {
        return;
    };
    let scope = Scope::Copy {
        map_id: m.map_id,
        instance_id,
    };

    // An empty match closes once nobody is still invited to it.
    if m.players.is_empty() {
        if m.invited == [0, 0] || m.status == MatchStatus::WaitLeave {
            close_match(world, instance_id);
        }
        return;
    }

    for step in steps {
        crate::game::pvp::objective::match_event(world, scope, step.as_str()).await;
        if step == StartStage::Begin {
            let guids: Vec<ObjectGuid> = m.players.keys().copied().collect();
            play_sound(world, &guids, SOUND_BG_START);
            return_players_to_start(world, &m);
            if world.config.battleground_queue_announcer_start {
                if let Some(template) = world.managers.battleground_mgr.template(m.bg_type) {
                    let text = format!(
                        "|cffff0000[BG Queue Announcer]:|r {} -- [{}-{}] Started!|r",
                        template.name, m.min_level, m.max_level
                    );
                    world
                        .managers
                        .broadcast_mgr
                        .broadcast_msg_to_all(&system_chat(&text, ChatMsg::System));
                }
            }
        }
    }
    if despawn_doors {
        crate::game::pvp::objective::match_event(world, scope, StartStage::DoorsDespawn.as_str()).await;
    }
    if let Some((before, after)) = premature {
        if after == 0 {
            let alliance_ok = m.players_of(Team::Alliance) >= m.min_per_team;
            let horde_ok = m.players_of(Team::Horde) >= m.min_per_team;
            let winner = match (alliance_ok, horde_ok) {
                (true, false) => Some(Team::Alliance),
                (false, true) => Some(Team::Horde),
                _ => None,
            };
            end_match(world, instance_id, winner).await;
        } else if after > 60_000 {
            if after / 60_000 != before / 60_000 {
                announce(
                    world,
                    instance_id,
                    &format!("Not enough players. This game will close in {} mins.", before / 60_000),
                    ChatMsg::System,
                    None,
                );
            }
        } else if after / 15_000 != before / 15_000 {
            announce(
                world,
                instance_id,
                &format!("Not enough players. This game will close in {} seconds.", before / 1000),
                ChatMsg::System,
                None,
            );
        }
    }
    if wave {
        resurrection_wave(world, &m).await;
    }
    if push {
        push_positions(world, instance_id);
    }
    if m.status == MatchStatus::WaitLeave && m.end_timer_ms <= 0 {
        for &player in m.players.keys() {
            remove_player_at_leave(world, instance_id, player, true, true).await;
        }
    }

    crate::game::instance::scripts::update(world, m.map_id, instance_id, diff_ms).await;
    crate::game::pvp::objective::watch_auras(world, scope).await;
}

/// How often carriers' positions are pushed to 1.14 clients.
pub(crate) fn position_period() -> u32 {
    POSITION_PUSH_MS
}

/// Players caught away from their start when the gates open go to a graveyard.
fn return_players_to_start(world: &World, m: &Match) {
    let Some(template) = world.managers.battleground_mgr.template(m.bg_type) else {
        return;
    };
    for (&player, &team) in &m.players {
        let Some((pos, gm)) = world
            .managers
            .player_mgr
            .with_player(player, |p| (p.movement.position, p.gm_mode))
        else {
            continue;
        };
        if gm {
            continue;
        }
        if distance(pos, template.start_location(team)) > START_AREA_RANGE {
            let _ = world.systems.death.repop_at_graveyard(player, world);
        }
    }
}

/// Tear a finished, empty match down.
fn close_match(world: &World, instance_id: u32) {
    let bgm = &world.managers.battleground_mgr;
    let Some((_, m)) = bgm.matches.remove(&instance_id) else {
        return;
    };
    bgm.release_client_id(m.bg_type, m.bracket, m.client_instance_id);
    // Anyone still holding an invitation to it loses the slot.
    for player in bgm.with_queue(m.bg_type, |q| q.invited_to(instance_id)) {
        leave_queue(world, player, m.bg_type, false);
        let slot = world
            .managers
            .player_mgr
            .with_player_mut(player, |p| p.battleground.remove_queue(m.bg_type))
            .flatten();
        if let Some(slot) = slot {
            send_status_none(world, player, slot, m.bg_type);
        }
    }
    world.managers.scope_states.clear(Scope::Copy {
        map_id: m.map_id,
        instance_id,
    });
    crate::game::instance::lifecycle::teardown_copy(world, m.map_id, instance_id, true);
    bgm.schedule(m.bg_type, m.bracket);
    tracing::info!("Battleground copy {} closed", instance_id);
}

// ==================== Login, logout, persistence ====================

/// Save (or clear) where a player entered their match from.
pub async fn save_player(world: &World, player: ObjectGuid) {
    let Some((instance_id, team, entry)) = world.managers.player_mgr.with_player(player, |p| {
        (p.battleground.instance_id, p.battleground.team, p.battleground.entry_point)
    }) else {
        return;
    };
    let pool = &world.databases.character;
    let guid = i64::from(player.counter());
    let result = match (instance_id, entry) {
        (id, Some(entry)) if id != 0 => sqlx::query(
            "INSERT INTO characters.character_battleground_data
                 (guid, instance_id, team, join_x, join_y, join_z, join_o, join_map)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (guid) DO UPDATE SET instance_id = EXCLUDED.instance_id,
                 team = EXCLUDED.team, join_x = EXCLUDED.join_x, join_y = EXCLUDED.join_y,
                 join_z = EXCLUDED.join_z, join_o = EXCLUDED.join_o, join_map = EXCLUDED.join_map",
        )
        .bind(guid)
        .bind(i64::from(id))
        .bind(team.map(|t| t as i16).unwrap_or(0))
        .bind(entry.position.x)
        .bind(entry.position.y)
        .bind(entry.position.z)
        .bind(entry.position.o)
        .bind(i64::from(entry.map_id))
        .execute(pool)
        .await
        .map(|_| ()),
        _ => sqlx::query("DELETE FROM characters.character_battleground_data WHERE guid = $1")
            .bind(guid)
            .execute(pool)
            .await
            .map(|_| ()),
    };
    if let Err(e) = result {
        tracing::warn!("Failed to save battleground data of {:?}: {}", player, e);
    }
}

/// For a character saved on a battleground map: where they entered it from,
/// clearing the record (the match did not survive their logout).
pub async fn take_login_entry_point(world: &World, player: ObjectGuid) -> Option<(u32, Position)> {
    let pool = &world.databases.character;
    let guid = i64::from(player.counter());
    let row = sqlx::query_as::<_, (i64, f32, f32, f32, f32)>(
        "SELECT join_map::bigint, join_x, join_y, join_z, join_o
         FROM characters.character_battleground_data WHERE guid = $1",
    )
    .bind(guid)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    let _ = sqlx::query("DELETE FROM characters.character_battleground_data WHERE guid = $1")
        .bind(guid)
        .execute(pool)
        .await;
    row.map(|(map, x, y, z, o)| (map as u32, Position::new(x, y, z, o)))
}

/// A player logged in: queues they waited in come back.
pub fn on_login(world: &World, player: ObjectGuid) {
    let bgm = &world.managers.battleground_mgr;
    let now = bgm.now_ms();
    for bg_type in BattleGroundTypeId::ALL {
        let group = bgm.with_queue(bg_type, |q| {
            q.set_online(player, true, now);
            q.group_of(player).cloned()
        });
        let Some(group) = group else {
            continue;
        };
        world.managers.player_mgr.with_player_mut(player, |p| {
            if let Some(slot) = p.battleground.add_queue(bg_type) {
                p.battleground.ticket_times[slot] = group.join_unix;
                p.battleground.invited_to[slot] = group.invited_to;
            }
        });
    }
    send_all_statuses(world, player);
}

/// A player is logging out: leave the match (as a deserter if it runs) and
/// keep their queues for a while.
pub async fn on_logout(world: &World, player: ObjectGuid) {
    if match_of(world, player).is_some() {
        leave_battleground(world, player, false).await;
        // They wake up where they entered from.
        let entry = world
            .managers
            .player_mgr
            .with_player(player, |p| p.battleground.entry_point)
            .flatten();
        if let Some(entry) = entry {
            world.managers.player_mgr.with_player_mut(player, |p| {
                p.map_id = entry.map_id;
                p.instance_id = 0;
                p.movement.position = entry.position;
            });
        }
    }
    let bgm = &world.managers.battleground_mgr;
    let now = bgm.now_ms();
    for bg_type in BattleGroundTypeId::ALL {
        bgm.with_queue(bg_type, |q| q.set_online(player, false, now));
    }
}

/// The queue slot a 1.14 ticket id names.
pub fn bg_type_of_ticket(world: &World, player: ObjectGuid, ticket_id: u32) -> Option<BattleGroundTypeId> {
    let slot = ticket_id.checked_sub(1)? as usize;
    world
        .managers
        .player_mgr
        .with_player(player, |p| p.battleground.queue_slots.get(slot).copied().flatten())
        .flatten()
}

/// Load battleground definitions.
pub async fn load(world: &World) -> anyhow::Result<()> {
    let patch = world.content_patch();
    let dbc = world.dbc.clone();
    let data = super::data::load(&world.databases.world, patch, move |id| {
        dbc.read()
            .get_world_safe_locs(id)
            .map(|loc| (loc.map_id, Position::new(loc.x, loc.y, loc.z, 0.0)))
    })
    .await?;
    tracing::info!(
        "Loaded {} battlegrounds, {} battlemasters, {} entrances",
        data.templates.len(),
        data.battlemasters.len(),
        data.entrances.len()
    );
    world.managers.battleground_mgr.set_data(data);
    let events = world.managers.bg_events.load(&world.databases.world).await?;
    tracing::info!("Loaded {} battleground event spawns", events);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::game::battleground::data::{test_template, BattlegroundData};
    use crate::game::player::player::Player;
    use oxcore_db::database::Databases;
    use sqlx::postgres::PgPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn lazy_pool() -> sqlx::PgPool {
        PgPoolOptions::new()
            .connect_lazy("postgres://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    fn test_world() -> World {
        let databases = Arc::new(Databases {
            world: lazy_pool(),
            character: lazy_pool(),
            auth: lazy_pool(),
            logs: oxcore_db::database::lazy_logs_pool(),
        });
        World::new(databases, Arc::new(Config::default()), 50, PathBuf::from("."))
    }

    /// A level 15 player standing at a battleground entrance portal.
    fn at_portal(world: &World, counter: u32, race: u8) -> ObjectGuid {
        let guid = ObjectGuid::new_player(counter);
        let mut player = Player::new(guid, format!("P{counter}"), 1, 0, 331, 15, race, 1, 0);
        player.battleground.entry_point = Some(EntryPoint {
            map_id: 1,
            position: Position::new(0.0, 0.0, 0.0, 0.0),
        });
        world.managers.player_mgr.add_player(player, counter);
        guid
    }

    fn warsong(world: &World) {
        let mut data = BattlegroundData::default();
        data.templates.insert(
            BattleGroundTypeId::WarsongGulch,
            test_template(BattleGroundTypeId::WarsongGulch, 10),
        );
        world.managers.battleground_mgr.set_data(data);
    }

    #[tokio::test]
    async fn both_sides_queued_open_a_match_and_are_invited() {
        let world = test_world();
        warsong(&world);
        let wsg = BattleGroundTypeId::WarsongGulch;
        let alliance = at_portal(&world, 1, 1);
        let horde = at_portal(&world, 2, 2);

        request_join(&world, alliance, alliance, wsg, 0, false);
        request_join(&world, horde, horde, wsg, 0, false);
        for p in [alliance, horde] {
            let slot = world
                .managers
                .player_mgr
                .with_player(p, |p| p.battleground.slot_of(wsg))
                .flatten();
            assert_eq!(slot, Some(0));
        }

        update(&world, 50).await;
        let ids = world.managers.battleground_mgr.matches_of(wsg, 0);
        assert_eq!(ids.len(), 1, "one match for the bracket");
        let m = world.managers.battleground_mgr.get_match(ids[0]).unwrap();
        assert_eq!(m.invited, [1, 1]);
        assert_eq!(m.client_instance_id, 1);
        assert_eq!((m.min_level, m.max_level), (10, 19));
        for p in [alliance, horde] {
            let invited = world
                .managers
                .player_mgr
                .with_player(p, |p| p.battleground.invited_to[0])
                .unwrap();
            assert_eq!(invited, m.instance_id);
        }

        // Declining frees the place and the slot.
        port(&world, alliance, wsg, false).await;
        let m = world.managers.battleground_mgr.get_match(ids[0]).unwrap();
        assert_eq!(m.invited, [0, 1]);
        assert_eq!(
            world
                .managers
                .player_mgr
                .with_player(alliance, |p| p.battleground.slot_of(wsg))
                .flatten(),
            None
        );
    }

    #[tokio::test]
    async fn one_side_alone_waits() {
        let world = test_world();
        warsong(&world);
        let wsg = BattleGroundTypeId::WarsongGulch;
        let alliance = at_portal(&world, 1, 1);
        request_join(&world, alliance, alliance, wsg, 0, false);
        update(&world, 50).await;
        assert!(world.managers.battleground_mgr.matches_of(wsg, 0).is_empty());

        // Testing mode starts a match with anyone queued.
        world.managers.battleground_mgr.toggle_testing();
        world.managers.battleground_mgr.schedule(wsg, 0);
        update(&world, 50).await;
        assert_eq!(world.managers.battleground_mgr.matches_of(wsg, 0).len(), 1);
    }

    #[tokio::test]
    async fn a_portal_queue_needs_the_player_at_the_portal() {
        let world = test_world();
        warsong(&world);
        let wsg = BattleGroundTypeId::WarsongGulch;
        let alliance = at_portal(&world, 1, 1);
        world.managers.player_mgr.with_player_mut(alliance, |p| {
            p.movement.position = Position::new(500.0, 0.0, 0.0, 0.0);
        });
        request_join(&world, alliance, alliance, wsg, 0, false);
        assert_eq!(
            world
                .managers
                .player_mgr
                .with_player(alliance, |p| p.battleground.slot_of(wsg))
                .flatten(),
            None
        );
    }

    #[tokio::test]
    async fn the_scoreboard_lists_every_line_with_its_columns() {
        let world = test_world();
        let mut m = Match::new(9, BattleGroundTypeId::WarsongGulch, 0, 1, (10, 19), 1, 10, 2);
        let g = ObjectGuid::new_player(7);
        m.scores.insert(g, Score::new(Team::Horde, 2));
        m.scores.get_mut(&g).unwrap().extra[0] = 3;
        let board = scoreboard(&world, &m);
        assert_eq!(board.players.len(), 1);
        assert_eq!(board.players[0].extra, vec![3, 0]);
        assert!(!board.players[0].is_alliance);
        assert_eq!(board.winner, None);
    }
}
