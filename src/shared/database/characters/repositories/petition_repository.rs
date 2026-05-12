use super::super::models::petition::*;
use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;

pub struct PetitionRepository {
    pool: Arc<MySqlPool>,
}

impl PetitionRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ========== QUERY METHODS (Read Operations) ==========

    /// Get the maximum petition GUID from the database (for generating next ID).
    pub async fn get_max_petition_guid(&self) -> Result<Option<u32>> {
        sqlx::query_scalar::<_, Option<u32>>("SELECT MAX(petition_guid) FROM petition")
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query max petition_guid")
    }

    /// Find a petition by charter GUID.
    pub async fn find_by_charter_guid(&self, charter_guid: u32) -> Result<Option<PetitionRow>> {
        sqlx::query_as::<_, PetitionRow>(
            r#"SELECT owner_guid, petition_guid, charter_guid, name FROM petition WHERE charter_guid = ?"#,
        )
        .bind(charter_guid)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch petition by charter GUID")
    }

    /// Find a petition by owner GUID.
    pub async fn find_by_owner_guid(&self, owner_guid: u32) -> Result<Option<PetitionRow>> {
        sqlx::query_as::<_, PetitionRow>(
            r#"SELECT owner_guid, petition_guid, charter_guid, name FROM petition WHERE owner_guid = ?"#,
        )
        .bind(owner_guid)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch petition by owner GUID")
    }

    /// Find all signatures for a petition.
    pub async fn find_signatures(&self, petition_guid: u32) -> Result<Vec<PetitionSignatureRow>> {
        sqlx::query_as::<_, PetitionSignatureRow>(
            r#"SELECT owner_guid, petition_guid, player_guid, player_account FROM petition_sign WHERE petition_guid = ?"#,
        )
        .bind(petition_guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch petition signatures")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Create a new petition.
    pub async fn create_petition(&self, petition: &PetitionRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO petition (owner_guid, petition_guid, charter_guid, name) VALUES (?, ?, ?, ?)"#,
        )
        .bind(petition.owner_guid)
        .bind(petition.petition_guid)
        .bind(petition.charter_guid)
        .bind(&petition.name)
        .execute(&*self.pool)
        .await
        .context("Failed to create petition")?;

        Ok(())
    }

    /// Add a signature to a petition.
    pub async fn add_signature(&self, signature: &PetitionSignatureRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO petition_sign (owner_guid, petition_guid, player_guid, player_account) VALUES (?, ?, ?, ?)"#,
        )
        .bind(signature.owner_guid)
        .bind(signature.petition_guid)
        .bind(signature.player_guid)
        .bind(signature.player_account)
        .execute(&*self.pool)
        .await
        .context("Failed to add petition signature")?;

        Ok(())
    }

    /// Update petition name.
    pub async fn update_petition_name(&self, charter_guid: u32, new_name: &str) -> Result<()> {
        sqlx::query(r#"UPDATE petition SET name = ? WHERE charter_guid = ?"#)
            .bind(new_name)
            .bind(charter_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update petition name")?;

        Ok(())
    }

    /// Delete a petition and all its signatures (transactional).
    pub async fn delete_petition(&self, charter_guid: u32, petition_guid: u32) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete signatures first
        sqlx::query("DELETE FROM petition_sign WHERE petition_guid = ?")
            .bind(petition_guid)
            .execute(&mut *tx)
            .await
            .context("Failed to delete petition signatures")?;

        // Delete petition
        sqlx::query("DELETE FROM petition WHERE charter_guid = ?")
            .bind(charter_guid)
            .execute(&mut *tx)
            .await
            .context("Failed to delete petition")?;

        tx.commit()
            .await
            .context("Failed to commit petition deletion")?;
        Ok(())
    }

    /// Delete all petitions owned by a player.
    pub async fn delete_player_petitions(&self, owner_guid: u32) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Get all petitions owned by player
        let petitions = sqlx::query_as::<_, PetitionRow>(
            r#"SELECT owner_guid, petition_guid, charter_guid, name FROM petition WHERE owner_guid = ?"#,
        )
        .bind(owner_guid)
        .fetch_all(&mut *tx)
        .await
        .context("Failed to fetch player petitions")?;

        // Delete signatures for each petition
        for petition in &petitions {
            sqlx::query("DELETE FROM petition_sign WHERE petition_guid = ?")
                .bind(petition.petition_guid)
                .execute(&mut *tx)
                .await
                .context("Failed to delete petition signatures")?;
        }

        // Delete all player's petitions
        sqlx::query("DELETE FROM petition WHERE owner_guid = ?")
            .bind(owner_guid)
            .execute(&mut *tx)
            .await
            .context("Failed to delete player petitions")?;

        tx.commit()
            .await
            .context("Failed to commit player petitions deletion")?;
        Ok(())
    }

    /// Delete all signatures for a specific player.
    pub async fn delete_player_signatures(&self, player_guid: u32) -> Result<()> {
        sqlx::query("DELETE FROM petition_sign WHERE player_guid = ?")
            .bind(player_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete player signatures")?;

        Ok(())
    }
}
