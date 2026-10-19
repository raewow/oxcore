use super::link_flags::LinkFlags;
use sqlx::PgPool;

/// Repository for loading creature linking data from database
pub struct LinkingRepository {
    pool: PgPool,
}

impl LinkingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Load the links between individual spawns (creature_linking).
    pub async fn load_all_links(&self) -> anyhow::Result<Vec<CreatureLinkRow>> {
        let rows = sqlx::query_as::<_, LinkRow>(
            "SELECT guid AS slave_guid, master_guid, flag FROM world.creature_linking",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| CreatureLinkRow {
                master_guid: row.master_guid as u32,
                slave_guid: row.slave_guid as u32,
                flags: LinkFlags::from_bits_truncate(row.flag as u32),
            })
            .collect())
    }

    /// Load the links between creature entries on a map (creature_linking_template).
    pub async fn load_all_entry_links(&self) -> anyhow::Result<Vec<CreatureEntryLinkRow>> {
        let rows = sqlx::query_as::<_, EntryLinkRow>(
            "SELECT entry, map::BIGINT AS map, master_entry, flag, search_range \
             FROM world.creature_linking_template",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| CreatureEntryLinkRow {
                slave_entry: row.entry as u32,
                map_id: row.map as u32,
                master_entry: row.master_entry as u32,
                flags: LinkFlags::from_bits_truncate(row.flag as u32),
                search_range: row.search_range as u32,
            })
            .collect())
    }
}

/// A link between two spawns.
#[derive(Debug, Clone)]
pub struct CreatureLinkRow {
    pub master_guid: u32,
    pub slave_guid: u32,
    pub flags: LinkFlags,
}

/// A link from every creature of one entry on a map to the nearest creature of another.
#[derive(Debug, Clone)]
pub struct CreatureEntryLinkRow {
    pub slave_entry: u32,
    pub map_id: u32,
    pub master_entry: u32,
    pub flags: LinkFlags,
    /// Slaves whose spawn point is within this range of the master's belong to it; 0 = any.
    pub search_range: u32,
}

#[derive(sqlx::FromRow)]
struct LinkRow {
    master_guid: i64,
    slave_guid: i64,
    flag: i64,
}

#[derive(sqlx::FromRow)]
struct EntryLinkRow {
    entry: i64,
    map: i64,
    master_entry: i64,
    flag: i64,
    search_range: i64,
}
