//! Auction House Manager

use anyhow::{Context, Result};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{error, info};

use crate::shared::database::characters::models::auction::{AuctionItemLoadRow, AuctionRow};
use crate::shared::database::characters::models::mail::MailRow;
use crate::shared::database::characters::repositories::auction_repository_trait::AuctionRepositoryTrait;
use crate::shared::database::characters::repositories::mail_repository_trait::MailRepositoryTrait;
use crate::shared::database::characters::repositories::CharacterRepository;
use crate::shared::game::auction::AuctionEntry;
use crate::shared::game::chat::Team;
use crate::shared::game::mail::{MailCheckMask, MailMessageType, MailStationery};
use crate::shared::protocol::{HighGuid, ObjectGuid};
use crate::world::dbc::structures::AuctionHouseEntry;
use crate::world::dbc::DbcManager;
use crate::world::game::auction::parsing::{parse_enchantments, parse_spell_charges};
use crate::world::game::items::manager::ItemTemplate;
use crate::world::game::items::Item;
use crate::world::game::ItemManager;

/// Goblin (neutral) auction house id used when sending cancel mail for invalid house_id rows.
const GOBLIN_AUCTION_HOUSE_ID: u32 = 7;

/// Patch version below which unlinked auction houses are allowed (1.9).
const WOW_PATCH_109: u32 = 109;

/// Mail subject suffix constants (Mail.h AuctionAction enum).
const AUCTION_SUCCESSFUL: u32 = 2;
const AUCTION_CANCELED: u32 = 5;

const MAIL_AUCTION_EXPIRE_SECS: i64 = 30 * 24 * 60 * 60;

/// In-memory auction house object (one per linked/cross-faction/unlinked partition).
pub struct AuctionHouseObject {
    auctions: DashMap<u32, AuctionEntry>,
}

impl AuctionHouseObject {
    pub fn new() -> Self {
        Self {
            auctions: DashMap::new(),
        }
    }

    pub fn add_auction(&self, auction: AuctionEntry) {
        self.auctions.insert(auction.id, auction);
    }

    pub fn auction_count(&self) -> usize {
        self.auctions.len()
    }
}

struct BarGoLink {
    total: u32,
    current: u32,
}

impl BarGoLink {
    fn new(total: u32) -> Self {
        Self { total, current: 0 }
    }

    fn step(&mut self) {
        self.current = self.current.saturating_add(1);
        let _ = (self.current, self.total);
    }
}

pub struct AuctionHouseManager {
    auction_repo: Arc<dyn AuctionRepositoryTrait>,
    character_repo: Arc<CharacterRepository>,
    mail_repo: Arc<dyn MailRepositoryTrait>,
    dbc: Arc<RwLock<DbcManager>>,
    item_mgr: Arc<ItemManager>,
    auction_items: DashMap<u32, Arc<Item>>,
    auction_houses: DashMap<u32, Arc<AuctionHouseObject>>,
}

impl AuctionHouseManager {
    pub fn new(
        auction_repo: Arc<dyn AuctionRepositoryTrait>,
        character_repo: Arc<CharacterRepository>,
        mail_repo: Arc<dyn MailRepositoryTrait>,
        dbc: Arc<RwLock<DbcManager>>,
        item_mgr: Arc<ItemManager>,
    ) -> Self {
        Self {
            auction_repo,
            character_repo,
            mail_repo,
            dbc,
            item_mgr,
            auction_items: DashMap::new(),
            auction_houses: DashMap::new(),
        }
    }

