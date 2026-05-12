use sqlx::MySqlPool;
use super::pool_types::{PoolTemplate, PoolMember, PoolMemberType};

/// Repository for loading pool data from database
pub struct PoolRepository {
    pool: MySqlPool,
}

impl PoolRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Load all pool data from database
    pub async fn load_all_pools(&self) -> anyhow::Result<PoolData> {
        let templates = self.load_templates().await?;
        let creature_members = self.load_creature_members().await?;
        let pool_members = self.load_pool_members().await?;

        tracing::info!(
            "Loaded {} pool templates, {} creature members, {} nested pools",
            templates.len(),
            creature_members.len(),
            pool_members.len()
        );

        Ok(PoolData {
            templates,
            creature_members,
            pool_members,
        })
    }

    /// Load pool templates
    async fn load_templates(&self) -> anyhow::Result<Vec<PoolTemplate>> {
        let rows = sqlx::query_as::<_, PoolTemplateRow>(
            "SELECT entry, max_limit, description FROM pool_template"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| PoolTemplate {
                pool_id: row.entry,
                max_limit: row.max_limit,
                description: row.description,
            })
            .collect())
    }

    /// Load creature members for all pools
    async fn load_creature_members(&self) -> anyhow::Result<Vec<PoolCreatureMember>> {
        let rows = sqlx::query_as::<_, PoolCreatureRow>(
            "SELECT pool_entry, guid, chance, description FROM pool_creature"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| PoolCreatureMember {
                pool_id: row.pool_entry,
                spawn_id: row.guid,
                chance: row.chance,
                description: row.description,
            })
            .collect())
    }

    /// Load nested pool members
    async fn load_pool_members(&self) -> anyhow::Result<Vec<PoolPoolMember>> {
        let rows = sqlx::query_as::<_, PoolPoolRow>(
            "SELECT pool_id, mother_pool, chance, description FROM pool_pool"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| PoolPoolMember {
                child_pool_id: row.pool_id,
                parent_pool_id: row.mother_pool,
                chance: row.chance,
                description: row.description,
            })
            .collect())
    }
}

/// All pool data loaded from database
pub struct PoolData {
    pub templates: Vec<PoolTemplate>,
    pub creature_members: Vec<PoolCreatureMember>,
    pub pool_members: Vec<PoolPoolMember>,
}

/// Creature member of a pool
pub struct PoolCreatureMember {
    pub pool_id: u32,
    pub spawn_id: u32,
    pub chance: f32,
    pub description: String,
}

/// Nested pool member
pub struct PoolPoolMember {
    pub child_pool_id: u32,
    pub parent_pool_id: u32,
    pub chance: f32,
    pub description: String,
}

#[derive(sqlx::FromRow)]
struct PoolTemplateRow {
    entry: u32,
    max_limit: u32,
    description: String,
}

#[derive(sqlx::FromRow)]
struct PoolCreatureRow {
    pool_entry: u32,
    guid: u32,
    chance: f32,
    description: String,
}

#[derive(sqlx::FromRow)]
struct PoolPoolRow {
    pool_id: u32,
    mother_pool: u32,
    chance: f32,
    description: String,
}
