#[derive(Debug, Clone)]
pub struct WebSessionRow {
    pub token_hash: String,
    pub created_at: i64,
    pub last_seen_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone)]
pub struct WebAuditRow {
    pub id: u64,
    pub occurred_at: i64,
    pub actor_account_id: Option<u32>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WebActivityRow {
    pub action: String,
    pub target_type: String,
    pub occurred_at: i64,
}

#[derive(Debug, Clone)]
pub struct WebSupportTicketRow {
    pub id: u64,
    pub subject: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}
