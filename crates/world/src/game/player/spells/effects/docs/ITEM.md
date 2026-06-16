# Item and Enchanting Effects Documentation

## File: `item.rs`

## Overview

Handles item creation, enchanting, disenchanting, and item manipulation effects.

## Effects (10 total)

### 24 - SPELL_EFFECT_CREATE_ITEM
**Function**: `effect_create_item()`

Creates items in the caster's inventory.

**Parameters**:
- `misc_value`: Item entry ID
- `base_value`: Item count

**Usage**:
- Mage: Conjure Food/Water
- Warlock: Create Soulstone/Healthstone
- Various profession items

**Implementation Details** (from MaNGOS `SpellEffects.cpp:1845-1938`):
```cpp
void Spell::DoCreateItem(SpellEffectIndex effIdx, uint32 itemtype)
{
    Player* player = ToPlayer(unitTarget);
    if (!player)
        return;

    uint32 newItemId = itemtype;
    ItemPrototype const* pProto = sObjectMgr.GetItemPrototype(newItemId);
    if (!pProto)
    {
        player->SendEquipError(EQUIP_ERR_ITEM_NOT_FOUND, nullptr, nullptr);
        return;
    }

    // bg reward have some special in code work
    uint32 bgType = 0;
    switch (m_spellInfo->Id)
    {
        case SPELL_AV_MARK_WINNER:
        case SPELL_AV_MARK_LOSER:
            bgType = BATTLEGROUND_AV;
            break;
        case SPELL_WS_ALLY_WINNER:
        case SPELL_WS_HORDE_WINNER:
        case SPELL_WS_OLD_LOSER:
        case SPELL_WS_MARK_WINNER:
        case SPELL_WS_MARK_LOSER:
            bgType = BATTLEGROUND_WS;
            break;
        case SPELL_AB_OLD_WINNER:
        case SPELL_AB_MARK_WINNER:
        case SPELL_AB_MARK_LOSER:
            bgType = BATTLEGROUND_AB;
            break;
        default:
            break;
    }

    uint32 numToAdd = damage;

    if (numToAdd < 1)
        numToAdd = 1;
    if (numToAdd > pProto->Stackable)
        numToAdd = pProto->Stackable;

    // can the player store the new item?
    ItemPosCountVec dest;
    uint32 noSpace = 0;
    InventoryResult msg = player->CanStoreNewItem(NULL_BAG, NULL_SLOT, dest, newItemId, numToAdd, &noSpace);
    if (msg != EQUIP_ERR_OK)
    {
        // ... error handling
    }

    // create the new item
    Item* pItem = player->StoreNewItem(dest, newItemId, true, Item::GenerateItemRandomPropertyId(newItemId));
    if (!pItem)
    {
        player->SendEquipError(EQUIP_ERR_ITEM_NOT_FOUND, nullptr, nullptr);
        return;
    }

    // set random property if needed
    if (pItem->GetProto()->RandomProperty || pItem->GetProto()->RandomSuffix)
    {
        int32 randomPropertyId = Item::GenerateItemRandomPropertyId(newItemId);
        if (randomPropertyId)
        {
            pItem->SetItemRandomProperties(randomPropertyId);
        }
    }

    // send notification
    player->SendNewItem(pItem, numToAdd, true, bgType != 0);

    // skillups
    if (m_CastItem)
        player->UpdateCraftSkill(m_spellInfo->Id);
}
```

**Key Behaviors**:
- Only works for player targets
- Validates item prototype exists
- Special handling for battleground reward items (marks)
- Item count from `damage` field, capped at stack limit
- Checks inventory space before creation
- Generates random properties for items that have them
- Sends item creation notification to player
- Updates craft skill if cast via item (professions)
- Item appears in first available inventory slot

---

### 34 - SPELL_EFFECT_SUMMON_CHANGE_ITEM
**Function**: `effect_summon_change_item()`

Transforms one item into another.

