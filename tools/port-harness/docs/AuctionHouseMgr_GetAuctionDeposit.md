# AuctionHouseMgr::GetAuctionDeposit

**File:** src/game/AuctionHouse/AuctionHouseMgr.cpp

## Summary
Computes the auction listing deposit for an item: scales item vendor sell value and stack count by auction duration (integer-divided by MIN_AUCTION_TIME), applies the auction house deposit percent, floors at the configured minimum deposit, then multiplies by the configured deposit rate and returns the result truncated to uint32.

## Source
```cpp
uint32 AuctionHouseMgr::GetAuctionDeposit(AuctionHouseEntry const* entry, uint32 time, Item *pItem)
{
    float deposit = float(pItem->GetProto()->SellPrice * pItem->GetCount() * (time / MIN_AUCTION_TIME));

    deposit = deposit * entry->depositPercent / 100.0f;

    float min_deposit = float(sWorld.getConfig(CONFIG_UINT32_AUCTION_DEPOSIT_MIN));

    if (deposit < min_deposit)
        deposit = min_deposit;

    return uint32(deposit * sWorld.getConfig(CONFIG_FLOAT_RATE_AUCTION_DEPOSIT));
}
```
