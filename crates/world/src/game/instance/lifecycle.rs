//! World-side dungeon lifecycle: choosing a copy for every transfer, reacting
//! to kills inside copies, running the reset schedule, removing players who are
//! no longer saved to the copy they are in, and tearing copies down.

use super::manager::{
    InstanceAdmission, InstanceAdmissionDenied, InstanceEntryError, InstanceEntryGroup,
    ResetOutcome,
};
use crate::dbc::structures::MapEntry;
use crate::core::session::WorldSession;
use crate::World;
use oxcore_shared::game::chat::{ChatMsg, ChatTag, Language};
use oxcore_shared::messages::chat::SmsgMessageChat;
use oxcore_shared::messages::instance::{
    RaidInstanceMessageType, SmsgInstanceReset, SmsgInstanceResetFailed, SmsgInstanceSaveCreated,
    SmsgRaidInstanceMessage,
};
use oxcore_shared::messages::movement::{SmsgTransferAborted, TransferAbortReason};
use oxcore_shared::protocol::{ObjectGuid, Position};
use std::time::{Duration, Instant};

/// How long a player may stay in a copy they are not saved to.
const UNSAVED_PLAYER_GRACE: Duration = Duration::from_secs(60);

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// The map table entry for a map, if known.
pub fn map_entry(world: &World, map_id: u32) -> Option<MapEntry> {
    world.dbc.read().get_map(map_id).cloned()
}

/// Whether a map is split into bound copies (dungeon or raid).
pub fn is_dungeon_map(world: &World, map_id: u32) -> bool {
    map_entry(world, map_id).is_some_and(|entry| entry.is_dungeon())
}

/// No game-master mode is modelled yet, so nobody bypasses the entry rules.
fn is_game_master(_world: &World, _player_guid: ObjectGuid) -> bool {
    false
}

fn group_context(
    world: &World,
    player_guid: ObjectGuid,
    dest_map: u32,
) -> Option<InstanceEntryGroup> {
    world
        .systems
        .group
        .get_player_group(player_guid)
        .map(|group| InstanceEntryGroup {
            leader_guid: group.leader_guid,
            is_raid: group.is_raid,
            members_outside: group
                .members
                .iter()
                .filter_map(|member| {
                    if member.guid == player_guid {
                        return None;
                    }
                    world
                        .managers
                        .player_mgr
                        .with_player(member.guid, |player| {
                            (player.map_id != dest_map).then_some(member.guid)
                        })
                        .flatten()
                })
                .collect(),
        })
}

/// Choose, admit and bind the copy a player is sent to when transferring to
/// `dest_map`. Continents (and unknown maps) use instance 0; a player already
/// on the destination map stays in their own copy.
pub async fn resolve_transfer_instance(
    world: &World,
    player_guid: ObjectGuid,
    dest_map: u32,
) -> Result<u32, InstanceEntryError> {
    let Some(entry) = map_entry(world, dest_map).filter(MapEntry::is_dungeon) else {
        return Ok(0);
    };

    let (current_map, current_instance, account_id) = world
        .managers
        .player_mgr
        .with_player(player_guid, |player| {
            (player.map_id, player.instance_id, player.account_id)
        })
        .unwrap_or((u32::MAX, 0, 0));
    if current_map == dest_map && current_instance != 0 {
        return Ok(current_instance);
    }

    let group = group_context(world, player_guid, dest_map);
    let players_inside = |map_id: u32, instance_id: u32| -> usize {
        world
            .managers
            .map_mgr
            .get_map(map_id, instance_id)
            .map(|map| map.player_count())
            .unwrap_or(0)
    };
    let admission = InstanceAdmission {
        is_raid: entry.is_raid(),
        is_raid_group: group.as_ref().is_some_and(|group| group.is_raid),
        max_players: entry.max_players,
        players_inside: Some(&players_inside),
        account_id: (account_id != 0).then_some(account_id),
        is_game_master: is_game_master(world, player_guid),
    };

    world
        .managers
        .instance_mgr
        .enter_instance(
            &world.databases,
            dest_map,
            player_guid,
            group.as_ref(),
            entry.is_raid(),
            &admission,
        )
        .await
}

/// The transfer-abort reason for a failed copy selection.
pub fn abort_reason(error: &InstanceEntryError) -> TransferAbortReason {
    match error {
        InstanceEntryError::Denied(denied) => (*denied).into(),
        InstanceEntryError::Internal(_) => TransferAbortReason::NotFound,
    }
}

