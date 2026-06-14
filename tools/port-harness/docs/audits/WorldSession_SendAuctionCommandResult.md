# Audit: WorldSession::SendAuctionCommandResult

**Status:** partial  
**Passed:** false  
**Coverage:** 11/19 claims

## Summary
The Rust code already implements the auction command result flow and all switch branches, but it is only a partial match to the C++ behavior. The main gaps are the missing explicit packet size hint and the fact that required branch payloads are modeled as optional, which can silently produce an incomplete packet if callers do not supply the data.

## Rust locations
- `SmsgAuctionCommandResult::to_world_packet` in `src/shared/messages/auction.rs`
- `send_auction_command_result` in `src/world/game/auction/session.rs`
- `handle_auction_sell_item` in `src/world/handlers/auction.rs`

## Issues
- [warning] `WorldPacket::new(Opcode::SMSG_AUCTION_COMMAND_RESULT)` does not preserve the C++ `16`-byte size hint/preallocation from `SendAuctionCommandResult`; the Rust packet starts with an empty buffer.
- [error] The branch payloads are optional in Rust (`inventory_error: Option<_>`, `bidder_guid: Option<_>`, `bid: Option<u32>`, `outbid: Option<u32>`). In the C++ function, `AUCTION_ERR_INVENTORY` and `AUCTION_ERR_HIGHER_BID` always serialize their required extra fields, so `None` would produce a shorter/malformed packet.
- [info] Rust models the auction parameter as `Option<&AuctionEntry>` instead of the C++ raw pointer. That is safer, but it does not encode the original null-dereference precondition for the `AUCTION_OK + AUCTION_BID_PLACED` and `AUCTION_ERR_HIGHER_BID` branches.
- [warning] Existing tests only assert opcode presence; they do not verify byte-for-byte payload length/order for the `Ok+BidPlaced`, `Inventory`, and `HigherBid` branches.

## Missing behaviours
- Explicit `16`-byte packet preallocation for `SMSG_AUCTION_COMMAND_RESULT`.
- Mandatory branch payload serialization for `AUCTION_ERR_INVENTORY` and `AUCTION_ERR_HIGHER_BID` instead of option-based omission.
- Exact C++ null-pointer call contract for `auc` in the bid-placed/higher-bid paths is not represented in the Rust API.

## Planning notes
- Update `src/shared/messages/auction.rs` so `SmsgAuctionCommandResult` cannot be constructed without the fields required by the active `AuctionError` branch. A branch-specific enum or dedicated constructors would prevent silent omission of inventory/bidder/bid/outbid data.
- If byte-for-byte parity matters, add a capacity-aware `WorldPacket` constructor or `reserve(16)` path and use it for `SMSG_AUCTION_COMMAND_RESULT` to mirror the C++ size hint.
- Keep `src/world/game/auction/session.rs` as the single entry point, but add validation or debug assertions before packet construction so `HigherBid` and `Ok+BidPlaced` must always have auction data.
- Add payload-level tests in `src/shared/messages/auction.rs` that assert opcode, total packet length, and serialized field order for: base-only, `Ok+BidPlaced`, `Inventory`, and `HigherBid`.
- Add a negative/guard test for `send_auction_command_result` that proves branch-required fields are rejected or impossible to omit, rather than merely omitted from the packet.
