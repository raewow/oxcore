# AuctionHouseMgr::LoadAuctions

**File:** src/game/AuctionHouse/AuctionHouseMgr.cpp

## Summary
Loads all rows from the `auction` character-database table into in-memory auction-house maps. Each row becomes an `AuctionEntry`; rows whose item GUID is missing are deleted from the DB and skipped, and rows whose `house_id` is unknown are canceled via mail to the owner (using goblin auction house id 7 as mail context), the item is removed from the auction item cache, the DB row is deleted, and the entry is skipped. Successfully validated auctions are registered via `GetAuctionsMap(...)->AddAuction`.

## Source
```cpp
void AuctionHouseMgr::LoadAuctions()
{
    std::unique_ptr<QueryResult> result = CharacterDatabase.Query("SELECT `id`, `house_id`, `item_guid`, `item_id`, `seller_guid`, `buyout_price`, `expire_time`, `buyer_guid`, `last_bid`, `start_bid`, `deposit` FROM `auction`");
    if (!result)
    {
        BarGoLink bar(1);
        bar.step();
        sLog.Out(LOG_BASIC, LOG_LVL_MINIMAL, "");
        sLog.Out(LOG_BASIC, LOG_LVL_MINIMAL, ">> Loaded 0 auctions. DB table `auction` is empty.");
        return;
    }

    BarGoLink bar(result->GetRowCount());
    uint32 count = 0;

    do
    {
        Field* fields = result->Fetch();

        bar.step();

        AuctionEntry* auction = new AuctionEntry;
        auction->Id = fields[0].GetUInt32();
        uint32 houseId  = fields[1].GetUInt32();
        auction->itemGuidLow = fields[2].GetUInt32();
        auction->itemTemplate = fields[3].GetUInt32();
        auction->owner = fields[4].GetUInt32();
        auction->buyout = fields[5].GetUInt32();
        auction->depositTime = 0;
        auction->expireTime = fields[6].GetUInt32();
        auction->bidder = fields[7].GetUInt32();
        auction->bid = fields[8].GetUInt32();
        auction->startbid = fields[9].GetUInt32();
        auction->deposit = fields[10].GetUInt32();
        auction->auctionHouseEntry = nullptr;                  // init later

        auction->ownerAccount = sObjectMgr.GetPlayerAccountIdByGUID(auction->owner);

        // check if sold item exists for guid
        // and item_template in fact (GetAItem will fail if problematic in result check in AuctionHouseMgr::LoadAuctionItems)
        Item* pItem = GetAItem(auction->itemGuidLow);
        if (!pItem)
        {
            auction->DeleteFromDB();
            sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "Auction %u has not a existing item : %u, deleted", auction->Id, auction->itemGuidLow);
            delete auction;
            continue;
        }

        auction->auctionHouseEntry = sAuctionHouseStore.LookupEntry(houseId);

        if (!auction->auctionHouseEntry)
        {
            // need for send mail, use goblin auctionhouse
            auction->auctionHouseEntry = sAuctionHouseStore.LookupEntry(7);

            // Attempt send item back to owner
            std::ostringstream msgAuctionCanceledOwner;
            msgAuctionCanceledOwner << auction->itemTemplate << ":0:" << AUCTION_CANCELED;

            // item will deleted or added to received mail list
            MailDraft(msgAuctionCanceledOwner.str(), "")    // TODO: fix body
            .AddItem(pItem)
            .SendMailTo(MailReceiver(ObjectGuid(HIGHGUID_PLAYER, auction->owner)), auction, MAIL_CHECK_MASK_COPIED);

            RemoveAItem(auction->itemGuidLow);
            auction->DeleteFromDB();
            delete auction;

            continue;
        }

        GetAuctionsMap(auction->auctionHouseEntry)->AddAuction(auction);
        ++count;
    }
    while (result->NextRow());

    sLog.Out(LOG_BASIC, LOG_LVL_MINIMAL, "");
    sLog.Out(LOG_BASIC, LOG_LVL_MINIMAL, ">> Loaded %u auctions", count);
}
```
