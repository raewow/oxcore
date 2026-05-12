use sqlx::MySqlPool;
use super::addon::CreatureAddon;

/// Repository for loading creature addon data from database
pub struct AddonRepository {
    pool: MySqlPool,
}

impl AddonRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Load all addon data from database
    pub async fn load_all_addons(&self) -> anyhow::Result<AddonData> {
        let guid_addons = self.load_guid_addons().await?;
        let template_addons = self.load_template_addons().await?;

        tracing::info!(
            "Loaded {} GUID addons, {} template addons",
            guid_addons.len(),
            template_addons.len()
        );

        Ok(AddonData {
            guid_addons,
            template_addons,
        })
    }

    /// Load GUID-specific addons from creature_addon table
    async fn load_guid_addons(&self) -> anyhow::Result<Vec<(u32, CreatureAddon)>> {
        let rows = sqlx::query_as::<_, AddonRow>(
            "SELECT guid, mount_display_id, stand_state, sheath_state, emote_state, \
             COALESCE(auras, '') AS auras \
             FROM creature_addon"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                (
                    row.guid,
                    CreatureAddon {
                        mount: row.mount_display_id as u32,
                        bytes1: row.stand_state as u32,
                        bytes2: row.sheath_state as u32,
                        emote: row.emote_state as u32,
                        auras: parse_auras(&row.auras),
                    },
                )
            })
            .collect())
    }

    /// Load template addons from creature_template_addon table
    /// This table may not exist in all database setups - handle gracefully
    async fn load_template_addons(&self) -> anyhow::Result<Vec<(u32, CreatureAddon)>> {
        let result = sqlx::query_as::<_, TemplateAddonRow>(
            "SELECT entry, mount_display_id, stand_state, sheath_state, emote_state, \
             COALESCE(auras, '') AS auras \
             FROM creature_template_addon"
        )
        .fetch_all(&self.pool)
        .await;

        match result {
            Ok(rows) => Ok(rows
                .into_iter()
                .map(|row| {
                    (
                        row.entry,
                        CreatureAddon {
                            mount: row.mount_display_id as u32,
                            bytes1: row.stand_state as u32,
                            bytes2: row.sheath_state as u32,
                            emote: row.emote_state as u32,
                            auras: parse_auras(&row.auras),
                        },
                    )
                })
                .collect()),
            Err(e) => {
                tracing::warn!("Could not load creature_template_addon (table may not exist): {}", e);
                Ok(Vec::new())
            }
        }
    }
}

/// All addon data loaded from database
pub struct AddonData {
    pub guid_addons: Vec<(u32, CreatureAddon)>,
    pub template_addons: Vec<(u32, CreatureAddon)>,
}

/// Parse space-separated aura string
fn parse_auras(auras_str: &str) -> Vec<u32> {
    auras_str
        .split_whitespace()
        .filter_map(|s| s.parse::<u32>().ok())
        .filter(|&id| id > 0)
        .collect()
}

#[derive(sqlx::FromRow)]
struct AddonRow {
    guid: u32,
    mount_display_id: i16,
    stand_state: u8,
    sheath_state: u8,
    emote_state: u16,
    auras: String,
}

#[derive(sqlx::FromRow)]
struct TemplateAddonRow {
    entry: u32,
    mount_display_id: i16,
    stand_state: u8,
    sheath_state: u8,
    emote_state: u16,
    auras: String,
}
