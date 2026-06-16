# PvP Effects Documentation

## File: `pvp.rs`

## Overview

Handles PvP-related effects including duels, sanctuary, and player corpse interaction.

## Effects (3 total)

### 83 - SPELL_EFFECT_DUEL
**Function**: `effect_duel()`

Initiates a duel with the target.

**Parameters**: None

**Usage**:
- Duel spell (right-click portrait or /duel)

**Implementation Details** (from MaNGOS `SpellEffects.cpp:4588-4699`):
```cpp
void Spell::EffectDuel(SpellEffectIndex effIdx)
{
    if (!m_casterUnit || !unitTarget || !m_casterUnit->IsPlayer() || !unitTarget->IsPlayer())
        return;

    Player* caster = (Player*)m_casterUnit;
    Player* target = (Player*)unitTarget;

    // if the caster is already in a duel or has issued a challenge
    if (caster->m_duel && caster->m_duel->opponent != target)
    {
        if (caster->m_duel->startTime)
            caster->DuelComplete(DUEL_WON);
        else
            caster->DuelComplete(DUEL_INTERRUPTED);

       delete caster->m_duel;
       delete target->m_duel;
       caster->m_duel = target->m_duel = nullptr;
    }

    // if the caster attempts to duel somebody they're already in a duel with
    if (caster->m_duel && caster->m_duel->opponent == target && caster->m_duel->startTime)
    {
        SendCastResult(SPELL_FAILED_TARGET_ENEMY);
        return;
    }

    // if the target already has a pending duel/is dueling, reject the request
    if (target->m_duel)
    {
        SendCastResult(SPELL_FAILED_TARGET_DUELING);
        return;
    }

    // caster or target already have requested duel
    if (caster->m_duel || !target->GetSocial() || target->GetSocial()->HasIgnore(caster->GetObjectGuid()) || target->FindMap() != caster->FindMap())
        return;

    // Players can only fight a duel with each other outside (=not inside dungeons and not in capital cities)
    const auto *casterAreaEntry = AreaEntry::GetById(caster->GetAreaId());
    if (casterAreaEntry && !(casterAreaEntry->Flags & AREA_FLAG_DUEL))
    {
        SendCastResult(SPELL_FAILED_NO_DUELING);            // Dueling isn't allowed here
        return;
    }

    const auto *targetAreaEntry = AreaEntry::GetById(target->GetAreaId());
    if (targetAreaEntry && !(targetAreaEntry->Flags & AREA_FLAG_DUEL))
    {
        SendCastResult(SPELL_FAILED_NO_DUELING);            // Dueling isn't allowed here
        return;
    }

    //CREATE DUEL FLAG OBJECT
    GameObject* pGameObj = new GameObject;

    uint32 gameobjectId = m_spellInfo->EffectMiscValue[effIdx];

    Map* map = m_casterUnit->GetMap();
    float x = (m_casterUnit->GetPositionX() + unitTarget->GetPositionX()) * 0.5f;
    float y = (m_casterUnit->GetPositionY() + unitTarget->GetPositionY()) * 0.5f;
    float z = m_casterUnit->GetPositionZ();

    if (!pGameObj->Create(map->GenerateLocalLowGuid(HIGHGUID_GAMEOBJECT), gameobjectId, map, x, y, z,
                          m_casterUnit->GetOrientation(), 0.0f, 0.0f, 0.0f, 0.0f, GO_ANIMPROGRESS_DEFAULT, GO_STATE_READY))
    {
        delete pGameObj;
        return;
    }

    pGameObj->SetUInt32Value(GAMEOBJECT_FACTION, m_casterUnit->GetFactionTemplateId());
    pGameObj->SetUInt32Value(GAMEOBJECT_LEVEL, m_casterUnit->GetLevel() + 1);
    int32 duration = m_spellInfo->GetDuration();
    pGameObj->SetRespawnTime(duration > 0 ? duration / IN_MILLISECONDS : 0);
    pGameObj->SetSpellId(m_spellInfo->Id);

    m_casterUnit->AddGameObject(pGameObj);
    map->Add(pGameObj);
    //END

    // Send request
    WorldPacket data(SMSG_DUEL_REQUESTED, 8 + 8);
    data << pGameObj->GetObjectGuid();
    data << caster->GetObjectGuid();
    caster->GetSession()->SendPacket(&data);
    target->GetSession()->SendPacket(&data);

    // create duel-info
    DuelInfo* duel   = new DuelInfo;
    duel->initiator  = caster;
    duel->opponent   = target;
    duel->startTime  = 0;
    duel->startTimer = 0;

    DuelInfo* duel2   = new DuelInfo;
    duel2->initiator  = caster;
    duel2->opponent   = caster;
    duel2->startTime  = 0;
    duel2->startTimer = 0;

    if (GenericTransport* t = caster->GetTransport())
    {
        duel->transportGuid  = t->GetGUIDLow();
        duel2->transportGuid = t->GetGUIDLow();
    }
    caster->m_duel     = duel;
    target->m_duel     = duel2;

    caster->SetGuidValue(PLAYER_DUEL_ARBITER, pGameObj->GetObjectGuid());
    target->SetGuidValue(PLAYER_DUEL_ARBITER, pGameObj->GetObjectGuid());
}
```

