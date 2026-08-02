use super::super::models::account::IpBannedRow;
use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct IpBanRepository {
    pool: Arc<MySqlPool>,
}

impl IpBanRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ========== QUERY METHODS (Read Operations) ==========

    /// Check if IP is currently banned
    pub async fn is_ip_banned(&self, ip: &str) -> Result<Option<IpBannedRow>> {
        sqlx::query_as::<_, IpBannedRow>(
            r#"SELECT `ip`, `bandate`, `unbandate`, `bannedby`, `banreason`
               FROM `ip_banned`
               WHERE `ip` = ?
               AND (`unbandate` = `bandate` OR `unbandate` > UNIX_TIMESTAMP())"#,
        )
        .bind(ip)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to check IP ban status")
    }

    /// Find all IP bans (for admin tools)
    pub async fn find_all(&self) -> Result<Vec<IpBannedRow>> {
        sqlx::query_as::<_, IpBannedRow>(
            "SELECT `ip`, `bandate`, `unbandate`, `bannedby`, `banreason` FROM `ip_banned`",
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch all IP bans")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Auto-ban IP for failed login attempts (uses ON DUPLICATE KEY UPDATE)
    pub async fn auto_ban_ip(&self, ip: &str, ban_duration_seconds: i64) -> Result<()> {
        let unban_timestamp = chrono::Utc::now().timestamp() + ban_duration_seconds;

        sqlx::query(
            r#"INSERT INTO `ip_banned` (`ip`, `bandate`, `unbandate`, `bannedby`, `banreason`)
               VALUES (?, UNIX_TIMESTAMP(), ?, '[AutoBan]', 'Too many failed login attempts')
               ON DUPLICATE KEY UPDATE `unbandate` = ?"#,
        )
        .bind(ip)
        .bind(unban_timestamp)
        .bind(unban_timestamp)
        .execute(&*self.pool)
        .await
        .context("Failed to auto-ban IP")?;

        Ok(())
    }

    /// Manually ban IP with custom reason
    pub async fn ban_ip(
        &self,
        ip: &str,
        ban_duration_seconds: i64,
        banned_by: &str,
        reason: &str,
    ) -> Result<()> {
        let ban_timestamp = chrono::Utc::now().timestamp();
        let unban_timestamp = ban_timestamp + ban_duration_seconds;

        sqlx::query(
            r#"INSERT INTO `ip_banned` (`ip`, `bandate`, `unbandate`, `bannedby`, `banreason`)
               VALUES (?, ?, ?, ?, ?)
               ON DUPLICATE KEY UPDATE `unbandate` = VALUES(`unbandate`), `banreason` = VALUES(`banreason`)"#,
        )
        .bind(ip)
        .bind(ban_timestamp)
        .bind(unban_timestamp)
        .bind(banned_by)
        .bind(reason)
        .execute(&*self.pool)
        .await
        .context("Failed to ban IP")?;

        Ok(())
    }

    /// Delete expired IP bans (cleanup)
    pub async fn delete_expired_bans(&self) -> Result<u64> {
        let result = sqlx::query(
            r#"DELETE FROM `ip_banned`
               WHERE `unbandate` <= UNIX_TIMESTAMP() AND `unbandate` <> `bandate`"#,
        )
        .execute(&*self.pool)
        .await
        .context("Failed to delete expired IP bans")?;

        Ok(result.rows_affected())
    }

    /// Unban IP (delete ban record)
    pub async fn unban_ip(&self, ip: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM `ip_banned` WHERE `ip` = ?")
            .bind(ip)
            .execute(&*self.pool)
            .await
            .context("Failed to unban IP")?;

        Ok(result.rows_affected() > 0)
    }
}
