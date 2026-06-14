# Audit: WorldSession::HandleAuctionListOwnerItems

## C++ Source
```cpp
void WorldSession::HandleAuctionListOwnerItems(WorldPackets::AuctionHouse::AuctionListOwnerItems const& packet)
{
    if (ReceivedAHListRequest())
        return;

    AuctionHouseEntry const* auctionHouseEntry = GetCheckedAuctionHouseForAuctioneer(packet.auctioneerGuid);
    if (!auctionHouseEntry)
        return;

    // remove fake death
    if (GetPlayer()->HasUnitState(UNIT_STATE_FEIGN_DEATH))
        GetPlayer()->RemoveSpellsCausingAura(SPELL_AURA_FEIGN_DEATH);

    AuctionHouseClientQueryTask task(AUCTION_QUERY_LIST_OWNER);
    task.auctionHouse = sAuctionMgr.GetAuctionsMap(auctionHouseEntry);
    task.accountId = GetAccountId();
    task.listfrom = packet.listfrom;
    SetReceivedAHListRequest(true);
    sWorld.AddAsyncTask({std::move(task)});
}
```

## Behaviour Claims

| Claim | Category | C++ Lines | Status |
|---|---|---|---|
| Reject duplicate AH list requests via `ReceivedAHListRequest()` | branch | 733 | Preserved |
| Validate auctioneer and resolve `AuctionHouseEntry` via `GetCheckedAuctionHouseForAuctioneer` | input | 736 | Preserved |
| Return silently when auction house resolution fails | branch | 737 | Preserved |
| Remove feign-death auras if `UNIT_STATE_FEIGN_DEATH` is present | side_effect | 741 | Preserved |
| Dereference `GetPlayer()` without null check (danger) | danger | 741 | Preserved by requiring `player_guid()` in Rust |
| Construct `AuctionHouseClientQueryTask` with `AUCTION_QUERY_LIST_OWNER` | side_effect | 744 | **Gap: async task system not ported** |
| Assign `task.auctionHouse` from `sAuctionMgr.GetAuctionsMap(...)` | side_effect | 745 | **Gap: async task system not ported** |
| Assign `task.accountId` from `GetAccountId()` | side_effect | 746 | **Gap: async task system not ported** |
| Assign `task.listfrom` from `packet.listfrom` | input | 747 | **Gap: async task system not ported** |
| Set `ReceivedAHListRequest(true)` before enqueue | side_effect | 748 | Preserved |
| Enqueue async task via `sWorld.AddAsyncTask` | side_effect | 749 | **Gap: async task system not ported** |

## Gaps

1. **Async task system**: `AuctionHouseClientQueryTask` and `sWorld.AddAsyncTask` do not exist in the Rust codebase. The Rust `World` struct has `background_tasks` but only for heartbeat/shutdown handlers, not for general work enqueueing.
2. **Async query response**: The actual list-building and `SMSG_AUCTION_LIST_RESULT` response is produced by the async task handler, which is out of scope for this symbol.

## Plan

1. Port the synchronous validation and gating logic into `src/world/handlers/auction.rs` as `handle_auction_list_owner_items`.
2. Wire the handler into `src/world/handlers/mod.rs` for `Opcode::CMSG_AUCTION_LIST_OWNER_ITEMS`.
3. Preserve the in-flight gate (`received_ah_list_request` / `set_received_ah_list_request`), auction house validation, feign-death cleanup, and packet parsing.
4. Mark the async task enqueue as a `TODO` with a tracing log, since the async task infrastructure is not yet available.
5. Add a unit test covering the duplicate-request rejection path and the success path (auction house validation + gate set + feign-death removal).
