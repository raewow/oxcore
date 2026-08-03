use super::super::models::auction::*;
use super::super::{PgAuctionRepository, PgAuctionRow};
use super::auction_repository_trait::AuctionRepositoryTrait;
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

pub struct AuctionRepository {
    pool: Arc<PgPool>,
}

impl AuctionRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    fn pg(&self) -> PgAuctionRepository {
        PgAuctionRepository::new(Arc::clone(&self.pool))
    }
    fn now() -> Result<i64> {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("System clock before UNIX epoch")?
            .as_secs()
            .try_into()
            .map_err(Into::into)
    }
}

#[async_trait]
impl AuctionRepositoryTrait for AuctionRepository {
    async fn get_max_auction_id(&self) -> Result<Option<u32>> {
        self.pg()
            .get_max_auction_id()
            .await?
            .map(|value| value.try_into().map_err(Into::into))
            .transpose()
    }
    async fn find_by_id(&self, id: u32) -> Result<Option<AuctionRow>> {
        self.pg()
            .find_by_id(id.into())
            .await?
            .map(TryInto::try_into)
            .transpose()
    }
    async fn find_by_house(&self, house_id: u32) -> Result<Vec<AuctionRow>> {
        self.pg()
            .find_by_house(house_id.into())
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
    async fn find_by_seller(&self, seller_guid: u32) -> Result<Vec<AuctionRow>> {
        self.pg()
            .find_by_seller(seller_guid.into())
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
    async fn find_by_bidder(&self, bidder_guid: u32) -> Result<Vec<AuctionRow>> {
        self.pg()
            .find_by_bidder(bidder_guid.into())
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
    async fn find_active_auctions(&self) -> Result<Vec<AuctionRow>> {
        self.pg()
            .find_active_auctions(Self::now()?)
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
    async fn find_active_by_house_with_account(
        &self,
        house_id: u32,
    ) -> Result<Vec<AuctionWithAccountRow>> {
        self.pg()
            .find_active_by_house_with_account(house_id.into(), Self::now()?)
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
    async fn find_all_for_load(&self) -> Result<Vec<AuctionRow>> {
        self.pg()
            .find_all_for_load()
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
    async fn find_all_items_for_load(&self) -> Result<Vec<AuctionItemLoadRow>> {
        self.pg()
            .find_all_items_for_load()
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
    async fn create_auction(&self, auction: &AuctionRow) -> Result<()> {
        self.pg().create_auction(&PgAuctionRow::from(auction)).await
    }
    async fn update_auction(&self, auction: &AuctionRow) -> Result<()> {
        self.pg().update_auction(&PgAuctionRow::from(auction)).await
    }
    async fn update_bid(&self, id: u32, bidder_guid: u32, new_bid: i32) -> Result<()> {
        self.pg()
            .update_bid(id.into(), bidder_guid.into(), new_bid)
            .await
    }
    async fn delete_auction(&self, id: u32) -> Result<()> {
        self.pg().delete_auction(id.into()).await
    }
    async fn delete_expired_auctions(&self) -> Result<u64> {
        self.pg().delete_expired_auctions(Self::now()?).await
    }
}
