use super::super::models::battleground::*;
use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;

pub struct BattlegroundRepository {
    pool: Arc<MySqlPool>,
}

impl BattlegroundRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ========== QUERY METHODS (Read Operations) ==========

    /// Get the maximum battleground instance ID from the database.
    pub async fn get_max_instance_id(&self) -> Result<Option<u32>> {
        sqlx::query_scalar::<_, Option<u32>>(
            r#"SELECT MAX(instance_id) FROM character_battleground_data"#,
        )
        .fetch_one(&*self.pool)
        .await
        .context("Failed to query max battleground instance ID")
    }

    /// Find battleground data for a specific character.
    pub async fn find_by_guid(&self, guid: u32) -> Result<Option<CharacterBattlegroundDataRow>> {
        sqlx::query_as::<_, CharacterBattlegroundDataRow>(
            r#"SELECT guid, instance_id, team, join_x, join_y, join_z, join_o, join_map FROM character_battleground_data WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch battleground data by GUID")
    }

    /// Find all battleground data for a specific instance.
    pub async fn find_by_instance_id(
        &self,
        instance_id: u32,
    ) -> Result<Vec<CharacterBattlegroundDataRow>> {
        sqlx::query_as::<_, CharacterBattlegroundDataRow>(
            r#"SELECT guid, instance_id, team, join_x, join_y, join_z, join_o, join_map FROM character_battleground_data WHERE instance_id = ?"#,
        )
        .bind(instance_id)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch battleground data by instance ID")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Save or update character battleground data.
    pub async fn save_player_data(&self, data: &CharacterBattlegroundDataRow) -> Result<()> {
        sqlx::query(
            r#"REPLACE INTO character_battleground_data (guid, instance_id, team, join_x, join_y, join_z, join_o, join_map) 
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(data.guid)
        .bind(data.instance_id)
        .bind(data.team)
        .bind(data.join_x)
        .bind(data.join_y)
        .bind(data.join_z)
        .bind(data.join_o)
        .bind(data.join_map)
        .execute(&*self.pool)
        .await
        .context("Failed to save character battleground data")?;

        Ok(())
    }

    /// Delete battleground data for a specific character.
    pub async fn delete_player_data(&self, guid: u32) -> Result<()> {
        sqlx::query(r#"DELETE FROM character_battleground_data WHERE guid = ?"#)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete character battleground data")?;

        Ok(())
    }

    /// Delete all battleground data for a specific instance.
    pub async fn delete_instance_data(&self, instance_id: u32) -> Result<()> {
        sqlx::query(r#"DELETE FROM character_battleground_data WHERE instance_id = ?"#)
            .bind(instance_id)
            .execute(&*self.pool)
            .await
            .context("Failed to delete battleground instance data")?;

        Ok(())
    }
}
