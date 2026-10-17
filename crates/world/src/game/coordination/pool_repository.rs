//! Loads spawn pool definitions from the world database
//! (`pool_template`, `pool_creature`, `pool_gameobject`, `pool_pool`, and the
//! entry-based `pool_creature_template` / `pool_gameobject_template`).

use super::pool_types::PoolTemplate;
use anyhow::Context;
use sqlx::PgPool;

/// Repository for loading pool data from database
pub struct PoolRepository {
    pool: PgPool,
    /// Content patch used to filter `patch_min`/`patch_max` rows.
    patch: u8,
}

impl PoolRepository {
    pub fn new(pool: PgPool) -> Self {
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
        let rows = sqlx::query_as::<sqlx::Postgres, PoolTemplateRow>(
            "SELECT entry::BIGINT AS entry, max_limit, flags, description FROM world.pool_template \
             WHERE $1 BETWEEN patch_min AND patch_max",
        )
        .bind(i16::from(self.patch))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(PoolTemplateRow::into_template)
            .collect()
    }

    /// Load creature members for all pools: explicit spawns, plus every spawn of
    /// each entry listed in `pool_creature_template`.
    async fn load_creature_members(&self) -> anyhow::Result<Vec<PoolObjectMember>> {
        let explicit = sqlx::query_as::<sqlx::Postgres, PoolObjectRow>(
            "SELECT pool_entry::BIGINT AS pool_entry, guid, chance, description FROM world.pool_creature \
             WHERE $1 BETWEEN patch_min AND patch_max",
        )
        .bind(i16::from(self.patch))
        .fetch_all(&self.pool)
        .await?;
        let by_entry = sqlx::query_as::<sqlx::Postgres, PoolObjectRow>(
            "SELECT t.pool_entry::BIGINT AS pool_entry, c.guid, t.chance, t.description \
             FROM world.pool_creature_template t JOIN world.creature c ON c.id = t.id \
             WHERE $1 BETWEEN t.patch_min AND t.patch_max \
               AND $1 BETWEEN c.patch_min AND c.patch_max",
        )
        .bind(i16::from(self.patch))
        .fetch_all(&self.pool)
        .await?;

        merge_members(explicit, by_entry, "pool_creature_template")
    }

    /// Load gameobject members for all pools: explicit spawns, plus every spawn
    /// of each entry listed in `pool_gameobject_template`.
    async fn load_gameobject_members(&self) -> anyhow::Result<Vec<PoolObjectMember>> {
        let explicit = sqlx::query_as::<sqlx::Postgres, PoolObjectRow>(
            "SELECT pool_entry::BIGINT AS pool_entry, guid, chance, description FROM world.pool_gameobject \
             WHERE $1 BETWEEN patch_min AND patch_max",
        )
        .bind(i16::from(self.patch))
        .fetch_all(&self.pool)
        .await?;
        let by_entry = sqlx::query_as::<sqlx::Postgres, PoolObjectRow>(
            "SELECT t.pool_entry::BIGINT AS pool_entry, g.guid, t.chance, t.description \
             FROM world.pool_gameobject_template t JOIN world.gameobject g ON g.id = t.id \
             WHERE $1 BETWEEN t.patch_min AND t.patch_max \
               AND $1 BETWEEN g.patch_min AND g.patch_max",
        )
        .bind(i16::from(self.patch))
        .fetch_all(&self.pool)
        .await?;

        merge_members(explicit, by_entry, "pool_gameobject_template")
    }

    /// Load nested pool members
    async fn load_pool_members(&self) -> anyhow::Result<Vec<PoolPoolMember>> {
        let rows = sqlx::query_as::<sqlx::Postgres, PoolPoolRow>(
            "SELECT pool_id::BIGINT AS pool_id, mother_pool::BIGINT AS mother_pool, chance, description FROM world.pool_pool",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(PoolPoolRow::into_member).collect()
    }
}

/// Combine explicit pool members with entry-expanded ones. A spawn may belong to
/// only one pool; an expanded spawn already listed explicitly is skipped.
fn merge_members(
    explicit: Vec<PoolObjectRow>,
    by_entry: Vec<PoolObjectRow>,
    table: &str,
) -> anyhow::Result<Vec<PoolObjectMember>> {
    let mut members: Vec<PoolObjectMember> = explicit
        .into_iter()
        .map(PoolObjectRow::into_member)
        .collect::<anyhow::Result<_>>()?;
    let mut seen: std::collections::HashMap<u32, u32> = members
        .iter()
        .map(|member| (member.spawn_id, member.pool_id))
        .collect();
    for row in by_entry {
        let member = row.into_member()?;
        if let Some(&pool) = seen.get(&member.spawn_id) {
            tracing::warn!(
                "`{}` spawn {} for pool {} is already in pool {}, skipped",
                table,
                member.spawn_id,
                member.pool_id,
                pool
            );
            continue;
        }
        seen.insert(member.spawn_id, member.pool_id);
        members.push(member);
    }
    Ok(members)
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
    entry: i64,
    max_limit: i64,
    flags: i64,
    description: String,
}

impl PoolTemplateRow {
    fn into_template(self) -> anyhow::Result<PoolTemplate> {
        Ok(PoolTemplate {
            pool_id: u32::try_from(self.entry)
                .context("pool_template.entry is outside u32 range")?,
            max_limit: u32::try_from(self.max_limit)
                .context("pool_template.max_limit is outside u32 range")?,
            flags: u32::try_from(self.flags).context("pool_template.flags is outside u32 range")?,
            description: self.description,
        })
    }
}

#[derive(sqlx::FromRow)]
struct PoolObjectRow {
    pool_entry: i64,
    guid: i64,
    chance: f32,
    description: String,
}

impl PoolObjectRow {
    fn into_member(self) -> anyhow::Result<PoolObjectMember> {
        Ok(PoolObjectMember {
            pool_id: u32::try_from(self.pool_entry)
                .context("pool member pool_entry is outside u32 range")?,
            spawn_id: u32::try_from(self.guid).context("pool member guid is outside u32 range")?,
            chance: self.chance,
            description: self.description,
        })
    }
}

#[derive(sqlx::FromRow)]
struct PoolPoolRow {
    pool_id: i64,
    mother_pool: i64,
    chance: f32,
    description: String,
}

impl PoolPoolRow {
    fn into_member(self) -> anyhow::Result<PoolPoolMember> {
        Ok(PoolPoolMember {
            child_pool_id: u32::try_from(self.pool_id)
                .context("pool_pool.pool_id is outside u32 range")?,
            parent_pool_id: u32::try_from(self.mother_pool)
                .context("pool_pool.mother_pool is outside u32 range")?,
            chance: self.chance,
            description: self.description,
        })
    }
}
