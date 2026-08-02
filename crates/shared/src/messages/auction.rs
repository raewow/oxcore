//! Auction system message structs
//!
//! This module contains type-safe message structures for all auction-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`MsgAuctionHello`] - Open auction house UI
//! - [`SmsgAuctionCommandResult`] - Result of an auction action
//! - [`SmsgAuctionListResult`] - Auction search results
//! - [`SmsgAuctionOwnerListResult`] - Auctions owned by the player
//! - [`SmsgAuctionBidderListResult`] - Auctions the player is bidding on
//! - [`SmsgAuctionBidderNotification`] - Notification of auction bid result
//! - [`SmsgAuctionOwnerNotification`] - Notification to auction owner
//! - [`SmsgAuctionRemovedNotification`] - Notification that an auction was removed

use crate::game::{AuctionAction, AuctionEntry, AuctionError, InventoryResult};
use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::ObjectGuid;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;

// ========== MODERN ENCODING HELPERS ==========

/// A 1.12 inventory-result code as 1.14 numbers it.
///
/// The enum was renumbered. 1.14 inserted `CantTradeGold` at 29, so every code from
/// `NotEnoughMoney` upward kept its name and gained one. Casting a 1.12 value straight into a 1.14
/// field therefore shows the *neighbouring* error for the whole upper half of the range — "you
/// don't have enough money" arrives as "not a bag", "inventory is full" as "bank is full".
///
/// Written as a name-based match with no catch-all on purpose: adding a variant to the 1.12 enum
/// must break this build rather than silently pick up whichever number happens to follow.
fn modern_inventory_result(result: InventoryResult) -> i32 {
    use crate::game::InventoryResult::*;
    match result {
        // Identical in both protocols.
        Ok
        | CantEquipLevelI
        | CantEquipSkill
        | ItemDoesntGoToSlot
        | BagFull
        | NonEmptyBagOverOtherBag
        | CantTradeEquipBags
        | OnlyAmmoCanGoHere
        | NoRequiredProficiency
        | NoEquipmentSlotAvailable
        | YouCanNeverUseThatItem
        | YouCanNeverUseThatItem2
        | NoEquipmentSlotAvailable2
        | CantEquipWithTwohanded
        | CantDualWield
        | ItemDoesntGoIntoBag
        | ItemDoesntGoIntoBag2
        | CantCarryMoreOfThis
        | NoEquipmentSlotAvailable3
        | ItemCantStack
        | ItemCantBeEquipped
        | ItemsCantBeSwapped
        | SlotIsEmpty
        | ItemNotFound
        | CantDropSoulbound
        | OutOfRange
        | TriedToSplitMoreThanCount
        | CouldntSplitItems
        | MissingReagent => result as i32,

        // Shifted one higher by the `CantTradeGold` code 1.14 inserted at 29.
        NotEnoughMoney
        | NotABag
        | CanOnlyDoWithEmptyBags
        | DontOwnThatItem
        | CanEquipOnly1Quiver
        | MustPurchaseThatBagSlot
        | TooFarAwayFromBank
        | ItemLocked
        | YouAreStunned
        | YouAreDead
        | CantDoRightNow
        | IntBagError
        | CanEquipOnly1Bolt
        | CanEquipOnly1Ammopouch
        | StackableCantBeWrapped
        | EquippedCantBeWrapped
        | WrappedCantBeWrapped
        | BoundCantBeWrapped
        | UniqueCantBeWrapped
        | BagsCantBeWrapped
        | AlreadyLooted
        | InventoryFull
        | BankFull
        | ItemIsCurrentlySoldOut
        | BagFull3
        | ItemNotFound2
        | ItemCantStack2
        | BagFull4
        | ItemSoldOut
        | ObjectIsBusy
        | None
        | NotInCombat
        | NotWhileDisarmed
        | BagFull6
        | CantEquipRank
        | CantEquipReputation
        | TooManySpecialBags
        | LootCantLootThatNow => result as i32 + 1,
    }
}

/// A bare item id with no bonuses or modifications, which is all a 1.12 item has.
fn write_modern_item_instance(writer: &mut BitWriter, item_id: u32, random_property: u32) {
    writer.write_u32(item_id);
    writer.write_u32(0); // RandomPropertiesSeed
    writer.write_u32(random_property); // RandomPropertiesID
    writer.write_bit(false); // HasItemBonus
    writer.flush_bits();
    writer.write_bits(0, 6); // ItemModList count
    writer.flush_bits();
}

