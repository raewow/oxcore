#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum AuctionHouseId {
    Alliance = 2,
    Horde = 6,
    Neutral = 7,
}

impl AuctionHouseId {
    pub fn from_team(team: crate::shared::game::chat::Team) -> Self {
        match team {
            crate::shared::game::chat::Team::Alliance => AuctionHouseId::Alliance,
            crate::shared::game::chat::Team::Horde => AuctionHouseId::Horde,
            _ => AuctionHouseId::Neutral,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AuctionError {
    Ok = 0,
    Inventory = 1,
    DatabaseError = 2,
    NotEnoughMoney = 3,
    ItemNotFound = 4,
    HigherBid = 5,
    BidIncrement = 7,
    BidOwn = 10,
    RestrictedAccount = 13,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AuctionAction {
    Started = 0,
    Removed = 1,
    BidPlaced = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AuctionQueryType {
    List = 0,
    ListOwner = 1,
    ListBidder = 2,
}

#[derive(Debug, Clone)]
pub struct AuctionEntry {
    pub id: u32,
    pub house_id: u32,
    pub item_guid: crate::shared::protocol::ObjectGuid,
    pub item_template: u32,
    pub seller_guid: crate::shared::protocol::ObjectGuid,
    pub seller_account: u32,
    pub start_bid: u32,
    pub current_bid: u32,
    pub buyout_price: u32,
    pub expire_time: u64,
    pub bidder_guid: crate::shared::protocol::ObjectGuid,
    pub deposit: u32,
    pub deposit_time: u64,
    pub locked_ip_address: String,
}

impl AuctionEntry {
    pub fn new(
        id: u32,
        house_id: u32,
        item_guid: crate::shared::protocol::ObjectGuid,
        item_template: u32,
        seller_guid: crate::shared::protocol::ObjectGuid,
        seller_account: u32,
        start_bid: u32,
        buyout_price: u32,
        expire_time: u64,
        deposit: u32,
    ) -> Self {
        let deposit_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            id,
            house_id,
            item_guid,
            item_template,
            seller_guid,
            seller_account,
            start_bid,
            current_bid: start_bid,
            buyout_price,
            expire_time,
            bidder_guid: crate::shared::protocol::ObjectGuid::empty(),
            deposit,
            deposit_time,
            locked_ip_address: String::new(),
        }
    }

    pub fn is_expired(&self, current_time: u64) -> bool {
        current_time >= self.expire_time
    }

    pub fn has_bid(&self) -> bool {
        !self.bidder_guid.is_empty()
    }

    pub fn get_auction_cut(&self, cut_percent: f32, rate: f32) -> u32 {
        ((self.current_bid as f32 * cut_percent * rate) / 100.0) as u32
    }

    pub fn get_outbid_amount(&self) -> u32 {
        let outbid = (self.current_bid / 100) * 5;
        if outbid == 0 {
            1
        } else {
            outbid
        }
    }

    pub fn is_available_for(&self, player_ip: &str, current_time: u64) -> bool {
        if !self.locked_ip_address.is_empty() {
            if current_time >= self.deposit_time + 300 {
                return true;
            }
            return self.locked_ip_address == player_ip;
        }
        true
    }

    pub fn calculate_min_bid(&self) -> u32 {
        if self.current_bid == self.start_bid {
            self.start_bid
        } else {
            self.current_bid + self.get_outbid_amount()
        }
    }

    pub fn set_locked_ip(&mut self, ip: String) {
        self.locked_ip_address = ip;
    }
}
