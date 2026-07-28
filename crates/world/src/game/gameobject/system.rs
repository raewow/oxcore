use crate::core::common::packet::WorldPacketGuidExt;
use crate::game::gameobject::{GOState, LootState};
use crate::World;
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::{ObjectGuid, Opcode, WorldPacket};

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn nearby_players(
    world: &World,
    map_id: u32,
    position: oxcore_shared::protocol::Position,
) -> Vec<ObjectGuid> {
    let map = world.managers.map_mgr.get_or_create_map(map_id, 0);
    map.get_objects_in_range(position, map.visibility_distance())
        .into_iter()
        .filter(|guid| guid.is_player())
        .collect()
}

/// Remove a fully looted chest and schedule it to return after its spawn delay.
pub fn despawn_looted_gameobject(player_guid: ObjectGuid, guid: ObjectGuid, world: &World) {
    let Some((map_id, position, delay_secs)) = world
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
            Some((go.map_id, go.position, go.spawntimesecs))
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
    world
        .managers
        .map_mgr
        .get_or_create_map(map_id, 0)
        .remove_gameobject(guid, position);

    let mut destroy = WorldPacket::new(Opcode::SMSG_DESTROY_OBJECT);
    destroy.write_guid_raw(guid.raw());
    for viewer_guid in nearby_players(world, map_id, position) {
        world.systems.visibility.remove_visible(viewer_guid, guid);
        world
            .managers
            .broadcast_mgr
            .send_to_player(viewer_guid, destroy.clone());
    }
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
    let due: Vec<ObjectGuid> = world
        .managers
        .gameobject_mgr
        .iter_gameobjects()
        .filter(|go| !go.in_world && go.respawn_time > 0 && go.respawn_time <= now)
        .map(|go| *go.key())
        .collect();

    for guid in due {
        let Some((map_id, position)) =
            world
                .managers
                .gameobject_mgr
                .with_gameobject_mut(guid, |go| {
                    go.in_world = true;
                    go.go_state = GOState::Ready;
                    go.loot_state = LootState::Ready;
                    go.respawn_time = 0;
                    (go.map_id, go.position)
                })
        else {
            continue;
        };

        world
            .managers
            .map_mgr
            .get_or_create_map(map_id, 0)
            .add_gameobject(guid, position);
        world
            .managers
            .gameobject_mgr
            .add_collision_model(guid, &world.managers.vmap_mgr);

        for viewer_guid in nearby_players(world, map_id, position) {
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