/// The 1.14 128-bit GUID for a game account.
///
/// Accounts are a global high type — no realm in the high half, the account id as the low half.
/// 1.12 has no GUID form for an account at all, so this is built rather than converted.
fn account_guid_128(account_id: u32) -> (u64, u64) {
    const WOW_ACCOUNT_HIGH_TYPE: u64 = 29;
    (WOW_ACCOUNT_HIGH_TYPE << 58, u64::from(account_id))
}

/// Nothing about this survives from vanilla's flat 14-field row. Every optional field is announced
/// in a leading bit run and then written, in a different order, after the fixed block — so the bits
/// and the payload have to agree exactly or the client reads the next row's fields as this one's.
///
/// **All four money values widen to u64.** Vanilla sends u32 copper; a 1.14 client reading a
/// 32-bit bid as 64 bits swallows the following field as its high word and shows an absurd price.
///
/// 1.12 has no item bonuses, gems, enchantments or bucket keys, so those counts are zero and the
/// matching bits are clear.
fn write_modern_auction_item(writer: &mut BitWriter, auction: &AuctionEntry, now: u64) {
    let has_bidder = !auction.bidder_guid.is_empty();

    writer.write_bit(true); // has Item
    writer.write_bits(0, 4); // Enchantments count
    writer.write_bits(0, 2); // Gems count
    writer.write_bit(true); // has MinBid
    writer.write_bit(true); // has MinIncrement
    writer.write_bit(true); // has BuyoutPrice
    writer.write_bit(false); // has UnitPrice -- commodities only, which Classic Era has none of
    writer.write_bit(false); // CensorServerSideInfo
    writer.write_bit(false); // CensorBidInfo
    writer.write_bit(false); // has AuctionBucketKey
    writer.write_bit(false); // has Creator -- 1.12 auctions do not record the crafter
    writer.write_bit(has_bidder);
    writer.write_bit(has_bidder); // has BidAmount
    writer.flush_bits();

    write_modern_item_instance(writer, auction.item_template, 0);

    writer.write_i32(1); // Count -- 1.12 auctions are always a single item
    writer.write_i32(0); // Charges
                         // The flag set a 1.14 client expects on a plain, non-commodity item listing; zero here makes
                         // the row render as an empty entry.
    writer.write_u32(196608); // Flags
    writer.write_u32(auction.id);

    let (high, low) = auction.seller_guid.to_guid128(DEFAULT_REALM_ID);
    writer.write_packed_guid_128(high, low); // Owner

    let time_left_ms = auction.expire_time.saturating_sub(now) * 1000;
    writer.write_i32(time_left_ms as i32); // DurationLeft, milliseconds
    writer.write_u8(0); // DeleteReason

    writer.write_u64(u64::from(auction.start_bid)); // MinBid
                                                    // The amount a competing bid must clear the current one by. `to_vanilla` puts the absolute
                                                    // minimum next bid in the equivalent slot; 1.14 wants the increment itself.
    let min_increment = if has_bidder {
        auction.get_outbid_amount()
    } else {
        0
    };
    writer.write_u64(u64::from(min_increment));
    writer.write_u64(u64::from(auction.buyout_price));

    // Only present because CensorServerSideInfo is clear.
    let (high, low) = auction.item_guid.to_guid128(DEFAULT_REALM_ID);
    writer.write_packed_guid_128(high, low); // ItemGuid
    let (high, low) = account_guid_128(auction.seller_account);
    writer.write_packed_guid_128(high, low); // OwnerAccountID
    writer.write_u32(auction.expire_time as u32); // EndTime, absolute

    if has_bidder {
        let (high, low) = auction.bidder_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Bidder
        writer.write_u64(u64::from(auction.current_bid)); // BidAmount
    }
}

/// Seconds since the epoch, right now.
fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// How long the client waits before it will re-run an auction search, in milliseconds.
///
/// 1.12 has no equivalent field — the 1.12 client throttles itself. Zero would let a modern client
/// hammer the search button, so the value the auction UI was designed around is sent instead.
const AUCTION_SEARCH_DELAY_MS: u32 = 300;

