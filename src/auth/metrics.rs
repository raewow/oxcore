use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct Metrics {
    pub connections_total: AtomicU64,
    pub connections_active: AtomicU64,
    pub authentications_success: AtomicU64,
    pub authentications_failed: AtomicU64,
    pub realm_list_requests: AtomicU64,
    pub patch_transfers: AtomicU64,
    pub patch_bytes_transferred: AtomicU64,
    pub ip_bans: AtomicU64,
    pub account_bans: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment_connections(&self) {
        self.connections_total.fetch_add(1, Ordering::Relaxed);
        self.connections_active.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement_connections(&self) {
        self.connections_active.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn increment_auth_success(&self) {
        self.authentications_success.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_auth_failed(&self) {
        self.authentications_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_realm_list(&self) {
        self.realm_list_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_patch_transfer(&self) {
        self.patch_transfers.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_patch_bytes(&self, bytes: u64) {
        self.patch_bytes_transferred
            .fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn increment_ip_ban(&self) {
        self.ip_bans.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_account_ban(&self) {
        self.account_bans.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            connections_total: self.connections_total.load(Ordering::Relaxed),
            connections_active: self.connections_active.load(Ordering::Relaxed),
            authentications_success: self.authentications_success.load(Ordering::Relaxed),
            authentications_failed: self.authentications_failed.load(Ordering::Relaxed),
            realm_list_requests: self.realm_list_requests.load(Ordering::Relaxed),
            patch_transfers: self.patch_transfers.load(Ordering::Relaxed),
            patch_bytes_transferred: self.patch_bytes_transferred.load(Ordering::Relaxed),
            ip_bans: self.ip_bans.load(Ordering::Relaxed),
            account_bans: self.account_bans.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub connections_total: u64,
    pub connections_active: u64,
    pub authentications_success: u64,
    pub authentications_failed: u64,
    pub realm_list_requests: u64,
    pub patch_transfers: u64,
    pub patch_bytes_transferred: u64,
    pub ip_bans: u64,
    pub account_bans: u64,
}

impl MetricsSnapshot {
    pub fn format(&self) -> String {
        format!(
            "Metrics:\n\
            \tConnections: {} total, {} active\n\
            \tAuthentications: {} success, {} failed\n\
            \tRealm list requests: {}\n\
            \tPatch transfers: {} ({} bytes)\n\
            \tBans: {} IP, {} account",
            self.connections_total,
            self.connections_active,
            self.authentications_success,
            self.authentications_failed,
            self.realm_list_requests,
            self.patch_transfers,
            self.patch_bytes_transferred,
            self.ip_bans,
            self.account_bans
        )
    }
}
