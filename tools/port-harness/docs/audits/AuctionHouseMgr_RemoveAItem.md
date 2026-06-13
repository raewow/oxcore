# Audit: AuctionHouseMgr::RemoveAItem

**Status:** complete  
**Passed:** true  
**Coverage:** 10/10 claims

## Summary
AuctionHouseMgr::RemoveAItem is fully ported as AuctionHouseManager::remove_a_item: it accepts a u32 GUID key, removes the entry from the in-memory auction_items map (DashMap keyed like mAitems), returns false when absent and true when removed, and does not itself destroy the item object.

## Rust locations
- `AuctionHouseManager::remove_a_item` in `src/world/game/auction/manager.rs`

## Issues
- [info] The sole call site at load_auctions line 271 ignores the bool return value; this does not affect remove_a_item itself but differs from C++ callers that may branch on failure.
- [info] Rust stores Arc<Item> instead of raw Item*; map removal only drops the map's Arc reference and does not force Item destruction while other Arc clones exist, matching the C++ erase-only contract.