    /// Initialize per-house auction maps from DBC (must run before load_auctions).
    pub fn load_auction_houses(
        &self,
        allow_cross_faction_auction: bool,
        unlinked_auction_houses: bool,
        wow_patch: u32,
    ) -> Result<()> {
        let dbc = self.dbc.read();
        let entries: Vec<AuctionHouseEntry> = dbc
            .get_all_auction_houses()
            .map(|(_, entry)| entry.clone())
            .collect();
        drop(dbc);

        if entries.is_empty() {
            return Ok(());
        }

        if allow_cross_faction_auction {
            let shared = Arc::new(AuctionHouseObject::new());
            for entry in entries {
                self.auction_houses
                    .insert(entry.house_id, Arc::clone(&shared));
            }
        } else if unlinked_auction_houses && wow_patch < WOW_PATCH_109 {
            for entry in entries {
                self.auction_houses
                    .insert(entry.house_id, Arc::new(AuctionHouseObject::new()));
            }
        } else {
            let alliance = Arc::new(AuctionHouseObject::new());
            let horde = Arc::new(AuctionHouseObject::new());
            let neutral = Arc::new(AuctionHouseObject::new());

            for entry in entries {
                let object = match get_auction_house_team(entry.house_id) {
                    Team::Alliance => Arc::clone(&alliance),
                    Team::Horde => Arc::clone(&horde),
                    _ => Arc::clone(&neutral),
                };
                self.auction_houses.insert(entry.house_id, object);
            }
        }

        Ok(())
    }

    pub async fn load_auction_items(&self) -> Result<()> {
        let query_result = self.auction_repo.find_all_items_for_load().await;

        // TODO: C++ treats null QueryResult the same as empty table; query failures follow that path too.
        let rows = match query_result {
            Ok(rows) => rows,
            Err(_) => {
                let mut bar = BarGoLink::new(1);
                bar.step();
                info!("");
                info!(">> Loaded 0 auction items");
                return Ok(());
            }
        };

        let mut bar = BarGoLink::new(rows.len() as u32);
        let mut count: u32 = 0;

        for row in rows {
            bar.step();

            let item_guid = row.item_guid;
            let item_id = row.item_id;

            let proto = self.item_mgr.get_template(item_id);

            if proto.is_none() {
                error!(
                    "AuctionHouseMgr::LoadAuctionItems: Unknown item (GUID: {} id: #{}) in auction, skipped.",
                    item_guid, item_id
                );
                continue;
            }
            let proto = proto.unwrap();

            let Some(item) = Self::load_auction_item_from_row(item_guid, item_id, &row, &proto) else {
                continue;
            };

            self.add_a_item(Arc::new(item));
            count = count.saturating_add(1);
        }

        info!("");
        info!(">> Loaded {} auction items", count);
        Ok(())
    }

    pub async fn load_auctions(&self) -> Result<()> {
        let query_result = self.auction_repo.find_all_for_load().await;

        // TODO: C++ treats null QueryResult the same as empty table; query failures follow that path too.
        let rows = match query_result {
            Ok(rows) => rows,
            Err(_) => {
                let mut bar = BarGoLink::new(1);
                bar.step();
                info!("");
                info!(">> Loaded 0 auctions. DB table `auction` is empty.");
                return Ok(());
            }
        };

        let mut bar = BarGoLink::new(rows.len() as u32);
        let mut count: u32 = 0;

        for row in rows {
            bar.step();

            let house_id = row.house_id;
            let item_guid_low = row.item_guid;
            let seller_guid = ObjectGuid::from_low(row.seller_guid);
            let bidder_guid = if row.buyer_guid == 0 {
                ObjectGuid::empty()
            } else {
                ObjectGuid::from_low(row.buyer_guid)
            };

            let mut auction = AuctionEntry {
                id: row.id,
                house_id,
                item_guid: ObjectGuid::new_without_entry(HighGuid::Item, item_guid_low),
                item_template: row.item_id,
                seller_guid,
                seller_account: 0,
                start_bid: row.start_bid as u32,
                current_bid: row.last_bid as u32,
                buyout_price: row.buyout_price as u32,
                expire_time: row.expire_time as u64,
                bidder_guid,
                deposit: row.deposit as u32,
                deposit_time: 0,
                locked_ip_address: String::new(),
            };

            auction.seller_account = self.get_player_account_id_by_guid(row.seller_guid).await;

            let p_item = self.get_a_item(item_guid_low);

            if p_item.is_none() {
                self.delete_auction_from_db(auction.id).await?;
                error!(
                    "Auction {} has not a existing item : {}, deleted",
                    auction.id, item_guid_low
                );
                continue;
            }
            let p_item = p_item.unwrap();

            let auction_house_entry = self.get_auction_house_entry(house_id);

            if auction_house_entry.is_none() {
                let goblin_house = self.get_auction_house_entry(GOBLIN_AUCTION_HOUSE_ID);
                auction.house_id = GOBLIN_AUCTION_HOUSE_ID;

                let subject = format!("{}:0:{}", auction.item_template, AUCTION_CANCELED);

                // TODO: fix body — C++ passes empty mail body string.
                // TODO: Ownership/lifetime of pItem after MailDraft(...).AddItem(pItem) is not specified.
                self.send_auction_mail_to_owner(
                    &subject,
                    "",
                    &auction,
                    &p_item,
                    goblin_house,
                    MailCheckMask::from(MailCheckMask::COPIED),
                )
                .await?;

                self.remove_a_item(item_guid_low);
                self.delete_auction_from_db(auction.id).await?;
                continue;
            }

            let auction_house_entry = auction_house_entry.unwrap();
            auction.house_id = auction_house_entry.house_id;

            let Some(auctions_map) = self.get_auctions_map(&auction_house_entry) else {
                error!(
                    "Auction {} references house {} with no in-memory map; skipped",
                    auction.id, auction_house_entry.house_id
                );
                continue;
            };
            auctions_map.add_auction(auction);
            count = count.saturating_add(1);
        }

        info!("");
        info!(">> Loaded {} auctions", count);
        Ok(())
    }

