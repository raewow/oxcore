# Audit: AuctionHouseMgr::GetAuctionsMap

**Status:** complete  
**Passed:** true  
**Coverage:** 10/10 claims

## Summary
GetAuctionsMap is fully ported as get_auctions_map: it looks up auction_houses (DashMap) by house.house_id and returns Some(Arc<AuctionHouseObject>) on hit or None on miss, with no map mutation. Bootstrap population lives separately in load_auction_houses and matches the documented cross-faction/unlinked/linked modes.

## Rust locations
- `get_auctions_map` in `src/world/game/auction/manager.rs`

## Issues
- [info] Rust returns Option<Arc<AuctionHouseObject>> instead of a raw AuctionHouseObject*; lookup semantics match but ownership is expressed via Arc rather than an unowned pointer.
- [info] C++ can UB on nullptr house; Rust takes &AuctionHouseEntry so null input is rejected at compile time rather than via an explicit runtime check.
- [info] Successful lookup performs Arc::clone (atomic refcount increment), a minor side effect not present when C++ returns an existing raw pointer.
- [info] get_auctions_map is a private method used only from load_auctions; C++ GetAuctionsMap is likely called from multiple sites, but the lookup behaviour itself is implemented.
