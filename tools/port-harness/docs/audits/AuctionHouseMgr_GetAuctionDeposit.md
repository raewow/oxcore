# Audit: AuctionHouseMgr::GetAuctionDeposit

**Status:** missing  
**Passed:** false  
**Coverage:** 0/22 claims

## Summary
GetAuctionDeposit is not implemented in the Rust codebase. Auction house support in src/ covers DBC loading (deposit_percent field), deposit storage on AuctionEntry/DB rows, and auction loading—but no function computes listing deposit from sell price, stack count, duration, house percent, minimum config, or rate config. All 22 behavioural claims and 3 flow branches are uncovered.

## Rust locations
(none)

## Issues
- [error] No Rust equivalent of AuctionHouseMgr::GetAuctionDeposit exists in src/; searches for GetAuctionDeposit, get_auction_deposit, AuctionDeposit, and auction_deposit returned no matches.
- [error] Base deposit formula (SellPrice * count * (time / MIN_AUCTION_TIME) cast to float) is not implemented anywhere in src/.
- [error] MIN_AUCTION_TIME is not defined in the Rust codebase; integer duration scaling cannot be replicated.
- [error] deposit_percent from AuctionHouseEntry is loaded from DBC (src/world/dbc/structures.rs) but never used in any deposit calculation; only test fixtures reference it.
- [error] World config CONFIG_UINT32_AUCTION_DEPOSIT_MIN is not present in src/world/config.rs or elsewhere; minimum deposit clamping is unimplemented.
- [error] World config CONFIG_FLOAT_RATE_AUCTION_DEPOSIT is not present in src/world/config.rs or elsewhere; final rate multiplication is unimplemented.
- [warning] No auction sell/listing handler exists in src/world/handlers/ (CMSG_AUCTION_SELL_ITEM opcode defined but unhandled); deposit would normally be computed at listing time.
- [info] AuctionEntry.deposit (src/shared/game/auction.rs) and auction DB rows store a deposit value loaded from the database but do not compute it; this is persistence, not GetAuctionDeposit behaviour.
- [info] AuctionEntry::get_auction_cut computes auction-house cut from current bid, not listing deposit; different formula and purpose.
