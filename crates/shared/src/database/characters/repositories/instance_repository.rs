use super::super::models::instance::*;
use anyhow::{Context, Result};
use sqlx::{FromRow, MySqlPool};
use std::sync::Arc;

/// Helper struct for expired instance queries
#[derive(FromRow, Debug)]
struct ExpiredInstance {
    id: u32,
    map: u32,
}

pub struct InstanceRepository {
    pool: Arc<MySqlPool>,
}

impl InstanceRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ========== QUERY METHODS (Read Operations) ==========

    /// Get the maximum instance ID for a specific map.
    pub async fn get_max_instance_id_by_map(&self, map_id: u32) -> Result<Option<u32>> {
        sqlx::query_scalar::<_, Option<u32>>(r#"SELECT MAX(id) FROM instance WHERE map = ?"#)
            .bind(map_id)
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query max instance ID by map")
    }

    /// Find an instance by ID and map.
    pub async fn find_by_id_and_map(
        &self,
        instance_id: u32,
        map_id: u32,
    ) -> Result<Option<InstanceRow>> {
        sqlx::query_as::<_, InstanceRow>(
            r#"SELECT id, map, reset_time, data FROM instance WHERE id = ? AND map = ?"#,
        )
        .bind(instance_id)
        .bind(map_id)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch instance by ID and map")
    }

    /// Find all instances.
    pub async fn find_all(&self) -> Result<Vec<InstanceRow>> {
        sqlx::query_as::<_, InstanceRow>(r#"SELECT id, map, reset_time, data FROM instance"#)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch all instances")
    }

    /// Find all character instances for a specific player.
    pub async fn find_character_instances(&self, guid: u32) -> Result<Vec<CharacterInstanceRow>> {
        sqlx::query_as::<_, CharacterInstanceRow>(
            r#"SELECT guid, instance, permanent, extend FROM character_instance WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch character instances")
    }

    /// Find all group instances for a specific group leader.
    pub async fn find_group_instances(&self, leader_guid: u32) -> Result<Vec<GroupInstanceRow>> {
        sqlx::query_as::<_, GroupInstanceRow>(
            r#"SELECT leader_guid, instance, permanent FROM group_instance WHERE leader_guid = ?"#,
        )
        .bind(leader_guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch group instances")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Create a new instance.
    pub async fn create_instance(&self, instance: &InstanceRow) -> Result<()> {
        sqlx::query(r#"INSERT INTO instance (id, map, reset_time, data) VALUES (?, ?, ?, ?)"#)
            .bind(instance.id)
            .bind(instance.map)
            .bind(instance.reset_time)
            .bind(&instance.data)
            .execute(&*self.pool)
            .await
            .context("Failed to create instance")?;

        Ok(())
    }

    /// Update instance data.
    pub async fn update_instance(&self, instance: &InstanceRow) -> Result<()> {
        sqlx::query(r#"UPDATE instance SET map = ?, reset_time = ?, data = ? WHERE id = ?"#)
            .bind(instance.map)
            .bind(instance.reset_time)
            .bind(&instance.data)
            .bind(instance.id)
            .execute(&*self.pool)
            .await
            .context("Failed to update instance")?;

        Ok(())
    }

    /// Delete an instance.
    pub async fn delete_instance(&self, instance_id: u32, map_id: u32) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete character and group bindings first
        sqlx::query(r#"DELETE FROM character_instance WHERE instance = ?"#)
            .bind(instance_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete character instance bindings")?;

        sqlx::query(r#"DELETE FROM group_instance WHERE instance = ?"#)
            .bind(instance_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete group instance bindings")?;

        // Delete instance
        sqlx::query(r#"DELETE FROM instance WHERE id = ? AND map = ?"#)
            .bind(instance_id)
            .bind(map_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete instance")?;

        tx.commit()
            .await
            .context("Failed to commit instance deletion")?;
        Ok(())
    }

    /// Create a character instance binding.
    pub async fn create_character_instance(
        &self,
        character_instance: &CharacterInstanceRow,
    ) -> Result<()> {
        sqlx::query(r#"REPLACE INTO character_instance (guid, instance, permanent, extend) VALUES (?, ?, ?, ?)"#)
            .bind(character_instance.guid)
            .bind(character_instance.instance)
            .bind(character_instance.permanent)
            .bind(character_instance.extend)
            .execute(&*self.pool)
            .await
            .context("Failed to create character instance")?;

        Ok(())
    }

    /// Create a group instance binding.
    pub async fn create_group_instance(&self, group_instance: &GroupInstanceRow) -> Result<()> {
        sqlx::query(
            r#"REPLACE INTO group_instance (leader_guid, instance, permanent) VALUES (?, ?, ?)"#,
        )
        .bind(group_instance.leader_guid)
        .bind(group_instance.instance)
        .bind(group_instance.permanent)
        .execute(&*self.pool)
        .await
        .context("Failed to create group instance")?;

        Ok(())
    }

    /// Delete all instances that have expired.
    pub async fn delete_expired_instances(&self) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let instances = sqlx::query_as::<_, ExpiredInstance>(
            r#"SELECT id, map FROM instance WHERE reset_time <= ?"#,
        )
        .bind(now as i64)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to find expired instances")?;

        for instance in instances {
            self.delete_instance(instance.id, instance.map).await?;
        }

        Ok(())
    }
}