/// MSG_AUCTION_HELLO - Open the auction house UI for the player
///
/// Sent in response to the player interacting with an auctioneer NPC.
#[derive(Debug, Clone)]
pub struct MsgAuctionHello {
    /// GUID of the auctioneer NPC
    pub auctioneer_guid: ObjectGuid,
    /// Auction house ID (0 = Alliance, 1 = Horde, 2 = Neutral)
    pub house_id: u32,
}

impl ToWorldPacket for MsgAuctionHello {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_AUCTION_HELLO);
        packet.write_u64(self.auctioneer_guid.raw());
        packet.write_u32(self.house_id);
        packet
    }

    /// `AuctionHelloResponse` in 1.14 — a dedicated server opcode, where vanilla answers on the
    /// same bidirectional number the client asked with.
    ///
    /// Same two fields plus an `OpenForBusiness` bit that 1.12 never sends. It must be set: a clear
    /// bit tells the client the auction house is closed and it refuses to open the window at all.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.auctioneer_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(self.house_id);
        writer.write_bit(true); // OpenForBusiness
        writer.flush_bits();
        Some(writer.finish(Opcode::MSG_AUCTION_HELLO))
    }
}

/// SMSG_AUCTION_COMMAND_RESULT - Result of an auction action
///
/// Sent in response to auction actions like creating, bidding, or canceling.
///
/// Each variant encodes the exact payload required by its branch, preventing
/// silent omission of mandatory fields (e.g. `HigherBid` always requires
/// bidder, bid, and outbid data).
#[derive(Debug, Clone)]
pub enum SmsgAuctionCommandResult {
    /// AUCTION_OK with no extra fields (Action != BidPlaced)
    Ok {
        auction_id: u32,
        action: AuctionAction,
    },
    /// AUCTION_OK + BidPlaced: appends outbid amount
    OkBidPlaced { auction_id: u32, outbid: u32 },
    /// AUCTION_ERR_INVENTORY: appends inventory error code
    Inventory {
        auction_id: u32,
        action: AuctionAction,
        inventory_error: crate::game::InventoryResult,
    },
    /// AUCTION_ERR_HIGHER_BID: appends bidder GUID, bid, and outbid
    HigherBid {
        auction_id: u32,
        action: AuctionAction,
        bidder_guid: ObjectGuid,
        bid: u32,
        outbid: u32,
    },
    /// All other errors: only base fields (auction_id, action, error)
    Other {
        auction_id: u32,
        action: AuctionAction,
        error: AuctionError,
    },
}

impl SmsgAuctionCommandResult {
    pub fn auction_id(&self) -> u32 {
        match self {
            Self::Ok { auction_id, .. }
            | Self::OkBidPlaced { auction_id, .. }
            | Self::Inventory { auction_id, .. }
            | Self::HigherBid { auction_id, .. }
            | Self::Other { auction_id, .. } => *auction_id,
        }
    }

    pub fn action(&self) -> AuctionAction {
        match self {
            Self::Ok { action, .. }
            | Self::Inventory { action, .. }
            | Self::HigherBid { action, .. }
            | Self::Other { action, .. } => *action,
            Self::OkBidPlaced { .. } => AuctionAction::BidPlaced,
        }
    }

    pub fn error(&self) -> AuctionError {
        match self {
            Self::Ok { .. } | Self::OkBidPlaced { .. } => AuctionError::Ok,
            Self::Inventory { .. } => AuctionError::Inventory,
            Self::HigherBid { .. } => AuctionError::HigherBid,
            Self::Other { error, .. } => *error,
        }
    }
}

impl ToWorldPacket for SmsgAuctionCommandResult {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::with_capacity(Opcode::SMSG_AUCTION_COMMAND_RESULT, 16);
        packet.write_u32(self.auction_id());
        packet.write_u32(self.action() as u32);
        packet.write_u32(self.error() as u32);

        match self {
            Self::OkBidPlaced { outbid, .. } => {
                packet.write_u32(*outbid);
            }
            Self::Inventory {
                inventory_error, ..
            } => {
                packet.write_u32(*inventory_error as u32);
            }
            Self::HigherBid {
                bidder_guid,
                bid,
                outbid,
                ..
            } => {
                packet.write_u64(bidder_guid.raw());
                packet.write_u32(*bid);
                packet.write_u32(*outbid);
            }
            Self::Ok { .. } | Self::Other { .. } => {}
        }