/// Start a far teleport, choosing the destination copy first. On refusal the
/// client gets a transfer-abort and the player stays where they are.
/// Returns whether the teleport was started.
pub async fn start_far_teleport(
    world: &World,
    session: &WorldSession,
    player_guid: ObjectGuid,
    dest_map: u32,
    dest_pos: Position,
) -> anyhow::Result<bool> {
    let instance_id = match resolve_transfer_instance(world, player_guid, dest_map).await {
        Ok(instance_id) => instance_id,
        Err(error) => {
            if let InstanceEntryError::Internal(e) = &error {
                tracing::error!("Failed to choose a copy of map {}: {}", dest_map, e);
            }
            session.send_msg(SmsgTransferAborted {
                map_id: dest_map,
                reason: abort_reason(&error),
            })?;
            return Ok(false);
        }
    };

    let old_position = world
        .managers
        .player_mgr
        .with_player(player_guid, |player| player.movement.position)
        .unwrap_or(dest_pos);
    session.send_far_teleport(old_position, dest_map, dest_pos)?;
    session.set_pending_teleport(Some((dest_map, instance_id, dest_pos)));
    Ok(true)
}

/// Send a player to their bind point (used when they must leave a copy).
pub fn send_home(world: &World, player_guid: ObjectGuid) -> anyhow::Result<()> {
    let Some(session) = world.session_mgr.get_session_by_player(player_guid) else {
        return Ok(());
    };
    if session.get_pending_teleport().is_some() {
        return Ok(());
    }
    let Some((map_id, position, old_position)) =
        world
            .managers
            .player_mgr
            .with_player(player_guid, |player| {
                (
                    player.homebind_map,
                    Position::new(
                        player.homebind_x,
                        player.homebind_y,
                        player.homebind_z,
                        0.0,
                    ),
                    player.movement.position,
                )
            })
    else {
        return Ok(());
    };
    session.send_far_teleport(old_position, map_id, position)?;
    session.set_pending_teleport(Some((map_id, 0, position)));
    Ok(())
}

fn send_system_message(world: &World, player_guid: ObjectGuid, message: &str) {
    let Some(session) = world.session_mgr.get_session_by_player(player_guid) else {
        return;
    };
    let chat = SmsgMessageChat {
        msgtype: ChatMsg::System,
        language: Language::Universal,
        sender_guid: ObjectGuid::empty(),
        sender_name: None,
        target_guid: None,
        channel_name: None,
        player_rank: None,
        message,
        chat_tag: ChatTag::None,
    };
    let _ = session.send_msg(chat);
}

/// A database creature died: persist its respawn deadline, keep a normal
/// dungeon alive past the respawn, and permanently save the raid when the
/// creature is raid-locking.
pub async fn on_creature_killed(
    world: &World,
    creature: ObjectGuid,
    map_id: u32,
    instance_id: u32,
    spawn_id: u32,
    entry: u32,
    killer: Option<ObjectGuid>,
    respawn_at_ms: u64,
) {
    let map = map_entry(world, map_id);
    let is_battleground = map.as_ref().is_some_and(MapEntry::is_battleground);
    if spawn_id != 0 && respawn_at_ms > now_unix_ms() && !is_battleground {
        crate::game::common::respawn_persistence::save_creature_respawn(
            world,
            spawn_id,
            map_id,
            instance_id,
            respawn_at_ms,
        );
    }

    let Some(map) = map.filter(MapEntry::is_dungeon) else {
        return;
    };
    if instance_id == 0 {
        return;
    }
    super::scripts::on_creature_death(world, map_id, instance_id, entry, creature).await;
    let instance_mgr = &world.managers.instance_mgr;

    if map.is_non_raid_dungeon() {
        if let Err(e) = instance_mgr
            .extend_dungeon_reset(&world.databases, map_id, instance_id, respawn_at_ms / 1000)
            .await
        {
            tracing::warn!(
                "Failed to extend reset of {}:{}: {}",
                map_id,
                instance_id,
                e
            );
        }
        return;
    }

    if !world.managers.creature_mgr.locks_raid_on_death(entry) {
        return;
    }
    let players_inside = world
        .managers
        .map_mgr
        .get_map(map_id, instance_id)
        .map(|copy| copy.player_guids())
        .unwrap_or_default();
    let leader = killer
        .and_then(|killer| world.systems.group.get_player_group(killer))
        .map(|group| group.leader_guid);
    match instance_mgr
        .perm_bind_raid(
            &world.databases,
            map_id,
            instance_id,
            &players_inside,
            leader,
        )
        .await
    {
        Ok(newly_saved) => {
            for player in newly_saved {
                world
                    .managers
                    .broadcast_mgr
                    .send_msg_to_player(player, SmsgInstanceSaveCreated);
            }
        }
        Err(e) => tracing::warn!(
            "Failed to save raid {}:{} to its players: {}",
            map_id,
            instance_id,
            e
        ),
    }
}

