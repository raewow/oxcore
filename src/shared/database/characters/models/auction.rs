use sqlx::FromRow;

#[derive(FromRow, Debug, Clone)]
pub struct AuctionRow {
    pub id: u32,
    pub house_id: u32,
    pub item_guid: u32,
    pub item_id: u32,
    pub seller_guid: u32,
    pub buyout_price: i32,
    pub expire_time: i64,
    pub buyer_guid: u32,
    pub last_bid: i32,
    pub start_bid: i32,
    pub deposit: i32,
}

/// Auction row with seller account info (joined from characters table)
#[derive(FromRow, Debug, Clone)]
pub struct AuctionWithAccountRow {
    pub id: u32,
    pub house_id: u32,
    pub item_guid: u32,
    pub item_id: u32,
    pub seller_guid: u32,
    pub buyout_price: i32,
    pub expire_time: i64,
    pub buyer_guid: u32,
    pub last_bid: i32,
    pub start_bid: i32,
    pub deposit: i32,
    pub account: u32,
}
