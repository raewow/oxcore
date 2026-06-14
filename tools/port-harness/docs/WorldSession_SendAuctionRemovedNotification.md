# WorldSession::SendAuctionRemovedNotification

**File:** src/game/Handlers/AuctionHouseHandler.cpp

## Summary
Builds a WorldPackets::AuctionHouse::AuctionRemovedNotification from an AuctionEntry, optionally resolves item random property via sAuctionMgr.GetAItem, and sends the packet to the session client with SendPacket.

## Source
```cpp
void WorldSession::SendAuctionRemovedNotification(AuctionEntry* auction)
{
    Item *item = sAuctionMgr.GetAItem(auction->itemGuidLow);

    auto packet = std::make_unique<WorldPackets::AuctionHouse::AuctionRemovedNotification>();
    packet->auctionId = auction->Id;
    packet->itemTemplate = auction->itemTemplate;
    packet->randomPropertyId = item ? item->GetItemRandomPropertyId() : 0;
    SendPacket(std::move(packet));
}
```
