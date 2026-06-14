# Plan: WorldSession::HandleAuctionHelloOpcode

**Target file:** `src/world/handlers/auction.rs`  
**Rust symbol:** `handle_auction_hello`

## Structs
- MsgAuctionHello

## Enums
—

## Notes
Implement the auction-hello flow here and wire it into the dispatcher. Add `Opcode::CMSG_AUCTION_HELLO` in `src/shared/protocol/opcodes.rs`, export this module from `src/world/handlers/mod.rs`, and dispatch the new opcode to `handle_auction_hello`. The handler should read the incoming auctioneer GUID from `WorldPacket` (matching the other NPC interaction handlers), validate that `session.player_guid()` exists, then resolve the player and creature. Mirror the C++ branch order: if the creature lookup/interaction validation fails, emit the debug log and return immediately; do not send `MsgAuctionHello` on that path. The Rust equivalent of `GetNPCIfCanInteractWith` needs to validate both existence and `UNIT_NPC_FLAG_AUCTIONEER`, and should ideally also preserve any existing reach/interact checks rather than only checking the flag. If the player currently has feign death, remove the aura(s) that cause `AURA_FEIGN_DEATH` before opening the auction UI; use the aura system API so client update/state bookkeeping stays consistent, and add a public helper if the current API only exposes spell-id removal. Resolve the auction house ID from the loaded auction-house/DBC data (or expose a small lookup helper on `AuctionHouseManager` if needed); do not hard-code house IDs in the handler, and return early if no mapping exists. On success, send `MsgAuctionHello { auctioneer_guid, house_id }` through `WorldSession::send_msg`. There is no persistence write path in this flow; the only state mutation is the feign-death cleanup, which must happen before the response is serialized. Tests should cover: invalid or missing auctioneer rejection, non-auctioneer rejection, feign-death aura removal, and the success path asserting opcode and payload contents. Add a fixture for an auctioneer creature template/instance and a player with feign death active so the handler can be exercised end-to-end.
