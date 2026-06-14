# Audit: WorldSession::HandleAuctionHelloOpcode

**Status:** missing  
**Passed:** false  
**Coverage:** 0/14 claims

## Summary
The C++ auction-hello handler is not implemented in Rust. Only supporting pieces exist (`MsgAuctionHello` serialization and a stubbed Lua `SendAuctioneer` action), so the NPC validation, feign-death cleanup, and response dispatch still need to be added.

## Rust locations
(none)

## Issues
- [error] There is no Rust world-session handler for the auction-hello flow, and `src/world/handlers/mod.rs` does not dispatch an auction opcode.
- [warning] The only auction-open path found in Rust is `LuaAction::SendAuctioneer` in `src/world/core/lua/gossip_executor.rs`, and it only logs that the action is not yet implemented.
- [warning] `src/shared/messages/auction.rs` provides the `MsgAuctionHello` serializer, but there is no code that validates an auctioneer, clears feign death, and sends it on success.

## Missing behaviours
- No Rust handler accepts the client auction-hello packet and extracts the auctioneer GUID.
- No validation path checks the player session, resolves the target NPC, and rejects invalid or unreachable auctioneers with a debug log and early return.
- No feign-death cleanup runs before opening the auction UI.
- No success path sends the auction-hello response packet with the resolved auctioneer GUID and auction house ID.

## Planning notes
- Add a dedicated auction handler under `src/world/handlers/` and wire its opcode into `src/world/handlers/mod.rs` dispatch.
- Introduce the missing client auction-hello opcode in `src/shared/protocol/opcodes.rs`; `MSG_AUCTION_HELLO` already exists as the server response opcode in `src/shared/messages/auction.rs`.
- Parse the incoming packet's auctioneer GUID first, then resolve `session.player_guid()` and the NPC from `world.managers.creature_mgr`; validate the target is an auctioneer before opening the UI.
- Mirror the C++ branch order: invalid lookup -> debug log and return; valid lookup -> remove feign-death aura if the player has the feign-death state; then build and send the response.
- Reuse the existing auction-house data to map the auctioneer to a house ID, and send `MsgAuctionHello { auctioneer_guid, house_id }` through `WorldSession::send_msg`.
- Add tests for invalid GUID or non-auctioneer rejection, feign-death aura removal, and the successful response packet contents/opcode; include a fixture for an auctioneer creature and a player state with feign death active.
