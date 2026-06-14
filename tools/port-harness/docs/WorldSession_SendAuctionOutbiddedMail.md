# WorldSession::SendAuctionOutbiddedMail

**File:** src/game/Handlers/AuctionHouseHandler.cpp

## Summary
Notifies the previous highest bidder that they were outbid: if the old bidder is online or resolvable to a non-zero account id, optionally sends an outbid client notification when online, then mails them gold equal to auction->bid with a templated subject derived from itemTemplate and AUCTION_OUTBIDDED. If neither an online Player nor a resolvable account id exists, the function performs no visible work.

## Source
```cpp
void WorldSession::SendAuctionOutbiddedMail(AuctionEntry* auction)
{
    ObjectGuid oldBidder_guid = ObjectGuid(HIGHGUID_PLAYER, auction->bidder);
    Player* oldBidder = sObjectMgr.GetPlayer(oldBidder_guid);

    uint32 oldBidder_accId = 0;
    if (!oldBidder)
        oldBidder_accId = sObjectMgr.GetPlayerAccountIdByGUID(oldBidder_guid);

    // old bidder exist
    if (oldBidder || oldBidder_accId)
    {
        std::ostringstream msgAuctionOutbiddedSubject;
        msgAuctionOutbiddedSubject << auction->itemTemplate << ":0:" << AUCTION_OUTBIDDED;

        if (oldBidder)
            oldBidder->GetSession()->SendAuctionBidderNotification(auction, false);

        MailDraft(msgAuctionOutbiddedSubject.str())
        .SetMoney(auction->bid)
        .SendMailTo(MailReceiver(oldBidder, oldBidder_guid), auction, MAIL_CHECK_MASK_COPIED);
    }
}
```