**Parameters**:
- `misc_value`: Target item entry ID
- `misc_value_b`: Source item entry ID

**Usage**:
- Item transmutation
- Quest item transformations

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2176-2261`):
```cpp
void Spell::EffectSummonChangeItem(SpellEffectIndex effIdx)
{
    Player* player = m_caster->ToPlayer();
    if (!player)
        return;

    // applied only to using item
    if (!m_CastItem)
        return;

    // ... only to item in own inventory/bank/equip_slot
    if (m_CastItem->GetOwnerGuid() != player->GetObjectGuid())
        return;

    uint32 newitemid = m_spellInfo->EffectItemType[effIdx];
    if (!newitemid)
        return;

    uint16 pos = m_CastItem->GetPos();

    Item *pNewItem = Item::CreateItem(newitemid, 1, player->GetObjectGuid());
    if (!pNewItem)
        return;

    for (uint8 j = PERM_ENCHANTMENT_SLOT; j <= TEMP_ENCHANTMENT_SLOT; ++j)
    {
        if (m_CastItem->GetEnchantmentId(EnchantmentSlot(j)))
            pNewItem->SetEnchantment(EnchantmentSlot(j), m_CastItem->GetEnchantmentId(EnchantmentSlot(j)), m_CastItem->GetEnchantmentDuration(EnchantmentSlot(j)), m_CastItem->GetEnchantmentCharges(EnchantmentSlot(j)));
    }

    if (m_CastItem->GetUInt32Value(ITEM_FIELD_DURABILITY) < m_CastItem->GetUInt32Value(ITEM_FIELD_MAXDURABILITY))
    {
        double loosePercent = 1 - m_CastItem->GetUInt32Value(ITEM_FIELD_DURABILITY) / double(m_CastItem->GetUInt32Value(ITEM_FIELD_MAXDURABILITY));
        player->DurabilityLoss(pNewItem, loosePercent);
    }

    if (player->IsInventoryPos(pos))
    {
        ItemPosCountVec dest;
        uint8 msg = player->CanStoreItem(m_CastItem->GetBagSlot(), m_CastItem->GetSlot(), dest, pNewItem, true);
        if (msg == EQUIP_ERR_OK)
        {
            player->DestroyItem(m_CastItem->GetBagSlot(), m_CastItem->GetSlot(), true);

            // prevent crash at access and unexpected charges counting with item update queue corrupt
            ClearCastItem();

            player->StoreItem(dest, pNewItem, true);
            return;
        }
    }
    else if (player->IsBankPos(pos))
    {
        ItemPosCountVec dest;
        uint8 msg = player->CanBankItem(m_CastItem->GetBagSlot(), m_CastItem->GetSlot(), dest, pNewItem, true);
        if (msg == EQUIP_ERR_OK)
        {
            player->DestroyItem(m_CastItem->GetBagSlot(), m_CastItem->GetSlot(), true);

            // prevent crash at access and unexpected charges counting with item update queue corrupt
            ClearCastItem();

            player->BankItem(dest, pNewItem, true);
            return;
        }
    }
    else if (player->IsEquipmentPos(pos))
    {
        uint16 dest;
        uint8 msg = player->CanEquipItem(m_CastItem->GetSlot(), dest, pNewItem, true, false);
        if (msg == EQUIP_ERR_OK)
        {
            player->DestroyItem(m_CastItem->GetBagSlot(), m_CastItem->GetSlot(), true);

            // prevent crash at access and unexpected charges counting with item update queue corrupt
            ClearCastItem();

            player->EquipItem(dest, pNewItem, true);
            player->AutoUnequipOffhandIfNeed();
            return;
        }
    }

    // fail
    delete pNewItem;
}
```

**Key Behaviors**:
- Only works for player casters
- Must be cast via an item (m_CastItem)
- Item must be owned by caster
- Target item ID from `EffectItemType[effIdx]`
- Preserves enchantments (both permanent and temporary)
- Transfers durability loss percentage to new item
- Handles inventory, bank, and equipment slots
- Destroys old item and creates new one in same position
- Clears cast item reference to prevent crashes
- Auto-unequips offhand if needed for 2H weapons

---

### 53 - SPELL_EFFECT_ENCHANT_ITEM
**Function**: `effect_enchant_item_perm()`

Applies a permanent enchantment to an item.

**Parameters**:
- `misc_value`: Enchantment ID

**Usage**:
- Enchanting profession
- Permanent stat bonuses

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2967-3011`):
```cpp
void Spell::EffectEnchantItemPerm(SpellEffectIndex effIdx)
{
    if (!itemTarget)
        return;

    Player* pCaster = m_caster->ToPlayer();
    if (!pCaster)
        return;

    // not grow at item use at item case
    pCaster->UpdateCraftSkill(m_spellInfo->Id);

    uint32 enchantId = m_spellInfo->EffectMiscValue[effIdx];
    if (!enchantId)
        return;

    SpellItemEnchantmentEntry const* pEnchant = sSpellItemEnchantmentStore.LookupEntry(enchantId);
    if (!pEnchant)
        return;

    // item can be in trade slot and have owner diff. from caster
    Player* pItemOwner = itemTarget->GetOwner();
    if (!pItemOwner)
        return;

    if (!sWorld.getConfig(CONFIG_BOOL_GM_ALLOW_TRADES) && pCaster->GetSession()->GetSecurity() > SEC_PLAYER)
        return;

    if (pItemOwner != pCaster && pCaster->GetSession()->GetSecurity() > SEC_PLAYER && sWorld.getConfig(CONFIG_BOOL_GM_LOG_TRADE))
    {
        sLog.Player(pCaster->GetSession(), LOG_GM, LOG_LVL_BASIC,
            "GM %s (Account: %u) enchanting(perm): %s (Entry: %d) for player: %s (Account: %u)",
            pCaster->GetName(), pCaster->GetSession()->GetAccountId(),
            itemTarget->GetProto()->Name1, itemTarget->GetEntry(),
            pItemOwner->GetName(), pItemOwner->GetSession()->GetAccountId());
    }

    // remove old enchanting before applying new if equipped
    pItemOwner->ApplyEnchantment(itemTarget, PERM_ENCHANTMENT_SLOT, false);

    itemTarget->SetEnchantment(PERM_ENCHANTMENT_SLOT, enchantId, 0, 0, m_caster->GetObjectGuid());

    // add new enchanting if equipped
    pItemOwner->ApplyEnchantment(itemTarget, PERM_ENCHANTMENT_SLOT, true);
}
```

