# WorldSession::HandleAuctionListBidderItems

**File:** src/game/Handlers/AuctionHouseHandler.cpp

## Summary
Handles the client Auction List Bidder Items request: rejects if another AH list request is already in flight, validates the auctioneer and resolves the auction house, clears feign-death auras if present, then enqueues an asynchronous AUCTION_QUERY_LIST_BIDDER task with paging and outbid-refresh auction IDs. The handler returns without sending a list response directly.

## Source
```cpp
void WorldSession::HandleAuctionListBidderItems(WorldPackets::AuctionHouse::AuctionListBidderItem const& packet)
{
    if (ReceivedAHListRequest())
        return; // Only one AH request at a time is allowed

    AuctionHouseEntry const* auctionHouseEntry = GetCheckedAuctionHouseForAuctioneer(packet.auctioneerGuid);
    if (!auctionHouseEntry)
        return;

    // remove fake death
    if (GetPlayer()->HasUnitState(UNIT_STATE_FEIGN_DEATH))
        GetPlayer()->RemoveSpellsCausingAura(SPELL_AURA_FEIGN_DEATH);

    AuctionHouseClientQueryTask task(AUCTION_QUERY_LIST_BIDDER);
    task.auctionHouse = sAuctionMgr.GetAuctionsMap(auctionHouseEntry);
    task.accountId = GetAccountId();
    task.listfrom = packet.pagingElementStartIndex;
    task.outbiddedAuctionIds = packet.bidAuctionIdsToRefresh;
    SetReceivedAHListRequest(true);
    sWorld.AddAsyncTask(std::move(task));
}
```
