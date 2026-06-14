# WorldSession::HandleAuctionListItems

**File:** src/game/Handlers/AuctionHouseHandler.cpp

## Summary
Handles the client Auction List Items opcode: bails out if an AH list request is already marked in-flight, builds an AUCTION_QUERY_LIST async task from packet filters, validates the auctioneer and resolves the auction house map, clears feign-death auras, converts the search name to lowercase wide string, then marks the list request received and enqueues the task. No direct list response is sent from this handler.

## Source
```cpp
void WorldSession::HandleAuctionListItems(WorldPackets::AuctionHouse::AuctionListItems const& packet)
{
    if (ReceivedAHListRequest())
        return;

    AuctionHouseClientQueryTask task(AUCTION_QUERY_LIST);
    task.accountId = GetAccountId();

    task.listfrom = packet.listfrom;
    task.levelmin = packet.levelmin;
    task.levelmax = packet.levelmax;
    task.auctionSlotID = packet.auctionSlotID;
    task.auctionMainCategory = packet.auctionMainCategory;
    task.auctionSubCategory = packet.auctionSubCategory;
    task.quality = packet.quality;
    task.usable = packet.usable;

    AuctionHouseEntry const* auctionHouseEntry = GetCheckedAuctionHouseForAuctioneer(packet.auctioneerGuid);
    if (!auctionHouseEntry)
        return;

    // always return pointer
    task.auctionHouse = sAuctionMgr.GetAuctionsMap(auctionHouseEntry);

    // remove fake death
    if (GetPlayer()->HasUnitState(UNIT_STATE_FEIGN_DEATH))
        GetPlayer()->RemoveSpellsCausingAura(SPELL_AURA_FEIGN_DEATH);

    // converting string that we try to find to lower case
    if (!Utf8toWStr(packet.searchedname, task.wsearchedname))
        return;

    wstrToLower(task.wsearchedname);
    SetReceivedAHListRequest(true);
    sWorld.AddAsyncTask(std::move(task));
}
```
