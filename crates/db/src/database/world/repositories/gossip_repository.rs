//! Gossip repository for database access
//!
//! Handles loading of gossip menus, menu items, NPC texts, and broadcast texts.

use anyhow::{Context, Result};
use sqlx::PgPool;
use std::sync::Arc;

/// Row structure for gossip_menu table
#[derive(sqlx::FromRow)]
pub struct GossipMenuRow {
    pub entry: i64,
    pub text_id: i64,
    pub condition_id: i64,
}

/// Row structure for gossip_menu_option table
#[derive(sqlx::FromRow)]
pub struct GossipMenuItemRow {
    pub menu_id: i64,
    pub id: i64,
    pub option_icon: i16,
    pub option_text: Option<String>,
    pub option_broadcast_text: i64,
    pub option_id: i64,
    pub npc_option_npcflag: i64,
    pub action_menu_id: i32,
    pub action_poi_id: i64,
    pub action_script_id: i64,
    pub box_coded: bool,
    pub box_money: i64,
    pub box_text: Option<String>,
    pub box_broadcast_text: i64,
    pub condition_id: i64,
}

/// Row structure for npc_text table
#[derive(sqlx::FromRow)]
pub struct NpcTextRow {
    #[sqlx(rename = "ID")]
    pub id: i64,
    #[sqlx(rename = "Probability0")]
    pub probability0: f32,
    #[sqlx(rename = "BroadcastTextID0")]
    pub broadcast_text_id0: i64,
    #[sqlx(rename = "Probability1")]
    pub probability1: f32,
    #[sqlx(rename = "BroadcastTextID1")]
    pub broadcast_text_id1: i64,
    #[sqlx(rename = "Probability2")]
    pub probability2: f32,
    #[sqlx(rename = "BroadcastTextID2")]
    pub broadcast_text_id2: i64,
    #[sqlx(rename = "Probability3")]
    pub probability3: f32,
    #[sqlx(rename = "BroadcastTextID3")]
    pub broadcast_text_id3: i64,
    #[sqlx(rename = "Probability4")]
    pub probability4: f32,
    #[sqlx(rename = "BroadcastTextID4")]
    pub broadcast_text_id4: i64,
    #[sqlx(rename = "Probability5")]
    pub probability5: f32,
    #[sqlx(rename = "BroadcastTextID5")]
    pub broadcast_text_id5: i64,
    #[sqlx(rename = "Probability6")]
    pub probability6: f32,
    #[sqlx(rename = "BroadcastTextID6")]
    pub broadcast_text_id6: i64,
    #[sqlx(rename = "Probability7")]
    pub probability7: f32,
    #[sqlx(rename = "BroadcastTextID7")]
    pub broadcast_text_id7: i64,
}

/// Row structure for broadcast_text table
#[derive(sqlx::FromRow)]
pub struct BroadcastTextRow {
    pub entry: i64,
    pub male_text: Option<String>,
    pub female_text: Option<String>,
    pub chat_type: i16,
    pub language_id: i16,
    #[sqlx(rename = "emote_id1")]
    pub emote_id1: i16,
    #[sqlx(rename = "emote_id2")]
    pub emote_id2: i16,
    #[sqlx(rename = "emote_id3")]
    pub emote_id3: i16,
    #[sqlx(rename = "emote_delay1")]
    pub emote_delay1: i64,
    #[sqlx(rename = "emote_delay2")]
    pub emote_delay2: i64,
    #[sqlx(rename = "emote_delay3")]
    pub emote_delay3: i64,
    pub sound_id: i16,
}

/// Row structure for creature_template gossip_menu_id
#[derive(sqlx::FromRow)]
pub struct CreatureGossipRow {
    pub entry: i64,
    pub gossip_menu_id: i64,
}

/// Row structure for npc_gossip per-spawn text overrides
#[derive(sqlx::FromRow)]
pub struct NpcGossipRow {
    pub npc_guid: i64,
    pub textid: i64,
}

