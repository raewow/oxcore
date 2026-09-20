use super::*;
use crate::core::session::WorldSession;
use crate::game::player::Player;
use crate::handlers::area_trigger::handle_area_trigger;
use oxcore_shared::protocol::{Opcode, Position, WorldPacket};
use tokio::sync::mpsc;

async fn entrance_world(pool: PgPool, map_id: u32, map_type: u32) -> World {
    let world = world(fixture(pool).await);
    world
        .databases
        .world
        .execute(
            r#"
        CREATE SCHEMA world;
        CREATE TABLE world.areatrigger_teleport (
            id BIGINT, required_level BIGINT, required_condition BIGINT, message TEXT,
            target_map BIGINT, target_position_x REAL, target_position_y REAL,
            target_position_z REAL, target_orientation REAL
        );
    "#,
        )
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO world.areatrigger_teleport VALUES (100, 0, 0, '', $1, 100, 100, 10, 0)",
    )
    .bind(i64::from(map_id))
    .execute(&world.databases.world)
    .await
    .unwrap();
    world.managers.area_trigger_mgr.load().await.unwrap();
    assert!(world.managers.area_trigger_mgr.get_teleport(100).is_some());
    {
        let mut dbc = world.dbc.write();
        dbc.map.insert(
            map_id,
            crate::dbc::structures::MapEntry {
                id: map_id,
                map_type,
                max_players: 5,
                reset_delay: 86400,
            },
        );
        dbc.area_trigger.insert(
            100,
            crate::dbc::structures::AreaTriggerEntry {
                id: 100,
                map_id: 0,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                radius: 10.0,
                box_x: 0.0,
                box_y: 0.0,
                box_z: 0.0,
                box_orientation: 0.0,
            },
        );
    }
    world
        .managers
        .instance_mgr
        .initialize(&world.databases)
        .await
        .unwrap();
    world
}

fn session(
    world: &World,
    id: u32,
    map_id: u32,
) -> (WorldSession, mpsc::UnboundedReceiver<WorldPacket>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let session = WorldSession::new(id, id, format!("player{id}"), 0, tx);
    session.set_player_guid(Some(guid(id)));
    let mut player = Player::new(guid(id), format!("player{id}"), map_id, 0, 1, 60, 1, 1, 0);
    player.movement.position = Position::new(0.0, 0.0, 0.0, 0.0);
    world.managers.player_mgr.add_player(player, id);
    (session, rx)
}

async fn enter(world: &World, session: &WorldSession) -> u32 {
    let mut packet = WorldPacket::new(Opcode::CMSG_AREATRIGGER);
    packet.write_u32(100);
    handle_area_trigger(session, &mut packet, world)
        .await
        .unwrap();
    let (map, instance, position) = session
        .get_pending_teleport()
        .expect("entrance queues a transfer");
    let destination = world.managers.area_trigger_mgr.get_teleport(100).unwrap();
    assert_eq!(map, destination.destination_map);
    assert_eq!(position, destination.destination);
    assert_ne!(instance, 0, "dungeons must select a real instance");
    instance
}

