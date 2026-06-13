# Audit: AuctionHouseMgr::AddAItem

**Status:** partial  
**Passed:** false  
**Coverage:** 10/12 claims

## Summary
add_a_item in src/world/game/auction/manager.rs implements the core C++ contract: assert-guarded, GUID-keyed cache insertion with duplicate detection and map storage on the success path. Both AddAItem flow branches (assert failure blocks insert; assert success inserts) are covered. Status is partial because Rust uses Arc<Item> shared ownership in a DashMap instead of raw Item* pointers with caller-owned deletion, and adds an extra zero-GUID assertion not present in the C++ source.

## Rust locations
- `add_a_item` in `src/world/game/auction/manager.rs`
- `auction_items` in `src/world/game/auction/manager.rs`
- `get_a_item` in `src/world/game/auction/manager.rs`
- `remove_a_item` in `src/world/game/auction/manager.rs`

## Issues
- [warning] C++ stores a raw Item* in mAitems with no ownership transfer; Rust stores Arc<Item> in auction_items, establishing shared reference-counted ownership rather than caller-owned raw pointers.
- [info] Rust takes Arc<Item> instead of Item*; null items cannot be passed, so the MANGOS_ASSERT(it) non-null guard is enforced by the type system rather than an explicit pointer check.
- [info] Rust adds assert!(guid_low != 0) before duplicate-key checking; this zero-GUID guard is not present in the C++ AddAItem snippet.
- [info] guid.low() is evaluated once and reused for both the duplicate check and insert, whereas C++ calls GetGUIDLow() twice; behavior is equivalent but call pattern differs.
- [info] mAitems concrete type is unspecified in C++; Rust uses DashMap<u32, Arc<Item>> (concurrent hash map) keyed by u32 GUID low.
