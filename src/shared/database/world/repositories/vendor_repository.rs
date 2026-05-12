//! Vendor repository for database access
//!
//! Handles loading of vendor items from npc_vendor and npc_vendor_template tables.

use anyhow::{Context, Result};
use sqlx::{MySqlPool, Row};
use std::sync::Arc;

/// Row structure for npc_vendor table (per-creature vendor items)
pub struct VendorItemRow {
    pub entry: u32,
    pub item_entry: u32,
    pub max_count: u8,
    pub incr_time: u32,
    pub itemflags: i64,
    pub condition_id: i64,
}

/// Row structure for npc_vendor_template table (shared vendor items)
pub struct VendorTemplateItemRow {
    pub entry: u32,
    pub item_entry: u32,
    pub max_count: u8,
    pub incr_time: u32,
    pub itemflags: i64,
    pub condition_id: i64,
}

/// Repository for vendor-related database operations
pub struct VendorRepository {
    pool: Arc<MySqlPool>,
}

impl VendorRepository {
    /// Create a new vendor repository
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    /// Load all vendor data from database
    pub async fn load_all(&self) -> Result<VendorLoadData> {
        let vendor_items = self.load_vendor_items().await?;
        let template_items = self.load_vendor_template_items().await?;

        tracing::info!(
            "Loaded {} vendor items, {} template items",
            vendor_items.len(),
            template_items.len()
        );

        Ok(VendorLoadData {
            vendor_items,
            template_items,
        })
    }

    /// Load vendor items from npc_vendor table
    async fn load_vendor_items(&self) -> Result<Vec<VendorItemRow>> {
        let query_str = r#"
            SELECT entry, item, maxcount, incrtime,
                   COALESCE(condition_id, 0) AS condition_id,
                   COALESCE(itemflags, 0) AS itemflags
            FROM npc_vendor
            ORDER BY entry, item
        "#;

        let rows = sqlx::query(query_str)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to load npc_vendor")?;

        let mut items = Vec::new();
        for row in rows {
            items.push(VendorItemRow {
                entry: row.get("entry"),
                item_entry: row.get("item"),
                max_count: row.get("maxcount"),
                incr_time: row.get("incrtime"),
                itemflags: row.get("itemflags"),
                condition_id: row.get("condition_id"),
            });
        }

        Ok(items)
    }

    /// Load vendor template items from npc_vendor_template table
    async fn load_vendor_template_items(&self) -> Result<Vec<VendorTemplateItemRow>> {
        let query_str = r#"
            SELECT entry, item, maxcount, incrtime,
                   COALESCE(condition_id, 0) AS condition_id,
                   COALESCE(itemflags, 0) AS itemflags
            FROM npc_vendor_template
            ORDER BY entry, item
        "#;

        let rows = sqlx::query(query_str)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to load npc_vendor_template")?;

        let mut items = Vec::new();
        for row in rows {
            items.push(VendorTemplateItemRow {
                entry: row.get("entry"),
                item_entry: row.get("item"),
                max_count: row.get("maxcount"),
                incr_time: row.get("incrtime"),
                itemflags: row.get("itemflags"),
                condition_id: row.get("condition_id"),
            });
        }

        Ok(items)
    }
}

/// All vendor data loaded from database
pub struct VendorLoadData {
    pub vendor_items: Vec<VendorItemRow>,
    pub template_items: Vec<VendorTemplateItemRow>,
}
