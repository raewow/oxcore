# Game Object Effects Documentation

## File: `object.rs`

## Overview

Handles game object interaction effects including doors, chests, and object spawning.

## Effects (6 total)

### 33 - SPELL_EFFECT_OPEN_LOCK
**Function**: `effect_open_lock()`

Opens a locked door or chest.

**Parameters**:
- Target: Locked game object

**Usage**:
- Pick Lock (Rogue)
- Opening chests
- Dungeon doors

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2060-2174`):
```cpp
void Spell::EffectOpenLock(SpellEffectIndex effIdx)
{
    Player* player = m_caster->ToPlayer();
    if (!player)
        return;

    uint32 lockId = 0;
    ObjectGuid guid;

    // Get lockId
    if (gameObjTarget)
    {
        GameObjectInfo const* goInfo = gameObjTarget->GetGOInfo();
        if (goInfo->CannotBeUsedUnderImmunity() && m_caster->HasFlag(UNIT_FIELD_FLAGS, UNIT_FLAG_IMMUNE))
            return;

        // Arathi Basin banner opening
        if ((goInfo->type == GAMEOBJECT_TYPE_BUTTON && goInfo->button.noDamageImmune) ||
                (goInfo->type == GAMEOBJECT_TYPE_GOOBER && goInfo->goober.losOK))
        {
            if (BattleGround *bg = player->GetBattleGround())
            {
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
- Validates caster can open lock via `CanOpenLock()`:
  - Checks lockpicking skill vs difficulty
  - Checks for required keys
  - Validates spell can open lock type
- Marks items as unlocked with `ITEM_DYNFLAG_UNLOCKED`
- Sends loot window to player
- Updates gathering skills (one skill-up per object until respawn)
- Used for lockpicking, chests, doors, and quest objects

---

### 50 - SPELL_EFFECT_TRANS_DOOR
**Function**: `effect_trans_door()` - Actually SPELL_EFFECT_TRANS_DOOR uses same function as SPELL_EFFECT_SUMMON_OBJECT_WILD

Transmits/spawns a game object (doors, portals, etc.).

**Parameters**:
- `misc_value`: Game object entry ID
- Target location

**Usage**:
- Door opening
- Portal activation
- Object spawning

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5584-5730`):
```cpp
void Spell::EffectTransmitted(SpellEffectIndex effIdx)
{
    if (!m_casterUnit)
        return;

    uint32 gameObjectId = m_spellInfo->EffectMiscValue[effIdx];

    GameObjectInfo const* goinfo = sObjectMgr.GetGameObjectTemplate(gameObjectId);
    if (!goinfo)
    {
        sLog.Out(LOG_DBERROR, LOG_LVL_MINIMAL, "Gameobject (Entry: %u) not exist and not created at spell (ID: %u) cast", gameObjectId, m_spellInfo->Id);
        return;
    }

    float fx, fy, fz;
    if (m_targets.m_targetMask & TARGET_FLAG_DEST_LOCATION)
    {
        fx = m_targets.m_destX;
        fy = m_targets.m_destY;
        fz = m_targets.m_destZ;
    }
    else if (m_spellInfo->EffectRadiusIndex[effIdx] && m_spellInfo->speed == 0)
    {
        float dis = GetSpellRadius(sSpellRadiusStore.LookupEntry(m_spellInfo->EffectRadiusIndex[effIdx]));
        float x, y, z;
        m_casterUnit->GetPosition(x, y, z);
        fx = x + dis * cos(m_casterUnit->GetOrientation());
        fy = y + dis * sin(m_casterUnit->GetOrientation());
        fz = z;
        m_casterUnit->GetMap()->GetLosHitPosition(x, y, z + 0.5f, fx, fy, fz, -1.5f);
    }
    else
    {
        float min_dis = GetSpellMinRange(sSpellRangeStore.LookupEntry(m_spellInfo->rangeIndex));
        float max_dis = GetSpellMaxRange(sSpellRangeStore.LookupEntry(m_spellInfo->rangeIndex));
        float dis = rand_norm_f() * (max_dis - min_dis) + min_dis;

        float max_angle = (max_dis - min_dis) / (max_dis + m_caster->GetObjectBoundingRadius());
        float angle_offset = max_angle * (rand_norm_f() - 0.5f);

        float x, y, z;
        m_casterUnit->GetPosition(x, y, z);
        fx = x + dis * cos(m_casterUnit->GetOrientation()+ angle_offset);
        fy = y + dis * sin(m_casterUnit->GetOrientation()+ angle_offset);
        fz = z;
        m_casterUnit->GetMap()->GetLosHitPosition(x, y, z + 2.0f, fx, fy, fz, -1.5f);
    }

    // Special handling for fishing nodes
    if (goinfo->type == GAMEOBJECT_TYPE_FISHINGNODE)
    {
        GridMapLiquidData liqData;
        if (!m_caster->GetTerrain()->IsSwimmable(fx, fy, m_caster->GetPositionZ() + 1.0f, 1.5f, &liqData))
            m_caster->GetTerrain()->IsSwimmable(fx, fy, liqData.level, 1.5f, &liqData);

        // Validate fishable water
        if ((abs(liqData.depth_level) < 1) || !(m_caster->GetMap()->isInLineOfSight(x, y, z + 2.0f, fx, fy, liqData.level)))
        {
            SendCastResult(SPELL_FAILED_NOT_FISHABLE);
            SendChannelUpdate(0);
            finish();
            return;
        }
        fz = liqData.level;
    }

    GameObject* pGameObj = new GameObject;
    if (!pGameObj->Create(cMap->GenerateLocalLowGuid(HIGHGUID_GAMEOBJECT), gameObjectId, cMap,
                          fx, fy, fz, m_casterUnit->GetOrientation(), 0.0f, 0.0f, 0.0f, 0.0f, GO_ANIMPROGRESS_DEFAULT, GO_STATE_READY))
    {
        delete pGameObj;
        return;
    }

    int32 duration = m_spellInfo->GetDuration();

    // Special handling for different GO types
    switch (goinfo->type)
    {
        case GAMEOBJECT_TYPE_FISHINGNODE:
            // Fishing bobber setup
            m_casterUnit->SetChannelObjectGuid(pGameObj->GetObjectGuid());
            m_casterUnit->AddGameObject(pGameObj);
            int32 lastSec = PickRandomValue(3, 7, 13, 17);
            duration = duration - lastSec * IN_MILLISECONDS + FISHING_BOBBER_READY_TIME * IN_MILLISECONDS;
            break;
        case GAMEOBJECT_TYPE_SUMMONING_RITUAL:
            // Summoning ritual setup
            break;
        case GAMEOBJECT_TYPE_FISHINGHOLE:
        case GAMEOBJECT_TYPE_CHEST:
            // Lootable objects
            break;
    }
}
```

**Key Behaviors**:
- Spawns game object at calculated position
- Position calculation:
  - Uses destination if specified
  - Otherwise uses radius + orientation
  - Applies LOS checks to find valid position
- Special handling for fishing nodes (water level check)
- Duration from spell duration
- Object types handled specially:
  - Fishing nodes: channeling setup, random bite time
  - Summoning rituals: multi-player channeling
  - Chests/Fishing holes: loot setup
- Linked traps are summoned automatically
- Not owned by caster (wild objects)

---

### 76 - SPELL_EFFECT_SUMMON_OBJECT_WILD
**Function**: `effect_summon_object_wild()`

Spawns a game object at target location (wild/unowned).

**Parameters**:
- `misc_value`: Game object entry ID
- Target location

**Usage**:
- Temporary objects
- Quest objects
- Traps
- Battleground flags

**Implementation Details** (from MaNGOS `SpellEffects.cpp:3532-3620`):
```cpp
void Spell::EffectSummonObjectWild(SpellEffectIndex effIdx)
{
    uint32 gameobjectId = m_spellInfo->EffectMiscValue[effIdx];

    GameObject* pGameObj = new GameObject;

    WorldObject* target = focusObject;
    if (!target)
        target = m_caster;

    float x, y, z, o;
    if (m_targets.m_targetMask & TARGET_FLAG_DEST_LOCATION)
    {
        x = m_targets.m_destX;
        y = m_targets.m_destY;
        z = m_targets.m_destZ;
        o = target->GetOrientation();
    }
    else
    {
        m_caster->GetPosition(x, y, z);
        o = m_caster->GetOrientation();
    }

    Map* map = target->GetMap();

    if (!pGameObj->Create(map->GenerateLocalLowGuid(HIGHGUID_GAMEOBJECT), gameobjectId, map,
                          x, y, z, o, 0.0f, 0.0f, 0.0f, 0.0f, GO_ANIMPROGRESS_DEFAULT, GO_STATE_READY))
    {
        delete pGameObj;
        return;
    }

    int32 duration = m_spellInfo->GetDuration();

    // Sapphirons summoned iceblocks have a duration *just* long enough to dissapear before the ice bomb.
    if (m_spellInfo->Id == 28535)
        duration = 30000;

    pGameObj->SetRespawnTime(duration > 0 ? duration / IN_MILLISECONDS : 0);
    pGameObj->SetSpellId(m_spellInfo->Id);

    // Wild object not have owner and check clickable by players
    map->Add(pGameObj);

    // Special handling for Warsong Gulch dropped flags
    if (pGameObj->GetGoType() == GAMEOBJECT_TYPE_FLAGDROP && m_caster->IsPlayer())
    {
        Player* pl = (Player*)m_caster;
        BattleGround* bg = ((Player*)m_caster)->GetBattleGround();

        switch (pGameObj->GetMapId())
        {
            case MAP_WARSONG_GULCH: //WS
            {
                if (bg && bg->GetTypeID() == BATTLEGROUND_WS && bg->GetStatus() == STATUS_IN_PROGRESS)
                {
                    Team team = ALLIANCE;
                    if (pl->GetTeam() == team)
                        team = HORDE;

                    ((BattleGroundWS*)bg)->SetDroppedFlagGuid(pGameObj->GetObjectGuid(), team);
                }
                break;
            }
        }
    }

    pGameObj->SetWorldMask(m_caster->GetWorldMask());
    pGameObj->SummonLinkedTrapIfAny();

    if (m_caster->IsCreature() && ((Creature*)m_caster)->AI())
        ((Creature*)m_caster)->AI()->JustSummoned(pGameObj);
    else if (m_caster->IsGameObject() && ((GameObject*)m_caster)->AI())
        ((GameObject*)m_caster)->AI()->JustSummoned(pGameObj);
}
```

**Key Behaviors**:
- Spawns game object at destination or caster position
- No owner (wild object)
- Duration from spell (with special case for Sapphiron iceblocks)
- Added to map directly
- Special handling for WSG dropped flags
- Summons linked traps automatically
- Notifies caster AI of summon
- Can be interacted with by anyone
- Used for temporary quest objects, traps, and BG flags

---

### 86 - SPELL_EFFECT_ACTIVATE_OBJECT
**Function**: `effect_activate_object()`

Activates a game object with specific action.

**Parameters**:
- Target: Game object to activate
- `misc_value`: GameObjectActions enum value

**Usage**:
- Lever pulling
- Button pressing
- Object interaction
- Door control

**Implementation Details** (from MaNGOS `SpellEffects.cpp:4743-4840`):
```cpp
void Spell::EffectActivateObject(SpellEffectIndex effIdx)
{
    if (!gameObjTarget)
        return;

    GameObjectActions action = (GameObjectActions)m_spellInfo->EffectMiscValue[effIdx];

    // Can be handled by script.
    if (gameObjTarget->AI() && gameObjTarget->AI()->OnActivateBySpell(m_caster, m_spellInfo->Id, (uint32)action))
        return;

    switch (action)
    {
        case GameObjectActions::None:
            sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "Spell::EffectActivateObject: Incorrect GameObjectActions::None action in spell %u", m_spellInfo->Id);
            break;
        case GameObjectActions::AnimateCustom0:
            gameObjTarget->SendGameObjectCustomAnim(0);
            break;
        case GameObjectActions::AnimateCustom1:
            gameObjTarget->SendGameObjectCustomAnim(1);
            break;
        case GameObjectActions::AnimateCustom2:
            gameObjTarget->SendGameObjectCustomAnim(2);
            break;
        case GameObjectActions::AnimateCustom3:
            gameObjTarget->SendGameObjectCustomAnim(3);
            break;
        case GameObjectActions::Disturb:
            if (m_casterUnit)
                gameObjTarget->Use(m_casterUnit);
            break;
        case GameObjectActions::Unlock:
            gameObjTarget->RemoveFlag(GAMEOBJECT_FLAGS, GO_FLAG_LOCKED);
            break;
        case GameObjectActions::Lock:
            gameObjTarget->SetFlag(GAMEOBJECT_FLAGS, GO_FLAG_LOCKED);
            break;
        case GameObjectActions::Open:
            if (m_casterUnit)
                gameObjTarget->Use(m_casterUnit);
            break;
        case GameObjectActions::OpenAndUnlock:
            gameObjTarget->UseDoorOrButton(0, false);
            gameObjTarget->RemoveFlag(GAMEOBJECT_FLAGS, GO_FLAG_LOCKED);
            break;
        case GameObjectActions::Close:
            gameObjTarget->ResetDoorOrButton();
            break;
        case GameObjectActions::Toggle:
            if (gameObjTarget->GetGoState() == GO_STATE_READY)
                gameObjTarget->UseDoorOrButton(0, false);
            else
                gameObjTarget->ResetDoorOrButton();
            break;
        case GameObjectActions::Destroy:
            gameObjTarget->SetLootState(GO_JUST_DEACTIVATED);
            break;
        case GameObjectActions::UseArtKit0:
        case GameObjectActions::UseArtKit1:
        case GameObjectActions::UseArtKit2:
        case GameObjectActions::UseArtKit3:
            gameObjTarget->SetGoArtKit(uint32(action) - uint32(GameObjectActions::UseArtKit0));
            break;
    }
}
```

**Key Behaviors**:
- Action type from `EffectMiscValue[effIdx]` (GameObjectActions enum)
- First checks if GO AI wants to handle activation
- Supports various actions:
  - **AnimateCustom0-3**: Play custom animations
  - **Disturb**: Trigger object use
  - **Unlock/Lock**: Toggle locked state
  - **Open**: Open door/button
  - **OpenAndUnlock**: Open and unlock simultaneously
  - **Close**: Reset door/button to closed
  - **Toggle**: Toggle between open/closed
  - **Destroy**: Deactivate object
  - **UseArtKit0-3**: Change visual appearance
- Used extensively in dungeons and raids
- Can trigger complex scripted sequences

---

### 104-107 - SPELL_EFFECT_SUMMON_OBJECT_SLOT1-4
**Function**: `effect_summon_object_slot()`

Spawns an object in a specific slot (despawns previous).

**Parameters**:
- `misc_value`: Game object entry ID
- `slot`: 1-4

**Usage**:
- Shaman totem objects (visual)
- Persistent objects
- Class-specific object slots

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5089-5165`):
```cpp
void Spell::EffectSummonObject(SpellEffectIndex effIdx)
{
    if (!m_casterUnit)
        return;

    uint32 goId = m_spellInfo->EffectMiscValue[effIdx];

    uint8 slot;
    switch (m_spellInfo->Effect[effIdx])
    {
        case SPELL_EFFECT_SUMMON_OBJECT_SLOT1:
            slot = 0;
            break;
        case SPELL_EFFECT_SUMMON_OBJECT_SLOT2:
            slot = 1;
            break;
        case SPELL_EFFECT_SUMMON_OBJECT_SLOT3:
            slot = 2;
            break;
        case SPELL_EFFECT_SUMMON_OBJECT_SLOT4:
            slot = 3;
            break;
        default:
            return;
    }

    // Despawn existing object in slot
    if (ObjectGuid guid = m_casterUnit->m_ObjectSlotGuid[slot])
    {
        if (GameObject* obj = m_casterUnit ? m_casterUnit->GetMap()->GetGameObject(guid) : nullptr)
            obj->SetLootState(GO_JUST_DEACTIVATED);
        m_casterUnit->m_ObjectSlotGuid[slot].Clear();
    }

    GameObject* pGameObj = new GameObject;

    float x, y, z;
    // If dest location if present
    if (m_targets.m_targetMask & TARGET_FLAG_DEST_LOCATION)
    {
        x = m_targets.m_destX;
        y = m_targets.m_destY;
        z = m_targets.m_destZ;
    }
    // Summon in random point all other units if location present
    else
        m_casterUnit->GetClosePoint(x, y, z, DEFAULT_WORLD_OBJECT_SIZE);

    Map* map = m_casterUnit->GetMap();
    if (!pGameObj->Create(map->GenerateLocalLowGuid(HIGHGUID_GAMEOBJECT), goId, map,
                          x, y, z, m_casterUnit->GetOrientation(), 0.0f, 0.0f, 0.0f, 0.0f, GO_ANIMPROGRESS_DEFAULT, GO_STATE_READY))
    {
        delete pGameObj;
        return;
    }

    pGameObj->SetUInt32Value(GAMEOBJECT_LEVEL, m_casterUnit->GetLevel());
    int32 duration = m_spellInfo->GetDuration();
    pGameObj->SetRespawnTime(duration > 0 ? duration / IN_MILLISECONDS : 0);
    pGameObj->SetSpellId(m_spellInfo->Id);
    m_casterUnit->AddGameObject(pGameObj);
    map->Add(pGameObj);

    m_casterUnit->m_ObjectSlotGuid[slot] = pGameObj->GetObjectGuid();

    // Summon linked trap if any
    pGameObj->SummonLinkedTrapIfAny();

    // Notify Summoner
    if (m_casterUnit->IsCreature() && ((Creature*)m_casterUnit)->AI())
        ((Creature*)m_casterUnit)->AI()->JustSummoned(pGameObj);

    if (m_spellScript)
        m_spellScript->OnSummon(this, pGameObj);
}
```

**Key Behaviors**:
- Slot determined by effect ID (SLOT1=0, SLOT2=1, etc.)
- Despawns existing object in slot before spawning new
- Stores object GUID in `m_ObjectSlotGuid[slot]`
- Spawns at destination or near caster
- Sets GO level to match caster
- Duration from spell
- Summons linked traps automatically
- Notifies caster AI
- Used for persistent class objects (Hunter traps, etc.)

---

### 130 - SPELL_EFFECT_DESPAWN_OBJECT
**Function**: `effect_despawn_object()`

Despawns a game object immediately.

**Parameters**:
- Target: Game object to despawn

**Usage**:
- Removing temporary objects
- Quest cleanup
- Scripted events

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5847-5853`):
```cpp
void Spell::EffectDespawnObject(SpellEffectIndex effIdx)
{
    sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "SPELL_EFFECT_DESPAWN_OBJECT");
    if (!gameObjTarget)
        return;
    gameObjTarget->AddObjectToRemoveList();
}
```

**Key Behaviors**:
- Immediately marks object for removal
- Uses `AddObjectToRemoveList()` for safe deletion
- Object removed at next map update
- Does not trigger respawn timer
- Used for:
  - Quest object cleanup
  - Scripted event cleanup
  - Temporary object removal
  - Puzzle/encounter resets

## Dependencies

Required systems:
- `GameObjectSystem` - For object management
- `LockSystem` - For lockpicking

## References

- MaNGOS: `SpellEffects.cpp` - Object effects
- MaNGOS: `GameObject.cpp` - Object implementation
- MaNGOS: `LockMgr.cpp` - Lock mechanics
