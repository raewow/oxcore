# Audit: WorldSession::SendAuctionHello

**Status:** missing  
**Passed:** false  
**Coverage:** 0/16 claims

## Summary
The documented C++ auction-hello behavior is not implemented in Rust yet. The repo has auction packet definitions and opcode constants, but no world-session handler that validates the auctioneer, clears feign death, resolves the auction house, and sends the hello response.

## Rust locations
(none)

## Issues
- [error] No Rust session handler equivalent to `WorldSession::SendAuctionHello` is present in `src/world/handlers`; `Opcode::MSG_AUCTION_HELLO` exists, but there is no dispatch path that resolves an auctioneer and sends the hello response.
- [error] The invalid-auctioneer branch from the C++ flow (`GetNPCIfCanInteractWith` null => debug log and return) is not implemented in Rust.
- [error] The feign-death cleanup branch (`UNIT_STATE_FEIGN_DEATH` => remove feign-death auras before proceeding) is not implemented in Rust.
- [warning] Rust defines auction message serialization (`MsgAuctionHello`) and auction opcode constants, but those definitions are not wired into a world-session handler that validates interaction and emits the packet.
- [warning] The C++ flow constructs a response with auctioneer GUID and house ID, then sends it through the session API; there is no matching Rust code path that builds and transmits this packet from an auction hello interaction.

## Missing behaviours
- No Rust handler that accepts an auctioneer target, resolves the auction house entry, and sends the auction hello packet.
- No null/invalid auctioneer guard with early return and debug logging.
- No feign-death aura removal before opening the auction house UI.
- No response construction that sets `auctioneerGuid` and `houseId` from the resolved unit/auction house entry.
- No session send call that transmits the auction hello response packet.

## Planning notes
- Add a dedicated auction handler under `src/world/handlers/` and wire it into `src/world/handlers/mod.rs` and packet dispatch for `Opcode::MSG_AUCTION_HELLO`.
- Re-use the existing `MsgAuctionHello` packet type in `src/shared/messages/auction.rs`; if the packet name or opcode mapping differs from the target client flow, add a dedicated outbound wrapper rather than guessing from names.
- Mirror the C++ branch order: validate the auctioneer interaction first, return early on invalid/unreachable NPC, clear feign-death auras if the player is in that state, then resolve the auction house entry and send the hello packet.
- You will need access to player/session state, the selected or targeted unit GUID, NPC interaction validation, DBC auction-house lookup, and the session send API used by other handlers.
- Preserve the C++ null-safety assumptions explicitly in Rust: do not dereference the target unit or auction-house entry until the handler has validated both; return a logged error or debug message on failure.
- Add tests for: valid auctioneer sends hello packet with correct GUID/house id, invalid auctioneer returns without sending, and feign-death state clears before packet emission. A fixture should cover at least one alliance, horde, or neutral auction house DBC row.
