# Plan: WorldSession::SendAuctionHello

**Target file:** `src/world/core/session/world_session.rs`  
**Rust symbol:** `WorldSession::send_auction_hello`

## Structs
- crate::shared::messages::auction::MsgAuctionHello
- crate::world::dbc::structures::AuctionHouseEntry
- crate::shared::protocol::ObjectGuid

## Enums
- crate::shared::game::auction::AuctionHouseId

## Notes
Map the C++ session send helper to a thin Rust wrapper around `WorldSession::send_msg(MsgAuctionHello { auctioneer_guid, house_id })`. Reuse the existing outbound packet type in `src/shared/messages/auction.rs`; do not add a second auction-hello packet struct. Preserve the payload order exactly: auctioneer GUID first, house ID second. In Rust, keep null-safety at the call site: the future auctioneer-interaction handler should resolve and validate the target unit and auction-house entry before calling this method, and should return early on an invalid or unreachable auctioneer. The feign-death cleanup and interaction validation called out in the audit belong in that caller-side handler, not inside this send wrapper. `MSG_AUCTION_HELLO` is an outbound server opcode, so do not wire this through packet dispatch as if it were an incoming client message. Tests should cover the caller path with at least one alliance/horde/neutral DBC fixture, plus the two failure cases from the audit: invalid auctioneer returns without sending and feign-death state is cleared before the packet is emitted.
