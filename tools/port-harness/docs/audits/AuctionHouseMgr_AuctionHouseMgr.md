# Audit: AuctionHouseMgr::AuctionHouseMgr

**Status:** partial  
**Passed:** false  
**Coverage:** 4/8 claims

## Summary
AuctionHouseManager::new is the Rust lifecycle entry point and produces the same post-construction container state as C++ (empty auction_items and auction_houses maps), but it is not a zero-argument empty constructor: it uses dependency injection, explicit field initialization, and DashMap::new() calls instead of an empty body with implicit default-initialization.

## Rust locations
- `AuctionHouseManager::new` in `src/world/game/auction/manager.rs`
- `AuctionHouseManager` in `src/world/game/auction/manager.rs`
- `World::new (auction_mgr construction)` in `src/world/world.rs`

## Issues
- [warning] C++ uses a zero-parameter default constructor; Rust requires four injected dependencies (auction_repo, character_repo, dbc, item_mgr) via AuctionHouseManager::new and has no Default impl.
- [warning] Rust explicitly initializes auction_items and auction_houses with DashMap::new() in the constructor equivalent, rather than relying on implicit compiler default-initialization with an empty body.
- [info] Rust constructor equivalent is not an empty body: it performs field assignments and calls DashMap::new(), unlike the C++ constructor which has zero executable statements and no function calls.
- [info] Rust stores four dependency fields (auction_repo, character_repo, dbc, item_mgr) at construction time; C++ default constructor initializes no such members (those are likely accessed elsewhere as globals/singletons).