/// Tear a copy down: despawn everything in it, release its collision, script
/// state and map. With `discard` the copy's save is gone, so its spawn slots,
/// respawn deadlines and pool rosters are forgotten too; otherwise they are
/// kept for the next load. Returns false while players are still inside.
pub fn teardown_copy(world: &World, map_id: u32, instance_id: u32, discard: bool) -> bool {
    if let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) {
        if map.player_count() > 0 {
            return false;
        }
    }

    let creature_mgr = &world.managers.creature_mgr;
    for guid in creature_mgr.creatures_in_copy(map_id, instance_id) {
        creature_mgr.save_respawn_state(guid);
        world.systems.pool.on_object_despawned(guid);
        creature_mgr.remove_creature(guid);
    }

    let gameobject_mgr = &world.managers.gameobject_mgr;
    for guid in gameobject_mgr.gameobjects_in_copy(map_id, instance_id) {
        world.systems.pool.on_object_despawned(guid);
        gameobject_mgr.remove_collision_model(guid, &world.managers.vmap_mgr);
        gameobject_mgr.remove_gameobject(guid);
    }

    if discard {
        creature_mgr.clear_instance_spawn_states(map_id, instance_id);
        gameobject_mgr.clear_instance_spawn_states(map_id, instance_id);
        if instance_id != 0 {
            world.managers.pool_mgr.forget_instance(instance_id);
        }
    }

    world.managers.vmap_mgr.unload_instance(map_id, instance_id);
    world
        .managers
        .lua_mgr
        .clear_instance_state(map_id, instance_id);
    world.managers.map_mgr.remove_map(map_id, instance_id);
    world
        .managers
        .instance_mgr
        .finish_teardown(map_id, instance_id);
    tracing::debug!(
        "Tore down copy {}:{} (save {})",
        map_id,
        instance_id,
        if discard { "reset" } else { "kept" }
    );
    true
}

/// Send everyone inside a copy home; used when its save is reset under them.
fn evacuate_copy(world: &World, map_id: u32, instance_id: u32) {
    if let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) {
        for player in map.player_guids() {
            if let Err(e) = send_home(world, player) {
                tracing::warn!("Failed to send {:?} home: {}", player, e);
            }
        }
    }
}

/// Copies of a map that are currently loaded.
fn loaded_copies(world: &World, map_id: u32) -> Vec<u32> {
    world
        .managers
        .map_mgr
        .get_active_map_keys()
        .into_iter()
        .filter(|&(map, _)| map == map_id)
        .map(|(_, instance)| instance)
        .collect()
}

/// Tear down (or evacuate) the copies of a player or group reset request.
pub fn apply_reset_outcome(world: &World, outcome: &ResetOutcome) {
    for &(map_id, instance_id) in &outcome.reset_copies {
        if !teardown_copy(world, map_id, instance_id, true) {
            evacuate_copy(world, map_id, instance_id);
        }
    }
}

/// Report a "reset all instances" outcome to the requester.
pub fn send_reset_replies(world: &World, requester: ObjectGuid, outcome: &ResetOutcome) {
    for &map_id in &outcome.reset_maps {
        world
            .managers
            .broadcast_mgr
            .send_msg_to_player(requester, SmsgInstanceReset { map_id });
    }
    for &(map_id, reason) in &outcome.failed {
        world
            .managers
            .broadcast_mgr
            .send_msg_to_player(requester, SmsgInstanceResetFailed { reason, map_id });
    }
}

