# AuctionHouseMgr::GetAuctionHouseEntry

**File:** src/game/AuctionHouse/AuctionHouseMgr.cpp

## Summary
Resolves which AuctionHouseEntry applies to a Unit by computing a houseId (default 1), optionally adjusting it when two-faction auction interaction is disabled (NPC faction mapping or player access mode/team), then returning sAuctionHouseStore.LookupEntry(houseId).

## Source
```cpp
AuctionHouseEntry const* AuctionHouseMgr::GetAuctionHouseEntry(Unit* unit)
{
    uint32 houseId = 1;                                     // dwarf auction house (used for normal cut/etc percents)

    if (!sWorld.getConfig(CONFIG_BOOL_ALLOW_TWO_SIDE_INTERACTION_AUCTION))
    {
        if (unit->GetTypeId() == TYPEID_UNIT)
        {
            // FIXME: found way for proper auctionhouse selection by another way
            // AuctionHouse.dbc have faction field with _player_ factions associated with auction house races.
            // but no easy way convert creature faction to player race faction for specific city
            houseId = GetAuctionHouseId(unit->GetFactionTemplateId());
        }
        else
        {
            Player* player = (Player*)unit;
            if (player->GetAuctionAccessMode() > 0)
                houseId = 7;
            else
            {
                switch (((Player*)unit)->GetTeam())
                {
                    case ALLIANCE:
                        houseId = player->GetAuctionAccessMode() == 0 ? 1 : 6;
                        break;
                    case HORDE:
                        houseId = player->GetAuctionAccessMode() == 0 ? 6 : 1;
                        break;
                }
            }
        }
    }

    return sAuctionHouseStore.LookupEntry(houseId);
}
```