/// Gossip data loaded from database
pub struct GossipLoadData {
    pub menus: Vec<GossipMenuRow>,
    pub options: Vec<GossipMenuItemRow>,
    pub npc_texts: Vec<NpcTextRow>,
    pub broadcast_texts: Vec<BroadcastTextRow>,
    pub creature_menus: Vec<CreatureGossipRow>,
    pub npc_gossip: Vec<NpcGossipRow>,
}

/// Repository for gossip-related database operations
pub struct GossipRepository {
    pool: Arc<PgPool>,
}

impl GossipRepository {
    /// Create a new gossip repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Load all gossip data from database
    pub async fn load_all(&self) -> Result<GossipLoadData> {
        let menus = self.load_gossip_menus().await?;
        let options = self.load_gossip_menu_items().await?;
        let npc_texts = self.load_npc_texts().await?;
        let broadcast_texts = self.load_broadcast_texts().await?;
        let creature_menus = self.load_creature_gossip().await?;
        let npc_gossip = self.load_npc_gossip().await?;

        Ok(GossipLoadData {
            menus,
            options,
            npc_texts,
            broadcast_texts,
            creature_menus,
            npc_gossip,
        })
    }

    /// Load gossip menus from gossip_menu table
    async fn load_gossip_menus(&self) -> Result<Vec<GossipMenuRow>> {
        sqlx::query_as::<_, GossipMenuRow>(
            "SELECT entry::BIGINT AS entry, text_id, condition_id FROM world.gossip_menu",
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load gossip menus")
    }

    /// Load gossip menu options from gossip_menu_option table
    async fn load_gossip_menu_items(&self) -> Result<Vec<GossipMenuItemRow>> {
        sqlx::query_as::<_, GossipMenuItemRow>(
            "SELECT menu_id::BIGINT AS menu_id, id::BIGINT AS id,
              option_icon::SMALLINT AS option_icon, option_text, option_broadcast_text, \
              option_id::BIGINT AS option_id, npc_option_npcflag, action_menu_id, action_poi_id, \
              action_script_id, box_coded <> 0 AS box_coded, box_money, box_text, box_broadcast_text, \
             condition_id \
              FROM world.gossip_menu_option ORDER BY menu_id, id",
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load gossip menu items")
    }

    /// Load NPC texts from npc_text table
    async fn load_npc_texts(&self) -> Result<Vec<NpcTextRow>> {
        sqlx::query_as::<_, NpcTextRow>("SELECT * FROM world.npc_text")
            .fetch_all(&*self.pool)
            .await
            .context("Failed to load NPC texts")
    }

    /// Load broadcast texts from broadcast_text table
    async fn load_broadcast_texts(&self) -> Result<Vec<BroadcastTextRow>> {
        sqlx::query_as::<_, BroadcastTextRow>(
            "SELECT entry, male_text, female_text, chat_type, language_id, \
              emote_id1::SMALLINT AS emote_id1, emote_id2::SMALLINT AS emote_id2, \
              emote_id3::SMALLINT AS emote_id3, emote_delay1, emote_delay2, emote_delay3, \
              sound_id::SMALLINT AS sound_id FROM world.broadcast_text",
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load broadcast texts")
    }

    /// Load creature default gossip menu IDs
    async fn load_creature_gossip(&self) -> Result<Vec<CreatureGossipRow>> {
        sqlx::query_as::<_, CreatureGossipRow>(
            "SELECT entry, gossip_menu_id FROM world.creature_template WHERE gossip_menu_id > 0",
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load creature gossip menus")
    }

    /// Load per-creature gossip text overrides
    async fn load_npc_gossip(&self) -> Result<Vec<NpcGossipRow>> {
        sqlx::query_as::<_, NpcGossipRow>("SELECT npc_guid, textid FROM world.npc_gossip")
            .fetch_all(&*self.pool)
            .await
            .context("Failed to load NPC gossip text overrides")
    }
}
