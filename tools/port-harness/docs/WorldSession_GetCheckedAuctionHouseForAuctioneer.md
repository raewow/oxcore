# WorldSession::GetCheckedAuctionHouseForAuctioneer

**File:** src/game/Handlers/AuctionHouseHandler.cpp

## Summary
Validates that the session player may open an auction house for the given ObjectGuid (either as self/GM command access or via a reachable auctioneer NPC), then returns the AuctionHouseEntry resolved from the validated Unit via AuctionHouseMgr::GetAuctionHouseEntry; returns nullptr on failed validation after logging a debug cheat attempt.

## Source
```cpp
AuctionHouseEntry const* WorldSession::GetCheckedAuctionHouseForAuctioneer(ObjectGuid guid)
{
    Unit* auctioneer = nullptr;

    // GM case
    if (guid == GetPlayer()->GetObjectGuid())
    {
        // command case will return only if player have real access to command
        // using special access modes (1,-1) done at mode set in command, so not need recheck
        if (GetPlayer()->GetAuctionAccessMode() == 0 && !ChatHandler(GetPlayer()).FindCommand("auction"))
        {
            sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "%s attempt open auction in cheating way.", guid.GetString().c_str());
            return nullptr;
        }

        auctioneer = GetPlayer();
    }
    // auctioneer case
    else
    {
        auctioneer = GetPlayer()->GetNPCIfCanInteractWith(guid, UNIT_NPC_FLAG_AUCTIONEER);
        if (!auctioneer)
        {
            sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "Auctioneeer %s accessed in cheating way.", guid.GetString().c_str());
            return nullptr;
        }
    }

    // always return pointer
    return AuctionHouseMgr::GetAuctionHouseEntry(auctioneer);
}
```
