#[derive(Debug, Clone)]
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

/// Row from auction JOIN item_instance for LoadAuctionItems.
#[derive(Debug, Clone)]
pub struct AuctionItemLoadRow {
    pub creator_guid: u32,
    pub gift_creator_guid: u32,
    pub count: u32,
    pub duration: i32,
    pub charges: Option<String>,
    pub flags: u32,
    pub enchantments: String,
    pub random_property_id: i16,
    pub durability: u16,
    pub text: u32,
    pub item_guid: u32,
    pub item_id: u32,
}

/// Auction row with seller account info (joined from characters table)
#[derive(Debug, Clone)]
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

impl TryFrom<crate::database::characters::PgAuctionRow> for AuctionRow {
    type Error = anyhow::Error;

    fn try_from(row: crate::database::characters::PgAuctionRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id.try_into()?,
            house_id: row.house_id.try_into()?,
            item_guid: row.item_guid.try_into()?,
            item_id: row.item_id.try_into()?,
            seller_guid: row.seller_guid.try_into()?,
            buyout_price: row.buyout_price,
            expire_time: row.expire_time,
            buyer_guid: row.buyer_guid.try_into()?,
            last_bid: row.last_bid,
            start_bid: row.start_bid,
            deposit: row.deposit,
        })
    }
}

impl From<&AuctionRow> for crate::database::characters::PgAuctionRow {
    fn from(row: &AuctionRow) -> Self {
        Self {
            id: row.id.into(),
            house_id: row.house_id.into(),
            item_guid: row.item_guid.into(),
            item_id: row.item_id.into(),
            seller_guid: row.seller_guid.into(),
            buyout_price: row.buyout_price,
            expire_time: row.expire_time,
            buyer_guid: row.buyer_guid.into(),
            last_bid: row.last_bid,
            start_bid: row.start_bid,
            deposit: row.deposit,
        }
    }
}

impl TryFrom<crate::database::characters::PgAuctionItemLoadRow> for AuctionItemLoadRow {
    type Error = anyhow::Error;

    fn try_from(
        row: crate::database::characters::PgAuctionItemLoadRow,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            creator_guid: row.creator_guid.try_into()?,
            gift_creator_guid: row.gift_creator_guid.try_into()?,
            count: row.count.try_into()?,
            duration: row.duration,
            charges: row.charges,
            flags: row.flags.try_into()?,
            enchantments: row.enchantments,
            random_property_id: row.random_property_id,
            durability: row.durability.try_into()?,
            text: row.text.try_into()?,
            item_guid: row.item_guid.try_into()?,
            item_id: row.item_id.try_into()?,
        })
    }
}

impl TryFrom<crate::database::characters::PgAuctionWithAccountRow> for AuctionWithAccountRow {
    type Error = anyhow::Error;

    fn try_from(
        row: crate::database::characters::PgAuctionWithAccountRow,
    ) -> Result<Self, Self::Error> {
        let auction = AuctionRow::try_from(row.auction)?;
        Ok(Self {
            id: auction.id,
            house_id: auction.house_id,
            item_guid: auction.item_guid,
            item_id: auction.item_id,
            seller_guid: auction.seller_guid,
            buyout_price: auction.buyout_price,
            expire_time: auction.expire_time,
            buyer_guid: auction.buyer_guid,
            last_bid: auction.last_bid,
            start_bid: auction.start_bid,
            deposit: auction.deposit,
            account: row.account.try_into()?,
        })
    }
}
