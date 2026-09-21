use super::*;

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn restart_restores_the_saved_raid_and_encounter_progress_on_entry(pool: PgPool) {
    let mut world = entrance_world(pool, 409, 2).await;
    personal(&world, 4, 409, 101, true).await;
    personal(&world, 4, 36, 301, false).await;
    let manager = &world.managers.instance_mgr;
    manager
        .load_instance(&world.databases, 409, 101)
        .await
        .unwrap();
    manager.mark_encounter_completed(409, 101, 7);
    manager
        .save_instance_data(&world.databases, 409, 101)
        .await
        .unwrap();

    // Discard every instance/save cache while retaining the database, as on restart.
    world.managers.instance_mgr = Arc::new(InstanceMgr::new());
    let manager = &world.managers.instance_mgr;
    manager.initialize(&world.databases).await.unwrap();
    assert!(manager.get_player_binding(guid(4), 409).is_none());
    let (solo, _packets) = session(&world, 4, 0);
    assert_eq!(enter(&world, &solo).await, 101);
    assert_eq!(
        saved(manager.get_player_binding(guid(4), 409)),
        Some((101, true))
    );
    assert_eq!(
        saved(manager.get_player_binding(guid(4), 36)),
        Some((301, false))
    );
    assert!(manager.is_encounter_completed(409, 101, 7));
    let copies: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM characters.instance WHERE map = 409")
            .fetch_one(&world.databases.character)
            .await
            .unwrap();
    assert_eq!(
        copies, 2,
        "entry must reuse the saved copy, not create a third raid"
    );
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn cold_personal_save_takes_precedence_over_an_existing_group_save(pool: PgPool) {
    let world = entrance_world(pool, 36, 1).await;
    world.databases.character.execute(
        "INSERT INTO characters.character_instance (guid, instance, permanent) VALUES (2, 301, 0);
         INSERT INTO characters.group_instance (leader_guid, instance, permanent) VALUES (1, 302, 0)"
    ).await.unwrap();
    let (member, _packets) = session(&world, 2, 0);
    assert_eq!(enter(&world, &member).await, 301);
    let manager = &world.managers.instance_mgr;
    assert_eq!(
        saved(manager.get_player_binding(guid(2), 36)),
        Some((301, false))
    );
    assert_eq!(
        saved(manager.get_group_binding(guid(1), 36)),
        Some((302, false))
    );
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn writing_one_maps_binding_does_not_hide_a_persisted_raid_save(pool: PgPool) {
    let world = entrance_world(pool, 409, 2).await;
    world.databases.character.execute(
        "INSERT INTO characters.character_instance (guid, instance, permanent) VALUES (4, 101, 1)"
    ).await.unwrap();
    // Simulate a binding written before this character's full save list was loaded.
    personal(&world, 4, 36, 301, false).await;
    let manager = &world.managers.instance_mgr;
    assert!(manager.get_player_binding(guid(4), 409).is_none());
    let (solo, _packets) = session(&world, 4, 0);
    assert_eq!(enter(&world, &solo).await, 101);
    assert_eq!(
        saved(manager.get_player_binding(guid(4), 409)),
        Some((101, true))
    );
    assert_eq!(
        saved(manager.get_player_binding(guid(4), 36)),
        Some((301, false))
    );
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn failed_personal_binding_load_aborts_without_creating_an_unsaved_copy(pool: PgPool) {
    let world = entrance_world(pool, 36, 1).await;
    world.databases.character.execute(
        "INSERT INTO characters.character_instance (guid, instance, permanent) VALUES (4, 301, 0);
         ALTER TABLE characters.character_instance RENAME TO unavailable_character_instance"
    ).await.unwrap();
    let (solo, mut packets) = session(&world, 4, 0);
    assert_entry_rejected(&world, &solo, &mut packets).await;
    assert!(world
        .managers
        .instance_mgr
        .get_player_binding(guid(4), 36)
        .is_none());
    let created: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM characters.instance WHERE id > 501")
            .fetch_one(&world.databases.character)
            .await
            .unwrap();
    assert_eq!(created, 0);

    sqlx::query(
        "ALTER TABLE characters.unavailable_character_instance RENAME TO character_instance",
    )
    .execute(&world.databases.character)
    .await
    .unwrap();
    assert_eq!(enter(&world, &solo).await, 301);
}

#[sqlx::test]
#[ignore = "requires a disposable PostgreSQL server via DATABASE_URL"]
async fn restored_expired_personal_save_does_not_override_a_valid_group_copy(pool: PgPool) {
    let world = entrance_world(pool, 36, 1).await;
    world.databases.character.execute(
        "UPDATE characters.instance SET reset_time = 1 WHERE id = 301;
         INSERT INTO characters.character_instance (guid, instance, permanent) VALUES (2, 301, 0);
         INSERT INTO characters.group_instance (leader_guid, instance, permanent) VALUES (1, 302, 0)"
    ).await.unwrap();
    let (member, _packets) = session(&world, 2, 0);
    assert_eq!(enter(&world, &member).await, 302);
    assert_eq!(
        saved(world.managers.instance_mgr.get_group_binding(guid(1), 36)),
        Some((302, false))
    );
}
