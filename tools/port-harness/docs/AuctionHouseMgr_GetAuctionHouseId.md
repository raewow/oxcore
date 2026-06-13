# AuctionHouseMgr::GetAuctionHouseId

**File:** src/game/AuctionHouse/AuctionHouseMgr.cpp

## Summary
Maps a faction template ID to an auction house ID (1–7) via a hardcoded switch table; unrecognized IDs fall back to ObjectMgr faction-template lookup using alliance/horde masks, defaulting to goblin house 7.

## Source
```cpp
uint32 AuctionHouseMgr::GetAuctionHouseId(uint32 factionTemplateId)
{
    uint32 houseId = 1;
    switch (factionTemplateId)
    {
        case   11:
        case   12:
            houseId = 1; // Human
            break;
        case   29:
        case   85:
            houseId = 6; // Orc
            break;
        case   55:
        case   57:
            houseId = 2; // Dwarf
            break;
        case   68:
        case   71:
            houseId = 4; // Undead
            break;
        case   79:
        case   80:
            houseId = 3; // Night Elf
            break;
        case  104:
        case  105:
            houseId = 5; // Tauren
            break;
        case  120:
            houseId = 7; // Booty Bay
            break;
        case  474:
            houseId = 7; // Gadgetzan
            break;
        case  534:
            houseId = 2; // Alliance Generic
            break;
        case  855:
            houseId = 7; // Everlook
            break;
        default:                                    // for unknown case
        {
            FactionTemplateEntry const* u_entry = sObjectMgr.GetFactionTemplateEntry(factionTemplateId);
            if (!u_entry)
                houseId = 7;                        // goblin auction house
            else if (u_entry->ourMask & FACTION_MASK_ALLIANCE)
                houseId = 1;                        // human auction house
            else if (u_entry->ourMask & FACTION_MASK_HORDE)
                houseId = 6;                        // orc auction house
            else
                houseId = 7;                        // goblin auction house
            break;
        }
    }
    return houseId;
}
```
