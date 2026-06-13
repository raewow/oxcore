use super::super::models::auction::*;
use anyhow::Result;
use async_trait::async_trait;

/// Trait abstraction for auction repository operations.
/// Enables dependency injection and mocking for tests.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AuctionRepositoryTrait: Send + Sync {
    // ========== QUERY METHODS (Read Operations) ==========

    /// Get the maximum auction ID (for generating next ID on startup)
    async fn get_max_auction_id(&self) -> Result<Option<u32>>;

    /// Find an auction by ID
    async fn find_by_id(&self, auction_id: u32) -> Result<Option<AuctionRow>>;

    /// Find all auctions for a specific house
    async fn find_by_house(&self, house_id: u32) -> Result<Vec<AuctionRow>>;

    /// Find all auctions by a specific seller
    async fn find_by_seller(&self, seller_guid: u32) -> Result<Vec<AuctionRow>>;

    /// Find all auctions with a specific bidder
    async fn find_by_bidder(&self, bidder_guid: u32) -> Result<Vec<AuctionRow>>;

    /// Find all active (non-expired) auctions
    async fn find_active_auctions(&self) -> Result<Vec<AuctionRow>>;

    /// Find active auctions for a specific house with seller account info
    async fn find_active_by_house_with_account(
        &self,
        house_id: u32,
    ) -> Result<Vec<AuctionWithAccountRow>>;

    /// Load all auction rows for world bootstrap (LoadAuctions).
    async fn find_all_for_load(&self) -> Result<Vec<AuctionRow>>;

    /// Load auction item rows joined with item_instance for bootstrap (LoadAuctionItems).
    async fn find_all_items_for_load(&self) -> Result<Vec<AuctionItemLoadRow>>;

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Create a new auction
    async fn create_auction(&self, auction: &AuctionRow) -> Result<()>;

    /// Update an existing auction
    async fn update_auction(&self, auction: &AuctionRow) -> Result<()>;

    /// Update the bid on an auction
    async fn update_bid(&self, auction_id: u32, bidder_guid: u32, new_bid: i32) -> Result<()>;

    /// Delete an auction
    async fn delete_auction(&self, auction_id: u32) -> Result<()>;

    /// Delete expired auctions and return count of deleted rows
    async fn delete_expired_auctions(&self) -> Result<u64>;
}
