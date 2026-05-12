use super::super::models::realm::*;
use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct RealmRepository {
    pool: Arc<MySqlPool>,
}

impl RealmRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ========== QUERY METHODS (Read Operations) ==========

    /// Find all active realms (excluding offline/invalid realms)
    /// Filters out realms with realmflags & 1 set (offline flag)
    pub async fn find_all_active_realms(&self) -> Result<Vec<RealmRow>> {
        sqlx::query_as::<_, RealmRow>(
            r#"SELECT `id`, `name`, `address`, `localAddress`, `localSubnetMask`, `port`, `icon`,
               `realmflags`, `timezone`, `allowedSecurityLevel`, `population`, `gamebuild_min`,
               `gamebuild_max`, `flag`, `realmbuilds`, `last_seen`
               FROM `realmlist`
               WHERE (`realmflags` & 1) = 0
               ORDER BY `name`"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load active realms")
    }

    /// Find all realms (including offline)
    pub async fn find_all_realms(&self) -> Result<Vec<RealmRow>> {
        sqlx::query_as::<_, RealmRow>(
            r#"SELECT `id`, `name`, `address`, `localAddress`, `localSubnetMask`, `port`, `icon`,
               `realmflags`, `timezone`, `allowedSecurityLevel`, `population`, `gamebuild_min`,
               `gamebuild_max`, `flag`, `realmbuilds`
               FROM `realmlist`
               ORDER BY `name`"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load all realms")
    }

    /// Find realm by ID
    pub async fn find_by_id(&self, realm_id: u32) -> Result<Option<RealmRow>> {
        sqlx::query_as::<_, RealmRow>(
            r#"SELECT `id`, `name`, `address`, `localAddress`, `localSubnetMask`, `port`, `icon`,
               `realmflags`, `timezone`, `allowedSecurityLevel`, `population`, `gamebuild_min`,
               `gamebuild_max`, `flag`, `realmbuilds`
               FROM `realmlist`
               WHERE `id` = ?"#,
        )
        .bind(realm_id)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to find realm by ID")
    }

    /// Get character count for an account on a specific realm
    pub async fn find_character_count(&self, realm_id: u32, account_id: u64) -> Result<Option<u8>> {
        sqlx::query_scalar::<_, u8>(
            "SELECT `numchars` FROM `realmcharacters` WHERE `realmid` = ? AND `acctid` = ?",
        )
        .bind(realm_id)
        .bind(account_id)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to get character count for realm")
    }

    /// Get all character counts for an account across all realms
    pub async fn find_all_character_counts(
        &self,
        account_id: u64,
    ) -> Result<Vec<RealmCharactersRow>> {
        sqlx::query_as::<_, RealmCharactersRow>(
            "SELECT `realmid`, `acctid`, `numchars` FROM `realmcharacters` WHERE `acctid` = ?",
        )
        .bind(account_id)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to get character counts for account")
    }

    /// Find all allowed client builds
    pub async fn find_allowed_clients(&self) -> Result<Vec<AllowedClientRow>> {
        sqlx::query_as::<_, AllowedClientRow>(
            r#"SELECT `major_version`, `minor_version`, `bugfix_version`, `hotfix_version`,
               `build`, `os`, `platform`, `integrity_hash`
               FROM `allowed_clients`
               ORDER BY `build` DESC"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load allowed client builds")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Update realm population
    pub async fn update_population(&self, realm_id: u32, population: f32) -> Result<()> {
        sqlx::query("UPDATE `realmlist` SET `population` = ? WHERE `id` = ?")
            .bind(population)
            .bind(realm_id)
            .execute(&*self.pool)
            .await
            .context("Failed to update realm population")?;
        Ok(())
    }

    /// Update character count for account on realm (upsert)
    pub async fn upsert_character_count(
        &self,
        realm_id: u32,
        account_id: u64,
        num_chars: u8,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO `realmcharacters` (`realmid`, `acctid`, `numchars`)
               VALUES (?, ?, ?)
               ON DUPLICATE KEY UPDATE `numchars` = ?"#,
        )
        .bind(realm_id)
        .bind(account_id)
        .bind(num_chars)
        .bind(num_chars)
        .execute(&*self.pool)
        .await
        .context("Failed to upsert character count")?;
        Ok(())
    }

    /// Delete character count record (when no characters remain)
    pub async fn delete_character_count(&self, realm_id: u32, account_id: u64) -> Result<bool> {
        let result =
            sqlx::query("DELETE FROM `realmcharacters` WHERE `realmid` = ? AND `acctid` = ?")
                .bind(realm_id)
                .bind(account_id)
                .execute(&*self.pool)
                .await
                .context("Failed to delete character count")?;

        Ok(result.rows_affected() > 0)
    }
}
