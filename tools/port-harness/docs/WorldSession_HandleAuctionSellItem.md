# WorldSession::HandleAuctionSellItem

**File:** src/game/Handlers/AuctionHouseHandler.cpp

## Summary
Handles a client auction listing request: validates packet fields, account/GM/trial restrictions, auction house, duration, and item eligibility; charges deposit; creates an AuctionEntry; moves the item out of inventory; persists auction/item/player state in a DB transaction; responds with SendAuctionCommandResult. Early validation failures mostly send AUCTION_STARTED with a specific error; missing bid/etime returns silently with no client message.

## Source
```cpp
void WorldSession::HandleAuctionSellItem(WorldPackets::AuctionHouse::AuctionSellItem const& packet)
{
    if (!packet.bid || !packet.etime)
        return;                                             // check for cheaters

    // Client limit
    if (packet.bid > 2000000000 || packet.buyout > 2000000000)
    {
        SendAuctionCommandResult(nullptr, AUCTION_STARTED, AUCTION_ERR_NOT_ENOUGH_MONEY);
        ProcessAnticheatAction("GoldDupe", "Putting too high auction price", CHEAT_ACTION_LOG);
        return;
    }
    if (packet.buyout && packet.bid > packet.buyout)
    {
        SendAuctionCommandResult(nullptr, AUCTION_STARTED, AUCTION_ERR_HIGHER_BID);
        ProcessAnticheatAction("GoldDupe", "bid > buyout", CHEAT_ACTION_LOG);
        return;
    }

    if (!sWorld.getConfig(CONFIG_BOOL_GM_ALLOW_TRADES) && GetSecurity() > SEC_PLAYER)
    {
        SendAuctionCommandResult(nullptr, AUCTION_STARTED, AUCTION_ERR_RESTRICTED_ACCOUNT);
        return;
    }

    if (HasTrialRestrictions())
    {
        SendAuctionCommandResult(nullptr, AUCTION_STARTED, AUCTION_ERR_RESTRICTED_ACCOUNT);
        return;
    }

    Player* pl = GetPlayer();

    AuctionHouseEntry const* auctionHouseEntry = GetCheckedAuctionHouseForAuctioneer(packet.auctioneerGuid);
    if (!auctionHouseEntry)
    {
        SendAuctionCommandResult(nullptr, AUCTION_STARTED, AUCTION_ERR_DATABASE);
        return;
    }

    // always return pointer
    AuctionHouseObject* auctionHouse = sAuctionMgr.GetAuctionsMap(auctionHouseEntry);

    uint32 limit = sWorld.getConfig(CONFIG_UINT32_ACCOUNT_CONCURRENT_AUCTION_LIMIT);
    if (!!limit && auctionHouse->GetAccountAuctionCount(GetAccountId()) >= limit)
    {
        pl->SendSysMessage("You have reached the limit of active auctions on your account.");
        SendAuctionCommandResult(nullptr, AUCTION_STARTED, AUCTION_ERR_DATABASE);
        return;
    }

    // client send time in minutes, convert to common used sec time
    uint32 etime = packet.etime * MINUTE;

    // client understand only 3 auction time
    switch (etime)
    {
        case 1*MIN_AUCTION_TIME:
            break;
        case 4*MIN_AUCTION_TIME:
            break;
        case 12*MIN_AUCTION_TIME:
            break;
        default:
            SendAuctionCommandResult(nullptr, AUCTION_STARTED, AUCTION_ERR_DATABASE);
            return;
    }

    // remove fake death
    if (GetPlayer()->HasUnitState(UNIT_STATE_FEIGN_DEATH))
        GetPlayer()->RemoveSpellsCausingAura(SPELL_AURA_FEIGN_DEATH);

    if (!packet.itemGuid)
    {
        SendAuctionCommandResult(nullptr, AUCTION_STARTED, AUCTION_ERR_ITEM_NOT_FOUND);
        return;
    }

    Item *it = pl->GetItemByGuid(packet.itemGuid);

    // do not allow to sell already auctioned items
    if (sAuctionMgr.GetAItem(packet.itemGuid.GetCounter()))
    {
        sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "AuctionError, %s is sending %s, but item is already in another auction", pl->GetGuidStr().c_str(), packet.itemGuid.GetString().c_str());
        SendAuctionCommandResult(nullptr, AUCTION_STARTED, AUCTION_ERR_INVENTORY, EQUIP_ERR_ITEM_NOT_FOUND);
        return;
    }

    // prevent sending bag with items (cheat: can be placed in bag after adding equipped empty bag to auction)
    if (!it)
    {
        SendAuctionCommandResult(nullptr, AUCTION_STARTED, AUCTION_ERR_INVENTORY, EQUIP_ERR_ITEM_NOT_FOUND);
        return;
    }

    // prevent selling item in bank slot
    if (_player->IsBankPos(it->GetPos()))
    {
        SendAuctionCommandResult(nullptr, AUCTION_STARTED, AUCTION_ERR_INVENTORY, EQUIP_ERR_ITEM_NOT_FOUND);
        return;
    }

    if (!it->CanBeTraded())
    {
        SendAuctionCommandResult(nullptr, AUCTION_STARTED, AUCTION_ERR_INVENTORY, EQUIP_ERR_ITEM_NOT_FOUND);
        return;
    }

    if ((it->GetProto()->Flags & ITEM_FLAG_CONJURED) || it->GetUInt32Value(ITEM_FIELD_DURATION))
    {
        SendAuctionCommandResult(nullptr, AUCTION_STARTED, AUCTION_ERR_INVENTORY, EQUIP_ERR_ITEM_NOT_FOUND);
        return;
    }

    // check money for deposit
    uint32 deposit = AuctionHouseMgr::GetAuctionDeposit(auctionHouseEntry, etime, it);

    if (pl->GetMoney() < deposit)
    {
        SendAuctionCommandResult(nullptr, AUCTION_STARTED, AUCTION_ERR_NOT_ENOUGH_MONEY);
        return;
    }

    if (GetSecurity() > SEC_PLAYER && sWorld.getConfig(CONFIG_BOOL_GM_LOG_TRADE))
    {
        sLog.Player(GetAccountId(), LOG_GM, LOG_LVL_BASIC,
            "GM %s (Account: %u) create auction: %s (Entry: %u Count: %u)",
            GetPlayerName(), GetAccountId(), it->GetProto()->Name1, it->GetEntry(), it->GetCount());
    }

    pl->ModifyMoney(-int32(deposit));

    uint32 auction_time = uint32(etime * sWorld.getConfig(CONFIG_FLOAT_RATE_AUCTION_TIME));

    AuctionEntry* AH = new AuctionEntry;
    AH->Id = sObjectMgr.GenerateAuctionID();
    AH->itemGuidLow = it->GetObjectGuid().GetCounter();
    AH->itemTemplate = it->GetEntry();
    AH->owner = pl->GetGUIDLow();
    AH->ownerAccount = pl->GetSession()->GetAccountId();
    AH->startbid = packet.bid;
    AH->bidder = 0;
    AH->bid = 0;
    AH->buyout = packet.buyout;
    AH->lockedIpAddress = GetRemoteAddress();
    AH->depositTime = time(nullptr);
    AH->expireTime = time(nullptr) + auction_time;
    AH->deposit = deposit;
    AH->auctionHouseEntry = auctionHouseEntry;

    sLog.Player(this, LOG_MONEY_TRADES, LOG_LVL_MINIMAL, "[AuctionHouse]: Player %s listing %s (%u) at auctioneer %s. Initial bid: %u, buyout: %u, duration: %u, auctionhouse: %u",
                pl->GetShortDescription().c_str(), it->GetGuidStr().c_str(), it->GetEntry(),
                packet.auctioneerGuid.GetString().c_str(), packet.bid, packet.buyout, auction_time, AH->GetHouseId());

    // Log this transaction
    PlayerTransactionData data;
    data.type = "PlaceAuction";
    data.parts[0].lowGuid = AH->owner;
    data.parts[0].itemsEntries[0] = AH->itemTemplate;
    data.parts[0].itemsCount[0] = it->GetCount();
    data.parts[0].itemsGuid[0] = it->GetGUIDLow();
    data.parts[0].money = packet.bid;
    data.parts[1].lowGuid = packet.auctioneerGuid.GetCounter();
    data.parts[1].money = packet.buyout;
    sWorld.LogTransaction(data);

    auctionHouse->AddAuction(AH);

    sAuctionMgr.AddAItem(it);
    pl->MoveItemFromInventory(it->GetBagSlot(), it->GetSlot(), true);

    CharacterDatabase.BeginTransaction(pl->GetGUIDLow());
    it->DeleteFromInventoryDB();
    it->SaveToDB();                                         // recursive and not have transaction guard into self, not in inventiory and can be save standalone
    AH->SaveToDB();
    pl->SaveInventoryAndGoldToDB();
    CharacterDatabase.CommitTransaction();

    SendAuctionCommandResult(AH, AUCTION_STARTED, AUCTION_OK);
}
```