        packet
    }

    /// `AuctionCommandResult` in 1.14.
    ///
    /// Vanilla's body is variable — which of the bidder GUID, bid, outbid and inventory error
    /// follow depends on the action and error. 1.14 writes **all** of them every time and lets the
    /// client read only the ones its error code cares about, so the branches here fill unused
    /// fields with zeroes instead of omitting them. A short body is read as a truncated packet.
    ///
    /// **Money widens to u64**: the bid and the outbid increment are 32-bit in vanilla, and a
    /// client reading 64 bits over vanilla's 32 takes the next field as the high word.
    ///
    /// The action and error enums are *not* renumbered — checked value by value, 1.12's
    /// `Started/Removed/BidPlaced` and its error list line up exactly with 1.14's — so both pass
    /// through unchanged. The inventory error does not; see [`modern_inventory_result`].
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.auction_id());
        writer.write_i32(self.action() as i32); // Command
        writer.write_i32(self.error() as i32); // ErrorCode

        let bag_result = match self {
            Self::Inventory {
                inventory_error, ..
            } => modern_inventory_result(*inventory_error),
            _ => 0,
        };
        writer.write_i32(bag_result);

        // Bidder, only carried by the higher-bid rejection.
        let bidder = match self {
            Self::HigherBid { bidder_guid, .. } => *bidder_guid,
            _ => ObjectGuid::empty(),
        };
        let (high, low) = bidder.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);

        // MinIncrement doubles as the "you were outbid by" amount on a successful bid.
        let (min_increment, money) = match self {
            Self::OkBidPlaced { outbid, .. } => (u64::from(*outbid), 0),
            Self::HigherBid { bid, outbid, .. } => (u64::from(*outbid), u64::from(*bid)),
            _ => (0, 0),
        };
        writer.write_u64(min_increment);
        writer.write_u64(money);
        writer.write_u32(AUCTION_SEARCH_DELAY_MS); // DesiredDelay

        Some(writer.finish(Opcode::SMSG_AUCTION_COMMAND_RESULT))
    }
}

/// SMSG_AUCTION_LIST_RESULT - Auction search results
///
/// Sent in response to an auction search query.
#[derive(Debug)]
pub struct SmsgAuctionListResult<'a> {
    /// Reference to array of auctions to send
    pub auctions: &'a [&'a AuctionEntry],
    /// Total number of auctions matching the search (for pagination)
    pub total_count: u32,
}

impl ToWorldPacket for SmsgAuctionListResult<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AUCTION_LIST_RESULT);
        let count = self.auctions.len().min(50) as u32;
        packet.write_u32(count);

        for auction in self.auctions.iter().take(50) {
            write_auction_list_item(&mut packet, auction);
        }

        packet.write_u32(self.total_count);
        packet
    }

    /// `AuctionListItemsResult` in 1.14.
    ///
    /// The header is reordered and grows: vanilla sends the count, then the rows, then the total.
    /// 1.14 sends count, total and a client-side search delay *before* any row — reading the rows
    /// where vanilla put them consumes the total as an item. It also carries an `OnlyUsable` echo,
    /// but **only when the result is non-empty**; writing it for an empty result adds a stray byte.
    fn to_modern(&self) -> Option<WorldPacket> {
        let now = now_unix();
        let auctions: Vec<&&AuctionEntry> = self.auctions.iter().take(50).collect();

        let mut writer = BitWriter::new();
        writer.write_i32(auctions.len() as i32);
        writer.write_i32(self.total_count as i32);
        writer.write_u32(AUCTION_SEARCH_DELAY_MS);
        if !auctions.is_empty() {
            // 1.12 does not echo the search filters back, and the client only uses this to keep its
            // own checkbox in sync.
            writer.write_u8(0); // OnlyUsable
        }

        for auction in auctions {
            write_modern_auction_item(&mut writer, auction, now);
        }

        Some(writer.finish(Opcode::SMSG_AUCTION_LIST_RESULT))
    }
}

/// SMSG_AUCTION_OWNER_LIST_RESULT - Auctions owned by the player
///
/// Sent in response to a request for the player's own auctions.
#[derive(Debug)]
pub struct SmsgAuctionOwnerListResult<'a> {
    /// Reference to array of auctions to send
    pub auctions: &'a [&'a AuctionEntry],
    /// Total number of auctions owned by the player
    pub total_count: u32,
}

