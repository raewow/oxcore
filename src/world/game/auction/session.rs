//! Auction-related session helpers
//!
//! Functions that send auction packets to a client session.
//! Kept here rather than on `WorldSession` to avoid polluting the generic session with
//! auction-specific logic.

use crate::shared::game::auction::{AuctionAction, AuctionEntry, AuctionError};
use crate::shared::messages::auction::SmsgAuctionCommandResult;
use crate::shared::messages::ToWorldPacket;
use crate::world::core::session::WorldSession;
use crate::world::game::inventory::inventory_types::InventoryResult;

/// Send SMSG_AUCTION_COMMAND_RESULT to the client session.
///
/// Mirrors C++ `WorldSession::SendAuctionCommandResult`.
/// The `auction` parameter is `None` only when the auction pointer is null;
/// callers must ensure `auction` is `Some` for `HigherBid` and `Ok+BidPlaced` branches,
/// matching the C++ assumption that those branches unconditionally dereference `auc`.
pub fn send_auction_command_result(
    session: &WorldSession,
    auction: Option<&AuctionEntry>,
    action: AuctionAction,
    error: AuctionError,
    inventory_error: Option<InventoryResult>,
) -> anyhow::Result<()> {
    let auction_id = auction.map(|a| a.id).unwrap_or(0);

    let msg = SmsgAuctionCommandResult {
        auction_id,
        action,
        error,
        inventory_error,
        bidder_guid: match error {
            AuctionError::HigherBid => auction.map(|a| a.bidder_guid),
            _ => None,
        },
        bid: match error {
            AuctionError::HigherBid => auction.map(|a| a.current_bid),
            _ => None,
        },
        outbid: match (error, action) {
            (AuctionError::Ok, AuctionAction::BidPlaced) => auction.map(|a| a.get_outbid_amount()),
            (AuctionError::HigherBid, _) => auction.map(|a| a.get_outbid_amount()),
            _ => None,
        },
    };

    session.send_msg(msg)
}
