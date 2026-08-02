use super::super::models::realm::*;
use anyhow::{anyhow, Context, Result};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct RealmRepository {
    pool: Arc<PgPool>,
}

impl RealmRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn find_all_active_realms(&self) -> Result<Vec<RealmRow>> {
        sqlx::query_as::<_, RealmRow>("SELECT id, name, address, \"localAddress\", \"localSubnetMask\", port, icon, realmflags, timezone, \"allowedSecurityLevel\", population, gamebuild_min, gamebuild_max, flag, realmbuilds, last_seen FROM auth.realmlist WHERE (realmflags & 1) = 0 ORDER BY name")
            .fetch_all(&*self.pool).await.context("Failed to load active realms")
    }
    pub async fn find_all_realms(&self) -> Result<Vec<RealmRow>> {
        sqlx::query_as::<_, RealmRow>("SELECT id, name, address, \"localAddress\", \"localSubnetMask\", port, icon, realmflags, timezone, \"allowedSecurityLevel\", population, gamebuild_min, gamebuild_max, flag, realmbuilds, last_seen FROM auth.realmlist ORDER BY name")
            .fetch_all(&*self.pool).await.context("Failed to load realms")
    }
    pub async fn find_by_id(&self, realm_id: u32) -> Result<Option<RealmRow>> {
        sqlx::query_as::<_, RealmRow>("SELECT id, name, address, \"localAddress\", \"localSubnetMask\", port, icon, realmflags, timezone, \"allowedSecurityLevel\", population, gamebuild_min, gamebuild_max, flag, realmbuilds, last_seen FROM auth.realmlist WHERE id = $1")
            .bind(i64::from(realm_id)).fetch_optional(&*self.pool).await.context("Failed to find realm by ID")
    }
    pub async fn find_character_count(&self, realm_id: u32, account_id: u64) -> Result<Option<u8>> {
        let value: Option<i16> = sqlx::query_scalar(
            "SELECT numchars FROM auth.realmcharacters WHERE realmid = $1 AND acctid = $2",
        )
        .bind(i64::from(realm_id))
        .bind(
            i64::try_from(account_id)
                .map_err(|_| anyhow!("account id is outside PostgreSQL range"))?,
        )
        .fetch_optional(&*self.pool)
        .await?;
        value
            .map(|value| {
                value
                    .try_into()
                    .map_err(|_| anyhow!("stored character count is outside u8 range"))
            })
            .transpose()
    }
    pub async fn find_all_character_counts(
        &self,
        account_id: u64,
    ) -> Result<Vec<RealmCharactersRow>> {
        sqlx::query_as(
            "SELECT realmid, acctid, numchars FROM auth.realmcharacters WHERE acctid = $1",
        )
        .bind(
            i64::try_from(account_id)
                .map_err(|_| anyhow!("account id is outside PostgreSQL range"))?,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to get character counts")
    }
    pub async fn find_allowed_clients(&self) -> Result<Vec<AllowedClientRow>> {
        sqlx::query_as("SELECT major_version, minor_version, bugfix_version, hotfix_version, build, os, platform, integrity_hash FROM auth.allowed_clients ORDER BY build DESC")
            .fetch_all(&*self.pool).await.context("Failed to load allowed client builds")
    }
    pub async fn update_population(&self, realm_id: u32, population: f32) -> Result<()> {
        sqlx::query("UPDATE auth.realmlist SET population = $1 WHERE id = $2")
            .bind(population)
            .bind(i64::from(realm_id))
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
    pub async fn upsert_character_count(
        &self,
        realm_id: u32,
        account_id: u64,
        num_chars: u8,
    ) -> Result<()> {
        sqlx::query("INSERT INTO auth.realmcharacters (realmid, acctid, numchars) VALUES ($1, $2, $3) ON CONFLICT (realmid, acctid) DO UPDATE SET numchars = EXCLUDED.numchars")
            .bind(i64::from(realm_id)).bind(i64::try_from(account_id).map_err(|_| anyhow!("account id is outside PostgreSQL range"))?).bind(i16::from(num_chars)).execute(&*self.pool).await?;
        Ok(())
    }
    pub async fn delete_character_count(&self, realm_id: u32, account_id: u64) -> Result<bool> {
        Ok(
            sqlx::query("DELETE FROM auth.realmcharacters WHERE realmid = $1 AND acctid = $2")
                .bind(i64::from(realm_id))
                .bind(
                    i64::try_from(account_id)
                        .map_err(|_| anyhow!("account id is outside PostgreSQL range"))?,
                )
                .execute(&*self.pool)
                .await?
                .rows_affected()
                > 0,
        )
    }
    pub async fn heartbeat(&self, realm_id: i32) -> Result<()> {
        sqlx::query("UPDATE auth.realmlist SET last_seen = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(i64::from(realm_id))
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
    pub async fn set_online(&self, realm_id: i32, name: &str, builds: &str) -> Result<u64> {
        Ok(sqlx::query("UPDATE auth.realmlist SET name = $1, realmflags = realmflags & ~2, population = 0, realmbuilds = $2, last_seen = CURRENT_TIMESTAMP WHERE id = $3").bind(name).bind(builds).bind(i64::from(realm_id)).execute(&*self.pool).await?.rows_affected())
    }
}