impl ToWorldPacket for SmsgAuctionOwnerListResult<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AUCTION_OWNER_LIST_RESULT);
        let count = self.auctions.len().min(50) as u32;
        packet.write_u32(count);

        for auction in self.auctions.iter().take(50) {
            write_auction_list_item(&mut packet, auction);
        }

        packet.write_u32(self.total_count);
        packet
    }

    /// `AuctionListMyItemsResult` in 1.14 — count, total and the search delay ahead of the rows,
    /// like the search result, but with **no** `OnlyUsable` byte. The owner and bidder lists share
    /// this shape; only the search result carries the extra byte.
    fn to_modern(&self) -> Option<WorldPacket> {
        let now = now_unix();
        let auctions: Vec<&&AuctionEntry> = self.auctions.iter().take(50).collect();

        let mut writer = BitWriter::new();
        writer.write_i32(auctions.len() as i32);
        writer.write_i32(self.total_count as i32);
        writer.write_u32(AUCTION_SEARCH_DELAY_MS);

        for auction in auctions {
            write_modern_auction_item(&mut writer, auction, now);
        }

        Some(writer.finish(Opcode::SMSG_AUCTION_OWNER_LIST_RESULT))
    }
}

/// SMSG_AUCTION_BIDDER_LIST_RESULT - Auctions the player is bidding on
///
/// Sent in response to a request for auctions the player is currently bidding on.
#[derive(Debug)]
pub struct SmsgAuctionBidderListResult<'a> {
    /// Reference to array of auctions to send
    pub auctions: &'a [&'a AuctionEntry],
    /// Total number of auctions the player is bidding on
    pub total_count: u32,
}

impl ToWorldPacket for SmsgAuctionBidderListResult<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AUCTION_BIDDER_LIST_RESULT);
        let count = self.auctions.len().min(50) as u32;
        packet.write_u32(count);

        for auction in self.auctions.iter().take(50) {
            write_auction_list_item(&mut packet, auction);
        }

        packet.write_u32(self.total_count);
        packet
    }

    /// `AuctionListMyItemsResult` in 1.14, the same shape the owner list uses: count, total and the
    /// search delay ahead of the rows, and no `OnlyUsable` byte.
    fn to_modern(&self) -> Option<WorldPacket> {
        let now = now_unix();
        let auctions: Vec<&&AuctionEntry> = self.auctions.iter().take(50).collect();

        let mut writer = BitWriter::new();
        writer.write_i32(auctions.len() as i32);
        writer.write_i32(self.total_count as i32);
        writer.write_u32(AUCTION_SEARCH_DELAY_MS);

        for auction in auctions {
            write_modern_auction_item(&mut writer, auction, now);
        }

        Some(writer.finish(Opcode::SMSG_AUCTION_BIDDER_LIST_RESULT))
    }
}

/// SMSG_AUCTION_BIDDER_NOTIFICATION - Notification of auction bid result
///
/// Sent to notify the player that they were outbid or won an auction.
#[derive(Debug, Clone)]
pub struct SmsgAuctionBidderNotification {
    /// Auction house ID
    pub house_id: u32,
    /// Auction ID
    pub auction_id: u32,
    /// GUID of the bidder
    pub bidder_guid: ObjectGuid,
    /// Whether the player was outbid (true) or won (false)
    pub won: bool,
    /// Amount by which the player was outbid
    pub outbid_amount: u32,
    /// Item template ID
    pub item_template: u32,
    /// Item random property ID
    pub item_random_property_id: u32,
}

