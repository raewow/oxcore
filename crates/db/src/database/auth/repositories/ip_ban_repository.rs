use super::super::models::account::IpBannedRow;
use anyhow::{Context, Result};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct IpBanRepository {
    pool: Arc<PgPool>,
}
impl IpBanRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    pub async fn is_ip_banned(&self, ip: &str) -> Result<Option<IpBannedRow>> {
        sqlx::query_as("SELECT ip, bandate, unbandate, bannedby, banreason FROM auth.ip_banned WHERE ip = $1 AND (unbandate = bandate OR unbandate > EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT)").bind(ip).fetch_optional(&*self.pool).await.context("Failed to check IP ban")
    }
    pub async fn find_all(&self) -> Result<Vec<IpBannedRow>> {
        sqlx::query_as("SELECT ip, bandate, unbandate, bannedby, banreason FROM auth.ip_banned")
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch IP bans")
    }
    pub async fn auto_ban_ip(&self, ip: &str, duration: i64) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO auth.ip_banned (ip, bandate, unbandate, bannedby, banreason) VALUES ($1, $2, $3, '[AutoBan]', 'Too many failed login attempts') ON CONFLICT (ip) DO UPDATE SET unbandate = EXCLUDED.unbandate").bind(ip).bind(now).bind(now + duration).execute(&*self.pool).await?;
        Ok(())
    }
    pub async fn ban_ip(
        &self,
        ip: &str,
        duration: i64,
        banned_by: &str,
        reason: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO auth.ip_banned (ip, bandate, unbandate, bannedby, banreason) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (ip) DO UPDATE SET unbandate = EXCLUDED.unbandate, banreason = EXCLUDED.banreason").bind(ip).bind(now).bind(now + duration).bind(banned_by).bind(reason).execute(&*self.pool).await?;
        Ok(())
    }
    pub async fn delete_expired_bans(&self) -> Result<u64> {
        Ok(sqlx::query("DELETE FROM auth.ip_banned WHERE unbandate <= EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT AND unbandate <> bandate").execute(&*self.pool).await?.rows_affected())
    }
    pub async fn unban_ip(&self, ip: &str) -> Result<bool> {
        Ok(sqlx::query("DELETE FROM auth.ip_banned WHERE ip = $1")
            .bind(ip)
            .execute(&*self.pool)
            .await?
            .rows_affected()
            > 0)
    }
}
