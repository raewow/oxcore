//! Moving a player to another place: the cleanup every teleport does first,
//! then either a near teleport (same map, same copy, no loading screen) or a
//! far one (map change, through the transfer handshake).
//!
//! Every teleport source (area triggers, spells, GM commands, scripts) should
//! come through [`teleport_to`] so they all leave combat, loot and pets
//! behind the same way.

use anyhow::Result;
use oxcore_shared::messages::combat::SmsgAttackStop;
use oxcore_shared::messages::movement::SmsgMoveTeleport;
use oxcore_shared::protocol::{ObjectGuid, Position};

use crate::core::common::{is_valid_map_coord, MoveFlags, MovementInfo};
use crate::core::session::WorldSession;
use crate::game::instance::lifecycle;
use crate::game::instance::manager::InstanceEntryError;
use crate::game::player::auras::interrupt::AuraInterruptFlags;
use crate::World;

/// Teleport option bits.
pub mod options {
    /// Stay in combat through a same-map teleport.
    pub const NOT_LEAVE_COMBAT: u32 = 0x04;
    /// Keep the pet out through a same-map teleport.
    pub const NOT_UNSUMMON_PET: u32 = 0x08;
    /// The player is teleporting themselves with a spell: leave their casting alone.
    pub const SPELL: u32 = 0x10;
    /// Use the far handshake even on the same map (loading screen).
    pub const FORCE_MAP_CHANGE: u32 = 0x20;
}

/// Movement flags that must not survive a teleport: moving, turning,
/// pitching, jumping and falling, and riding a transport.
const CLEARED_MOVEMENT_FLAGS: u32 = 0x0000_00FF | 0x0000_2000 | 0x0000_4000 | 0x0200_0000;

/// Channel and aura interrupts for moving or turning.
const MOVING_OR_TURNING: u32 = AuraInterruptFlags::MOVING.0 | AuraInterruptFlags::TURNING.0;

/// A pet further than this from a same-map destination is put away for the
/// trip and summoned again on landing.
const PET_TRAVEL_DISTANCE: f32 = 100.0;

/// Teleport a player. Returns whether the teleport started; a refused
/// dungeon entry tells the client why and leaves the player where they are.
pub async fn teleport_to(
    world: &World,
    player_guid: ObjectGuid,
    map_id: u32,
    dest: Position,
    options: u32,
) -> Result<bool> {
    let Some(session) = world.session_mgr.get_session_by_player(player_guid) else {
        return Ok(false);
    };
    if !is_valid_map_coord(dest.x, dest.y, dest.z, dest.o) {
        tracing::warn!(
            "Refusing teleport of {:?} to invalid location map {} ({}, {}, {})",
            player_guid,
            map_id,
            dest.x,
            dest.y,
            dest.z
        );
        return Ok(false);
    }
    let Some(entry) = lifecycle::map_entry(world, map_id) else {
        return Ok(false);
    };
    let Some((current_map, looting)) = world
        .managers
        .player_mgr
        .with_player(player_guid, |player| (player.map_id, player.looting_target))
    else {
        return Ok(false);
    };
    // Battlegrounds are only entered through their queue, which assigns the
    // player to a match; nothing else may send a player there. Moves inside
    // the battleground they are in are fine.
    if entry.is_battleground()
        && current_map != map_id
        && crate::game::battleground::system::transfer_instance(world, player_guid, map_id).is_none()
    {
        return Ok(false);
    }
    // Leaving a battleground's map for anywhere else leaves the match.
    if current_map != map_id {
        if let Some(instance_id) = crate::game::battleground::system::match_of(world, player_guid) {
            let leaving = world
                .managers
                .battleground_mgr
                .with_match(instance_id, |m| m.map_id == current_map && m.map_id != map_id)
                .unwrap_or(false);
            if leaving {
                crate::game::battleground::system::leave_battleground(world, player_guid, false).await;
            }
        }
    }

    let near = current_map == map_id && options & options::FORCE_MAP_CHANGE == 0;

    // A teleport ends any flight (summons, GM moves, scripts).
    crate::game::taxi::flight::interrupt(world, player_guid);

    // A far teleport checks the destination first, so a refusal leaves the
    // player's combat, pet and casting untouched.
    let instance_id = if near {
        None
    } else {
        match lifecycle::resolve_transfer_instance(world, player_guid, map_id).await {
            Ok(instance_id) => Some(instance_id),
            Err(error) => {
                match &error {
                    InstanceEntryError::Denied(denied) => tracing::debug!(
                        "Player {:?} denied entry to map {}: {:?}",
                        player_guid,
                        map_id,
                        denied
                    ),
                    InstanceEntryError::Internal(e) => {
                        tracing::error!("Failed to choose a copy of map {}: {}", map_id, e)
                    }
                }
                lifecycle::send_entry_refusal(&session, map_id, &error)?;
                return Ok(false);
            }
        }
    };

    if let Some(target) = looting {
        world
            .systems
            .loot
            .handle_loot_release(player_guid, target, world)
            .await?;
    }
    world
        .systems
        .spells
        .interrupt_spells_with_channel_flags(player_guid, MOVING_OR_TURNING, 0, world)
        .await?;

    match instance_id {
        None => {
            if options & options::NOT_UNSUMMON_PET == 0 {
                put_pet_away_if_far(world, player_guid, dest);
            }
            if options & options::NOT_LEAVE_COMBAT == 0 {
                stop_combat(world, player_guid, false);
            }
            send_near_teleport(world, &session, player_guid, dest)?;
        }
        Some(instance_id) => {
            leave_world(world, player_guid, options).await?;
            let old_position = world
                .managers
                .player_mgr
                .with_player(player_guid, |player| player.movement.position)
                .unwrap_or(dest);
            session.send_far_teleport(old_position, map_id, dest)?;
            session.set_pending_teleport(Some((map_id, instance_id, dest)));
        }
    }
    Ok(true)
}

