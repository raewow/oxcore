# WorldSession::HandleAuctionPlaceBid

**File:** src/game/Handlers/AuctionHouseHandler.cpp

## Summary
Handles a client auction bid or buyout: validates GM/trial restrictions, packet fields, auction house and listing, ownership rules, bid increments, and player gold; on a normal bid updates bidder/bid in memory and DB, notifies the online owner, and sends success; on buyout charges buyout price, logs a transaction, sends success/won mail, removes the auction and item, then persists the bidder's inventory and gold in a character DB transaction.

## Source
```cpp
void WorldSession::HandleAuctionPlaceBid(WorldPackets::AuctionHouse::AuctionPlaceBid const& packet)
{
    if (!sWorld.getConfig(CONFIG_BOOL_GM_ALLOW_TRADES) && GetSecurity() > SEC_PLAYER)
    {
        SendAuctionCommandResult(nullptr, AUCTION_BID_PLACED, AUCTION_ERR_RESTRICTED_ACCOUNT);
        return;
    }

    if (HasTrialRestrictions())
    {
        SendAuctionCommandResult(nullptr, AUCTION_BID_PLACED, AUCTION_ERR_RESTRICTED_ACCOUNT);
        return;
    }

    if (!packet.auctionId || !packet.price)
    {
        SendAuctionCommandResult(nullptr, AUCTION_BID_PLACED, AUCTION_ERR_ITEM_NOT_FOUND);
        return;
    }

    AuctionHouseEntry const* auctionHouseEntry = GetCheckedAuctionHouseForAuctioneer(packet.auctioneerGuid);
    if (!auctionHouseEntry)
    {
        SendAuctionCommandResult(nullptr, AUCTION_BID_PLACED, AUCTION_ERR_ITEM_NOT_FOUND);
        return;
    }

    // always return pointer
    AuctionHouseObject* auctionHouse = sAuctionMgr.GetAuctionsMap(auctionHouseEntry);

    // remove fake death
    if (GetPlayer()->HasUnitState(UNIT_STATE_FEIGN_DEATH))
        GetPlayer()->RemoveSpellsCausingAura(SPELL_AURA_FEIGN_DEATH);

    AuctionEntry* auction = auctionHouse->GetAuction(packet.auctionId);
    Player* pl = GetPlayer();

    if (!auction)
    {
        // item not found; auction may have expired, or been bought out
        SendAuctionCommandResult(nullptr, AUCTION_BID_PLACED, AUCTION_ERR_ITEM_NOT_FOUND);
        return;
    }

    if (auction->owner == pl->GetGUIDLow())
    {
        // you cannot bid your own auction:
        SendAuctionCommandResult(nullptr, AUCTION_BID_PLACED, AUCTION_ERR_BID_OWN);
        return;
    }

    ObjectGuid ownerGuid = ObjectGuid(HIGHGUID_PLAYER, auction->owner);

    // impossible have online own another character (use this for speedup check in case online owner)
    Player* auction_owner = sObjectMgr.GetPlayer(ownerGuid);
    if (!auction_owner && sObjectMgr.GetPlayerAccountIdByGUID(ownerGuid) == pl->GetSession()->GetAccountId())
    {
        // you cannot bid your another character auction:
        SendAuctionCommandResult(nullptr, AUCTION_BID_PLACED, AUCTION_ERR_BID_OWN);
        return;
    }

    // cheating
    if (packet.price < auction->startbid)
    {
        SendAuctionCommandResult(nullptr, AUCTION_BID_PLACED, AUCTION_ERR_BID_INCREMENT);
        return;
    }

    // cheating or client lags
    if (packet.price <= auction->bid)
    {
        // client test but possible in result lags
        SendAuctionCommandResult(auction, AUCTION_BID_PLACED, AUCTION_ERR_HIGHER_BID);
        return;
    }

    // price too low for next bid if not buyout
    if ((packet.price < auction->buyout || auction->buyout == 0) &&
         packet.price < auction->bid + auction->GetAuctionOutBid())
    {
        // client test but possible in result lags
        SendAuctionCommandResult(auction, AUCTION_BID_PLACED, AUCTION_ERR_BID_INCREMENT);
        return;
    }

    if (packet.price > pl->GetMoney())
    {
        // you don't have enough money!, client tests!
        // SendAuctionCommandResult(auction->auctionId, AUCTION_ERR_INVENTORY, EQUIP_ERR_NOT_ENOUGH_MONEY);
        return;
    }

    if ((packet.price < auction->buyout) || (auction->buyout == 0))// bid
    {
        if (pl->GetGUIDLow() == auction->bidder)
            pl->LogModifyMoney(-int32(packet.price - auction->bid), "AuctionBid", ObjectGuid(HIGHGUID_PLAYER, auction->owner), auction->itemTemplate);
        else
        {
            pl->LogModifyMoney(-int32(packet.price), "AuctionBid", ObjectGuid(HIGHGUID_PLAYER, auction->owner), auction->itemTemplate);
            if (auction->bidder)                            // return money to old bidder if present
                SendAuctionOutbiddedMail(auction);
        }

        auction->bidder = pl->GetGUIDLow();
        auction->bid = packet.price;

        if (auction_owner)
            auction_owner->GetSession()->SendAuctionOwnerNotification(auction, false);

        // after this update we should save player's money ...
        CharacterDatabase.PExecute("UPDATE `auction` SET `buyer_guid` = '%u', `last_bid` = '%u' WHERE `id` = '%u'", auction->bidder, auction->bid, auction->Id);

        SendAuctionCommandResult(auction, AUCTION_BID_PLACED, AUCTION_OK);
    }
    else                                                    // buyout
    {
        if (pl->GetGUIDLow() == auction->bidder)
            pl->LogModifyMoney(-int32(auction->buyout - auction->bid), "AuctionBuyout", ObjectGuid(HIGHGUID_PLAYER, auction->owner), auction->itemTemplate);
        else
        {
            pl->LogModifyMoney(-int32(auction->buyout), "AuctionBuyout", ObjectGuid(HIGHGUID_PLAYER, auction->owner), auction->itemTemplate);
            if (auction->bidder)                            // return money to old bidder if present
                SendAuctionOutbiddedMail(auction);
        }

        auction->bidder = pl->GetGUIDLow();
        auction->bid = auction->buyout;

        PlayerTransactionData data;
        data.type = "Buyout";
        data.parts[0].lowGuid = auction->owner;
        data.parts[0].itemsEntries[0] = auction->itemTemplate;
        Item* item = sAuctionMgr.GetAItem(auction->itemGuidLow);
        data.parts[0].itemsCount[0] = item ? item->GetCount() : 0;
        data.parts[0].itemsGuid[0] = auction->itemGuidLow;
        data.parts[1].lowGuid = auction->bidder;
        data.parts[1].money = auction->bid;
        sWorld.LogTransaction(data);

        sAuctionMgr.SendAuctionSuccessfulMail(auction);
        sAuctionMgr.SendAuctionWonMail(auction);

        SendAuctionCommandResult(auction, AUCTION_BID_PLACED, AUCTION_OK);

        sAuctionMgr.RemoveAItem(auction->itemGuidLow);
        auctionHouse->RemoveAuction(auction);
        auction->DeleteFromDB();

        delete auction;
    }
    CharacterDatabase.BeginTransaction(pl->GetGUIDLow());
    pl->SaveInventoryAndGoldToDB();
    CharacterDatabase.CommitTransaction();
}
```
