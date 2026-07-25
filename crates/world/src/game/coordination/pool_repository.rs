//! Loads spawn pool definitions from the world database
//! (`pool_template`, `pool_creature`, `pool_gameobject`, `pool_pool`).

use super::pool_types::PoolTemplate;
use sqlx::MySqlPool;

/// Repository for loading pool data from database
pub struct PoolRepository {
    pool: MySqlPool,
    /// Content patch used to filter `patch_min`/`patch_max` rows.
    patch: u8,
}

impl PoolRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool, patch: 10 }
    }

    /// Set the content patch used for `patch_min`/`patch_max` filtering.
    pub fn with_patch(mut self, patch: u8) -> Self {
        self.patch = patch;
        self
    }

    /// Load all pool data from database
    pub async fn load_all_pools(&self) -> anyhow::Result<PoolData> {
        let templates = self.load_templates().await?;
        let creature_members = self.load_creature_members().await?;
        let gameobject_members = self.load_gameobject_members().await?;
        let pool_members = self.load_pool_members().await?;

        tracing::info!(
            "Loaded {} pool templates, {} creature members, {} gameobject members, {} nested pools",
            templates.len(),
            creature_members.len(),
            gameobject_members.len(),
            pool_members.len()
        );

        Ok(PoolData {
            templates,
            creature_members,
            gameobject_members,
            pool_members,
        })
    }

    /// Load pool templates
    async fn load_templates(&self) -> anyhow::Result<Vec<PoolTemplate>> {
        let rows = sqlx::query_as::<_, PoolTemplateRow>(
            "SELECT `entry`, `max_limit`, `flags`, `description` FROM `pool_template` \
             WHERE ? BETWEEN `patch_min` AND `patch_max`",
        )
        .bind(self.patch)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| PoolTemplate {
                pool_id: row.entry,
                max_limit: row.max_limit,
                flags: row.flags,
                description: row.description,
            })
            .collect())
    }

    /// Load creature members for all pools
    async fn load_creature_members(&self) -> anyhow::Result<Vec<PoolObjectMember>> {
        let rows = sqlx::query_as::<_, PoolObjectRow>(
            "SELECT `pool_entry`, `guid`, `chance`, `description` FROM `pool_creature` \
             WHERE ? BETWEEN `patch_min` AND `patch_max`",
        )
        .bind(self.patch)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(PoolObjectRow::into_member).collect())
    }

    /// Load gameobject members for all pools
    async fn load_gameobject_members(&self) -> anyhow::Result<Vec<PoolObjectMember>> {
        let rows = sqlx::query_as::<_, PoolObjectRow>(
            "SELECT `pool_entry`, `guid`, `chance`, `description` FROM `pool_gameobject` \
             WHERE ? BETWEEN `patch_min` AND `patch_max`",
        )
        .bind(self.patch)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(PoolObjectRow::into_member).collect())
    }

    /// Load nested pool members
    async fn load_pool_members(&self) -> anyhow::Result<Vec<PoolPoolMember>> {
        let rows = sqlx::query_as::<_, PoolPoolRow>(
            "SELECT `pool_id`, `mother_pool`, `chance`, `description` FROM `pool_pool`",
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
    pub creature_members: Vec<PoolObjectMember>,
    pub gameobject_members: Vec<PoolObjectMember>,
    pub pool_members: Vec<PoolPoolMember>,
}

/// Creature or gameobject member of a pool
pub struct PoolObjectMember {
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
    flags: u32,
    description: String,
}

#[derive(sqlx::FromRow)]
struct PoolObjectRow {
    pool_entry: u32,
    guid: u32,
    chance: f32,
    description: String,
}

impl PoolObjectRow {
    fn into_member(self) -> PoolObjectMember {
        PoolObjectMember {
            pool_id: self.pool_entry,
            spawn_id: self.guid,
            chance: self.chance,
            description: self.description,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PoolPoolRow {
    pool_id: u32,
    mother_pool: u32,
    chance: f32,
    description: String,
}