/// Only the "you won" half has a complete 1.14 encoding; see [`Self::to_modern`].
impl ToWorldPacket for SmsgAuctionBidderNotification {
    /// `AuctionWonNotification` in 1.14 — and nothing at all when the player was outbid.
    ///
    /// Vanilla says "won or outbid" inside one opcode by whether the bid sum is zero. 1.14 splits
    /// the two into separate opcodes with different bodies, so the flag has to select the opcode
    /// here rather than ride in the payload.
    ///
    /// The win notification needs only the auction, the bidder and the item, all of which this
    /// struct has. The outbid notification additionally carries the **bid sum** the player is being
    /// refunded, and this struct does not have it: `to_vanilla` collapses that field into a 0/1
    /// won-or-not flag, discarding the amount before it ever reaches the message. Rather than
    /// invent a number the client would display as the player's returned gold, the outbid case is
    /// left unencoded — a modern client silently misses the "you have been outbid" toast and still
    /// sees the refund arrive by mail. Carrying the bid sum would need a new field on the struct.
    ///
    /// The leading `Command` is 1.14's bidder-notification discriminator, not an auction action:
    /// it is a fixed 2 for both bidder notifications.
    fn to_modern(&self) -> Option<WorldPacket> {
        if !self.won {
            return None; // Outbid -- see above.
        }

        let mut writer = BitWriter::new();
        writer.write_u32(2); // Command
        writer.write_u32(self.auction_id);
        let (high, low) = self.bidder_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Bidder
        write_modern_item_instance(
            &mut writer,
            self.item_template,
            self.item_random_property_id,
        );

        Some(writer.finish(Opcode::SMSG_AUCTION_WON_NOTIFICATION))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AUCTION_BIDDER_NOTIFICATION);
        packet.write_u32(self.house_id);
        packet.write_u32(self.auction_id);
        packet.write_u64(self.bidder_guid.raw());
        packet.write_u32(if self.won { 0 } else { 1 }); // 0 = won, 1 = outbid
        packet.write_u32(self.outbid_amount);
        packet.write_u32(self.item_template);
        packet.write_u32(self.item_random_property_id);
        packet
    }
}

/// SMSG_AUCTION_OWNER_NOTIFICATION - Notification to auction owner
///
/// Sent to notify the auction owner that their item sold or expired.
/// `bidder_guid` is `None` when the auction is sold; it is only assigned for
/// auctions that did not sell.
#[derive(Debug, Clone)]
pub struct SmsgAuctionOwnerNotification {
    /// Auction ID
    pub auction_id: u32,
    /// Highest bid amount
    pub bid: u32,
    /// Amount by which the auction was outbid
    pub auction_outbid: u32,
    /// GUID of the bidder (None when sold)
    pub bidder_guid: Option<ObjectGuid>,
    /// Item template ID
    pub item_template: u32,
    /// Item random property ID
    pub item_random_property_id: u32,
}

impl ToWorldPacket for SmsgAuctionOwnerNotification {
    /// `AuctionClosedNotification` or `AuctionOwnerBidNotification` in 1.14, chosen by whether
    /// there is a bidder.
    ///
    /// Vanilla folds three different messages to the seller into one opcode and distinguishes them
    /// by which fields are filled. 1.14 has an opcode each, so the choice moves here:
    ///
    /// - no bidder — the auction ended. `AuctionClosedNotification`, with `Sold` set from whether
    ///   any money came in, which is exactly how vanilla's client tells "sold" from "expired".
    /// - a bidder — someone has just bid. `AuctionOwnerBidNotification`, which appends the outbid
    ///   increment and the bidder's GUID.
    ///
    /// Both share a leading block whose **bid amount widens to u64**.
    ///
    /// The proceeds delay has no 1.12 source; it is the one-hour delay the seller's mail is
    /// actually held for, and it is only a countdown label.
    fn to_modern(&self) -> Option<WorldPacket> {
        /// Seconds the auction house holds a seller's proceeds before mailing them.
        const PROCEEDS_MAIL_DELAY_SECONDS: f32 = 3600.0;

        let mut writer = BitWriter::new();
        // AuctionOwnerNotification, shared by both bodies.
        writer.write_u32(self.auction_id);
        writer.write_u64(u64::from(self.bid)); // BidAmount
        write_modern_item_instance(
            &mut writer,
            self.item_template,
            self.item_random_property_id,
        );

        match self.bidder_guid {
            Some(bidder) => {
                writer.write_u64(u64::from(self.auction_outbid)); // MinIncrement
                let (high, low) = bidder.to_guid128(DEFAULT_REALM_ID);
                writer.write_packed_guid_128(high, low);
                Some(writer.finish(Opcode::SMSG_AUCTION_OWNER_BID_NOTIFICATION))
            }
            None => {
                writer.write_f32(PROCEEDS_MAIL_DELAY_SECONDS);
                writer.write_bit(self.bid != 0); // Sold -- no money means it expired
                writer.flush_bits();
                Some(writer.finish(Opcode::SMSG_AUCTION_CLOSED_NOTIFICATION))
            }
        }
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AUCTION_OWNER_NOTIFICATION);
        packet.write_u32(self.auction_id);
        packet.write_u32(self.bid);
        packet.write_u32(self.auction_outbid);
        packet.write_u64(self.bidder_guid.map(|g| g.raw()).unwrap_or(0));
        packet.write_u32(self.item_template);
        packet.write_u32(self.item_random_property_id);
        packet
    }
}

