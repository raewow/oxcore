# AuctionHouseMgr::MakeNewAuctionHouseObject

**File:** src/game/AuctionHouse/AuctionHouseMgr.cpp

## Summary
Appends a default-constructed AuctionHouseObject owned by a new std::unique_ptr to m_vRealAuctionHouses, then returns a non-owning raw pointer to that newly appended object via back().get().

## Source
```cpp
AuctionHouseObject* AuctionHouseMgr::MakeNewAuctionHouseObject()
{
    m_vRealAuctionHouses.push_back(std::make_unique<AuctionHouseObject>());
    return m_vRealAuctionHouses.back().get();
}
```