async fn personal(world: &World, player: u32, map: u32, instance: u32, permanent: bool) {
    world
        .managers
        .instance_mgr
        .bind_player_to_instance(
            &world.databases,
            guid(player),
            map,
            instance,
            permanent,
            4_000_000_000,
        )
        .await
        .unwrap();
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn entrance_uses_the_party_save_when_a_member_enters_before_the_leader(pool: PgPool) {
    let world = entrance_world(pool, 36, 1).await;
    let (member, mut packets) = session(&world, 2, 0);
    let (leader, _leader_packets) = session(&world, 1, 0);
    let instance = enter(&world, &member).await;
    assert_eq!(
        packets.try_recv().unwrap().opcode(),
        Opcode::SMSG_TRANSFER_PENDING
    );
    assert_eq!(packets.try_recv().unwrap().opcode(), Opcode::SMSG_NEW_WORLD);
    assert!(packets.try_recv().is_err());
    assert_eq!(enter(&world, &leader).await, instance);
    let manager = &world.managers.instance_mgr;
    assert_eq!(
        saved(manager.get_group_binding(guid(1), 36)),
        Some((instance, false))
    );
    assert!(manager.get_player_binding(guid(1), 36).is_none());
    assert!(manager.get_player_binding(guid(2), 36).is_none());

    let (solo, _solo_packets) = session(&world, 4, 0);
    let solo_instance = enter(&world, &solo).await;
    assert_ne!(solo_instance, instance);
    assert_eq!(
        saved(manager.get_player_binding(guid(4), 36)),
        Some((solo_instance, false))
    );
    // A cold group cache must recover the persisted group save, without
    // relying on a personal binding for either the leader or arriving member.
    manager.unload_group_bindings(guid(1));
    assert_eq!(enter(&world, &member).await, instance);
    assert_eq!(
        saved(manager.get_group_binding(guid(1), 36)),
        Some((instance, false))
    );
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn simultaneous_party_arrivals_create_one_copy(pool: PgPool) {
    let world = entrance_world(pool, 36, 1).await;
    let (one, _rx1) = session(&world, 1, 0);
    let (two, _rx2) = session(&world, 2, 0);
    let (three, _rx3) = session(&world, 3, 0);
    let (first, second, third) = tokio::join!(
        enter(&world, &one),
        enter(&world, &two),
        enter(&world, &three)
    );
    assert_eq!((first, first), (second, third));
    let created: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM characters.instance WHERE map = 36 AND id NOT IN (301, 302)",
    )
    .fetch_one(&world.databases.character)
    .await
    .unwrap();
    assert_eq!(created, 1);
    let group_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM characters.group_instance WHERE leader_guid = 1")
            .fetch_one(&world.databases.character)
            .await
            .unwrap();
    assert_eq!(group_rows, 1);
    for player in [1, 2, 3] {
        assert!(world
            .managers
            .instance_mgr
            .get_player_binding(guid(player), 36)
            .is_none());
    }
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn first_group_entry_converts_only_eligible_temporary_saves(pool: PgPool) {
    let world = entrance_world(pool, 36, 1).await;
    let mut group = world.systems.group.get_group(1).unwrap();
    group.add_member(guid(4), "saved member".into()).unwrap();
    world.systems.group.add_group_test(group);
    world.systems.group.add_player_to_group_test(guid(4), 1);
    let (leader, _rx1) = session(&world, 1, 0);
    let (entrant, _rx2) = session(&world, 2, 0);
    let (_inside, _rx3) = session(&world, 3, 36);
    let (_saved, _rx4) = session(&world, 4, 0);
    personal(&world, 1, 36, 302, false).await;
    personal(&world, 2, 36, 301, false).await;
    personal(&world, 2, 329, 501, false).await;
    personal(&world, 3, 36, 302, false).await;
    personal(&world, 4, 36, 301, true).await;
    sqlx::query("UPDATE characters.instance SET data = '7' WHERE id = 301")
        .execute(&world.databases.character)
        .await
        .unwrap();

    assert_eq!(enter(&world, &entrant).await, 301);
    assert_eq!(enter(&world, &leader).await, 301);
    let manager = &world.managers.instance_mgr;
    let reloaded = InstanceMgr::new();
    for player in [1, 2, 3, 4] {
        reloaded
            .load_player_bindings(&world.databases, guid(player))
            .await
            .unwrap();
    }
    for manager in [manager.as_ref(), &reloaded] {
        assert!(manager.get_player_binding(guid(1), 36).is_none());
        assert!(manager.get_player_binding(guid(2), 36).is_none());
        assert_eq!(
            saved(manager.get_player_binding(guid(2), 329)),
            Some((501, false))
        );
        assert_eq!(
            saved(manager.get_player_binding(guid(3), 36)),
            Some((302, false))
        );
        assert_eq!(
            saved(manager.get_player_binding(guid(4), 36)),
            Some((301, true))
        );
    }
    assert!(manager.is_encounter_completed(36, 301, 7));
    // The member still inside its solo copy retains that personal selection,
    // exactly as GetBoundInstanceSaveForSelfOrGroup does in the reference.
    let context = InstanceEntryGroup {
        leader_guid: guid(1),
        members_outside: Vec::new(),
    };
    assert_eq!(
        manager
            .enter_instance(&world.databases, 36, guid(3), Some(&context), false)
            .await
            .unwrap(),
        302
    );
    assert_eq!(
        saved(manager.get_group_binding(guid(1), 36)),
        Some((301, false))
    );
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn permanent_personal_entry_preserves_the_groups_other_copy(pool: PgPool) {
    let world = entrance_world(pool, 409, 2).await;
    let (member, _rx2) = session(&world, 2, 0);
    let (leader, _rx1) = session(&world, 1, 0);
    personal(&world, 2, 409, 101, true).await;
    let manager = &world.managers.instance_mgr;
    manager
        .bind_group_to_instance(&world.databases, guid(1), 409, 102, false, 4_000_000_000)
        .await
        .unwrap();
    assert_eq!(enter(&world, &member).await, 101);
    assert_eq!(enter(&world, &leader).await, 102);
    assert_eq!(
        saved(manager.get_player_binding(guid(2), 409)),
        Some((101, true))
    );
    assert_eq!(
        saved(manager.get_group_binding(guid(1), 409)),
        Some((102, false))
    );
    assert!(manager.get_player_binding(guid(1), 409).is_none());
    // A permanent personal entry also leaves an unbound group unbound.
    manager
        .unbind_group_from_instance(&world.databases, guid(1), 409, 102)
        .await
        .unwrap();
    assert_eq!(enter(&world, &member).await, 101);
    assert!(manager.get_group_binding(guid(1), 409).is_none());
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn permanent_group_entry_binds_an_unbound_member(pool: PgPool) {
    let world = entrance_world(pool, 409, 2).await;
    let (member, _rx) = session(&world, 2, 0);
    let manager = &world.managers.instance_mgr;
    manager
        .bind_group_to_instance(&world.databases, guid(1), 409, 102, true, 4_000_000_000)
        .await
        .unwrap();
    assert_eq!(enter(&world, &member).await, 102);
    assert_eq!(
        saved(manager.get_player_binding(guid(2), 409)),
        Some((102, true))
    );
    assert_eq!(
        saved(manager.get_group_binding(guid(1), 409)),
        Some((102, true))
    );
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn failed_solo_conversion_does_not_publish_a_group_binding(pool: PgPool) {
    let world = entrance_world(pool, 36, 1).await;
    personal(&world, 2, 36, 301, false).await;
    // Force the personal-save deletion to fail after the group row is inserted.
    world.databases.character.execute("CREATE TABLE retained_save (guid BIGINT, instance BIGINT, FOREIGN KEY (guid, instance) REFERENCES characters.character_instance(guid, instance)); INSERT INTO retained_save VALUES (2, 301)").await.unwrap();
    let manager = &world.managers.instance_mgr;
    let context = InstanceEntryGroup {
        leader_guid: guid(1),
        members_outside: Vec::new(),
    };
    assert!(manager
        .enter_instance(&world.databases, 36, guid(2), Some(&context), false)
        .await
        .is_err());
    assert!(manager.get_group_binding(guid(1), 36).is_none());
    assert_eq!(
        saved(manager.get_player_binding(guid(2), 36)),
        Some((301, false))
    );
    let reloaded = InstanceMgr::new();
    assert!(reloaded
        .load_group_bindings(&world.databases, guid(1))
        .await
        .unwrap()
        .is_empty());
    reloaded
        .load_player_bindings(&world.databases, guid(2))
        .await
        .unwrap();
    assert_eq!(
        saved(reloaded.get_player_binding(guid(2), 36)),
        Some((301, false))
    );
}
