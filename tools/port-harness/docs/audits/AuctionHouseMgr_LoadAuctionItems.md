# Audit: AuctionHouseMgr::LoadAuctionItems

**Status:** missing  
**Passed:** false  
**Coverage:** 0/23 claims

## Summary
AuctionHouseMgr::LoadAuctionItems is not implemented in the Rust codebase. AuctionHouseManager has consumer-side infrastructure (auction_items cache, get_a_item, remove_a_item) used by load_auctions, but there is no load_auction_items function, no auction JOIN item_instance query, no prototype validation loop, no item creation/population from DB rows, and no AddAItem equivalent to register loaded items.

## Rust locations
(none)

## Issues
- [error] No load_auction_items (or equivalent) function exists in src/; grep finds no matches for LoadAuctionItems or load_auction_items.
- [error] No Rust code executes the C++ join query (SELECT creator_guid..item_id FROM auction JOIN item_instance ON item_guid = guid); no auction+item_instance JOIN exists anywhere in src/.
- [error] Empty/null query early-return path with BarGoLink step and '>> Loaded 0 auction items' log is not implemented.
- [error] Unknown item prototype branch (GetItemPrototype null → error log and skip row) is not implemented for auction item loading.
- [error] LoadFromDB failure branch (delete allocated item, skip row) is not implemented; Item::from_db_row is a simple constructor, not a LoadFromDB equivalent with field-array parsing and validation.
- [error] Success branch (AddAItem + increment count) is not implemented; no add_a_item method and no auction_items.insert call exists anywhere in src/.
- [error] Final summary log '>> Loaded %u auction items' after row iteration is not implemented.
- [warning] AuctionHouseManager defines auction_items DashMap and get_a_item/remove_a_item accessors in src/world/game/auction/manager.rs, but nothing populates the cache; load_auctions consumes get_a_item and will always find items missing.
- [info] BarGoLink helper and progress-bar pattern exist in the same manager.rs file but are only wired into load_auctions, not auction item loading.
- [info] ItemManager::get_template exists for prototype lookup and Item::from_db_row exists, but neither is used in an auction-item bootstrap flow matching the C++ LoadAuctionItems sequence.
