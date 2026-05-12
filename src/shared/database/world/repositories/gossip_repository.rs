//! Gossip repository for database access
//!
//! Handles loading of gossip menus, menu items, NPC texts, and broadcast texts.

use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;

/// Row structure for gossip_menu table
#[derive(sqlx::FromRow)]
pub struct GossipMenuRow {
    pub entry: u32,
    pub text_id: u32,
    pub condition_id: u32,
}

/// Row structure for gossip_menu_option table
#[derive(sqlx::FromRow)]
pub struct GossipMenuItemRow {
    pub menu_id: u32,
    pub id: u32,
    pub option_icon: u8,
    pub option_text: Option<String>,
    pub option_broadcast_text: u32,
    pub option_id: u32,
    pub npc_option_npcflag: u32,
    pub action_menu_id: i32,
    pub action_poi_id: u32,
    pub action_script_id: u32,
    pub box_coded: bool,
    pub box_money: u32,
    pub box_text: Option<String>,
    pub box_broadcast_text: u32,
    pub condition_id: u32,
}

/// Row structure for npc_text table
#[derive(sqlx::FromRow)]
pub struct NpcTextRow {
    #[sqlx(rename = "ID")]
    pub id: u32,
    #[sqlx(rename = "Probability0")]
    pub probability0: f32,
    #[sqlx(rename = "BroadcastTextID0")]
    pub broadcast_text_id0: u32,
    #[sqlx(rename = "Probability1")]
    pub probability1: f32,
    #[sqlx(rename = "BroadcastTextID1")]
    pub broadcast_text_id1: u32,
    #[sqlx(rename = "Probability2")]
    pub probability2: f32,
    #[sqlx(rename = "BroadcastTextID2")]
    pub broadcast_text_id2: u32,
    #[sqlx(rename = "Probability3")]
    pub probability3: f32,
    #[sqlx(rename = "BroadcastTextID3")]
    pub broadcast_text_id3: u32,
    #[sqlx(rename = "Probability4")]
    pub probability4: f32,
    #[sqlx(rename = "BroadcastTextID4")]
    pub broadcast_text_id4: u32,
    #[sqlx(rename = "Probability5")]
    pub probability5: f32,
    #[sqlx(rename = "BroadcastTextID5")]
    pub broadcast_text_id5: u32,
    #[sqlx(rename = "Probability6")]
    pub probability6: f32,
    #[sqlx(rename = "BroadcastTextID6")]
    pub broadcast_text_id6: u32,
    #[sqlx(rename = "Probability7")]
    pub probability7: f32,
    #[sqlx(rename = "BroadcastTextID7")]
    pub broadcast_text_id7: u32,
}

/// Row structure for broadcast_text table
#[derive(sqlx::FromRow)]
pub struct BroadcastTextRow {
    pub entry: u32,
    pub male_text: Option<String>,
    pub female_text: Option<String>,
    pub chat_type: u8,
    pub language_id: u8,
    #[sqlx(rename = "emote_id1")]
    pub emote_id1: u16,
    #[sqlx(rename = "emote_id2")]
    pub emote_id2: u16,
    #[sqlx(rename = "emote_id3")]
    pub emote_id3: u16,
    #[sqlx(rename = "emote_delay1")]
    pub emote_delay1: u32,
    #[sqlx(rename = "emote_delay2")]
    pub emote_delay2: u32,
    #[sqlx(rename = "emote_delay3")]
    pub emote_delay3: u32,
    pub sound_id: u16,
}

/// Row structure for creature_template gossip_menu_id
#[derive(sqlx::FromRow)]
pub struct CreatureGossipRow {
    pub entry: u32,
    pub gossip_menu_id: u32,
}

/// Gossip data loaded from database
pub struct GossipLoadData {
    pub menus: Vec<GossipMenuRow>,
    pub options: Vec<GossipMenuItemRow>,
    pub npc_texts: Vec<NpcTextRow>,
    pub broadcast_texts: Vec<BroadcastTextRow>,
    pub creature_menus: Vec<CreatureGossipRow>,
}

/// Repository for gossip-related database operations
pub struct GossipRepository {
    pool: Arc<MySqlPool>,
}

impl GossipRepository {
    /// Create a new gossip repository
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    /// Load all gossip data from database
    pub async fn load_all(&self) -> Result<GossipLoadData> {
        let menus = self.load_gossip_menus().await?;
        let options = self.load_gossip_menu_items().await?;
        let npc_texts = self.load_npc_texts().await?;
        let broadcast_texts = self.load_broadcast_texts().await?;
        let creature_menus = self.load_creature_gossip().await?;

        Ok(GossipLoadData {
            menus,
            options,
            npc_texts,
            broadcast_texts,
            creature_menus,
        })
    }

    /// Load gossip menus from gossip_menu table
    async fn load_gossip_menus(&self) -> Result<Vec<GossipMenuRow>> {
        sqlx::query_as::<_, GossipMenuRow>("SELECT entry, text_id, condition_id FROM gossip_menu")
            .fetch_all(&*self.pool)
            .await
            .context("Failed to load gossip menus")
    }

    /// Load gossip menu options from gossip_menu_option table
    async fn load_gossip_menu_items(&self) -> Result<Vec<GossipMenuItemRow>> {
        sqlx::query_as::<_, GossipMenuItemRow>(
            "SELECT menu_id, id, option_icon, option_text, option_broadcast_text, \
             option_id, npc_option_npcflag, action_menu_id, action_poi_id, \
             action_script_id, box_coded, box_money, box_text, box_broadcast_text, \
             condition_id \
             FROM gossip_menu_option ORDER BY menu_id, id",
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load gossip menu items")
    }

    /// Load NPC texts from npc_text table
    async fn load_npc_texts(&self) -> Result<Vec<NpcTextRow>> {
        sqlx::query_as::<_, NpcTextRow>(
            "SELECT ID, \
             Probability0, BroadcastTextID0, Probability1, BroadcastTextID1, \
             Probability2, BroadcastTextID2, Probability3, BroadcastTextID3, \
             Probability4, BroadcastTextID4, Probability5, BroadcastTextID5, \
             Probability6, BroadcastTextID6, Probability7, BroadcastTextID7 \
             FROM npc_text",
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load NPC texts")
    }

    /// Load broadcast texts from broadcast_text table
    async fn load_broadcast_texts(&self) -> Result<Vec<BroadcastTextRow>> {
        sqlx::query_as::<_, BroadcastTextRow>(
            "SELECT entry, male_text, female_text, chat_type, language_id, \
             emote_id1, emote_id2, emote_id3, emote_delay1, emote_delay2, emote_delay3, \
             sound_id FROM broadcast_text",
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load broadcast texts")
    }

    /// Load creature default gossip menu IDs
    async fn load_creature_gossip(&self) -> Result<Vec<CreatureGossipRow>> {
        sqlx::query_as::<_, CreatureGossipRow>(
            "SELECT entry, gossip_menu_id FROM creature_template WHERE gossip_menu_id > 0",
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load creature gossip menus")
    }
}
