# AuctionHouseMgr::RemoveAItem

**File:** src/game/AuctionHouse/AuctionHouseMgr.cpp

## Summary
Looks up `id` in the `mAitems` map; if missing returns false, otherwise erases that entry and returns true.

## Source
```cpp
bool AuctionHouseMgr::RemoveAItem(uint32 id)
{
    ItemMap::iterator i = mAitems.find(id);
    if (i == mAitems.end())
        return false;
    mAitems.erase(i);
    return true;
}
```
