# AuctionHouseMgr::GetAuctionsMap

**File:** src/game/AuctionHouse/AuctionHouseMgr.cpp

## Summary
Looks up the in-memory auction house map member `m_mAuctionHouses` using `house->houseId` as the key and returns the associated `AuctionHouseObject*` when found, otherwise returns `nullptr`. The function performs a read-only lookup and does not modify any state in the shown code.

## Source
```cpp
AuctionHouseObject * AuctionHouseMgr::GetAuctionsMap(AuctionHouseEntry const* house)
{
    auto itr = m_mAuctionHouses.find(house->houseId);
    if (itr != m_mAuctionHouses.end())
        return itr->second;

    return nullptr;
}
```
