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

use crate::shared::game::{AuctionAction, AuctionEntry, AuctionError};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::guid::ObjectGuid;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;

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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_AUCTION_HELLO);
        packet.write_u64(self.auctioneer_guid.raw());
        packet.write_u32(self.house_id);
        packet
    }
}

/// SMSG_AUCTION_COMMAND_RESULT - Result of an auction action
///
/// Sent in response to auction actions like creating, bidding, or canceling.
/// Mirrors C++ `WorldSession::SendAuctionCommandResult` wire format.
#[derive(Debug, Clone)]
pub struct SmsgAuctionCommandResult {
    /// Auction ID (0 when auc is null)
    pub auction_id: u32,
    /// Type of auction action performed
    pub action: AuctionAction,
    /// Result of the action
    pub error: AuctionError,
    /// Inventory error (serialized when error == AUCTION_ERR_INVENTORY)
    pub inventory_error: Option<crate::world::game::inventory::inventory_types::InventoryResult>,
    /// Bidder GUID (serialized when error == AUCTION_ERR_HIGHER_BID)
    pub bidder_guid: Option<ObjectGuid>,
    /// Current bid (serialized when error == AUCTION_ERR_HIGHER_BID)
    pub bid: Option<u32>,
    /// Outbid amount (serialized when error == AUCTION_OK + BidPlaced, or AUCTION_ERR_HIGHER_BID)
    pub outbid: Option<u32>,
}

impl ToWorldPacket for SmsgAuctionCommandResult {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AUCTION_COMMAND_RESULT);
        // Base fields always present (C++ lines 73-75)
        packet.write_u32(self.auction_id);
        packet.write_u32(self.action as u32);
        packet.write_u32(self.error as u32);

        match self.error {
            AuctionError::Ok => {
                if self.action == AuctionAction::BidPlaced {
                    packet.write_u32(self.outbid.unwrap_or(0));
                }
            }
            AuctionError::Inventory => {
                if let Some(inv) = self.inventory_error {
                    packet.write_u32(inv as u32);
                }
            }
            AuctionError::HigherBid => {
                if let Some(guid) = self.bidder_guid {
                    packet.write_u64(guid.raw());
                }
                if let Some(bid) = self.bid {
                    packet.write_u32(bid);
                }
                if let Some(outbid) = self.outbid {
                    packet.write_u32(outbid);
                }
            }
            _ => {}
        }

        packet
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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AUCTION_LIST_RESULT);
        let count = self.auctions.len().min(50) as u32;
        packet.write_u32(count);

        for auction in self.auctions.iter().take(50) {
            write_auction_list_item(&mut packet, auction);
        }

        packet.write_u32(self.total_count);
        packet
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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AUCTION_OWNER_LIST_RESULT);
        let count = self.auctions.len().min(50) as u32;
        packet.write_u32(count);

        for auction in self.auctions.iter().take(50) {
            write_auction_list_item(&mut packet, auction);
        }

        packet.write_u32(self.total_count);
        packet
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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AUCTION_BIDDER_LIST_RESULT);
        let count = self.auctions.len().min(50) as u32;
        packet.write_u32(count);

        for auction in self.auctions.iter().take(50) {
            write_auction_list_item(&mut packet, auction);
        }

        packet.write_u32(self.total_count);
        packet
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

impl ToWorldPacket for SmsgAuctionBidderNotification {
    fn to_world_packet(&self) -> WorldPacket {
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
/// `bidder_guid` is `None` when the auction is sold (`sold=true` in C++), matching
/// the C++ behavior where `bidderGuid` is only assigned in the `!sold` branch.
#[derive(Debug, Clone)]
pub struct SmsgAuctionOwnerNotification {
    /// Auction ID
    pub auction_id: u32,
    /// Highest bid amount
    pub bid: u32,
    /// Amount by which the auction was outbid
    pub auction_outbid: u32,
    /// GUID of the bidder (None when sold, matching C++ default-initialized field)
    pub bidder_guid: Option<ObjectGuid>,
    /// Item template ID
    pub item_template: u32,
    /// Item random property ID
    pub item_random_property_id: u32,
}

impl ToWorldPacket for SmsgAuctionOwnerNotification {
    fn to_world_packet(&self) -> WorldPacket {
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
/// Sent to notify the player that an auction they were watching was removed.
#[derive(Debug, Clone)]
pub struct SmsgAuctionRemovedNotification {
    /// Item template ID
    pub item_template: u32,
    /// Item random property ID
    pub item_random_property_id: u32,
}

impl ToWorldPacket for SmsgAuctionRemovedNotification {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AUCTION_REMOVED_NOTIFICATION);
        packet.write_u32(self.item_template);
        packet.write_u32(self.item_template); // Item field (same as template for now)
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
    use crate::shared::game::auction::{AuctionAction, AuctionError};
    use crate::shared::protocol::Opcode;

    #[test]
    fn test_msg_auction_hello() {
        let msg = MsgAuctionHello {
            auctioneer_guid: ObjectGuid::from_low(123),
            house_id: 0,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::MSG_AUCTION_HELLO);
        assert_eq!(packet.data().len(), 12);
        assert_eq!(u64::from_le_bytes(packet.data()[0..8].try_into().unwrap()), 123);
        assert_eq!(u32::from_le_bytes(packet.data()[8..12].try_into().unwrap()), 0);
    }

    #[test]
    fn test_smsg_auction_command_result_ok_bid_placed() {
        let msg = SmsgAuctionCommandResult {
            auction_id: 123,
            action: AuctionAction::BidPlaced,
            error: AuctionError::Ok,
            inventory_error: None,
            bidder_guid: None,
            bid: None,
            outbid: Some(100),
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_COMMAND_RESULT);
    }

    #[test]
    fn test_smsg_auction_command_result_inventory_error() {
        let msg = SmsgAuctionCommandResult {
            auction_id: 0,
            action: AuctionAction::Started,
            error: AuctionError::Inventory,
            inventory_error: Some(crate::world::game::inventory::inventory_types::InventoryResult::BagFull),
            bidder_guid: None,
            bid: None,
            outbid: None,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_COMMAND_RESULT);
    }

    #[test]
    fn test_smsg_auction_command_result_higher_bid() {
        let msg = SmsgAuctionCommandResult {
            auction_id: 456,
            action: AuctionAction::BidPlaced,
            error: AuctionError::HigherBid,
            inventory_error: None,
            bidder_guid: Some(ObjectGuid::from_low(789)),
            bid: Some(1000),
            outbid: Some(50),
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_COMMAND_RESULT);
    }

    #[test]
    fn test_smsg_auction_command_result_ok_no_extra() {
        let msg = SmsgAuctionCommandResult {
            auction_id: 123,
            action: AuctionAction::Started,
            error: AuctionError::Ok,
            inventory_error: None,
            bidder_guid: None,
            bid: None,
            outbid: None,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_COMMAND_RESULT);
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
        let packet = msg.to_world_packet();
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
        let packet = msg.to_world_packet();
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
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_OWNER_NOTIFICATION);
    }

    #[test]
    fn test_smsg_auction_removed_notification() {
        let msg = SmsgAuctionRemovedNotification {
            item_template: 123,
            item_random_property_id: 45,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_AUCTION_REMOVED_NOTIFICATION);
    }
}
