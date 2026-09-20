use super::*;
use std::collections::BTreeMap;
use tokio::{sync::Barrier, task::JoinSet};

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn first_instances_on_different_maps_have_unique_nonzero_ids(pool: PgPool) {
    let databases = fixture(pool).await;
    sqlx::query("DELETE FROM characters.instance")
        .execute(&databases.character)
        .await
        .unwrap();
    let manager = InstanceMgr::new();
    manager.initialize(&databases).await.unwrap();

    let first = manager
        .create_instance(&databases, 36, 0, false)
        .await
        .unwrap();
    let second = manager
        .create_instance(&databases, 33, 0, false)
        .await
        .unwrap();
    assert_ne!(first, 0);
    assert_ne!(second, 0);
    assert_ne!(first, second);
    let saved: Vec<(i64, i64)> =
        sqlx::query_as("SELECT id, map FROM characters.instance ORDER BY id")
            .fetch_all(&databases.character)
            .await
            .unwrap();
    assert_eq!(saved, vec![(i64::from(first), 36), (i64::from(second), 33)]);
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn concurrent_allocations_and_restart_preserve_global_ids_and_existing_saves(pool: PgPool) {
    let databases = Arc::new(fixture(pool).await);
    sqlx::query("UPDATE characters.instance SET data = '1,7' WHERE id = 501")
        .execute(&databases.character)
        .await
        .unwrap();
    let manager = Arc::new(InstanceMgr::new());
    manager.initialize(&databases).await.unwrap();
    manager
        .bind_player_to_instance(&databases, guid(1), 329, 501, true, 4_000_000_000)
        .await
        .unwrap();

    let barrier = Arc::new(Barrier::new(33));
    let mut tasks = JoinSet::new();
    for index in 0..32 {
        let databases = Arc::clone(&databases);
        let manager = Arc::clone(&manager);
        let barrier = Arc::clone(&barrier);
        tasks.spawn(async move {
            // Exercise simultaneous requests on both the same and different maps,
            // including a map with no previous saves.
            let map_id = [36, 33, 409, 329][index % 4];
            barrier.wait().await;
            let instance_id = manager
                .get_or_create_instance(&databases, map_id, None, None, false)
                .await
                .unwrap();
            (i64::from(instance_id), i64::from(map_id))
        });
    }
    barrier.wait().await;
    let mut created = BTreeMap::new();
    while let Some(result) = tasks.join_next().await {
        let (instance_id, map_id) = result.unwrap();
        assert!(
            instance_id > 501,
            "new IDs must exceed every persisted save"
        );
        assert!(created.insert(instance_id, map_id).is_none());
    }
    assert_eq!(created.len(), 32);
    let saved: Vec<(i64, i64)> =
        sqlx::query_as("SELECT id, map FROM characters.instance WHERE id > 501")
            .fetch_all(&databases.character)
            .await
            .unwrap();
    assert_eq!(saved.into_iter().collect::<BTreeMap<_, _>>(), created);
    drop(manager);

    let restarted = InstanceMgr::new();
    restarted.initialize(&databases).await.unwrap();
    for map_id in [36, 389] {
        let id = restarted
            .create_instance(&databases, map_id, 0, false)
            .await
            .unwrap();
        assert!(i64::from(id) > *created.last_key_value().unwrap().0);
        assert!(created.insert(i64::from(id), i64::from(map_id)).is_none());
    }
    restarted
        .load_player_bindings(&databases, guid(1))
        .await
        .unwrap();
    assert_eq!(
        restarted
            .get_or_create_instance(&databases, 329, Some(guid(1)), None, false)
            .await
            .unwrap(),
        501
    );
    assert!(restarted.is_encounter_completed(329, 501, 1));
    assert!(restarted.is_encounter_completed(329, 501, 7));
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn exhausted_ids_never_wrap_to_zero_or_overwrite_an_existing_save(pool: PgPool) {
    let databases = fixture(pool).await;
    sqlx::query("INSERT INTO characters.instance VALUES ($1, 33, 4000000000, '9')")
        .bind(i64::from(u32::MAX - 1))
        .execute(&databases.character)
        .await
        .unwrap();
    let manager = InstanceMgr::new();
    manager.initialize(&databases).await.unwrap();
    assert_eq!(
        manager
            .create_instance(&databases, 36, 0, false)
            .await
            .unwrap(),
        u32::MAX
    );
    assert!(manager
        .create_instance(&databases, 33, 0, false)
        .await
        .is_err());
    assert!(manager
        .create_instance(&databases, 36, 0, false)
        .await
        .is_err());

    let restarted = InstanceMgr::new();
    restarted.initialize(&databases).await.unwrap();
    assert!(restarted
        .create_instance(&databases, 389, 0, false)
        .await
        .is_err());
    let zero_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM characters.instance WHERE id = 0")
            .fetch_one(&databases.character)
            .await
            .unwrap();
    assert_eq!(zero_count, 0);
    let last: (i64, String) =
        sqlx::query_as("SELECT map, data FROM characters.instance WHERE id = $1")
            .bind(i64::from(u32::MAX - 1))
            .fetch_one(&databases.character)
            .await
            .unwrap();
    assert_eq!(last, (33, "9".into()));
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn initialization_rejects_persisted_ids_outside_the_runtime_range(pool: PgPool) {
    let databases = fixture(pool).await;
    sqlx::query("INSERT INTO characters.instance VALUES ($1, 33, 4000000000, '')")
        .bind(i64::from(u32::MAX) + 1)
        .execute(&databases.character)
        .await
        .unwrap();
    assert!(InstanceMgr::new().initialize(&databases).await.is_err());
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn reinitialization_does_not_reuse_an_id_reserved_for_a_failed_insert(pool: PgPool) {
    let databases = fixture(pool).await;
    sqlx::query("ALTER TABLE characters.instance ADD CHECK (map <> 999)")
        .execute(&databases.character)
        .await
        .unwrap();
    let manager = InstanceMgr::new();
    manager.initialize(&databases).await.unwrap();
    assert!(manager
        .create_instance(&databases, 999, 0, false)
        .await
        .is_err());
    assert!(manager
        .load_instance(&databases, 999, 502)
        .await
        .unwrap()
        .is_none());
    manager.initialize(&databases).await.unwrap();
    assert_eq!(
        manager
            .create_instance(&databases, 33, 0, false)
            .await
            .unwrap(),
        503
    );
}
