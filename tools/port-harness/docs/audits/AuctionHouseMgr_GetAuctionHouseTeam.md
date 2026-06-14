# Audit: AuctionHouseMgr::GetAuctionHouseTeam

**Status:** complete  
**Passed:** true  
**Coverage:** 14/14 claims

## Summary
get_auction_house_team in manager.rs fully ports GetAuctionHouseTeam: house ids 1–3 map to Alliance, 4–6 to Horde, and 7 or any other id to neutral (Team::None = 0). The function is pure, has no side effects, is wired into linked-mode load_auction_houses, and is covered by unit tests for representative ids including 7 and 99.

## Rust locations
- `get_auction_house_team` in `src/world/game/auction/manager.rs`
- `load_auction_houses` in `src/world/game/auction/manager.rs`

## Issues
- [info] Rust takes u32 house_id instead of AuctionHouseEntry const*; the caller in load_auction_houses extracts entry.house_id before calling, preserving the same lookup semantics without raw-pointer dereference.
- [info] C++ null-pointer UB on house is eliminated in Rust by using a value-type house_id parameter.
- [info] The C++ comment explaining why houseId is used instead of the faction field is not replicated in Rust, though the implementation follows the same id-based strategy.
- [info] Rust returns the typed Team enum (#[repr(u32)]: None=0, Alliance=469, Horde=67) instead of raw uint32, matching the C++ switch cases in LoadAuctionHouses.
