# WorldSession::SendAuctionCancelledToBidderMail

**File:** src/game/Handlers/AuctionHouseHandler.cpp

## Summary
When an auction is cancelled, resolves the bidder by GUID from auction->bidder (online Player lookup, otherwise account id by GUID). If either resolves, builds a mail subject from itemTemplate and AUCTION_CANCELLED_TO_BIDDER, optionally sends AuctionRemovedNotification to an online bidder, and mails gold equal to auction->bid via MailDraft. If neither an online Player nor a non-zero account id is found, the function does nothing.

## Source
```cpp
void WorldSession::SendAuctionCancelledToBidderMail(AuctionEntry* auction)
{
    ObjectGuid bidder_guid = ObjectGuid(HIGHGUID_PLAYER, auction->bidder);
    Player* bidder = sObjectMgr.GetPlayer(bidder_guid);

    uint32 bidder_accId = 0;
    if (!bidder)
        bidder_accId = sObjectMgr.GetPlayerAccountIdByGUID(bidder_guid);

    // bidder exist
    if (bidder || bidder_accId)
    {
        std::ostringstream msgAuctionCancelledSubject;
        msgAuctionCancelledSubject << auction->itemTemplate << ":0:" << AUCTION_CANCELLED_TO_BIDDER;

        if (bidder)
            bidder->GetSession()->SendAuctionRemovedNotification(auction);

        MailDraft(msgAuctionCancelledSubject.str())
        .SetMoney(auction->bid)
        .SendMailTo(MailReceiver(bidder, bidder_guid), auction, MAIL_CHECK_MASK_COPIED);
    }
}
```