/// SMSG_AUCTION_REMOVED_NOTIFICATION - Notification that an auction was removed
///
/// Sent to notify the player that an auction listing was removed or cancelled.
/// Wire format: auctionId (u32), itemTemplate (u32), randomPropertyId (u32).
#[derive(Debug, Clone)]
pub struct SmsgAuctionRemovedNotification {
    pub auction_id: u32,
    pub item_template: u32,
    pub item_random_property_id: u32,
}

/// Left unported: 1.14 has no "your listing was removed" notification.
///
/// The three 1.14 seller-facing notifications each mean something narrower than this one —
/// the auction closed (sold or expired), a bid arrived, or a bid was beaten — and all three need
/// figures this message does not carry: a bid amount, a mail delay, or a bidder. Encoding it as an
/// auction-closed notification with zeroed money would tell the seller their listing expired
/// unsold, which is a *different* and possibly false statement rather than a formatting error.
///
/// Nothing is lost in practice: the removal is already reflected in the owner list the client
/// re-requests, so the item leaves the "Auctions" tab either way.
impl ToWorldPacket for SmsgAuctionRemovedNotification {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AUCTION_REMOVED_NOTIFICATION);
        packet.write_u32(self.auction_id);
        packet.write_u32(self.item_template);
        packet.write_u32(self.item_random_property_id);
        packet
    }
}

