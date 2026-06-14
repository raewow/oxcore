# Audit: AuctionHouseMgr::LoadAuctionHouses

**Status:** complete  
**Passed:** true  
**Coverage:** 24/24 claims

## Summary
LoadAuctionHouses is fully ported as load_auction_houses with equivalent three-way topology selection (cross-faction, unlinked with wow_patch < 109, linked team-shared), DBC-driven house_id population, get_auction_house_team faction mapping (1-3 Alliance, 4-6 Horde, else neutral), and companion get_auctions_map lookup returning None on miss. Config is wired from world.rs. Minor differences are Rust idioms (Result, Arc, HashMap iteration) and DashMap duplicate-key overwrite semantics.

## Rust locations
- `load_auction_houses` in `src/world/game/auction/manager.rs`
- `get_auction_house_team` in `src/world/game/auction/manager.rs`
- `get_auctions_map` in `src/world/game/auction/manager.rs`
- `World::load (auction bootstrap call site)` in `src/world/world.rs`
- `load_auction_houses_* tests` in `src/world/game/auction/tests.rs`

## Issues
- [info] Rust returns Result<()> instead of C++ void; success path always returns Ok(()) with no error branches in the topology logic.
- [info] auction_houses is not cleared before inserts, matching the C++ assumption that prior map contents persist unless cleared elsewhere.
- [info] Rust collects valid DBC entries via get_all_auction_houses() (HashMap iterator) rather than index 0..GetNumRows() with per-index LookupEntry; null/missing rows are implicitly skipped, producing the same insert set for normal DBC data.
- [info] Shared topology uses Arc<AuctionHouseObject> instead of raw AuctionHouseObject*; cross-faction and linked modes still share one object across multiple house_id keys.
- [warning] DashMap::insert overwrites an existing house_id key, whereas C++ std::map::insert leaves the first entry unchanged on duplicate keys.
- [info] Cross-faction test asserts map presence but does not verify Arc pointer equality across house_ids; implementation does use Arc::clone of a single shared object.