**Key Behaviors**:
- Only works for player casters
- Updates craft skill (for profession leveling)
- Enchant ID from `EffectMiscValue[effIdx]`
- Validates enchantment exists in database
- Supports trade window enchanting (different owner)
- GM trade restrictions and logging
- Removes old permanent enchant before applying new
- Applies to `PERM_ENCHANTMENT_SLOT`
- Re-applies enchantment stats if item is equipped
- Permanent enchantments persist through death and zoning

---

### 54 - SPELL_EFFECT_ENCHANT_ITEM_TEMPORARY
**Function**: `effect_enchant_item_tmp()`

Applies a temporary enchantment to an item.

**Parameters**:
- `misc_value`: Enchantment ID
- `base_value`: Duration in seconds

**Usage**:
- Sharpening stones
- Weightstones
- Poisons (rogue)
- Oils (caster weapons)

**Implementation Details** (from MaNGOS `SpellEffects.cpp:3013-3062`):
```cpp
void Spell::EffectEnchantItemTmp(SpellEffectIndex effIdx)
{
    Player* pCaster = m_caster->ToPlayer();
    if (!pCaster)
        return;

    if (!itemTarget)
        return;

    uint32 enchantId  = m_spellInfo->EffectMiscValue[effIdx];
    uint32 charges    = sSpellMgr.GetSpellEnchantCharges(m_spellInfo->Id);

    if (!enchantId)
    {
        sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "Spell %u Effect %u (SPELL_EFFECT_ENCHANT_ITEM_TEMPORARY) have 0 as enchanting id", m_spellInfo->Id, effIdx);
        return;
    }

    SpellItemEnchantmentEntry const* pEnchant = sSpellItemEnchantmentStore.LookupEntry(enchantId);
    if (!pEnchant)
    {
        sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "Spell %u Effect %u (SPELL_EFFECT_ENCHANT_ITEM_TEMPORARY) have nonexistent enchanting id %u ", m_spellInfo->Id, effIdx, enchantId);
        return;
    }

    // item can be in trade slot and have owner diff. from caster
    Player* pItemOwner = itemTarget->GetOwner();
    if (!pItemOwner)
        return;

    if (!sWorld.getConfig(CONFIG_BOOL_GM_ALLOW_TRADES) && pCaster->GetSession()->GetSecurity() > SEC_PLAYER)
        return;

    if (pItemOwner != pCaster && pCaster->GetSession()->GetSecurity() > SEC_PLAYER && sWorld.getConfig(CONFIG_BOOL_GM_LOG_TRADE))
    {
        sLog.Player(pCaster->GetSession(), LOG_GM, LOG_LVL_BASIC,
            "GM %s (Account: %u) enchanting(temp): %s (Entry: %d) for player: %s (Account: %u)",
            pCaster->GetName(), pCaster->GetSession()->GetAccountId(),
            itemTarget->GetProto()->Name1, itemTarget->GetEntry(),
            pItemOwner->GetName(), pItemOwner->GetSession()->GetAccountId());
    }

    // remove old enchant before applying new
    pItemOwner->ApplyEnchantment(itemTarget, TEMP_ENCHANTMENT_SLOT, false);

    itemTarget->SetEnchantment(TEMP_ENCHANTMENT_SLOT, enchantId, damage * 1000, charges, m_caster->GetObjectGuid());

    // add new enchanting if equipped
    pItemOwner->ApplyEnchantment(itemTarget, TEMP_ENCHANTMENT_SLOT, true);
}
```

