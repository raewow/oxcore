# AuctionHouseMgr::SendAuctionExpiredMail

**File:** src/game/AuctionHouse/AuctionHouseMgr.cpp

## Summary
Returns an expired auction item to its owner by mail when the item exists in the auction item map and the owner is online or resolvable to a non-zero account id; otherwise deletes the item from the database and heap. Logs an error and returns early if the auction item cannot be found.

## Source
```cpp
void AuctionHouseMgr::SendAuctionExpiredMail(AuctionEntry* auction)
{
    // return an item in auction to its owner by mail
    Item *pItem = GetAItem(auction->itemGuidLow);
    if (!pItem)
    {
        sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "Auction item (GUID: %u) not found, and lost.", auction->itemGuidLow);
        return;
    }

    ObjectGuid owner_guid = ObjectGuid(HIGHGUID_PLAYER, auction->owner);
    Player* owner = sObjectMgr.GetPlayer(owner_guid);

    uint32 owner_accId = 0;
    if (!owner)
        owner_accId = sObjectMgr.GetPlayerAccountIdByGUID(owner_guid);

    // owner exist
    if (owner || owner_accId)
    {
        std::ostringstream subject;
        subject << auction->itemTemplate << ":0:" << AUCTION_EXPIRED;

        if (owner)
            owner->GetSession()->SendAuctionOwnerNotification(auction, false);
        else
            RemoveAItem(pItem->GetGUIDLow());               // we have to remove the item, before we delete it !!

        // will delete item or place to receiver mail list
        MailDraft(subject.str())
        .AddItem(pItem)
        .SendMailTo(MailReceiver(owner, owner_guid), auction, MAIL_CHECK_MASK_COPIED);
    }
    // owner not found
    else
    {
        CharacterDatabase.PExecute("DELETE FROM `item_instance` WHERE `guid`='%u'", pItem->GetGUIDLow());
        RemoveAItem(pItem->GetGUIDLow());                   // we have to remove the item, before we delete it !!
        delete pItem;
    }
}
```
