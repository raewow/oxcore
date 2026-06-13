# Audit: AuctionHouseMgr::LoadAuctions

**Status:** missing  
**Passed:** false  
**Coverage:** 0/40 claims

## Summary
AuctionHouseMgr::LoadAuctions is not implemented in the Rust codebase. The repo has auction DB models/repository methods, packet types, and DBC auction-house lookup, but no bootstrap loader that reads all auction rows, validates items via an in-memory cache, handles orphaned/invalid-house auctions, or registers entries into per-house maps. All four flow branches are uncovered; status is missing.

## Rust locations
(none)

## Issues
- [error] No LoadAuctions/load_auctions bootstrap function exists in src/; world init (src/world/world.rs) never loads auction rows into memory.
- [error] No unconditional SELECT of all auction table rows at startup. AuctionRepository only exposes filtered queries (by id, house, seller, bidder, active) — none match the full-table bootstrap query.
- [error] Empty-result early-return path (log 0 auctions loaded, return without loading) is not implemented.
- [error] No AuctionHouseMgr equivalent, no per-house GetAuctionsMap/AddAuction in-memory registry, and no auction house object maps in SystemManager.
- [error] No GetAItem/AddAItem/RemoveAItem auction-item cache; LoadAuctionItems prerequisite flow is also absent, so item existence checks during auction load cannot occur.
- [error] Missing-item cleanup branch (DeleteFromDB, error log, skip AddAuction) is not wired into any bootstrap loader.
- [error] Invalid house_id recovery (fallback to goblin AH id 7, AUCTION_CANCELED mail with item to owner, RemoveAItem, DeleteFromDB) is not implemented.
- [error] No startup logic maps DB rows to AuctionEntry with depositTime=0, ownerAccount from seller GUID lookup, and DBC house resolution.
- [warning] Supporting pieces exist in isolation (AuctionRow model, AuctionRepository CRUD, AuctionEntry struct, DbcManager::get_auction_house, delete_auction) but are not composed into LoadAuctions behaviour.
- [info] AuctionEntry::new sets deposit_time to current Unix time rather than 0; this only matters once a loader is implemented.
