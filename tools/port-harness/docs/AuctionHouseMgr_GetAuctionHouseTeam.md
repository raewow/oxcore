# AuctionHouseMgr::GetAuctionHouseTeam

**File:** src/game/AuctionHouse/AuctionHouseMgr.cpp

## Summary
Maps an auction house entry to a faction team id by switching on house->houseId: ids 1–3 return ALLIANCE, 4–6 return HORDE, and 7 or any other id return 0 (neutral). The function does not use the entry's faction field; it performs a read-only lookup with no observable side effects in the shown code.

## Source
```cpp
uint32 AuctionHouseMgr::GetAuctionHouseTeam(AuctionHouseEntry const* house)
{
    // auction houses have faction field pointing to PLAYER,* factions,
    // but player factions not have filled team field, and hard go from faction value to faction_template value,
    // so more easy just sort by auction house ids
    switch (house->houseId)
    {
        case 1:
        case 2:
        case 3:
            return ALLIANCE;
        case 4:
        case 5:
        case 6:
            return HORDE;
        case 7:
        default:
            return 0;                                       // neutral
    }
}
```
