//! PlayerCreateInfo repository for querying starting positions, items, and actions

use anyhow::{Context, Result};
use sqlx::{FromRow, PgPool};
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
    pool: Arc<PgPool>,
}

impl PlayerCreateInfoRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Get starting position for race/class combination
    pub async fn get_create_info(
        &self,
        race: u8,
        class: u8,
    ) -> Result<Option<PlayerCreateInfoRow>> {
        sqlx::query_as::<_, (i16, i16, i64, i64, f32, f32, f32, f32)>(
            r#"SELECT race, class, map, zone, position_x, position_y, position_z, orientation
               FROM world.playercreateinfo
               WHERE race = $1 AND class = $2"#,
        )
        .bind(i16::from(race))
        .bind(i16::from(class))
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch player create info")?
        .map(
            |(race, class, map, zone, position_x, position_y, position_z, orientation)| {
                Ok(PlayerCreateInfoRow {
                    race: u8::try_from(race).context("playercreateinfo race exceeds u8")?,
                    class: u8::try_from(class).context("playercreateinfo class exceeds u8")?,
                    map: u32::try_from(map).context("playercreateinfo map exceeds u32")?,
                    zone: u32::try_from(zone).context("playercreateinfo zone exceeds u32")?,
                    position_x,
                    position_y,
                    position_z,
                    orientation,
                })
            },
        )
        .transpose()
    }

    /// Get starting items for race/class combination
    pub async fn get_create_info_items(
        &self,
        race: u8,
        class: u8,
    ) -> Result<Vec<PlayerCreateInfoItemRow>> {
        let rows = sqlx::query_as::<_, (i16, i16, i64, i16)>(
            r#"SELECT race, class, itemid, amount
               FROM world.playercreateinfo_item
               WHERE race = $1 AND class = $2"#,
        )
        .bind(i16::from(race))
        .bind(i16::from(class))
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch player create info items")?;
        rows.into_iter()
            .map(|(race, class, itemid, amount)| {
                Ok(PlayerCreateInfoItemRow {
                    race: u8::try_from(race).context("playercreateinfo_item race exceeds u8")?,
                    class: u8::try_from(class).context("playercreateinfo_item class exceeds u8")?,
                    itemid: u32::try_from(itemid)
                        .context("playercreateinfo_item item ID exceeds u32")?,
                    amount: u8::try_from(amount)
                        .context("playercreateinfo_item amount exceeds u8")?,
                })
            })
            .collect()
    }

    /// Get starting spells for race/class combination
    pub async fn get_create_info_spells(
        &self,
        race: u8,
        class: u8,
    ) -> Result<Vec<PlayerCreateInfoSpellRow>> {
        let rows = sqlx::query_as::<_, (i16, i16, i64)>(
            r#"SELECT race, class, spell
               FROM world.playercreateinfo_spell
               WHERE race = $1 AND class = $2"#,
        )
        .bind(i16::from(race))
        .bind(i16::from(class))
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch player create info spells")?;
        rows.into_iter()
            .map(|(race, class, spell)| {
                Ok(PlayerCreateInfoSpellRow {
                    race: u8::try_from(race).context("playercreateinfo_spell race exceeds u8")?,
                    class: u8::try_from(class)
                        .context("playercreateinfo_spell class exceeds u8")?,
                    spell: u32::try_from(spell).context("playercreateinfo_spell ID exceeds u32")?,
                })
            })
            .collect()
    }

    /// Get starting action buttons for race/class combination
    pub async fn get_create_info_actions(
        &self,
        race: u8,
        class: u8,
    ) -> Result<Vec<PlayerCreateInfoActionRow>> {
        let rows = sqlx::query_as::<_, (i16, i16, i16, i64, i16)>(
            r#"SELECT race, class, button, action, type
               FROM world.playercreateinfo_action
               WHERE race = $1 AND class = $2"#,
        )
        .bind(i16::from(race))
        .bind(i16::from(class))
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch player create info actions")?;
        rows.into_iter()
            .map(|(race, class, button, action, action_type)| {
                Ok(PlayerCreateInfoActionRow {
                    race: u8::try_from(race).context("playercreateinfo_action race exceeds u8")?,
                    class: u8::try_from(class)
                        .context("playercreateinfo_action class exceeds u8")?,
                    button: u16::try_from(button)
                        .context("playercreateinfo_action button exceeds u16")?,
                    action: u32::try_from(action)
                        .context("playercreateinfo_action ID exceeds u32")?,
                    action_type: u16::try_from(action_type)
                        .context("playercreateinfo_action type exceeds u16")?,
                })
            })
            .collect()
    }
}
