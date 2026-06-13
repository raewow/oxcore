# AuctionHouseMgr::LoadAuctionHouses

**File:** src/game/AuctionHouse/AuctionHouseMgr.cpp

## Summary
Initializes member map m_mAuctionHouses by iterating every row index of sAuctionHouseStore, skipping null LookupEntry results, and associating each valid houseId with an AuctionHouseObject pointer. Exactly one of three mutually exclusive modes runs: cross-faction (single shared object for all entries), unlinked (one new object per entry, only when patch is below WOW_PATCH_109), or linked (three shared objects partitioned by GetAuctionHouseTeam). The function returns void and does not clear m_mAuctionHouses before inserting.

## Source
```cpp
void AuctionHouseMgr::LoadAuctionHouses()
{
    // Cross Faction - Single AH for all
    if (sWorld.getConfig(CONFIG_BOOL_ALLOW_TWO_SIDE_INTERACTION_AUCTION))
    {
        AuctionHouseObject* CrossFactionAuctionHouse = MakeNewAuctionHouseObject();

        for (uint32 i = 0; i < sAuctionHouseStore.GetNumRows(); i++)
        {
            AuctionHouseEntry const* houseEntry = sAuctionHouseStore.LookupEntry(i);
            if (!houseEntry)
                continue;

            m_mAuctionHouses.insert(std::make_pair(houseEntry->houseId, CrossFactionAuctionHouse));
        }
    }
    // Non-Linked Auction Houses - Separate AH for every DBC entry
    else if (sWorld.getConfig(CONFIG_BOOL_UNLINKED_AUCTION_HOUSES) && (sWorld.GetWowPatch() < WOW_PATCH_109))
    {
        for (uint32 i = 0; i < sAuctionHouseStore.GetNumRows(); i++)
        {
            AuctionHouseEntry const* houseEntry = sAuctionHouseStore.LookupEntry(i);
            if (!houseEntry)
                continue;

            m_mAuctionHouses.insert(std::make_pair(houseEntry->houseId, MakeNewAuctionHouseObject()));
        }
    }
    // Linked Auction Houses - One per faction
    else
    {
        AuctionHouseObject* AllianceAuctionHouse = MakeNewAuctionHouseObject();
        AuctionHouseObject* HordeAuctionHouse = MakeNewAuctionHouseObject();
        AuctionHouseObject* NeutralAuctionHouse = MakeNewAuctionHouseObject();

        for (uint32 i = 0; i < sAuctionHouseStore.GetNumRows(); i++)
        {
            AuctionHouseEntry const* houseEntry = sAuctionHouseStore.LookupEntry(i);
            if (!houseEntry)
                continue;

            switch (GetAuctionHouseTeam(houseEntry))
            {
                case ALLIANCE:
                    m_mAuctionHouses.insert(std::make_pair(houseEntry->houseId, AllianceAuctionHouse));
                    break;
                case HORDE:
                    m_mAuctionHouses.insert(std::make_pair(houseEntry->houseId, HordeAuctionHouse));
                    break;
                default:
                    m_mAuctionHouses.insert(std::make_pair(houseEntry->houseId, NeutralAuctionHouse));
                    break;
            }
        }
    }
}
```
