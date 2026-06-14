# WorldSession::SendAuctionBidderNotification

**File:** src/game/Handlers/AuctionHouseHandler.cpp

## Summary
Builds a WorldPackets::AuctionHouse::AuctionBidderNotification from an AuctionEntry and won flag, optionally looks up the auction item for randomPropertyId, and sends it to the session client via SendPacket.

## Source
```cpp
void WorldSession::SendAuctionBidderNotification(AuctionEntry* auction, bool won)
{
    auto notification = std::make_unique<WorldPackets::AuctionHouse::AuctionBidderNotification>();
    notification->houseId = auction->GetHouseId();
    notification->auctionId = auction->Id;
    notification->bidderGuid = ObjectGuid(HIGHGUID_PLAYER, auction->bidder);
    // if 0, client shows ERR_AUCTION_WON_S, else ERR_AUCTION_OUTBID_S
    notification->bidOrZero = won ? 0 : auction->bid;
    notification->outBid = auction->GetAuctionOutBid();
    notification->itemTemplate = auction->itemTemplate;

    Item *item = sAuctionMgr.GetAItem(auction->itemGuidLow);
    notification->randomPropertyId = item ? item->GetItemRandomPropertyId() : 0;

    SendPacket(std::move(notification));
}
```
