use super::*;
use crate::{config::Config, World};
use oxcore_shared::game::group::GroupData;
use sqlx::{Executor, PgPool};

mod allocation;
mod entry;

fn guid(id: u32) -> ObjectGuid {
    ObjectGuid::new_player(id)
}

fn binding(map_id: u32, instance_id: u32, permanent: bool) -> InstanceBinding {
    InstanceBinding {
        map_id,
        instance_id,
        permanent,
        reset_time: 4_000_000_000,
    }
}

fn saved(binding: Option<InstanceBinding>) -> Option<(u32, bool)> {
    binding.map(|binding| (binding.instance_id, binding.permanent))
}

#[test]
fn group_lookup_never_uses_the_leaders_personal_save() {
    let manager = InstanceMgr::new();
    manager
        .player_bindings
        .write()
        .insert(guid(1), HashMap::from([(409, binding(409, 101, true))]));
    assert!(manager.get_group_binding(guid(1), 409).is_none());
    manager
        .group_bindings
        .write()
        .insert(guid(1), HashMap::from([(409, binding(409, 102, false))]));
    assert_eq!(
        saved(manager.get_group_binding(guid(1), 409)),
        Some((102, false))
    );
    manager.unload_group_bindings(guid(1));
    assert!(manager.get_group_binding(guid(1), 409).is_none());
    assert_eq!(
        saved(manager.get_player_binding(guid(1), 409)),
        Some((101, true))
    );
}

#[tokio::test]
async fn group_reset_checks_the_group_copy_even_when_the_leader_has_a_solo_copy() {
    let manager = InstanceMgr::new();
    manager
        .player_bindings
        .write()
        .insert(guid(1), HashMap::from([(36, binding(36, 101, false))]));
    manager
        .group_bindings
        .write()
        .insert(guid(1), HashMap::from([(36, binding(36, 102, false))]));
    let pool = PgPool::connect_lazy("postgres://localhost/unused_instance_test").unwrap();
    let databases = Databases {
        world: pool.clone(),
        character: pool.clone(),
        auth: pool.clone(),
        logs: pool,
    };
    let (reset, failed) = manager
        .reset_group_instances(&databases, guid(1), |map, instance| {
            assert_eq!((map, instance), (36, 102));
            vec![guid(2)]
        })
        .await;
    assert!(reset.is_empty());
    assert_eq!(
        failed,
        vec![(36, super::super::InstanceResetFailReason::Offline)]
    );
    assert_eq!(
        saved(manager.get_player_binding(guid(1), 36)),
        Some((101, false))
    );
}