    fn get_a_item(&self, guid: u32) -> Option<Arc<Item>> {
        self.auction_items
            .get(&guid)
            .map(|entry| Arc::clone(entry.value()))
    }

    fn add_a_item(&self, item: Arc<Item>) {
        let guid_low = item.guid.low();
        assert!(guid_low != 0);
        assert!(
            !self.auction_items.contains_key(&guid_low),
            "duplicate auction item GUID {guid_low}"
        );
        self.auction_items.insert(guid_low, item);
    }

    fn load_auction_item_from_row(
        item_guid: u32,
        item_id: u32,
        row: &AuctionItemLoadRow,
        proto: &ItemTemplate,
    ) -> Option<Item> {
        let guid = ObjectGuid::new_without_entry(HighGuid::Item, item_guid);
        let owner_guid = ObjectGuid::empty();

        let enchantments = parse_enchantments(&row.enchantments);
        let creator_guid = if row.creator_guid != 0 {
            Some(ObjectGuid::new_without_entry(
                HighGuid::Player,
                row.creator_guid,
            ))
        } else {
            None
        };
        let gift_creator_guid = if row.gift_creator_guid != 0 {
            Some(ObjectGuid::new_without_entry(
                HighGuid::Player,
                row.gift_creator_guid,
            ))
        } else {
            None
        };

        // TODO: Exact Item::LoadFromDB validation/failure conditions are not yet ported.
        let _ = row.text;
        Some(Item::from_db_row(
            guid,
            item_id,
            row.count,
            owner_guid,
            0,
            0,
            row.flags,
            row.durability as u32,
            proto.max_durability,
            enchantments,
            row.random_property_id as i32,
            creator_guid,
            gift_creator_guid,
            row.duration as u32,
            parse_spell_charges(row.charges.as_deref()),
        ))
    }

    fn remove_a_item(&self, guid: u32) -> bool {
        self.auction_items.remove(&guid).is_some()
    }

    async fn get_player_account_id_by_guid(&self, guid: u32) -> u32 {
        match self.character_repo.find_by_guid(guid).await {
            Ok(Some(character)) => character.account,
            _ => 0,
        }
    }

    fn get_auction_house_entry(&self, house_id: u32) -> Option<AuctionHouseEntry> {
        self.dbc.read().get_auction_house(house_id).cloned()
    }

    fn get_auctions_map(&self, house: &AuctionHouseEntry) -> Option<Arc<AuctionHouseObject>> {
        self.auction_houses
            .get(&house.house_id)
            .map(|entry| Arc::clone(entry.value()))
    }

