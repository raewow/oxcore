use crate::game::gameobject::{GOState, LootState};
use crate::World;
use oxcore_shared::messages::inventory::SmsgDestroyObject;
use oxcore_shared::protocol::{ObjectGuid, Position};

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn nearby_players(
    world: &World,
    map_id: u32,
    instance_id: u32,
    position: Position,
) -> Vec<ObjectGuid> {
    let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) else {
        return Vec::new();
    };
    map.get_players_in_range(position, map.visibility_distance())
}

/// Remove a fully looted chest and schedule it to return after its spawn delay.
pub fn despawn_looted_gameobject(player_guid: ObjectGuid, guid: ObjectGuid, world: &World) {
    let Some((map_id, instance_id, position, delay_secs)) = world
        .managers
        .gameobject_mgr
        .with_gameobject_mut(guid, |go| {
            if !go.in_world {
                return None;
            }
            go.in_world = false;
            go.go_state = GOState::Ready;
            go.loot_state = LootState::JustDeactivated;
            go.respawn_time = if go.spawntimesecs > 0 {
                now_ms() + u64::from(go.spawntimesecs) * 1000
            } else {
                0
            };
            Some((go.map_id, go.instance_id, go.position, go.spawntimesecs))
        })
        .flatten()
    else {
        return;
    };

    world.systems.loot.remove_loot(guid);
    world
        .managers
        .gameobject_mgr
        .remove_collision_model(guid, &world.managers.vmap_mgr);
    if let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) {
        map.remove_gameobject(guid, position);
    }

    // A message, not a pre-encoded packet: 1.14 has no `SMSG_DESTROY_OBJECT` opcode, so a vanilla
    // body is refused for modern clients and the looted object stays standing on their screen.
    let destroy = SmsgDestroyObject { guid };
    let viewers = nearby_players(world, map_id, instance_id, position);
    world
        .managers
        .broadcast_mgr
        .broadcast_msg_to_players(&viewers, &destroy);
    tracing::debug!(
        "Despawned looted gameobject {:?} for {}s (looter {:?})",
        guid,
        delay_secs,
        player_guid
    );
}

