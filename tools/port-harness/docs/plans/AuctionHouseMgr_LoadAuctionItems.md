# Plan: AuctionHouseMgr::LoadAuctionItems

**Target file:** `src/world/game/auction/manager.rs`  
**Rust symbol:** `load_auction_items`

## Structs
- AuctionHouseManager
- AuctionItemLoadRow
- Item
- ItemTemplate
- BarGoLink

## Enums
—

## Notes
Implement as a public async fn on AuctionHouseManager returning Result<()>, mirroring the existing load_auctions pattern in the same file. Execute the exact C++ JOIN SQL via sqlx::query_as against character_db, selecting columns in order: creator_guid, gift_creator_guid, count, duration, charges, flags, enchantments, random_property_id, durability, text, item_guid, item_id. Define AuctionItemLoadRow as a sqlx::FromRow struct (recommended location: src/shared/database/characters/models/auction.rs alongside AuctionRow) with field types aligned to ItemInstanceRow (charges as Option<String>, random_property_id as i16, durability as u16, text as u32). On query failure or no rows, match C++ null-result behavior: BarGoLink::new(1).step(), blank info log, info log with exactly '>> Loaded 0 auction items', return Ok(()). On success, initialize BarGoLink with row count, iterate rows, step bar before reading each row, track a local count u32. AuctionHouseManager currently lacks ItemManager; add Arc<ItemManager> (injected via new) for prototype lookup via get_template (C++ sObjectMgr.GetItemPrototype). Unknown template: error! with GUID and item_id, skip row without incrementing count. C++ NewItemOrBag(proto) has no Rust equivalent; for auction cache (DashMap<u32, Arc<Item>>) construct Item via a new Item::load_from_db or equivalent that accepts the row fields and ItemTemplate, using ObjectGuid::new_without_entry(HighGuid::Item, item_guid) and ObjectGuid::empty() owner (matching empty ObjectGuid() second arg). Reuse inventory field parsing semantics for enchantments/charges (InventorySystem::parse_enchantments / parse_spell_charges are private today—either expose shared helpers or implement parsing inside Item::load_from_db). load_from_db must return bool: false skips row (no allocation cleanup needed in Rust); true registers item. Add private add_a_item(guid, Arc<Item>) matching C++ AddAItem: assert non-null equivalent, assert no duplicate key in auction_items (panic or expect to mirror MANGOS_ASSERT), then insert. On success increment count. C++ raw Item* ownership becomes Arc<Item> in the DashMap; no delete on failure. NewItemOrBag Bag subclass is not modeled in auction_items; bag-template auction listings are represented as Item only (container slot state is irrelevant until post-auction handling). Must be called before load_auctions during world bootstrap so get_a_item resolves entries. Wire into startup once AuctionHouseManager is integrated into SystemManager/World init.