    /// Maps a creature faction template ID to an auction house ID.
    ///
    /// Mirrors C++ `AuctionHouseMgr::GetAuctionHouseId` (AuctionHouseMgr.cpp:522-578).
    pub fn get_auction_house_id_from_faction_template(faction_template_id: u32) -> u32 {
        match faction_template_id {
            11 | 12 => 1,   // Human
            29 | 85 => 6,   // Orc
            55 | 57 => 2,   // Dwarf
            68 | 71 => 4,   // Undead
            79 | 80 => 3,   // Night Elf
            104 | 105 => 5, // Tauren
            120 => 7,       // Booty Bay
            474 => 7,       // Gadgetzan
            534 => 2,       // Alliance Generic
            855 => 7,       // Everlook
            _ => {
                // Fallback: use ourMask to determine alliance/horde/neutral
                // FACTION_MASK_ALLIANCE = 2, FACTION_MASK_HORDE = 4
                // Since we don't have the FactionTemplate entry here without the DBC,
                // callers that need the exact fallback should use the DBC lookup.
                7 // goblin (neutral) as default
            }
        }
    }

    /// Returns the auction house entry for a player, based on team and access mode.
    ///
    /// Mirrors C++ `AuctionHouseMgr::GetAuctionHouseEntry` player branch (lines 594-610).
    pub fn get_auction_house_for_player(
        &self,
        team: crate::shared::game::chat::Team,
        auction_access_mode: i8,
    ) -> Option<AuctionHouseEntry> {
        let house_id = if auction_access_mode > 0 {
            7 // neutral
        } else {
            match team {
                crate::shared::game::chat::Team::Alliance => {
                    if auction_access_mode == 0 { 1 } else { 6 }
                }
                crate::shared::game::chat::Team::Horde => {
                    if auction_access_mode == 0 { 6 } else { 1 }
                }
                _ => 7,
            }
        };
        self.get_auction_house_entry(house_id)
    }

    /// Returns the auction house entry for an NPC, based on faction template.
    ///
    /// Mirrors C++ `AuctionHouseMgr::GetAuctionHouseEntry` creature branch (lines 586-592).
    pub fn get_auction_house_for_npc(
        &self,
        faction_template_id: u32,
    ) -> Option<AuctionHouseEntry> {
        let house_id =
            Self::get_auction_house_id_from_faction_template(faction_template_id);
        self.get_auction_house_entry(house_id)
    }

    /// Calculates auction deposit for a given item, time, and house entry.
    ///
    /// Mirrors C++ `AuctionHouseMgr::GetAuctionDeposit` (AuctionHouseMgr.cpp:98-110).
    /// Preserved behaviour claims:
    /// - integer division `(time / MIN_AUCTION_TIME)` before float cast → zero when time < 7200s.
    /// - unsigned wrapping on inner product `sell_price * count * (time / MIN_AUCTION_TIME)`.
    /// - deposit is scaled by `entry.deposit_percent / 100.0f`.
    /// - min deposit floor from config.
    /// - final truncation via `u32` cast (fractional copper discarded).
    pub fn get_auction_deposit(
        &self,
        entry: &AuctionHouseEntry,
        time: u32,
        item: &Item,
        min_deposit: u32,
        rate: f32,
    ) -> u32 {
        const MIN_AUCTION_TIME: u32 = 2 * 3600; // 2 hours

        let proto = match self.item_mgr.get_template(item.entry) {
            Some(t) => t,
            None => return 0,
        };

        // C++ computes SellPrice * GetCount() * (time / MIN_AUCTION_TIME) as uint32 first.
        let base = proto
            .sell_price
            .wrapping_mul(item.count)
            .wrapping_mul(time / MIN_AUCTION_TIME);
        let mut deposit = base as f32;

        deposit = deposit * entry.deposit_percent as f32 / 100.0;

        let min_deposit_f = min_deposit as f32;
        if deposit < min_deposit_f {
            deposit = min_deposit_f;
        }

        (deposit * rate) as u32
    }

    async fn delete_auction_from_db(&self, auction_id: u32) -> Result<()> {
        self.auction_repo
            .delete_auction(auction_id)
            .await
            .context("Failed to delete auction from database")
    }

