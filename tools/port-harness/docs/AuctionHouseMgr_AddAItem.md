# AuctionHouseMgr::AddAItem

**File:** src/game/AuctionHouse/AuctionHouseMgr.cpp

## Summary
Registers a non-null Item pointer in the AuctionHouseMgr member map mAitems, keyed by the item's low GUID, after asserting the pointer is valid and that no entry with that key already exists.

## Source
```cpp
void AuctionHouseMgr::AddAItem(Item* it)
{
    MANGOS_ASSERT(it);
    MANGOS_ASSERT(mAitems.find(it->GetGUIDLow()) == mAitems.end());
    mAitems[it->GetGUIDLow()] = it;
}
```
