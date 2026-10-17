//! Persisted respawn deadlines for database spawns.
//!
//! A killed creature or a looted/used gameobject stays unavailable until its
//! respawn delay elapses, in its own map copy only. Keeping that deadline in
//! memory is not enough: grid unloads, dungeon copy teardown and restarts all
//! drop the runtime object. The deadline is written to `creature_respawn` /
//! `gameobject_respawn` when it starts, removed when the object returns, and
//! loaded back into the spawn tables at startup so a reload honours it.

use crate::World;
use anyhow::{Context, Result};
use sqlx::PgPool;

fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Run a write without blocking the caller. Respawn bookkeeping happens inside
/// the world tick, which must not wait on the database.
fn spawn_write<F>(what: &'static str, write: F)
where
    F: std::future::Future<Output = Result<(), sqlx::Error>> + Send + 'static,
{
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    handle.spawn(async move {
        if let Err(e) = write.await {
            tracing::warn!("Failed to {}: {}", what, e);
        }
    });
}

/// Record when a killed creature's spawn may return (`respawn_at_ms` is unix ms).
pub fn save_creature_respawn(
    world: &World,
    spawn_id: u32,
    map_id: u32,
    instance_id: u32,
    respawn_at_ms: u64,
) {
    if spawn_id == 0 {
        return;
    }
    let pool = world.databases.character.clone();
    let respawn_at = (respawn_at_ms / 1000) as i64;
    spawn_write("save creature respawn time", async move {
        sqlx::query(
            "INSERT INTO characters.creature_respawn (guid, respawn_time, instance, map)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (guid, instance) DO UPDATE SET respawn_time = EXCLUDED.respawn_time",
        )
        .bind(i64::from(spawn_id))
        .bind(respawn_at)
        .bind(i64::from(instance_id))
        .bind(i64::from(map_id))
        .execute(&pool)
        .await
        .map(|_| ())
    });
}

/// Forget a creature spawn's deadline once it has respawned.
pub fn clear_creature_respawn(world: &World, spawn_id: u32, instance_id: u32) {
    if spawn_id == 0 {
        return;
    }
    let pool = world.databases.character.clone();
    spawn_write("clear creature respawn time", async move {
        sqlx::query("DELETE FROM characters.creature_respawn WHERE guid = $1 AND instance = $2")
            .bind(i64::from(spawn_id))
            .bind(i64::from(instance_id))
            .execute(&pool)
            .await
            .map(|_| ())
    });
}

/// Record when a despawned gameobject's spawn may return (`respawn_at_ms` is unix ms).
pub fn save_gameobject_respawn(
    world: &World,
    spawn_id: u32,
    map_id: u32,
    instance_id: u32,
    respawn_at_ms: u64,
) {
    if spawn_id == 0 {
        return;
    }
    let pool = world.databases.character.clone();
    let respawn_at = (respawn_at_ms / 1000) as i64;
    spawn_write("save gameobject respawn time", async move {
        sqlx::query(
            "INSERT INTO characters.gameobject_respawn (guid, respawn_time, instance, map)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (guid, instance) DO UPDATE SET respawn_time = EXCLUDED.respawn_time",
        )
        .bind(i64::from(spawn_id))
        .bind(respawn_at)
        .bind(i64::from(instance_id))
        .bind(i64::from(map_id))
        .execute(&pool)
        .await
        .map(|_| ())
    });
}

/// Forget a gameobject spawn's deadline once it has respawned.
pub fn clear_gameobject_respawn(world: &World, spawn_id: u32, instance_id: u32) {
    if spawn_id == 0 {
        return;
    }
    let pool = world.databases.character.clone();
    spawn_write("clear gameobject respawn time", async move {
        sqlx::query("DELETE FROM characters.gameobject_respawn WHERE guid = $1 AND instance = $2")
            .bind(i64::from(spawn_id))
            .bind(i64::from(instance_id))
            .execute(&pool)
            .await
            .map(|_| ())
    });
}

/// Drop every respawn deadline of a dungeon copy whose save is being reset.
pub async fn delete_instance_respawns(pool: &PgPool, instance_id: u32) -> Result<()> {
    if instance_id == 0 {
        return Ok(());
    }
    sqlx::query("DELETE FROM characters.creature_respawn WHERE instance = $1")
        .bind(i64::from(instance_id))
        .execute(pool)
        .await
        .context("Failed to delete creature respawn times")?;
    sqlx::query("DELETE FROM characters.gameobject_respawn WHERE instance = $1")
        .bind(i64::from(instance_id))
        .execute(pool)
        .await
        .context("Failed to delete gameobject respawn times")?;
    Ok(())
}

/// Load unexpired respawn deadlines into the spawn tables and discard the rest.
///
/// Run after instance cleanup at startup, so rows of copies that no longer
/// exist have already been removed.
pub async fn load_respawn_times(world: &World) -> Result<()> {
    let pool = &world.databases.character;
    let now = now_unix_secs() as i64;

    sqlx::query("DELETE FROM characters.creature_respawn WHERE respawn_time <= $1")
        .bind(now)
        .execute(pool)
        .await
        .context("Failed to purge expired creature respawn times")?;
    sqlx::query("DELETE FROM characters.gameobject_respawn WHERE respawn_time <= $1")
        .bind(now)
        .execute(pool)
        .await
        .context("Failed to purge expired gameobject respawn times")?;

    let creatures = sqlx::query_as::<_, (i64, i64, i64, Option<i64>)>(
        "SELECT guid, respawn_time, instance, map FROM characters.creature_respawn",
    )
    .fetch_all(pool)
    .await
    .context("Failed to load creature respawn times")?;
    let creature_count = creatures.len();
    for (spawn_id, respawn_at, instance_id, map_id) in creatures {
        let map_id = match map_id {
            Some(map_id) => u32::try_from(map_id)?,
            None => match world
                .managers
                .creature_mgr
                .get_spawn_data_by_id(u32::try_from(spawn_id)?)
            {
                Some(spawn) => spawn.map_id,
                None => continue,
            },
        };
        world.managers.creature_mgr.restore_respawn_deadline(
            map_id,
            u32::try_from(instance_id)?,
            u32::try_from(spawn_id)?,
            u64::try_from(respawn_at)? * 1000,
        );
    }

    let gameobjects = sqlx::query_as::<_, (i64, i64, i64, Option<i64>)>(
        "SELECT guid, respawn_time, instance, map FROM characters.gameobject_respawn",
    )
    .fetch_all(pool)
    .await
    .context("Failed to load gameobject respawn times")?;
    let gameobject_count = gameobjects.len();
    for (spawn_id, respawn_at, instance_id, map_id) in gameobjects {
        let map_id = match map_id {
            Some(map_id) => u32::try_from(map_id)?,
            None => match world
                .managers
                .gameobject_mgr
                .get_spawn_data_by_id(u32::try_from(spawn_id)?)
            {
                Some(spawn) => spawn.map_id,
                None => continue,
            },
        };
        world.managers.gameobject_mgr.restore_respawn_deadline(
            map_id,
            u32::try_from(instance_id)?,
            u32::try_from(spawn_id)?,
            u64::try_from(respawn_at)? * 1000,
        );
    }

    tracing::info!(
        "Restored {} creature and {} gameobject respawn times",
        creature_count,
        gameobject_count
    );
    Ok(())
}