/// Restore gameobjects whose respawn delay elapsed.
pub fn update_respawns(world: &World) {
    let now = now_ms();
    let due: Vec<_> = world
        .managers
        .gameobject_mgr
        .iter_gameobjects()
        .filter(|go| !go.in_world && go.respawn_time > 0 && go.respawn_time <= now)
        .map(|go| (*go.key(), go.map_id, go.instance_id))
        .collect();

    for (guid, map_id, instance_id) in due {
        // A retained object cannot recreate a map that has been unloaded/reset.
        let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) else {
            continue;
        };
        let Some(position) = world
            .managers
            .gameobject_mgr
            .with_gameobject_mut(guid, |go| {
                if go.in_world || go.respawn_time == 0 || go.respawn_time > now {
                    return None;
                }
                go.in_world = true;
                go.go_state = GOState::Ready;
                go.loot_state = LootState::Ready;
                go.respawn_time = 0;
                Some(go.position)
            })
            .flatten()
        else {
            continue;
        };

        map.add_gameobject(guid, position);
        world
            .managers
            .gameobject_mgr
            .add_collision_model(guid, &world.managers.vmap_mgr);

        for viewer_guid in nearby_players(world, map_id, instance_id, position) {
            if let Some(create) =
                world
                    .managers
                    .gameobject_mgr
                    .build_create_msg(guid, viewer_guid, world)
            {
                world
                    .managers
                    .broadcast_mgr
                    .send_msg_to_player(viewer_guid, create);
            }
        }
        tracing::debug!("Respawned gameobject {:?}", guid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::game::gameobject::{GameObjectSpawnData, GameObjectTemplate};
    use crate::game::player::{Player, PlayerBroadcaster};
    use oxcore_db::database::Databases;
    use oxcore_shared::protocol::{Opcode, Protocol, WorldPacket};
    use std::{path::PathBuf, sync::Arc};
    use tokio::sync::mpsc;

    const MAP_ID: u32 = 36;

    fn test_world() -> World {
        let pool = || {
            sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://test:test@localhost/test")
                .unwrap()
        };
        let world = World::new(
            Arc::new(Databases {
                world: pool(),
                character: pool(),
                auth: pool(),
                logs: oxcore_db::database::lazy_logs_pool(),
            }),
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        );
        world
            .managers
            .gameobject_mgr
            .add_template_for_test(GameObjectTemplate {
                entry: 70,
                go_type: 3,
                display_id: 1,
                name: "chest".into(),
                icon_name: String::new(),
                cast_bar_caption: String::new(),
                faction: 0,
                flags: 0,
                size: 1.0,
                data: [0; 24],
            });
        world
    }

    fn spawn() -> GameObjectSpawnData {
        GameObjectSpawnData {
            spawn_id: 1,
            entry: 70,
            map_id: MAP_ID,
            position: Position::new(100.0, 100.0, 0.0, 0.0),
            rotation0: 0.0,
            rotation1: 0.0,
            rotation2: 0.0,
            rotation3: 1.0,
            spawntimesecs: 300,
            animprogress: 0,
            state: 1,
        }
    }

    fn observer(
        world: &World,
        counter: u32,
        instance_id: u32,
        position: Position,
        protocol: Protocol,
    ) -> (ObjectGuid, mpsc::UnboundedReceiver<WorldPacket>) {
        let guid = ObjectGuid::new_player(counter);
        let mut player = Player::new(guid, "observer".into(), MAP_ID, instance_id, 1, 60, 1, 1, 0);
        player.movement.position = position;
        let (tx, rx) = mpsc::unbounded_channel();
        let mut broadcaster = PlayerBroadcaster::new(tx, guid);
        broadcaster.set_protocol(protocol);
        broadcaster.set_map_id(MAP_ID as u16);
        player.set_broadcaster(Arc::new(broadcaster));
        world
            .managers
            .map_mgr
            .get_or_create_map(MAP_ID, instance_id)
            .add_player(guid, position);
        world.managers.player_mgr.add_player(player, counter);
        (guid, rx)
    }

    #[tokio::test]
    async fn looted_chest_respawns_and_notifies_only_its_own_instance() {
        for protocol in [Protocol::Vanilla, Protocol::Modern] {
            let world = test_world();
            let spawn = spawn();
            let mut copies = Vec::new();
            for instance_id in [1, 2, 0] {
                let map = world
                    .managers
                    .map_mgr
                    .get_or_create_map(MAP_ID, instance_id);
                let guid = world
                    .managers
                    .gameobject_mgr
                    .spawn_gameobject(&spawn, instance_id)
                    .unwrap();
                map.add_gameobject(guid, spawn.position);
                assert_eq!(
                    world
                        .managers
                        .gameobject_mgr
                        .with_gameobject(guid, |go| go.instance_id),
                    Some(instance_id)
                );
                copies.push((map, guid));
            }
            let chest = copies[0].1;
            let mut observers = Vec::new();
            for (counter, instance_id, position) in [
                (1, 1, spawn.position),
                (2, 1, spawn.position),
                (3, 2, spawn.position),
                (4, 0, spawn.position),
                (5, 1, Position::new(2000.0, 2000.0, 0.0, 0.0)),
            ] {
                observers.push(observer(&world, counter, instance_id, position, protocol));
            }

            despawn_looted_gameobject(observers[0].0, chest, &world);
            assert!(!copies[0]
                .0
                .get_objects_in_range(spawn.position, 5.0)
                .contains(&chest));
            assert_eq!(
                world
                    .managers
                    .gameobject_mgr
                    .with_gameobject(chest, |go| (go.in_world, go.loot_state)),
                Some((false, LootState::JustDeactivated))
            );
            for (index, (_, rx)) in observers.iter_mut().enumerate() {
                if index < 2 {
                    let packet = rx
                        .try_recv()
                        .expect("nearby player in owning instance receives removal");
                    match protocol {
                        Protocol::Vanilla => {
                            assert_eq!(packet.opcode(), Opcode::SMSG_DESTROY_OBJECT);
                            assert_eq!(packet.contents(), chest.raw().to_le_bytes());
                        }
                        Protocol::Modern => {
                            assert_eq!(packet.opcode(), Opcode::SMSG_UPDATE_OBJECT);
                            assert_eq!(
                                u16::from_le_bytes(packet.contents()[4..6].try_into().unwrap()),
                                MAP_ID as u16
                            );
                            assert_eq!(
                                u32::from_le_bytes(packet.contents()[..4].try_into().unwrap()),
                                0
                            );
                            assert_ne!(
                                packet.contents()[6] & 0x80,
                                0,
                                "modern removal list is present"
                            );
                        }
                    }
                }
                assert!(
                    rx.try_recv().is_err(),
                    "no duplicate or cross-instance removal"
                );
            }

            // The original deadline is still in the future; ticking cannot revive it yet.
            update_respawns(&world);
            assert_eq!(
                world
                    .managers
                    .gameobject_mgr
                    .with_gameobject(chest, |go| go.in_world),
                Some(false)
            );
            world
                .managers
                .gameobject_mgr
                .with_gameobject_mut(chest, |go| go.respawn_time = 1);
            update_respawns(&world);

            assert!(copies[0]
                .0
                .get_objects_in_range(spawn.position, 5.0)
                .contains(&chest));
            assert_eq!(
                world.managers.gameobject_mgr.with_gameobject(chest, |go| (
                    go.in_world,
                    go.loot_state,
                    go.respawn_time
                )),
                Some((true, LootState::Ready, 0))
            );
            for (map, other) in &copies[1..] {
                let objects = map.get_objects_in_range(spawn.position, 5.0);
                assert!(objects.contains(other));
                assert!(!objects.contains(&chest));
                assert_eq!(
                    world
                        .managers
                        .gameobject_mgr
                        .with_gameobject(*other, |go| (go.in_world, go.respawn_time)),
                    Some((true, 0))
                );
            }
            for (index, (_, rx)) in observers.iter_mut().enumerate() {
                if index < 2 {
                    let packet = rx
                        .try_recv()
                        .expect("nearby player in owning instance receives respawn");
                    assert_eq!(packet.opcode(), Opcode::SMSG_UPDATE_OBJECT);
                    assert_eq!(
                        u32::from_le_bytes(packet.contents()[..4].try_into().unwrap()),
                        1,
                        "one object creation block"
                    );
                }
                assert!(
                    rx.try_recv().is_err(),
                    "no duplicate or cross-instance creation"
                );
            }
            update_respawns(&world);
            assert!(observers.iter_mut().all(|(_, rx)| rx.try_recv().is_err()));
            assert_eq!(world.managers.map_mgr.map_count(), 3);
        }
    }

    #[tokio::test]
    async fn respawn_waits_for_its_owning_map_without_creating_instance_zero() {
        let world = test_world();
        let spawn = spawn();
        let guid = world
            .managers
            .gameobject_mgr
            .spawn_gameobject(&spawn, 7)
            .unwrap();
        despawn_looted_gameobject(ObjectGuid::new_player(1), guid, &world);
        world
            .managers
            .gameobject_mgr
            .with_gameobject_mut(guid, |go| go.respawn_time = 1);
        update_respawns(&world);
        assert_eq!(world.managers.map_mgr.map_count(), 0);
        assert_eq!(
            world
                .managers
                .gameobject_mgr
                .with_gameobject(guid, |go| (go.in_world, go.respawn_time)),
            Some((false, 1))
        );

        let map = world.managers.map_mgr.get_or_create_map(MAP_ID, 7);
        update_respawns(&world);
        assert!(map
            .get_objects_in_range(spawn.position, 5.0)
            .contains(&guid));
        assert_eq!(
            world
                .managers
                .gameobject_mgr
                .with_gameobject(guid, |go| go.in_world),
            Some(true)
        );
        assert!(world.managers.map_mgr.get_map(MAP_ID, 0).is_none());
    }
}