/// Lifecycle tick, run about once a second from the world update.
pub async fn update(world: &World) -> anyhow::Result<()> {
    let instance_mgr = &world.managers.instance_mgr;
    let events = instance_mgr.update(&world.databases).await?;

    for (map_id, time_left) in events.warnings {
        let message = SmsgRaidInstanceMessage {
            kind: RaidInstanceMessageType::for_time_left(time_left),
            map_id,
            time_left: time_left as u32,
        };
        for instance_id in loaded_copies(world, map_id) {
            if let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) {
                for player in map.player_guids() {
                    world
                        .managers
                        .broadcast_mgr
                        .send_msg_to_player(player, message.clone());
                }
            }
        }
    }

    for map_id in events.reset_maps {
        for instance_id in loaded_copies(world, map_id) {
            instance_mgr.mark_resetting(map_id, instance_id);
            evacuate_copy(world, map_id, instance_id);
        }
    }
    for (map_id, instance_id) in events.reset_copies {
        evacuate_copy(world, map_id, instance_id);
    }

    let unload_delay = Duration::from_secs(world.config.instance_unload_delay_secs);
    let now = Instant::now();
    for (map_id, instance_id) in world.managers.map_mgr.get_active_map_keys() {
        if !is_dungeon_map(world, map_id) {
            continue;
        }
        let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) else {
            continue;
        };
        let players = map.player_guids();
        let empty_for =
            instance_mgr.observe_copy_population(map_id, instance_id, players.len(), now);
        let resetting = instance_mgr.is_reset_pending(map_id, instance_id);
        // A dungeon copy without a save (instance 0, or one whose save is gone)
        // has no business staying loaded.
        let has_save = instance_id != 0 && instance_mgr.has_instance(map_id, instance_id);

        if let Some(empty_for) = empty_for {
            if resetting || !has_save {
                teardown_copy(world, map_id, instance_id, true);
            } else if empty_for >= unload_delay {
                teardown_copy(world, map_id, instance_id, false);
            }
            continue;
        }

        super::scripts::update(world, map_id, instance_id, 1000).await;

        for player in players {
            if resetting {
                let _ = send_home(world, player);
                continue;
            }
            if is_game_master(world, player) {
                continue;
            }
            let leader = world
                .systems
                .group
                .get_player_group(player)
                .map(|group| group.leader_guid);
            let saved = instance_mgr.is_saved_to_copy(player, leader, map_id, instance_id);
            match instance_mgr.track_unsaved_player(player, saved, now, UNSAVED_PLAYER_GRACE) {
                Some(true) => send_system_message(
                    world,
                    player,
                    "You are not saved to this instance. You will be returned to your bind location in 60 seconds.",
                ),
                Some(false) => {
                    let _ = send_home(world, player);
                }
                None => {}
            }
        }
    }

    Ok(())
}

/// Where a player logging in inside a dungeon actually ends up: their saved
/// copy if it is still valid and they may enter it; otherwise the dungeon's
/// entrance (or their bind point when the dungeon has none).
pub async fn validate_login_location(
    world: &World,
    player_guid: ObjectGuid,
    account_id: u32,
    map_id: u32,
    instance_id: u32,
    position: Position,
    homebind: (u32, Position),
) -> (u32, u32, Position) {
    let Some(entry) = map_entry(world, map_id) else {
        return (map_id, instance_id, position);
    };
    if !entry.is_dungeon() {
        return (map_id, 0, position);
    }

    let leader = world
        .systems
        .group
        .get_player_group(player_guid)
        .map(|group| group.leader_guid);
    let instance_mgr = &world.managers.instance_mgr;
    let bound = instance_mgr
        .bound_copy_for(&world.databases, player_guid, leader, map_id)
        .await;
    if let Ok(Some(_)) = bound {
        let group = group_context(world, player_guid, map_id);
        let players_inside = |map: u32, instance: u32| -> usize {
            world
                .managers
                .map_mgr
                .get_map(map, instance)
                .map(|copy| copy.player_count())
                .unwrap_or(0)
        };
        let admission = InstanceAdmission {
            is_raid: entry.is_raid(),
            is_raid_group: group.as_ref().is_some_and(|group| group.is_raid),
            max_players: entry.max_players,
            players_inside: Some(&players_inside),
            account_id: (account_id != 0).then_some(account_id),
            is_game_master: is_game_master(world, player_guid),
        };
        match instance_mgr
            .enter_instance(
                &world.databases,
                map_id,
                player_guid,
                group.as_ref(),
                entry.is_raid(),
                &admission,
            )
            .await
        {
            Ok(copy) => {
                super::scripts::on_player_enter(world, map_id, copy, player_guid).await;
                return (map_id, copy, position);
            }
            Err(InstanceEntryError::Denied(reason)) => tracing::info!(
                "Login of {:?} into map {} refused ({:?}); moving to the entrance",
                player_guid,
                map_id,
                reason
            ),
            Err(InstanceEntryError::Internal(e)) => tracing::warn!(
                "Login of {:?} into map {} failed: {}; moving to the entrance",
                player_guid,
                map_id,
                e
            ),
        }
    } else if let Err(e) = bound {
        tracing::warn!(
            "Could not read saves of {:?} for map {}: {}",
            player_guid,
            map_id,
            e
        );
    }

    match dungeon_exit(world, &entry) {
        Some((exit_map, exit_pos)) => (exit_map, 0, exit_pos),
        None => (homebind.0, 0, homebind.1),
    }
}

