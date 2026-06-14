# WorldSession::SendAuctionOwnerNotification

**File:** src/game/Handlers/AuctionHouseHandler.cpp

## Summary
Builds a WorldPackets::AuctionHouse::AuctionOwnerNotification from an AuctionEntry and sold flag, optionally sets bidderGuid when not sold, resolves randomPropertyId via sAuctionMgr.GetAItem, and sends the packet to the session client with SendPacket.

## Source
```cpp
void WorldSession::SendAuctionOwnerNotification(AuctionEntry* auction, bool sold)
{
    auto notification = std::make_unique<WorldPackets::AuctionHouse::AuctionOwnerNotification>();
    notification->auctionId = auction->Id;
    notification->bid = auction->bid;
    notification->outBid = auction->GetAuctionOutBid();

    if (!sold)                                               // not sold yet
        notification->bidderGuid = ObjectGuid(HIGHGUID_PLAYER, auction->bidder);

    notification->itemTemplate = auction->itemTemplate;

    Item *item = sAuctionMgr.GetAItem(auction->itemGuidLow);
    notification->randomPropertyId = item ? item->GetItemRandomPropertyId() : 0;

    SendPacket(std::move(notification));
}
```
