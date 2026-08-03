use super::super::models::social::*;
use super::super::{PgSocialRepository, PgSocialRow};
use super::social_repository_trait::SocialRepositoryTrait;
use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

pub struct SocialRepository {
    pool: Arc<PgPool>,
}
impl SocialRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    fn pg(&self) -> PgSocialRepository {
        PgSocialRepository::new(Arc::clone(&self.pool))
    }
    fn row(row: PgSocialRow) -> Result<CharacterSocialRow> {
        Ok(CharacterSocialRow {
            guid: row.guid.try_into()?,
            friend: row.friend.try_into()?,
            flags: row.flags.try_into()?,
        })
    }
}
#[async_trait]
impl SocialRepositoryTrait for SocialRepository {
    async fn find_by_guid(&self, guid: u32) -> Result<Vec<CharacterSocialRow>> {
        self.pg()
            .find_by_guid(guid.into())
            .await?
            .into_iter()
            .map(Self::row)
            .collect()
    }
    async fn find_player_guid_by_name(&self, name: &str) -> Result<Option<u32>> {
        self.pg()
            .find_player_guid_by_name(name)
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)
    }
    async fn exists(&self, guid: u32, friend: u32) -> Result<bool> {
        self.pg().exists(guid.into(), friend.into()).await
    }
    async fn get_character_name(&self, guid: u32) -> Result<Option<String>> {
        self.pg().get_character_name(guid.into()).await
    }
    async fn add_or_update(&self, guid: u32, friend: u32, flags: u8) -> Result<()> {
        self.pg()
            .add_or_update(guid.into(), friend.into(), flags.into())
            .await
    }
    async fn update_flags(&self, guid: u32, friend: u32, flags: u8) -> Result<()> {
        self.pg()
            .update_flags(guid.into(), friend.into(), flags.into())
            .await
    }
    async fn add_flags(&self, guid: u32, friend: u32, flags: u8) -> Result<()> {
        self.pg()
            .add_flags(guid.into(), friend.into(), flags.into())
            .await
    }
    async fn remove_flags(&self, guid: u32, friend: u32, flags: u8) -> Result<()> {
        self.pg()
            .remove_flags(guid.into(), friend.into(), flags.into())
            .await
    }
    async fn remove(&self, guid: u32, friend: u32) -> Result<()> {
        self.pg().remove(guid.into(), friend.into()).await
    }
    async fn delete_all_for_character(&self, guid: u32) -> Result<()> {
        self.pg().delete_all_for_character(guid.into()).await
    }
}