    /// Sends the "auction sold" mail to the seller with the profit (bid + deposit - cut).
    ///
    /// Mirrors C++ AuctionHouseMgr::SendAuctionSuccessfulMail. The online-owner
    /// packet notification (SendAuctionOwnerNotification) is the caller's responsibility
    /// because this manager has no session/broadcast access.
    pub async fn send_auction_successful_mail(
        &self,
        auction: &AuctionEntry,
        cut_percent: f32,
        cut_rate: f32,
    ) -> Result<()> {
        let owner_guid_low = auction.seller_guid.low();
        let owner_acc_id = self.get_player_account_id_by_guid(owner_guid_low).await;

        // No known owner account — nothing to deliver to.
        if owner_acc_id == 0 {
            return Ok(());
        }

        let subject = format!("{}:0:{}", auction.item_template, AUCTION_SUCCESSFUL);

        let auction_cut = auction.get_auction_cut(cut_percent, cut_rate);
        // Body format matches C++: bidder as 16-wide right-aligned hex, then decimal fields.
        let body = format!(
            "{:>16x}:{}:{}:{}:{}",
            auction.bidder_guid.low(),
            auction.current_bid,
            auction.buyout_price,
            auction.deposit,
            auction_cut,
        );

        tracing::debug!("AuctionSuccessful body string : {}", body);

        // TODO: if owner is online, send SendAuctionOwnerNotification(auction, sold=true)
        // via broadcast_mgr — needs caller to handle since manager has no session access.

        // profit = bid + deposit - cut; wrapping matches C++ uint32 arithmetic.
        let profit = auction
            .current_bid
            .wrapping_add(auction.deposit)
            .wrapping_sub(auction_cut);

        let item_text_id = match self.mail_repo.create_item_text(&body).await {
            Ok(id) => id,
            Err(e) => {
                error!("SendAuctionSuccessfulMail: failed to create item text: {e}");
                0
            }
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mail_row = MailRow {
            id: 0,
            message_type: MailMessageType::Auction as u8,
            stationery: MailStationery::Auction as i8,
            mail_template_id: 0,
            sender_guid: auction.id,
            receiver_guid: owner_guid_low,
            subject: Some(subject),
            item_text_id,
            has_items: 0,
            expire_time: now + MAIL_AUCTION_EXPIRE_SECS,
            deliver_time: now,
            money: profit,
            cod: 0,
            checked: MailCheckMask::COPIED,
        };

        // TODO need a mail system
        if let Err(e) = self.mail_repo.create(&mail_row).await {
            error!("SendAuctionSuccessfulMail: failed to create mail: {e}");
        }

        Ok(())
    }

    async fn send_auction_mail_to_owner(
        &self,
        subject: &str,
        body: &str,
        auction: &AuctionEntry,
        item: &Item,
        _house_entry: Option<AuctionHouseEntry>,
        _check_mask: MailCheckMask,
    ) -> Result<()> {
        let _ = (subject, body, auction, item);
        // TODO: Implement MailDraft::SendMailTo semantics (partial failure behavior not defined in C++ LoadAuctions).
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn insert_item_for_test(&self, item: Arc<Item>) {
        self.add_a_item(item);
    }

    #[cfg(test)]
    pub(crate) fn item_count(&self) -> usize {
        self.auction_items.len()
    }

    #[cfg(test)]
    pub(crate) fn auction_count_for_house(&self, house_id: u32) -> usize {
        self.auction_houses
            .get(&house_id)
            .map(|house| house.auction_count())
            .unwrap_or(0)
    }

    #[cfg(test)]
    pub(crate) fn has_auction_house_map(&self, house_id: u32) -> bool {
        self.auction_houses.contains_key(&house_id)
    }
}

/// Maps auction house id to faction team (C++ GetAuctionHouseTeam).
fn get_auction_house_team(house_id: u32) -> Team {
    match house_id {
        1 | 2 | 3 => Team::Alliance,
        4 | 5 | 6 => Team::Horde,
        _ => Team::None,
    }
}

#[cfg(test)]
mod team_tests {
    use super::*;

    #[test]
    fn get_auction_house_team_maps_faction_houses() {
        assert_eq!(get_auction_house_team(1), Team::Alliance);
        assert_eq!(get_auction_house_team(3), Team::Alliance);
        assert_eq!(get_auction_house_team(4), Team::Horde);
        assert_eq!(get_auction_house_team(6), Team::Horde);
        assert_eq!(get_auction_house_team(7), Team::None);
        assert_eq!(get_auction_house_team(99), Team::None);
    }
}
