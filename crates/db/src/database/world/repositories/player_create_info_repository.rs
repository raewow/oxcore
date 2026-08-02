//! PlayerCreateInfo repository for querying starting positions, items, and actions

use anyhow::{Context, Result};
use sqlx::{FromRow, MySqlPool};
use std::sync::Arc;

/// Row from playercreateinfo table
#[derive(FromRow, Debug, Clone)]
pub struct PlayerCreateInfoRow {
    pub race: u8,
    pub class: u8,
    pub map: u32,
    pub zone: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
}

/// Row from playercreateinfo_item table
#[derive(FromRow, Debug, Clone)]
pub struct PlayerCreateInfoItemRow {
    pub race: u8,
    pub class: u8,
    pub itemid: u32,
    pub amount: u8,
}

/// Row from playercreateinfo_spell table
#[derive(FromRow, Debug, Clone)]
pub struct PlayerCreateInfoSpellRow {
    pub race: u8,
    pub class: u8,
    pub spell: u32,
}

/// Row from playercreateinfo_action table
#[derive(FromRow, Debug, Clone)]
pub struct PlayerCreateInfoActionRow {
    pub race: u8,
    pub class: u8,
    pub button: u16,
    pub action: u32,
    #[sqlx(rename = "type")]
    pub action_type: u16,
}

pub struct PlayerCreateInfoRepository {
    pool: Arc<MySqlPool>,
}

impl PlayerCreateInfoRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    /// Get starting position for race/class combination
    pub async fn get_create_info(
        &self,
        race: u8,
        class: u8,
    ) -> Result<Option<PlayerCreateInfoRow>> {
        sqlx::query_as::<_, PlayerCreateInfoRow>(
            r#"SELECT race, class, map, zone, position_x, position_y, position_z, orientation
               FROM playercreateinfo
               WHERE race = ? AND class = ?"#,
        )
        .bind(race)
        .bind(class)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch player create info")
    }

    /// Get starting items for race/class combination
    pub async fn get_create_info_items(
        &self,
        race: u8,
        class: u8,
    ) -> Result<Vec<PlayerCreateInfoItemRow>> {
        sqlx::query_as::<_, PlayerCreateInfoItemRow>(
            r#"SELECT race, class, itemid, amount
               FROM playercreateinfo_item
               WHERE race = ? AND class = ?"#,
        )
        .bind(race)
        .bind(class)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch player create info items")
    }

    /// Get starting spells for race/class combination
    pub async fn get_create_info_spells(
        &self,
        race: u8,
        class: u8,
    ) -> Result<Vec<PlayerCreateInfoSpellRow>> {
        sqlx::query_as::<_, PlayerCreateInfoSpellRow>(
            r#"SELECT race, class, spell
               FROM playercreateinfo_spell
               WHERE race = ? AND class = ?"#,
        )
        .bind(race)
        .bind(class)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch player create info spells")
    }

    /// Get starting action buttons for race/class combination
    pub async fn get_create_info_actions(
        &self,
        race: u8,
        class: u8,
    ) -> Result<Vec<PlayerCreateInfoActionRow>> {
        sqlx::query_as::<_, PlayerCreateInfoActionRow>(
            r#"SELECT race, class, button, action, `type`
               FROM playercreateinfo_action
               WHERE race = ? AND class = ?"#,
        )
        .bind(race)
        .bind(class)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch player create info actions")
    }
}
