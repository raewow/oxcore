# WorldSession::HandleAuctionRemoveItem

**File:** src/game/Handlers/AuctionHouseHandler.cpp

## Summary
Handles a client auction-remove request: validates auctioneer access, clears feign death, cancels the listing only if the session player owns the auction, mails the item back to the owner (refunding an active bidder and charging the owner the auction cut when bidder > 0), notifies the client on success, then deletes the auction from the database and in-memory auction manager state.

## Source
```cpp
void WorldSession::HandleAuctionRemoveItem(WorldPackets::AuctionHouse::AuctionRemoveItem const& packet)
{
    AuctionHouseEntry const* auctionHouseEntry = GetCheckedAuctionHouseForAuctioneer(packet.auctioneerGuid);
    if (!auctionHouseEntry)
        return;

    // remove fake death
    if (GetPlayer()->HasUnitState(UNIT_STATE_FEIGN_DEATH))
        GetPlayer()->RemoveSpellsCausingAura(SPELL_AURA_FEIGN_DEATH);

    // always return pointer
    AuctionHouseObject* auctionHouse = sAuctionMgr.GetAuctionsMap(auctionHouseEntry);
    AuctionEntry* auction = auctionHouse->GetAuction(packet.auctionId);
    Player* pl = GetPlayer();

    if (auction && auction->owner == pl->GetGUIDLow())
    {
        Item *pItem = sAuctionMgr.GetAItem(auction->itemGuidLow);
        if (pItem)
        {
            if (auction->bidder > 0)                        // If we have a bidder, we have to send him the money he paid
            {
                uint32 auctionCut = auction->GetAuctionCut();
                if (pl->GetMoney() < auctionCut)            // player doesn't have enough money, maybe message needed
                    return;

                SendAuctionCancelledToBidderMail(auction);
                pl->ModifyMoney(-int32(auctionCut));
            }
            // Return the item by mail
            std::ostringstream msgAuctionCanceledOwner;
            msgAuctionCanceledOwner << auction->itemTemplate << ":0:" << AUCTION_CANCELED;

            // item will deleted or added to received mail list
            MailDraft(msgAuctionCanceledOwner.str())
            .AddItem(pItem)
            .SendMailTo(pl, auction, MAIL_CHECK_MASK_COPIED);
        }
        else
        {
            sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "Auction id: %u has nonexistent item (item guid : %u)!!!", auction->Id, auction->itemGuidLow);
            SendAuctionCommandResult(nullptr, AUCTION_REMOVED, AUCTION_ERR_INVENTORY, EQUIP_ERR_ITEM_NOT_FOUND);
            return;
        }
    }
    else
    {
        SendAuctionCommandResult(nullptr, AUCTION_REMOVED, AUCTION_ERR_DATABASE);
        // this code isn't possible ... maybe there should be ASSERT
        sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "CHEATER : %u, he tried to cancel auction (id: %u) of another player, or auction is nullptr", pl->GetGUIDLow(), packet.auctionId);
        return;
    }

    // inform player, that auction is removed
    SendAuctionCommandResult(auction, AUCTION_REMOVED, AUCTION_OK);
    // Now remove the auction
    CharacterDatabase.BeginTransaction(pl->GetGUIDLow());
    auction->DeleteFromDB();
    pl->SaveInventoryAndGoldToDB();
    CharacterDatabase.CommitTransaction();
    sAuctionMgr.RemoveAItem(auction->itemGuidLow);
    auctionHouse->RemoveAuction(auction);
    delete auction;
}
```