/// Send the same-map teleport to the player's client and wait for its ack.
/// Sends nothing else: callers that need combat or pet handling go through
/// [`teleport_to`].
pub fn send_near_teleport(
    world: &World,
    session: &WorldSession,
    player_guid: ObjectGuid,
    dest: Position,
) -> Result<()> {
    let Some((counter, info)) =
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                let counter = player.movement.next_movement_counter();
                player.movement.flags &= !CLEARED_MOVEMENT_FLAGS;
                player.movement.movement_flags &= !CLEARED_MOVEMENT_FLAGS;
                let mut info = MovementInfo::new();
                info.mover_guid = player_guid;
                info.flags = MoveFlags::from(player.movement.flags);
                info.time = player.movement.timestamp;
                info.position = dest;
                (counter, info)
            })
    else {
        return Ok(());
    };
    session.send_msg(SmsgMoveTeleport {
        mover: player_guid,
        counter,
        info: info.into(),
    })?;
    session.set_pending_near_teleport(Some(dest));
    Ok(())
}

/// Bring back a pet put away for a teleport, now that the player has landed.
pub fn resummon_pet(world: &World, player_guid: ObjectGuid) {
    let Some(entry) = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| player.teleport_unsummoned_pet.take())
        .flatten()
    else {
        return;
    };
    if let Err(e) = world.systems.pet.summon(player_guid, entry, world) {
        tracing::warn!("Could not bring back pet {} for {:?}: {}", entry, player_guid, e);
    }
}

/// Everything a player drops when leaving the map: target, combat and every
/// creature's grudge, the pet, area spells, casting, and auras that end on
/// leaving the world or moving.
async fn leave_world(world: &World, player_guid: ObjectGuid, options: u32) -> Result<()> {
    world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            player.selection = None;
            player.spells.remove_all_dyn_objects();
        });
    stop_combat(world, player_guid, true);
    put_pet_away(world, player_guid);
    if options & options::SPELL == 0 {
        world
            .systems
            .spells
            .interrupt_non_melee_spells(player_guid, true, None, world)
            .await?;
    }
    world
        .systems
        .auras
        .remove_auras_with_interrupt_flag(
            player_guid,
            AuraInterruptFlags::LEAVE_WORLD_CANCELS.0 | MOVING_OR_TURNING,
            world,
        )
        .await?;
    Ok(())
}

/// Stop the player's auto-attack and drop them from combat. Creatures
/// attacking them stop too; with `forget`, the creatures also strike the
/// player from their threat lists (the player is leaving the map).
pub(crate) fn stop_combat(world: &World, player_guid: ObjectGuid, forget: bool) {
    if let Some(target) = world
        .systems
        .combat
        .get_attack_target(player_guid, &world.managers.player_mgr)
    {
        world.managers.broadcast_mgr.broadcast_msg_nearby(
            player_guid,
            &SmsgAttackStop {
                attacker_guid: player_guid,
                target_guid: target,
                unk: 0,
            },
            true,
        );
    }
    world
        .systems
        .combat
        .leave_combat(player_guid, &world.managers.player_mgr);

    let creatures: Vec<ObjectGuid> = world
        .managers
        .creature_mgr
        .iter_creatures()
        .filter(|creature| {
            creature.combat.attackers.contains(&player_guid)
                || (forget && creature.threat_manager.has_target(player_guid))
        })
        .map(|creature| *creature.key())
        .collect();
    for creature_guid in creatures {
        world
            .managers
            .creature_mgr
            .with_creature_mut(creature_guid, |creature| {
                creature.combat.remove_attacker(player_guid);
                if forget {
                    creature.threat_manager.remove_target(player_guid);
                }
            });
    }
}