/// Helper: Write a single auction item to packet
fn write_auction_list_item(packet: &mut WorldPacket, auction: &AuctionEntry) {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let time_left_ms = if auction.expire_time > current_time {
        ((auction.expire_time - current_time) * 1000) as u32
    } else {
        0
    };

    packet.write_u32(auction.id);
    packet.write_u32(auction.item_template);
    packet.write_u32(0); // enchantment
    packet.write_u32(0); // random property id
    packet.write_u32(0); // suffix factor
    packet.write_u32(1); // item count
    packet.write_u32(0); // charges
    packet.write_u64(auction.seller_guid.raw());
    packet.write_u32(auction.start_bid);
    packet.write_u32(auction.calculate_min_bid());
    packet.write_u32(auction.buyout_price);
    packet.write_u32(time_left_ms);
    packet.write_u64(auction.bidder_guid.raw());
    packet.write_u32(auction.current_bid);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::auction::{AuctionAction, AuctionError};
    use crate::protocol::Opcode;

    #[test]
    fn test_msg_auction_hello() {
        let msg = MsgAuctionHello {
            auctioneer_guid: ObjectGuid::from_low(123),
            house_id: 0,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::MSG_AUCTION_HELLO);
        assert_eq!(packet.data().len(), 12);
        assert_eq!(
            u64::from_le_bytes(packet.data()[0..8].try_into().unwrap()),
            123
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[8..12].try_into().unwrap()),
            0
        );
    }

    #[test]
    fn test_smsg_auction_command_result_ok_no_extra() {
        let msg = SmsgAuctionCommandResult::Ok {
            auction_id: 123,
            action: AuctionAction::Started,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_COMMAND_RESULT);
        assert_eq!(packet.data().len(), 12); // 3 * u32
        assert_eq!(
            u32::from_le_bytes(packet.data()[0..4].try_into().unwrap()),
            123
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[4..8].try_into().unwrap()),
            AuctionAction::Started as u32
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[8..12].try_into().unwrap()),
            AuctionError::Ok as u32
        );
    }

    #[test]
    fn test_smsg_auction_command_result_ok_bid_placed() {
        let msg = SmsgAuctionCommandResult::OkBidPlaced {
            auction_id: 123,
            outbid: 100,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_COMMAND_RESULT);
        assert_eq!(packet.data().len(), 16); // 4 * u32
        assert_eq!(
            u32::from_le_bytes(packet.data()[0..4].try_into().unwrap()),
            123
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[4..8].try_into().unwrap()),
            AuctionAction::BidPlaced as u32
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[8..12].try_into().unwrap()),
            AuctionError::Ok as u32
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[12..16].try_into().unwrap()),
            100
        );
    }

    #[test]
    fn test_smsg_auction_command_result_inventory_error() {
        let msg = SmsgAuctionCommandResult::Inventory {
            auction_id: 0,
            action: AuctionAction::Started,
            inventory_error: crate::game::InventoryResult::BagFull,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_COMMAND_RESULT);
        assert_eq!(packet.data().len(), 16); // 4 * u32
        assert_eq!(
            u32::from_le_bytes(packet.data()[0..4].try_into().unwrap()),
            0
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[4..8].try_into().unwrap()),
            AuctionAction::Started as u32
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[8..12].try_into().unwrap()),
            AuctionError::Inventory as u32
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[12..16].try_into().unwrap()),
            crate::game::InventoryResult::BagFull as u32
        );
    }

    #[test]
    fn test_smsg_auction_command_result_higher_bid() {
        let bidder_guid = ObjectGuid::from_low(789);
        let msg = SmsgAuctionCommandResult::HigherBid {
            auction_id: 456,
            action: AuctionAction::BidPlaced,
            bidder_guid,
            bid: 1000,
            outbid: 50,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_COMMAND_RESULT);
        assert_eq!(packet.data().len(), 28); // 3 * u32 + u64 + 2 * u32
        assert_eq!(
            u32::from_le_bytes(packet.data()[0..4].try_into().unwrap()),
            456
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[4..8].try_into().unwrap()),
            AuctionAction::BidPlaced as u32
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[8..12].try_into().unwrap()),
            AuctionError::HigherBid as u32
        );
        assert_eq!(
            u64::from_le_bytes(packet.data()[12..20].try_into().unwrap()),
            bidder_guid.raw()
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[20..24].try_into().unwrap()),
            1000
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[24..28].try_into().unwrap()),
            50
        );
    }

    #[test]
    fn test_smsg_auction_command_result_other_error() {
        let msg = SmsgAuctionCommandResult::Other {
            auction_id: 0,
            action: AuctionAction::Started,
            error: AuctionError::NotEnoughMoney,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_COMMAND_RESULT);
        assert_eq!(packet.data().len(), 12); // 3 * u32
        assert_eq!(
            u32::from_le_bytes(packet.data()[0..4].try_into().unwrap()),
            0
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[4..8].try_into().unwrap()),
            AuctionAction::Started as u32
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[8..12].try_into().unwrap()),
            AuctionError::NotEnoughMoney as u32
        );
    }

    #[test]
    fn test_smsg_auction_bidder_notification() {
        let msg = SmsgAuctionBidderNotification {
            house_id: 0,
            auction_id: 123,
            bidder_guid: ObjectGuid::from_low(456),
            won: false,
            outbid_amount: 100,
            item_template: 789,
            item_random_property_id: 0,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_BIDDER_NOTIFICATION);
    }

    #[test]
    fn test_smsg_auction_owner_notification() {
        let msg = SmsgAuctionOwnerNotification {
            auction_id: 123,
            bid: 1000,
            auction_outbid: 50,
            bidder_guid: Some(ObjectGuid::from_low(456)),
            item_template: 789,
            item_random_property_id: 0,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_OWNER_NOTIFICATION);
    }

    #[test]
    fn test_smsg_auction_owner_notification_sold_no_bidder() {
        let msg = SmsgAuctionOwnerNotification {
            auction_id: 123,
            bid: 1000,
            auction_outbid: 50,
            bidder_guid: None,
            item_template: 789,
            item_random_property_id: 0,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_OWNER_NOTIFICATION);
    }

    #[test]
    fn test_smsg_auction_removed_notification() {
        let msg = SmsgAuctionRemovedNotification {
            auction_id: 999,
            item_template: 123,
            item_random_property_id: 45,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_REMOVED_NOTIFICATION);
        assert_eq!(
            u32::from_le_bytes(packet.data()[0..4].try_into().unwrap()),
            999
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[4..8].try_into().unwrap()),
            123
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[8..12].try_into().unwrap()),
            45
        );
    }
}