**Key Behaviors**:
- Only works for player casters
- Enchant ID from `EffectMiscValue[effIdx]`
- Charges from `sSpellMgr.GetSpellEnchantCharges()`
- Duration from `damage` field (converted to milliseconds)
- Supports trade window enchanting
- GM trade restrictions and logging
- Removes old temporary enchant before applying new
- Applies to `TEMP_ENCHANTMENT_SLOT` (separate from permanent)
- Can stack with permanent enchantments
- Visual glow effect based on enchantment type
- Charges are consumed on use (e.g., poison applications)

---

### 59 - SPELL_EFFECT_OPEN_LOCK_ITEM
**Function**: `effect_open_lock_item()`

Opens a locked item using a key item.

**Parameters**:
- Target: Locked item/container
- Cast item: Key

**Usage**:
- Opening locked chests with keys
- Quest keys

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2060-2174`):
```cpp
void Spell::EffectOpenLock(SpellEffectIndex effIdx)
{
    Player* player = m_caster->ToPlayer();
    if (!player)
    {
        sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "WORLD: Open Lock - No Player Caster!");
        return;
    }

    uint32 lockId = 0;
    ObjectGuid guid;

    // Get lockId
    if (gameObjTarget)
    {
        GameObjectInfo const* goInfo = gameObjTarget->GetGOInfo();

        if (goInfo->CannotBeUsedUnderImmunity() && m_caster->HasFlag(UNIT_FIELD_FLAGS, UNIT_FLAG_IMMUNE))
            return;

        // Arathi Basin banner opening !
        if ((goInfo->type == GAMEOBJECT_TYPE_BUTTON && goInfo->button.noDamageImmune) ||
                (goInfo->type == GAMEOBJECT_TYPE_GOOBER && goInfo->goober.losOK))
        {
            //CanUseBattleGroundObject() already called in CheckCast()
            // in battleground check
            if (BattleGround *bg = player->GetBattleGround())
            {
                // check if it's correct bg
                if (bg->GetTypeID() == BATTLEGROUND_AB || bg->GetTypeID() == BATTLEGROUND_AV)
                    bg->EventPlayerClickedOnFlag(player, gameObjTarget);
                return;
            }
        }
        lockId = goInfo->GetLockId();
        guid = gameObjTarget->GetObjectGuid();
    }
    else if (itemTarget)
    {
        lockId = itemTarget->GetProto()->LockID;
        guid = itemTarget->GetObjectGuid();
    }

    SkillType skillId = SKILL_NONE;
    int32 reqSkillValue = 0;
    int32 skillValue;

    SpellCastResult res = CanOpenLock(effIdx, lockId, skillId, reqSkillValue, skillValue);
    if (res != SPELL_CAST_OK)
    {
        SendCastResult(res);
        return;
    }

    // mark item as unlocked
    if (itemTarget)
    {
        itemTarget->SetFlag(ITEM_FIELD_FLAGS, ITEM_DYNFLAG_UNLOCKED);
        itemTarget->SetState(ITEM_CHANGED);
    }

    SendLoot(guid, LOOT_SKINNING, LockType(m_spellInfo->EffectMiscValue[effIdx]));

    // update skill if really known
    if (!m_CastItem && skillId != SKILL_NONE)
    {
        if (uint32 pureSkillValue = player->GetSkillValuePure(skillId))
        {
            if (gameObjTarget)
            {
                // Allow one skill-up until respawned
                if (!gameObjTarget->IsInSkillupList(player) &&
                        player->UpdateGatherSkill(skillId, pureSkillValue, reqSkillValue))
                    gameObjTarget->AddToSkillupList(player);
            }
            else if (itemTarget)
            {
                // Do one skill-up
                player->UpdateGatherSkill(skillId, pureSkillValue, reqSkillValue);
            }
        }
    }
}
```

**Key Behaviors**:
- Only works for player casters
- Gets lock ID from gameobject or item target
- Special handling for battleground objects (AB/AV flags)
- Validates caster can open lock via `CanOpenLock()`
- Checks skill requirements (lockpicking, etc.)
- Marks items as unlocked with `ITEM_DYNFLAG_UNLOCKED`
- Sends loot window to player
- Updates gathering skills (one skill-up per object until respawn)
- Supports lockpicking skill progression

---

### 92 - SPELL_EFFECT_ENCHANT_HELD_ITEM
**Function**: `effect_enchant_held_item()`

Enchants the item in the main hand.

**Parameters**:
- `misc_value`: Enchantment ID

**Usage**:
- Weapon buffs that auto-target main hand
- Shaman weapon enchants

**Implementation Details** (from MaNGOS `SpellEffects.cpp:4947-4961`):
```cpp
void Spell::EffectEnchantHeldItem(SpellEffectIndex effIdx)
{
    // this is only item spell effect applied to main-hand weapon of target player (players in area)
    Player* itemOwner = ToPlayer(unitTarget);
    if (!itemOwner)
        return;

    Item* item = itemOwner->GetItemByPos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND);

    if (!item)
        return;

    // must be equipped
    if (!item ->IsEquipped())
        return;

    // ... rest of enchant logic similar to EffectEnchantItemTmp
}
```

**Key Behaviors**:
- Automatically targets main hand weapon of unit target
- Only works if weapon is equipped
- Uses same enchantment logic as temporary enchants
- No target selection needed (auto-targets main hand)
- Used for area-effect weapon buffs
- Shaman weapon enchants (Rockbiter, Flametongue, etc.)

---

### 99 - SPELL_EFFECT_DISENCHANT
**Function**: `effect_disenchant()`

Disenchants an item into enchanting materials.

**Parameters**:
- Target: Item to disenchant

**Usage**:
- Enchanting profession
- Breaking down unwanted items

**Implementation Details** (from MaNGOS `SpellEffects.cpp:4997-5012`):
```cpp
void Spell::EffectDisEnchant(SpellEffectIndex /*effIdx*/)
{
    if (m_caster->GetTypeId() != TYPEID_PLAYER)
        return;

    if (!itemTarget || !itemTarget->GetProto()->DisenchantID)
        return;

    Player* pCaster = static_cast<Player*>(m_caster);

    itemTarget->SetBinding(true);
    pCaster->UpdateCraftSkill(m_spellInfo->Id);
    pCaster->SendLoot(itemTarget->GetObjectGuid(), LOOT_DISENCHANTING);

    // item will be removed at disenchanting end
}
```

**Key Behaviors**:
- Only works for player casters
- Item must have a valid `DisenchantID` in its prototype
- Binds item to caster (soulbound)
- Updates enchanting craft skill
- Sends disenchant loot window to player
- Item is destroyed when looting completes
- Loot table determined by `DisenchantID` based on item level:
  - Strange Dust / Lesser Magic Essence (level 1-20)
  - Soul Dust / Greater Magic Essence (level 21-30)
  - Vision Dust / Lesser Mystic Essence (level 31-40)
  - Dream Dust / Greater Mystic Essence (level 41-50)
  - Illusion Dust / Lesser Eternal Essence (level 51-60)
  - Large Brilliant Shards (rare/epic items)

---

### 101 - SPELL_EFFECT_FEED_PET
**Function**: `effect_feed_pet()`

Feeds the caster's pet with a food item.

**Parameters**:
- Target: Food item
- Pet must be active

**Usage**:
- Hunter pet feeding
- Restores happiness

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5029-5070`):
```cpp
void Spell::EffectFeedPet(SpellEffectIndex effIdx)
{
    Player* pPlayer = m_caster->ToPlayer();
    if (!pPlayer)
        return;

    Item* foodItem = itemTarget;
    if (!foodItem)
        return;

    Pet* pet = pPlayer->GetPet();
    if (!pet)
        return;

    if (!pet->IsAlive())
        return;

    if (!m_spellInfo->IsTargetInRange(pPlayer, pet))
    {
        SendCastResult(SPELL_FAILED_OUT_OF_RANGE);
        return;
    }

    if (!pet->IsWithinLOSInMap(pPlayer))
    {
        SendCastResult(SPELL_FAILED_LINE_OF_SIGHT);
        return;
    }

    int32 benefit = pet->GetCurrentFoodBenefitLevel(foodItem->GetProto()->ItemLevel);
    if (benefit <= 0)
        return;

    ExecuteLogInfo info;
    info.feedPet.itemEntry = foodItem->GetProto()->ItemId;

    uint32 count = 1;
    pPlayer->DestroyItemCount(foodItem, count, true);
    pPlayer->CastCustomSpell(pPlayer, m_spellInfo->EffectTriggerSpell[effIdx], benefit, {}, {}, true);

    AddExecuteLogInfo(effIdx, info);
}
```

