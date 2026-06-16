use super::link_flags::LinkFlags;
use sqlx::MySqlPool;

/// Repository for loading creature linking data from database
pub struct LinkingRepository {
    pool: MySqlPool,
}

impl LinkingRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Load all creature links from database
    pub async fn load_all_links(&self) -> anyhow::Result<Vec<CreatureLinkRow>> {
        let rows = sqlx::query_as::<_, LinkRow>(
            "SELECT guid AS slave_guid, master_guid, flag FROM creature_linking",
        )
        .fetch_all(&self.pool)
        .await?;

        let links: Vec<CreatureLinkRow> = rows
            .into_iter()
            .map(|row| CreatureLinkRow {
                master_guid: row.master_guid,
                slave_guid: row.slave_guid,
                flags: LinkFlags::from_bits_truncate(row.flag),
            })
            .collect();

        tracing::info!("Loaded {} creature links", links.len());
        Ok(links)
    }
}

/// Creature link data from database
#[derive(Debug, Clone)]
pub struct CreatureLinkRow {
    pub master_guid: u32,
    pub slave_guid: u32,
    pub flags: LinkFlags,
}

#[derive(sqlx::FromRow)]
struct LinkRow {
    master_guid: u32,
    slave_guid: u32,
    flag: u32,
}