/// Put the pet away for the trip, remembering it for [`resummon_pet`].
pub(crate) fn put_pet_away(world: &World, player_guid: ObjectGuid) {
    let Some(pet_guid) = world
        .managers
        .player_mgr
        .with_player(player_guid, |player| player.active_pet)
        .flatten()
    else {
        return;
    };
    let entry = world
        .managers
        .creature_mgr
        .with_creature(pet_guid, |pet| pet.entry);
    world.systems.pet.dismiss(player_guid, world);
    if let Some(entry) = entry {
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.teleport_unsummoned_pet = Some(entry)
            });
    }
}

/// On a same-map teleport the pet only goes away when the destination is
/// out of its reach.
fn put_pet_away_if_far(world: &World, player_guid: ObjectGuid, dest: Position) {
    let Some(pet_guid) = world
        .managers
        .player_mgr
        .with_player(player_guid, |player| player.active_pet)
        .flatten()
    else {
        return;
    };
    let far = world
        .managers
        .creature_mgr
        .with_creature(pet_guid, |pet| {
            let (dx, dy, dz) = (
                pet.position.x - dest.x,
                pet.position.y - dest.y,
                pet.position.z - dest.z,
            );
            (dx * dx + dy * dy + dz * dz).sqrt() > PET_TRAVEL_DISTANCE
        })
        .unwrap_or(false);
    if far {
        put_pet_away(world, player_guid);
    }
}

/// Make the player's current spot their home, where the hearthstone takes
/// them: saved, then confirmed to the client. `binder` is the innkeeper (or
/// whoever cast the bind).
pub async fn bind_home(world: &World, player_guid: ObjectGuid, binder: ObjectGuid) -> Result<()> {
    use oxcore_db::database::characters::PgCharacterRepository;
    use oxcore_shared::messages::login::{SmsgBindPointUpdate, SmsgPlayerBound};

    let Some((map_id, zone_id, position)) =
        world.managers.player_mgr.with_player(player_guid, |player| {
            (player.map_id, player.zone_id, player.movement.position)
        })
    else {
        return Ok(());
    };
    // The home is named after the area (sub-zone) the player stands in.
    let area_id = area_at(world, map_id, position).unwrap_or(zone_id);
    let Some(character) = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            player.homebind_map = map_id;
            player.homebind_zone = area_id;
            player.homebind_x = position.x;
            player.homebind_y = position.y;
            player.homebind_z = position.z;
            player.guid.counter()
        })
    else {
        return Ok(());
    };

    let repository =
        PgCharacterRepository::new(std::sync::Arc::new(world.databases.character.clone()));
    if let Err(e) = repository
        .save_homebind(
            i64::from(character),
            i64::from(map_id),
            i64::from(area_id),
            position.x,
            position.y,
            position.z,
        )
        .await
    {
        tracing::warn!("Failed to save the home of {:?}: {}", player_guid, e);
    }

    let broadcast = &world.managers.broadcast_mgr;
    broadcast.send_msg_to_player(
        player_guid,
        SmsgBindPointUpdate {
            x: position.x,
            y: position.y,
            z: position.z,
            map_id,
            zone_id: area_id,
        },
    );
    broadcast.send_msg_to_player(
        player_guid,
        SmsgPlayerBound { binder, area_id },
    );
    Ok(())
}

/// The AreaTable area at a spot, from the terrain's area flag.
pub(crate) fn area_at(world: &World, map_id: u32, position: Position) -> Option<u32> {
    let flag = u32::from(
        world
            .managers
            .terrain_mgr
            .get(map_id)
            .get_area_flag(position.x, position.y),
    );
    if flag == 0 || flag == 0xFFFF {
        return None;
    }
    let dbc = world.dbc.read();
    let mut fallback = None;
    for (_, area) in dbc.area_table.entries() {
        if area.explore_flag == flag {
            if area.map_id == map_id {
                return Some(area.id);
            }
            fallback.get_or_insert(area.id);
        }
    }
    fallback
}
