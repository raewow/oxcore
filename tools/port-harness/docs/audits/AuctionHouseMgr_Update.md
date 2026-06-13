# Audit: AuctionHouseMgr::Update

**Status:** missing  
**Passed:** false  
**Coverage:** 0/9 claims

## Summary
AuctionHouseMgr::Update is not implemented in Rust. AuctionHouseManager and AuctionHouseObject exist for loading and in-memory storage, but neither defines an update tick, and the world main loop never invokes auction manager updates. The unrelated Update matches found elsewhere are false positives.

## Rust locations
(none)

## Issues
- [error] AuctionHouseManager in src/world/game/auction/manager.rs has load/init helpers but no update() method that forwards a periodic tick.
- [error] AuctionHouseObject in src/world/game/auction/manager.rs exposes new/add_auction/auction_count only; there is no update() method to delegate per-house tick work to.
- [error] World::update in src/world/world.rs (lines 383-480) runs the main tick loop but never calls auction_mgr or any auction-house update path.
- [warning] Rust stores houses in auction_houses: DashMap<u32, Arc<AuctionHouseObject>> with shared Arc clones for linked/cross-faction modes, whereas C++ iterates m_vRealAuctionHouses (unique real objects). A future port must deduplicate Arc values when iterating to avoid multiple Update calls on the same logical house.
- [info] Repo search hits for Update in src/auth/auth/socket.rs, src/auth/common/codes.rs (FailVersionUpdate), src/auth/realm/list.rs (update_if_needed), src/shared/database/auth/repositories/*.rs (SQL UPDATE), and src/bin/world.rs (world update loop comment) are unrelated to AuctionHouseMgr::Update.
