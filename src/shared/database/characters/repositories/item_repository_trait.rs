use anyhow::Result;
use async_trait::async_trait;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ItemRepositoryTrait: Send + Sync {
    async fn update_owner(&self, guid: u32, owner_guid: u32) -> Result<()>;
    async fn delete(&self, guid: u32) -> Result<()>;
}