**Key Behaviors**:
- Only works between players
- Cancels existing duels if caster already dueling someone else
- Fails if target already has pending duel
- Checks ignore list - cannot duel ignored players
- Both players must be in same map
- Area check: AREA_FLAG_DUEL required (no dueling in cities/dungeons)
- Creates duel flag gameobject at midpoint between players
- Sends SMSG_DUEL_REQUESTED to both players
- Creates DuelInfo for both players
- Stores transport GUID if on transport
- Sets duel arbiter (flag object) for both players
- Target must accept within duration or duel expires

---

### 100 - SPELL_EFFECT_INEBRIATE
**Function**: `effect_inebriate()`

Applies drunk effect to the target.

**Parameters**:
- `base_value`: Drunkenness level

**Usage**:
- Alcoholic beverages
- Party effects

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5014-5027`):
```cpp
void Spell::EffectInebriate(SpellEffectIndex /*effIdx*/)
{
    Player* player = ToPlayer(unitTarget);
    if (!player)
        return;

    uint16 currentDrunk = player->GetDrunkValue();
    uint16 drunkMod = damage * 256;
    if (currentDrunk + drunkMod > 0xFFFF)
        currentDrunk = 0xFFFF;
    else
        currentDrunk += drunkMod;
    player->SetDrunkValue(currentDrunk, m_CastItem ? m_CastItem->GetEntry() : 0);
}
```

**Key Behaviors**:
- Only works for player targets
- Drunkenness value from `damage` field
- Multiplied by 256 for internal storage
- Capped at 0xFFFF (65535) maximum
- Adds to current drunkenness (stacks)
- Tracks source item if cast via item
- Visual effects:
  - Screen swaying/blur
  - Character staggering animation
  - Slurred speech in chat
- Effects scale with drunkenness level:
  - 0-1999: Slightly drunk
  - 2000-5999: Drunk
  - 6000+: Completely smashed
- Wears off over time (decreases gradually)
- Some quests require specific drunkenness levels
- Can trigger vomiting emote at high levels

---

### 116 - SPELL_EFFECT_SKIN_PLAYER_CORPSE
**Function**: `effect_skin_player_corpse()`

Removes insignia from player corpse (Battlegrounds).

**Parameters**: None

**Usage**:
- Remove Insignia in battlegrounds

**Implementation Notes**:
- Only works in battlegrounds
- Makes corpse lootable by enemy
- Can loot money and items
- No honor gain
- Corpse must be enemy player

## Dependencies

Required systems:
- `PvPSystem` - For duel and BG mechanics
- `PlayerSystem` - For corpse interaction

## References

- MaNGOS: `SpellEffects.cpp` - PvP effects
- MaNGOS: `DuelHandler.cpp` - Duel mechanics
- MaNGOS: `BattleGround.cpp` - BG corpse looting