/// The teleport that takes a player out of a dungeon to its outside entrance:
/// a trigger inside the dungeon whose destination is the map holding the
/// entrance.
pub fn dungeon_exit(world: &World, entry: &MapEntry) -> Option<(u32, Position)> {
    let outside = u32::try_from(entry.ghost_entrance_map).ok()?;
    let triggers = world.managers.area_trigger_mgr.teleports();
    let dbc = world.dbc.read();
    triggers
        .into_iter()
        .filter(|(_, teleport)| teleport.destination_map == outside)
        .find(|(id, _)| {
            world
                .managers
                .area_trigger_mgr
                .get_template(*id)
                .map(|template| template.map_id)
                .or_else(|| dbc.get_area_trigger(*id).map(|trigger| trigger.map_id))
                == Some(entry.id)
        })
        .map(|(_, teleport)| (teleport.destination_map, teleport.destination))
}

/// A player arrived on a new map: count the entry, cancel any removal timer,
/// send the raid welcome, and release a solo save the group's save replaces.
pub async fn on_player_changed_map(
    world: &World,
    player_guid: ObjectGuid,
    old_map: u32,
    old_instance: u32,
    new_map: u32,
    new_instance: u32,
) {
    let instance_mgr = &world.managers.instance_mgr;
    instance_mgr.clear_unsaved_player(player_guid);
    let new_entry = map_entry(world, new_map);

    if (old_map, old_instance) != (new_map, new_instance) {
        super::scripts::on_player_leave(world, old_map, old_instance, player_guid).await;
        super::scripts::on_player_enter(world, new_map, new_instance, player_guid).await;
    }

    if new_instance != 0 && new_entry.as_ref().is_some_and(MapEntry::is_dungeon) {
        let account_id = world
            .managers
            .player_mgr
            .with_player(player_guid, |player| player.account_id)
            .unwrap_or(0);
        if account_id != 0 {
            instance_mgr.record_account_instance_enter(account_id, new_instance);
        } else {
            instance_mgr.record_instance_enter(player_guid, new_instance);
        }
        if new_entry.as_ref().is_some_and(MapEntry::is_raid) {
            if let Some(reset) = instance_mgr.raid_reset_time(new_map) {
                let left = reset.saturating_sub(now_unix_ms() / 1000);
                world.managers.broadcast_mgr.send_msg_to_player(
                    player_guid,
                    SmsgRaidInstanceMessage {
                        kind: RaidInstanceMessageType::for_time_left(left),
                        map_id: new_map,
                        time_left: left as u32,
                    },
                );
            }
        }
        return;
    }

    // Leaving a dungeon for the outside world: a solo save kept because the
    // player was inside it when they joined their group gives way now.
    if old_map != new_map && is_dungeon_map(world, old_map) {
        let Some(leader) = world
            .systems
            .group
            .get_player_group(player_guid)
            .map(|group| group.leader_guid)
        else {
            return;
        };
        let players_in = |map: u32, instance: u32| {
            world
                .managers
                .map_mgr
                .get_map(map, instance)
                .map(|copy| copy.player_guids())
                .unwrap_or_default()
        };
        match instance_mgr
            .release_solo_save_on_leave(&world.databases, player_guid, leader, old_map, players_in)
            .await
        {
            Ok(Some((map_id, instance_id))) => {
                teardown_copy(world, map_id, instance_id, true);
            }
            Ok(None) => {}
            Err(e) => tracing::warn!(
                "Failed to release solo save of {:?} for map {}: {}",
                player_guid,
                old_map,
                e
            ),
        }
    }
}

impl From<InstanceAdmissionDenied> for TransferAbortReason {
    fn from(denied: InstanceAdmissionDenied) -> Self {
        match denied {
            // The dedicated raid-group-only notice has no verified modern opcode,
            // so a not-found abort is the closest response both protocols carry.
            InstanceAdmissionDenied::RaidGroupRequired => TransferAbortReason::NotFound,
            InstanceAdmissionDenied::TooManyInstances => TransferAbortReason::TooManyInstances,
            InstanceAdmissionDenied::MaxPlayers => TransferAbortReason::MaxPlayers,
            InstanceAdmissionDenied::ResetInProgress => TransferAbortReason::NotFound,
        }
    }
}
