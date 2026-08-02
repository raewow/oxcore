use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;

use crate::database::web::models::{
    WebActivityRow, WebAuditRow, WebSessionRow, WebSupportTicketRow,
};

#[derive(Clone)]
pub struct WebRepository {
    pool: Arc<PgPool>,
}

impl WebRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn create_session(&self, token_hash: &[u8], account_id: u32) -> Result<()> {
        sqlx::query("INSERT INTO web.web_sessions (token_hash, account_id, expires_at) VALUES ($1, $2, CURRENT_TIMESTAMP + INTERVAL '30 days')")
            .bind(token_hash).bind(i64::from(account_id)).execute(&*self.pool).await?;
        Ok(())
    }

    pub async fn delete_session(&self, token_hash: &[u8]) -> Result<()> {
        sqlx::query("DELETE FROM web.web_sessions WHERE token_hash = $1")
            .bind(token_hash)
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_account_sessions(&self, account_id: u32) -> Result<()> {
        sqlx::query("DELETE FROM web.web_sessions WHERE account_id = $1")
            .bind(i64::from(account_id))
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_account_session_by_hex(
        &self,
        account_id: u32,
        token_hash: &str,
    ) -> Result<u64> {
        Ok(sqlx::query(
            "DELETE FROM web.web_sessions WHERE account_id = $1 AND token_hash = decode($2, 'hex')",
        )
        .bind(i64::from(account_id))
        .bind(token_hash)
        .execute(&*self.pool)
        .await?
        .rows_affected())
    }

    pub async fn session_account(&self, token_hash: &[u8]) -> Result<Option<u32>> {
        let account_id: Option<i64> = sqlx::query_scalar("SELECT account_id FROM web.web_sessions WHERE token_hash = $1 AND expires_at > CURRENT_TIMESTAMP")
            .bind(token_hash).fetch_optional(&*self.pool).await?;
        account_id
            .map(u32::try_from)
            .transpose()
            .map_err(Into::into)
    }

    pub async fn touch_session(&self, token_hash: &[u8]) -> Result<()> {
        sqlx::query(
            "UPDATE web.web_sessions SET last_seen_at = CURRENT_TIMESTAMP WHERE token_hash = $1",
        )
        .bind(token_hash)
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    pub async fn sessions(&self, account_id: u32, active_only: bool) -> Result<Vec<WebSessionRow>> {
        let filter = if active_only {
            " AND expires_at > CURRENT_TIMESTAMP"
        } else {
            ""
        };
        let query = format!("SELECT encode(token_hash, 'hex'), EXTRACT(EPOCH FROM created_at)::BIGINT, EXTRACT(EPOCH FROM last_seen_at)::BIGINT, EXTRACT(EPOCH FROM expires_at)::BIGINT FROM web.web_sessions WHERE account_id = $1{filter} ORDER BY last_seen_at DESC");
        Ok(sqlx::query_as::<_, (String, i64, i64, i64)>(&query)
            .bind(i64::from(account_id))
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(
                |(token_hash, created_at, last_seen_at, expires_at)| WebSessionRow {
                    token_hash,
                    created_at,
                    last_seen_at,
                    expires_at,
                },
            )
            .collect())
    }

    pub async fn record_audit(
        &self,
        actor_account_id: u32,
        action: &str,
        target_type: &str,
        target_id: Option<&str>,
        reason: Option<&str>,
    ) -> Result<()> {
        sqlx::query("INSERT INTO web.web_audit_log (actor_account_id, action, target_type, target_id, reason) VALUES ($1, $2, $3, $4, $5)")
            .bind(i64::from(actor_account_id)).bind(action).bind(target_type).bind(target_id).bind(reason).execute(&*self.pool).await?;
        Ok(())
    }

    pub async fn activity(&self, account_id: u32) -> Result<Vec<WebActivityRow>> {
        Ok(sqlx::query_as::<_, (String, String, i64)>("SELECT action, target_type, EXTRACT(EPOCH FROM occurred_at)::BIGINT FROM web.web_audit_log WHERE actor_account_id = $1 ORDER BY occurred_at DESC LIMIT 50")
            .bind(i64::from(account_id)).fetch_all(&*self.pool).await?.into_iter()
            .map(|(action, target_type, occurred_at)| WebActivityRow { action, target_type, occurred_at }).collect())
    }

    pub async fn audit_log(&self, account_id: Option<u32>) -> Result<Vec<WebAuditRow>> {
        let rows = if let Some(account_id) = account_id {
            let id = account_id.to_string();
            let role_prefix = format!("{account_id}:%");
            sqlx::query_as::<_, (i64, i64, Option<i64>, String, String, Option<String>, Option<String>)>("SELECT id, EXTRACT(EPOCH FROM occurred_at)::BIGINT, actor_account_id, action, target_type, target_id, reason FROM web.web_audit_log WHERE actor_account_id = $1 OR (target_type = 'account' AND target_id = $2) OR (target_type = 'realm_role' AND target_id LIKE $3) ORDER BY occurred_at DESC LIMIT 100")
                .bind(i64::from(account_id)).bind(id).bind(role_prefix).fetch_all(&*self.pool).await?
        } else {
            sqlx::query_as::<_, (i64, i64, Option<i64>, String, String, Option<String>, Option<String>)>("SELECT id, EXTRACT(EPOCH FROM occurred_at)::BIGINT, actor_account_id, action, target_type, target_id, reason FROM web.web_audit_log ORDER BY occurred_at DESC LIMIT 200")
                .fetch_all(&*self.pool).await?
        };
        rows.into_iter()
            .map(
                |(id, occurred_at, actor_account_id, action, target_type, target_id, reason)| {
                    Ok(WebAuditRow {
                        id: u64::try_from(id)?,
                        occurred_at,
                        actor_account_id: actor_account_id.map(u32::try_from).transpose()?,
                        action,
                        target_type,
                        target_id,
                        reason,
                    })
                },
            )
            .collect()
    }

    pub async fn create_support_ticket(
        &self,
        account_id: u32,
        subject: &str,
        message: &str,
    ) -> Result<()> {
        sqlx::query("INSERT INTO web.web_support_tickets (account_id, subject, message) VALUES ($1, $2, $3)")
            .bind(i64::from(account_id)).bind(subject).bind(message).execute(&*self.pool).await?;
        Ok(())
    }

    pub async fn support_tickets(&self, account_id: u32) -> Result<Vec<WebSupportTicketRow>> {
        let rows = sqlx::query_as::<_, (i64, String, String, i64, i64)>("SELECT id, subject, status, EXTRACT(EPOCH FROM created_at)::BIGINT, EXTRACT(EPOCH FROM updated_at)::BIGINT FROM web.web_support_tickets WHERE account_id = $1 ORDER BY updated_at DESC")
            .bind(i64::from(account_id)).fetch_all(&*self.pool).await?;
        rows.into_iter()
            .map(|(id, subject, status, created_at, updated_at)| {
                Ok(WebSupportTicketRow {
                    id: u64::try_from(id)?,
                    subject,
                    status,
                    created_at,
                    updated_at,
                })
            })
            .collect()
    }

    pub async fn open_support_ticket_count(&self) -> Result<u64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM web.web_support_tickets WHERE status IN ('open', 'awaiting_player')").fetch_one(&*self.pool).await?;
        Ok(u64::try_from(count)?)
    }
}
