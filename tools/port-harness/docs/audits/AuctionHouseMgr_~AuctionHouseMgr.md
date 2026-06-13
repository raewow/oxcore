# Audit: AuctionHouseMgr::~AuctionHouseMgr

**Status:** partial  
**Passed:** false  
**Coverage:** 7/11 claims

## Summary
AuctionHouseManager exists with an auction_items map equivalent to mAitems, and auction items are freed when the manager is dropped via implicit DashMap/Arc RAII. There is no explicit destructor matching the C++ range-for delete loop, ownership uses Arc instead of raw delete, and the manager is Arc-wrapped so teardown timing differs; core heap cleanup is present but the port is not a complete literal match.

## Rust locations
- `AuctionHouseManager` in `src/world/game/auction/manager.rs`
- `auction_items (implicit Drop teardown)` in `src/world/game/auction/manager.rs`
- `World::new (Arc<AuctionHouseManager> construction)` in `src/world/world.rs`

## Issues
- [warning] No impl Drop for AuctionHouseManager and no explicit loop over auction_items; teardown relies on compiler-generated field drops instead of a ported ~AuctionHouseMgr body.
- [warning] C++ uses raw delete on Item* values in mAitems; Rust stores Arc<Item> in auction_items and frees items only when the last Arc reference is dropped, not via unconditional raw delete.
- [warning] AuctionHouseManager is held as Arc<AuctionHouseManager> in World, so destruction is deferred until the last Arc clone is dropped, unlike a directly owned C++ AuctionHouseMgr instance whose destructor runs at end of scope.
- [info] Rust uses DashMap<u32, Arc<Item>> instead of the unspecified C++ mAitems type; iteration and deallocation order are unspecified and may differ from std::map order.
- [info] C++ destructor body deletes values but does not erase map entries in the loop; Rust DashMap drop consumes and drops all entries as part of container destruction.
- [info] Range-for const auto itr binding and C++ delete-on-null-pointer semantics have no direct Rust equivalent; Arc<Item> enforces non-null owned references at compile time.
