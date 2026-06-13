# AuctionHouseMgr::Update

**File:** src/game/AuctionHouse/AuctionHouseMgr.cpp

## Summary
AuctionHouseMgr::Update is a parameterless void tick handler that range-for iterates m_vRealAuctionHouses and calls Update() on each stored AuctionHouseObject via unique_ptr operator->.

## Source
```cpp
void AuctionHouseMgr::Update()
{
    for (const auto& itr : m_vRealAuctionHouses)
        itr->Update();
}
```
