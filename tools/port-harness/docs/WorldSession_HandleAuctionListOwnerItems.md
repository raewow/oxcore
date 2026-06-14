# WorldSession::HandleAuctionListOwnerItems

**File:** src/game/Handlers/AuctionHouseHandler.cpp

## Summary
Handles the client auction-house list-owner-items request: rejects when an AH list request is already in flight, validates the auctioneer and resolves an auction house entry, optionally clears feign-death auras, then enqueues an asynchronous AUCTION_QUERY_LIST_OWNER query with account id and paging offset. No list response is sent synchronously from this handler.

## Source
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
