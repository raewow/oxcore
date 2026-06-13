# AuctionHouseMgr::~AuctionHouseMgr

**File:** src/game/AuctionHouse/AuctionHouseMgr.cpp

## Summary
When an AuctionHouseMgr instance is destroyed, the destructor iterates every entry in member container mAitems and calls delete on each entry's .second pointer; the loop body performs no other operations and contains no branches.

## Source
```cpp
AuctionHouseMgr::~AuctionHouseMgr()
{
    for (const auto itr : mAitems)
        delete itr.second;
}
```
