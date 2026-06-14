//! Auction-related session helpers
//!
//! Functions that send auction packets to a client session.
//! Kept here rather than on `WorldSession` to avoid polluting the generic session with
//! auction-specific logic.

use crate::shared::game::auction::{AuctionAction, AuctionEntry, AuctionError};
use crate::shared::messages::auction::{SmsgAuctionCommandResult, SmsgAuctionOwnerNotification};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::ObjectGuid;
use crate::world::core::session::WorldSession;
use crate::world::game::auction::AuctionHouseManager;
use crate::world::game::inventory::inventory_types::InventoryResult;

/// Send SMSG_AUCTION_COMMAND_RESULT to the client session.
///
/// Mirrors C++ `WorldSession::SendAuctionCommandResult`.
/// The `auction` parameter is `None` only when the auction pointer is null;
/// callers must ensure `auction` is `Some` for `HigherBid` and `Ok+BidPlaced` branches,
/// matching the C++ assumption that those branches unconditionally dereference `auc`.
///
/// Returns an error when the required auction data is missing for a branch that
/// needs it (e.g. `HigherBid` without an `AuctionEntry`). This prevents the
/// silent packet-truncation bug that the C++ code exhibits when `auc` is null.
pub fn send_auction_command_result(
    session: &WorldSession,
    auction: Option<&AuctionEntry>,
    action: AuctionAction,
    error: AuctionError,
    inventory_error: Option<InventoryResult>,
) -> anyhow::Result<()> {
    let auction_id = auction.map(|a| a.id).unwrap_or(0);

    let msg = match error {
        AuctionError::Ok => {
            if action == AuctionAction::BidPlaced {
                let outbid = auction
                    .map(|a| a.get_outbid_amount())
                    .ok_or_else(|| anyhow::anyhow!("Ok+BidPlaced requires auction data"))?;
                SmsgAuctionCommandResult::OkBidPlaced { auction_id, outbid }
            } else {
                SmsgAuctionCommandResult::Ok { auction_id, action }
            }
        }
        AuctionError::Inventory => {
            let inventory_error = inventory_error
                .ok_or_else(|| anyhow::anyhow!("Inventory error requires inventory_error"))?;
            SmsgAuctionCommandResult::Inventory {
                auction_id,
                action,
                inventory_error,
            }
        }
        AuctionError::HigherBid => {
            let auction = auction
                .ok_or_else(|| anyhow::anyhow!("HigherBid requires auction data"))?;
            SmsgAuctionCommandResult::HigherBid {
                auction_id,
                action,
                bidder_guid: auction.bidder_guid,
                bid: auction.current_bid,
                outbid: auction.get_outbid_amount(),
            }
        }
        _ => SmsgAuctionCommandResult::Other {
            auction_id,
            action,
            error,
        },
    };

    session.send_msg(msg)
}

/// Send SMSG_AUCTION_OWNER_NOTIFICATION to the client session.
///
/// Mirrors C++ `WorldSession::SendAuctionOwnerNotification`.
/// The `auction` pointer is assumed non-null (matching C++).
/// `item_random_property_id` is the item's random property from the auction manager
/// (looked up via `GetAItem` in C++); pass `0` when the item is not found.
pub fn send_auction_owner_notification(
    session: &WorldSession,
    auction: &AuctionEntry,
    sold: bool,
    item_random_property_id: i32,
) -> anyhow::Result<()> {
    let msg = SmsgAuctionOwnerNotification {
        auction_id: auction.id,
        bid: auction.current_bid,
        auction_outbid: auction.get_outbid_amount(),
        bidder_guid: if !sold {
            Some(auction.bidder_guid)
        } else {
            None
        },
        item_template: auction.item_template,
        item_random_property_id: item_random_property_id as u32,
    };

    session.send_msg(msg)
}

/// Validate auctioneer access and return the corresponding auction house entry.
///
/// Mirrors C++ `WorldSession::GetCheckedAuctionHouseForAuctioneer`.
/// Returns `None` if the player is not allowed to use the auction (GM/self without permission,
/// or NPC the player cannot interact with).
///
/// The `npc_interaction_validator` is called when the auctioneer GUID differs from the player
/// GUID. It should return `Some(faction_template_id)` if the player can interact with the NPC,
/// or `None` otherwise. When the auction system does not yet have NPC interaction validation,
/// callers can pass `None` for the NPC path to always deny.
pub fn get_checked_auction_house_for_auctioneer(
    player: &crate::world::game::player::Player,
    auctioneer_guid: ObjectGuid,
    manager: &AuctionHouseManager,
    npc_interaction_validator: Option<u32>, // faction template ID if valid, None if invalid
) -> Option<crate::world::dbc::structures::AuctionHouseEntry> {
    // GM/self path
    if auctioneer_guid == player.guid {
        if player.auction_access_mode == 0 {
            // TODO: Check if player has "auction" command access (ChatHandler.FindCommand)
            // C++ uses GetPlayer()->GetAuctionAccessMode() == 0 && !ChatHandler(...).FindCommand("auction")
            // For now, default to denying when auction_access_mode == 0 and no command permission.
            tracing::debug!(
                "{} attempt open auction in cheating way.",
                auctioneer_guid
            );
            return None;
        }
        return manager.get_auction_house_for_player(player.get_team(), player.auction_access_mode);
    }

    // NPC path
    let faction_template_id = match npc_interaction_validator {
        Some(id) => id,
        None => {
            tracing::debug!(
                "Auctioneer {} accessed in cheating way.",
                auctioneer_guid
            );
            return None;
        }
    };

    manager.get_auction_house_for_npc(faction_template_id)
}