**Key Behaviors**:
- Only works for player casters with an active pet
- Pet must be alive
- Validates range and line of sight to pet
- Food benefit calculated based on item level vs pet level
- Returns 0 if pet doesn't like the food type
- Consumes exactly 1 food item
- Triggers a follow-up spell (from `EffectTriggerSpell`) with benefit as value
- Benefit determines happiness restoration amount
- Used for Hunter pet feeding system

---

### 111 - SPELL_EFFECT_DURABILITY_DAMAGE
**Function**: `effect_durability_damage()`

Damages an item's durability.

**Parameters**:
- `base_value`: Damage amount
- `misc_value`: Equipment slot

**Usage**:
- Sunder Armor (damages target's armor)
- Various item-damaging abilities

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5512-5546`):
```cpp
void Spell::EffectDurabilityDamage(SpellEffectIndex effIdx)
{
    if (!unitTarget || unitTarget->GetTypeId() != TYPEID_PLAYER)
        return;

    int32 slot = m_spellInfo->EffectMiscValue[effIdx];

    // FIXME: some spells effects have value -1/-2
    // Possibly its mean -1 all player equipped items and -2 all items
    if (slot < 0)
    {
        ((Player*)unitTarget)->DurabilityPointsLossAll(damage, (slot < -1));

        ExecuteLogInfo info(unitTarget->GetObjectGuid());
        info.durabilityDamage.itemEntry = -1;
        info.durabilityDamage.unk = -1;
        AddExecuteLogInfo(effIdx, info);

        return;
    }

    // invalid slot value
    if (slot >= INVENTORY_SLOT_BAG_END)
        return;

    if (Item* item = ((Player*)unitTarget)->GetItemByPos(INVENTORY_SLOT_BAG_0, slot))
    {
        ((Player*)unitTarget)->DurabilityPointsLoss(item, damage);

        ExecuteLogInfo info(unitTarget->GetObjectGuid());
        info.durabilityDamage.itemEntry = item->GetProto()->ItemId;
        info.durabilityDamage.unk = -1;
        AddExecuteLogInfo(effIdx, info);
    }
}
```

**Key Behaviors**:
- Only works on player targets
- Slot values:
  - `-1`: All equipped items
  - `-2`: All items (including bags)
  - `0-18`: Specific equipment slot
- Uses `DurabilityPointsLossAll()` for slot < 0
- Uses `DurabilityPointsLoss()` for specific slots
- Damage amount is flat durability points
- Items break when durability reaches 0
- Logs item entry in execute log
- Used by abilities that damage equipment

---

### 115 - SPELL_EFFECT_DURABILITY_DAMAGE_PCT
**Function**: `effect_durability_damage_pct()`

Damages all equipped items' durability by percentage.

**Parameters**:
- `base_value`: Damage percentage

**Usage**:
- Death durability loss
- Special boss abilities

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5548-5582`):
```cpp
void Spell::EffectDurabilityDamagePCT(SpellEffectIndex effIdx)
{
    if (!unitTarget || unitTarget->GetTypeId() != TYPEID_PLAYER)
        return;

    int32 slot = m_spellInfo->EffectMiscValue[effIdx];

    // FIXME: some spells effects have value -1/-2
    // Possibly its mean -1 all player equipped items and -2 all items
    if (slot < 0)
    {
        ((Player*)unitTarget)->DurabilityLossAll(damage / 100.0f, (slot < -1));
        return;
    }

    // invalid slot value
    if (slot >= INVENTORY_SLOT_BAG_END)
        return;

    if (Item* item = ((Player*)unitTarget)->GetItemByPos(INVENTORY_SLOT_BAG_0, slot))
        ((Player*)unitTarget)->DurabilityLoss(item, damage / 100.0f);
}
```

**Key Behaviors**:
- Only works on player targets
- Slot values:
  - `-1`: All equipped items
  - `-2`: All items (including bags)
  - `0-18`: Specific equipment slot
- Percentage calculated as `damage / 100.0f` (e.g., damage=100 means 100%)
- Uses `DurabilityLossAll()` for slot < 0 (percentage-based)
- Uses `DurabilityLoss()` for specific slots
- Applied as percentage of max durability
- Items break when durability reaches 0
- Used for death durability loss (typically 10%)
- Also used by some boss mechanics

## Dependencies

Required systems:
- `InventorySystem` - For item management
- `ItemSystem` - For enchantments and durability
- `LootSystem` - For disenchant loot generation

## References

- MaNGOS: `SpellEffects.cpp` - `EffectCreateItem()`, `EffectEnchantItemPerm()`, etc.
- MaNGOS: `Item.cpp` - Enchantment and durability handling