// sqlx creates a separate database per test. These minimal tables retain the
// production ownership keys, types, and foreign keys relevant to instance saves.
async fn fixture(pool: PgPool) -> Databases {
    pool.execute(r#"
        CREATE SCHEMA characters;
        CREATE TABLE characters.characters (guid BIGINT PRIMARY KEY);
        CREATE TABLE characters.groups (
            group_id BIGINT PRIMARY KEY,
            leader_guid BIGINT NOT NULL UNIQUE REFERENCES characters.characters(guid)
        );
        CREATE TABLE characters.group_member (
            group_id BIGINT NOT NULL REFERENCES characters.groups(group_id) ON DELETE CASCADE,
            member_guid BIGINT NOT NULL REFERENCES characters.characters(guid) ON DELETE CASCADE,
            assistant SMALLINT NOT NULL DEFAULT 0,
            subgroup SMALLINT NOT NULL DEFAULT 0,
            PRIMARY KEY (group_id, member_guid)
        );
        CREATE TABLE characters.instance (
            id BIGINT PRIMARY KEY, map BIGINT NOT NULL, reset_time BIGINT NOT NULL, data TEXT
        );
        CREATE TABLE characters.character_instance (
            guid BIGINT NOT NULL REFERENCES characters.characters(guid) ON DELETE CASCADE,
            instance BIGINT NOT NULL REFERENCES characters.instance(id) ON DELETE CASCADE,
            permanent SMALLINT NOT NULL DEFAULT 0, extend SMALLINT NOT NULL DEFAULT 0,
            PRIMARY KEY (guid, instance)
        );
        CREATE TABLE characters.group_instance (
            leader_guid BIGINT NOT NULL REFERENCES characters.groups(leader_guid) ON DELETE CASCADE,
            instance BIGINT NOT NULL REFERENCES characters.instance(id) ON DELETE CASCADE,
            permanent SMALLINT NOT NULL DEFAULT 0,
            PRIMARY KEY (leader_guid, instance)
        );
        INSERT INTO characters.characters VALUES (1), (2), (3), (4);
        INSERT INTO characters.groups VALUES (1, 1), (2, 4);
        INSERT INTO characters.group_member (group_id, member_guid) VALUES (1, 1), (1, 2), (1, 3), (2, 4);
        INSERT INTO characters.instance (id, map, reset_time, data) VALUES
            (101, 409, 4000000000, ''), (102, 409, 4000000000, ''),
            (201, 531, 4000000000, ''), (202, 531, 4000000000, ''),
            (301, 36, 4000000000, ''), (302, 36, 4000000000, ''),
            (401, 230, 4000000000, ''), (501, 329, 4000000000, '');
    "#).await.unwrap();
    Databases {
        world: pool.clone(),
        character: pool.clone(),
        auth: pool.clone(),
        logs: pool,
    }
}

fn world(databases: Databases) -> World {
    let world = World::new(
        Arc::new(databases),
        Arc::new(Config::default()),
        50,
        "/nonexistent-instance-test-data".into(),
    );
    let mut group = GroupData::new(1, guid(1), "old leader".into());
    group.add_member(guid(2), "new leader".into()).unwrap();
    group.add_member(guid(3), "member".into()).unwrap();
    world.systems.group.add_group_test(group);
    for id in [1, 2, 3] {
        world.systems.group.add_player_to_group_test(guid(id), 1);
    }
    world
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn loading_and_binding_groups_preserves_personal_saves(pool: PgPool) {
    let databases = fixture(pool).await;
    let manager = InstanceMgr::new();
    manager
        .bind_player_to_instance(&databases, guid(1), 409, 101, true, 4_000_000_000)
        .await
        .unwrap();
    assert!(manager
        .load_group_bindings(&databases, guid(1))
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        saved(manager.get_player_binding(guid(1), 409)),
        Some((101, true))
    );
    manager
        .bind_group_to_instance(&databases, guid(1), 409, 102, false, 4_000_000_000)
        .await
        .unwrap();
    assert_eq!(
        saved(manager.get_group_binding(guid(1), 409)),
        Some((102, false))
    );
    assert_eq!(
        saved(manager.get_player_binding(guid(1), 409)),
        Some((101, true))
    );

    // Selection reads the newly written group cache even before a reload.
    assert_eq!(
        manager
            .get_or_create_instance(&databases, 409, Some(guid(2)), Some(guid(1)), false)
            .await
            .unwrap(),
        102
    );
    let reloaded = InstanceMgr::new();
    reloaded
        .load_group_bindings(&databases, guid(1))
        .await
        .unwrap();
    reloaded
        .load_player_bindings(&databases, guid(1))
        .await
        .unwrap();
    assert_eq!(
        saved(reloaded.get_player_binding(guid(1), 409)),
        Some((101, true))
    );
    assert_eq!(
        saved(reloaded.get_group_binding(guid(1), 409)),
        Some((102, false))
    );
    reloaded
        .load_group_bindings(&databases, guid(1))
        .await
        .unwrap();
    assert_eq!(
        saved(reloaded.get_player_binding(guid(1), 409)),
        Some((101, true))
    );
}

async fn check_leader_transfer(pool: PgPool, automatic: bool) {
    let world = world(fixture(pool).await);
    let manager = &world.managers.instance_mgr;
    let databases = &world.databases;
    for (owner, map, instance, permanent) in [
        (1, 409, 101, true),
        (1, 329, 501, false),
        (2, 531, 201, true),
        (2, 230, 401, false),
    ] {
        manager
            .bind_player_to_instance(
                databases,
                guid(owner),
                map,
                instance,
                permanent,
                4_000_000_000,
            )
            .await
            .unwrap();
    }
    for (owner, map, instance, permanent) in [
        (1, 409, 102, true),
        (1, 36, 301, false),
        (1, 531, 202, false),
        (4, 36, 302, false),
    ] {
        manager
            .bind_group_to_instance(
                databases,
                guid(owner),
                map,
                instance,
                permanent,
                4_000_000_000,
            )
            .await
            .unwrap();
    }
    if automatic {
        world
            .systems
            .group
            .leave_group(&world, guid(1))
            .await
            .unwrap();
    } else {
        world
            .systems
            .group
            .set_leader(&world, guid(1), guid(2))
            .await
            .unwrap();
    }
    assert_eq!(
        world.systems.group.get_group(1).unwrap().leader_guid,
        guid(2)
    );
    let leader: i64 =
        sqlx::query_scalar("SELECT leader_guid FROM characters.groups WHERE group_id = 1")
            .fetch_one(&databases.character)
            .await
            .unwrap();
    assert_eq!(leader, 2);

    let reloaded = InstanceMgr::new();
    for owner in [1, 2, 4] {
        reloaded
            .load_player_bindings(databases, guid(owner))
            .await
            .unwrap();
        reloaded
            .load_group_bindings(databases, guid(owner))
            .await
            .unwrap();
    }
    for manager in [manager.as_ref(), &reloaded] {
        assert_eq!(
            saved(manager.get_player_binding(guid(1), 409)),
            Some((101, true))
        );
        assert_eq!(
            saved(manager.get_player_binding(guid(1), 329)),
            Some((501, false))
        );
        assert_eq!(
            saved(manager.get_player_binding(guid(1), 531)),
            Some((202, false))
        );
        assert_eq!(
            saved(manager.get_player_binding(guid(2), 531)),
            Some((201, true))
        );
        assert!(manager.get_player_binding(guid(2), 409).is_none());
        assert!(manager.get_player_binding(guid(2), 230).is_none());
        assert_eq!(
            saved(manager.get_group_binding(guid(2), 36)),
            Some((301, false))
        );
        assert_eq!(
            saved(manager.get_group_binding(guid(2), 531)),
            Some((201, true))
        );
        assert_eq!(
            saved(manager.get_group_binding(guid(2), 230)),
            Some((401, false))
        );
        assert!(manager.get_group_binding(guid(2), 409).is_none());
        for map in [409, 36, 531] {
            assert!(manager.get_group_binding(guid(1), map).is_none());
        }
        assert_eq!(
            saved(manager.get_group_binding(guid(4), 36)),
            Some((302, false))
        );
    }
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn changing_leader_preserves_each_owners_saves(pool: PgPool) {
    check_leader_transfer(pool, false).await;
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn automatic_leader_change_transfers_the_original_leaders_group_bindings(pool: PgPool) {
    check_leader_transfer(pool, true).await;
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn failed_transfer_preserves_bindings_and_live_leadership(pool: PgPool) {
    let world = world(fixture(pool).await);
    let manager = &world.managers.instance_mgr;
    let databases = &world.databases;
    manager
        .bind_group_to_instance(databases, guid(1), 531, 202, false, 4_000_000_000)
        .await
        .unwrap();
    manager
        .bind_player_to_instance(databases, guid(2), 531, 201, true, 4_000_000_000)
        .await
        .unwrap();
    // The new leader already owns a different group: the UNIQUE constraint
    // fails after the attempted save handoff and deletion of the old group rows.
    sqlx::query("UPDATE characters.groups SET leader_guid = 2 WHERE group_id = 2")
        .execute(&databases.character)
        .await
        .unwrap();
    assert!(world
        .systems
        .group
        .set_leader(&world, guid(1), guid(2))
        .await
        .is_err());
    assert_eq!(
        world.systems.group.get_group(1).unwrap().leader_guid,
        guid(1)
    );
    assert_eq!(
        saved(manager.get_group_binding(guid(1), 531)),
        Some((202, false))
    );
    assert!(manager.get_group_binding(guid(2), 531).is_none());
    assert!(manager.get_player_binding(guid(1), 531).is_none());
    assert_eq!(
        saved(manager.get_player_binding(guid(2), 531)),
        Some((201, true))
    );
    let reloaded = InstanceMgr::new();
    reloaded
        .load_group_bindings(databases, guid(1))
        .await
        .unwrap();
    reloaded
        .load_player_bindings(databases, guid(1))
        .await
        .unwrap();
    assert_eq!(
        saved(reloaded.get_group_binding(guid(1), 531)),
        Some((202, false))
    );
    assert!(reloaded.get_player_binding(guid(1), 531).is_none());
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn removing_a_group_binding_preserves_personal_and_other_group_bindings(pool: PgPool) {
    let databases = fixture(pool).await;
    let manager = InstanceMgr::new();
    manager
        .bind_player_to_instance(&databases, guid(1), 36, 301, false, 4_000_000_000)
        .await
        .unwrap();
    manager
        .bind_group_to_instance(&databases, guid(1), 36, 301, false, 4_000_000_000)
        .await
        .unwrap();
    manager
        .bind_group_to_instance(&databases, guid(4), 36, 302, false, 4_000_000_000)
        .await
        .unwrap();
    manager
        .unbind_group_from_instance(&databases, guid(1), 36, 302)
        .await
        .unwrap();
    assert_eq!(
        saved(manager.get_group_binding(guid(1), 36)),
        Some((301, false))
    );
    manager
        .unbind_group_from_instance(&databases, guid(1), 36, 301)
        .await
        .unwrap();
    assert!(manager.get_group_binding(guid(1), 36).is_none());
    assert!(manager
        .load_group_bindings(&databases, guid(1))
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        saved(manager.get_player_binding(guid(1), 36)),
        Some((301, false))
    );
    assert_eq!(
        saved(manager.get_group_binding(guid(4), 36)),
        Some((302, false))
    );
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn group_reset_does_not_select_the_leaders_personal_dungeons(pool: PgPool) {
    let databases = fixture(pool).await;
    let manager = InstanceMgr::new();
    manager
        .bind_player_to_instance(&databases, guid(1), 329, 501, false, 4_000_000_000)
        .await
        .unwrap();
    manager
        .bind_group_to_instance(&databases, guid(1), 36, 301, false, 4_000_000_000)
        .await
        .unwrap();
    manager
        .bind_group_to_instance(&databases, guid(1), 531, 201, true, 4_000_000_000)
        .await
        .unwrap();
    let (reset, failed) = manager
        .reset_group_instances(&databases, guid(1), |map, instance| {
            assert_eq!((map, instance), (36, 301));
            Vec::new()
        })
        .await;
    assert_eq!(reset, vec![36]);
    assert!(failed.is_empty());
    assert!(manager.get_group_binding(guid(1), 36).is_none());
    assert_eq!(
        saved(manager.get_player_binding(guid(1), 329)),
        Some((501, false))
    );
    assert_eq!(
        saved(manager.get_group_binding(guid(1), 531)),
        Some((201, true))
    );
    assert!(manager
        .load_instance(&databases, 36, 301)
        .await
        .unwrap()
        .is_none());
    assert!(manager
        .load_instance(&databases, 329, 501)
        .await
        .unwrap()
        .is_some());
}
