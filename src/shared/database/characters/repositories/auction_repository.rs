use super::super::models::auction::*;
use super::auction_repository_trait::AuctionRepositoryTrait;
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;
use std::sync::Arc;

pub struct AuctionRepository {
    pool: Arc<MySqlPool>,
}

impl AuctionRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuctionRepositoryTrait for AuctionRepository {
    // ========== QUERY METHODS (Read Operations) ==========

    /// Get the maximum auction ID from the database (for generating next ID).
    async fn get_max_auction_id(&self) -> Result<Option<u32>> {
        sqlx::query_scalar::<_, Option<u32>>("SELECT MAX(id) FROM auction")
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query max auction id")
    }

    /// Find an auction by ID.
    async fn find_by_id(&self, auction_id: u32) -> Result<Option<AuctionRow>> {
        sqlx::query_as::<_, AuctionRow>(
            r#"SELECT id, house_id, item_guid, item_id, seller_guid, buyout_price, expire_time, buyer_guid, last_bid, start_bid, deposit FROM auction WHERE id = ?"#,
        )
        .bind(auction_id)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch auction by ID")
    }

    /// Find all auctions for a specific house.
    async fn find_by_house(&self, house_id: u32) -> Result<Vec<AuctionRow>> {
        sqlx::query_as::<_, AuctionRow>(
            r#"SELECT id, house_id, item_guid, item_id, seller_guid, buyout_price, expire_time, buyer_guid, last_bid, start_bid, deposit FROM auction WHERE house_id = ?"#,
        )
        .bind(house_id)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch auctions by house ID")
    }

    /// Find all auctions by a specific seller.
    async fn find_by_seller(&self, seller_guid: u32) -> Result<Vec<AuctionRow>> {
        sqlx::query_as::<_, AuctionRow>(
            r#"SELECT id, house_id, item_guid, item_id, seller_guid, buyout_price, expire_time, buyer_guid, last_bid, start_bid, deposit FROM auction WHERE seller_guid = ?"#,
        )
        .bind(seller_guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch auctions by seller GUID")
    }

    /// Find all auctions with a specific bidder.
    async fn find_by_bidder(&self, bidder_guid: u32) -> Result<Vec<AuctionRow>> {
        sqlx::query_as::<_, AuctionRow>(
            r#"SELECT id, house_id, item_guid, item_id, seller_guid, buyout_price, expire_time, buyer_guid, last_bid, start_bid, deposit FROM auction WHERE buyer_guid = ?"#,
        )
        .bind(bidder_guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch auctions by bidder GUID")
    }

    /// Find all active auctions (not expired).
    async fn find_active_auctions(&self) -> Result<Vec<AuctionRow>> {
        sqlx::query_as::<_, AuctionRow>(
            r#"SELECT id, house_id, item_guid, item_id, seller_guid, buyout_price, expire_time, buyer_guid, last_bid, start_bid, deposit FROM auction WHERE expire_time > UNIX_TIMESTAMP()"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch active auctions")
    }

    /// Find active auctions for a specific house with seller account info
    async fn find_active_by_house_with_account(
        &self,
        house_id: u32,
    ) -> Result<Vec<AuctionWithAccountRow>> {
        sqlx::query_as::<_, AuctionWithAccountRow>(
            r#"SELECT a.id, a.house_id, a.item_guid, a.item_id, a.seller_guid, a.buyout_price,
                      a.expire_time, a.buyer_guid, a.last_bid, a.start_bid, a.deposit,
                      c.account
               FROM auction a
               INNER JOIN characters c ON c.guid = a.seller_guid
               WHERE a.house_id = ? AND a.expire_time > UNIX_TIMESTAMP()"#,
        )
        .bind(house_id)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch active auctions with account info")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Create a new auction.
    async fn create_auction(&self, auction: &AuctionRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO auction (id, house_id, item_guid, item_id, seller_guid, buyout_price, expire_time, buyer_guid, last_bid, start_bid, deposit) 
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(auction.id)
        .bind(auction.house_id)
        .bind(auction.item_guid)
        .bind(auction.item_id)
        .bind(auction.seller_guid)
        .bind(auction.buyout_price)
        .bind(auction.expire_time)
        .bind(auction.buyer_guid)
        .bind(auction.last_bid)
        .bind(auction.start_bid)
        .bind(auction.deposit)
        .execute(&*self.pool)
        .await
        .context("Failed to create auction")?;

        Ok(())
    }

    /// Update an existing auction.
    async fn update_auction(&self, auction: &AuctionRow) -> Result<()> {
        sqlx::query(
            r#"UPDATE auction SET house_id = ?, item_guid = ?, item_id = ?, seller_guid = ?, buyout_price = ?, expire_time = ?, buyer_guid = ?, last_bid = ?, start_bid = ?, deposit = ? WHERE id = ?"#,
        )
        .bind(auction.house_id)
        .bind(auction.item_guid)
        .bind(auction.item_id)
        .bind(auction.seller_guid)
        .bind(auction.buyout_price)
        .bind(auction.expire_time)
        .bind(auction.buyer_guid)
        .bind(auction.last_bid)
        .bind(auction.start_bid)
        .bind(auction.deposit)
        .bind(auction.id)
        .execute(&*self.pool)
        .await
        .context("Failed to update auction")?;

        Ok(())
    }

    /// Update the bid on an auction.
    async fn update_bid(&self, auction_id: u32, bidder_guid: u32, new_bid: i32) -> Result<()> {
        sqlx::query(r#"UPDATE auction SET buyer_guid = ?, last_bid = ? WHERE id = ?"#)
            .bind(bidder_guid)
            .bind(new_bid)
            .bind(auction_id)
            .execute(&*self.pool)
            .await
            .context("Failed to update auction bid")?;

        Ok(())
    }

    /// Delete an auction.
    async fn delete_auction(&self, auction_id: u32) -> Result<()> {
        sqlx::query("DELETE FROM auction WHERE id = ?")
            .bind(auction_id)
            .execute(&*self.pool)
            .await
            .context("Failed to delete auction")?;

        Ok(())
    }

    /// Delete expired auctions and return count of deleted rows.
    async fn delete_expired_auctions(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM auction WHERE expire_time <= UNIX_TIMESTAMP()")
            .execute(&*self.pool)
            .await
            .context("Failed to delete expired auctions")?;

        Ok(result.rows_affected())
    }
}
