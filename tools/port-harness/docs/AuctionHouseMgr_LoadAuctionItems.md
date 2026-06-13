# AuctionHouseMgr::LoadAuctionItems

**File:** src/game/AuctionHouse/AuctionHouseMgr.cpp

## Summary
Loads auction-listed items from the character database by joining `auction` with `item_instance`, hydrates each row into an `Item` via prototype lookup and `LoadFromDB`, registers successful items in the manager cache with `AddAItem`, skips invalid rows with error logging, and reports a final loaded count.

## Source
```cpp
void AuctionHouseMgr::LoadAuctionItems()
{
    //                                                                     0               1                    2        3           4          5        6               7                     8             9       10                           11
    std::unique_ptr<QueryResult> result = CharacterDatabase.Query("SELECT `creator_guid`, `gift_creator_guid`, `count`, `duration`, `charges`, `flags`, `enchantments`, `random_property_id`, `durability`, `text`, `item_guid`, `item_instance`.`item_id` FROM `auction` JOIN `item_instance` ON `item_guid` = `guid`");

    if (!result)
    {
        BarGoLink bar(1);
        bar.step();
        sLog.Out(LOG_BASIC, LOG_LVL_MINIMAL, "");
        sLog.Out(LOG_BASIC, LOG_LVL_MINIMAL, ">> Loaded 0 auction items");
        return;
    }

    BarGoLink bar(result->GetRowCount());

    uint32 count = 0;

    Field* fields;
    do
    {
        bar.step();

        fields = result->Fetch();
        uint32 itemGuid = fields[10].GetUInt32();
        uint32 itemId = fields[11].GetUInt32();

        ItemPrototype const* proto = sObjectMgr.GetItemPrototype(itemId);

        if (!proto)
        {
            sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "AuctionHouseMgr::LoadAuctionItems: Unknown item (GUID: %u id: #%u) in auction, skipped.", itemGuid, itemId);
            continue;
        }

        Item *item = NewItemOrBag(proto);

        if (!item->LoadFromDB(itemGuid, ObjectGuid(), fields, itemId))
        {
            delete item;
            continue;
        }
        AddAItem(item);

        ++count;
    }
    while (result->NextRow());

    sLog.Out(LOG_BASIC, LOG_LVL_MINIMAL, "");
    sLog.Out(LOG_BASIC, LOG_LVL_MINIMAL, ">> Loaded %u auction items", count);
}
```
