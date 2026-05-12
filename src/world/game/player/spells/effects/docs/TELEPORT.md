# Teleportation Effects Documentation

## File: `teleport.rs`

## Overview

Handles all teleportation, binding, and transportation effects including Hearthstone, portals, flight paths, and player summoning.

## Effects (7 total)

### 5 - SPELL_EFFECT_TELEPORT_UNITS
**Function**: `effect_teleport_units()`

Teleports the target to a specific location.

**Parameters**:
- Destination varies by spell:
  - Hearthstone: Player's home bind location
  - Portals: Fixed location from SpellTargetPosition.dbc
  - Recall spells: Class-specific locations

**Usage**:
- Hearthstone
- Mage portals (Stormwind, Orgrimmar, etc.)
- Class teleports (Astral Recall, Teleport: Moonglade)
- Dungeon exits

**Implementation Details** (from MaNGOS `SpellEffects.cpp:1560-1626`):
```cpp
void Spell::EffectTeleportUnits(SpellEffectIndex effIdx)
{
    if (!unitTarget || unitTarget->IsTaxiFlying())
        return;

    switch (m_spellInfo->EffectImplicitTargetB[effIdx])
    {
        case TARGET_LOCATION_CASTER_HOME_BIND:
        {
            // Only players can teleport to innkeeper
            if (unitTarget->GetTypeId() != TYPEID_PLAYER)
                return;

            ((Player*)unitTarget)->TeleportToHomebind(unitTarget == m_caster ? TELE_TO_SPELL : 0, false);
            return;
        }
        case TARGET_ENUM_UNITS_SCRIPT_AOE_AT_SRC_LOC:                     // in all cases first TARGET_LOCATION_DATABASE
        case TARGET_LOCATION_DATABASE:
        {
            SpellTargetPosition const* st = sSpellMgr.GetSpellTargetPosition(m_spellInfo->Id);
            if (!st)
            {
                sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "Spell::EffectTeleportUnits - unknown Teleport coordinates for spell ID %u", m_spellInfo->Id);
                return;
            }

            if (st->mapId == unitTarget->GetMapId())
                unitTarget->NearTeleportTo(*st, TELE_TO_NOT_LEAVE_COMBAT | TELE_TO_NOT_UNSUMMON_PET | (unitTarget == m_caster ? TELE_TO_SPELL : 0));
            else if (unitTarget->GetTypeId() == TYPEID_PLAYER)
                ((Player*)unitTarget)->TeleportTo(*st, unitTarget == m_caster ? TELE_TO_SPELL : 0);
            return;
        }
        case TARGET_LOCATION_CASTER_DEST:
        {
            if (!m_casterUnit)
                return;

            // m_destN filled, but sometimes for wrong dest and does not have TARGET_FLAG_DEST_LOCATION
            float x = unitTarget->GetPositionX();
            float y = unitTarget->GetPositionY();
            float z = unitTarget->GetPositionZ();
            float orientation = m_caster->GetOrientation();

            m_casterUnit->NearTeleportTo(x, y, z, orientation, TELE_TO_NOT_LEAVE_COMBAT | TELE_TO_NOT_UNSUMMON_PET | (unitTarget == m_caster ? TELE_TO_SPELL : 0));
            return;
        }
        default:
        {
            // If not exist data for dest location - return
            if (!(m_targets.m_targetMask & TARGET_FLAG_DEST_LOCATION))
                return;

            // Init dest coordinates
            float x = m_targets.m_destX;
            float y = m_targets.m_destY;
            float z = m_targets.m_destZ;
            float orientation = m_caster->GetOrientation();

            // Teleport player to destination
            if (unitTarget->GetTypeId() == TYPEID_PLAYER)
                ((Player*)unitTarget)->TeleportTo(unitTarget->GetMapId(), x, y, z, orientation, unitTarget == m_caster ? TELE_TO_SPELL : 0);
            else
                unitTarget->NearTeleportTo(x, y, z, orientation, TELE_TO_NOT_LEAVE_COMBAT | TELE_TO_NOT_UNSUMMON_PET);
        }
    }
}
```

**Key Behaviors**:
- Cannot teleport units that are taxi flying
- Target type determines destination:
  - `TARGET_LOCATION_CASTER_HOME_BIND`: Player's home bind location (Hearthstone)
  - `TARGET_LOCATION_DATABASE`: Fixed location from SpellTargetPosition.dbc (portals)
  - `TARGET_LOCATION_CASTER_DEST`: Caster's destination location
  - Default: Uses spell target destination coordinates
- Same map: Uses `NearTeleportTo()` (instant, no loading screen)
- Different map: Uses `TeleportTo()` (loading screen, full teleport)
- Teleport flags:
  - `TELE_TO_NOT_LEAVE_COMBAT`: Stay in combat
  - `TELE_TO_NOT_UNSUMMON_PET`: Keep pet summoned
  - `TELE_TO_SPELL`: Teleport initiated by spell

---

### 11 - SPELL_EFFECT_BIND
**Function**: `effect_bind()`

Sets the player's home bind location (Hearthstone location).

**Parameters**: None

**Usage**:
- Innkeepers (when setting hearthstone)
- Special bind locations

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5812-5845`):
```cpp
void Spell::EffectBind(SpellEffectIndex effIdx)
{
    Player* player = ToPlayer(unitTarget);
    if (!player)
        return;

    uint32 areaId;
    WorldLocation loc;
    player->GetPosition(loc);
    areaId = player->GetAreaId();

    player->SetHomebindToLocation(loc, areaId);

    // binding
    WorldPacket data(SMSG_BINDPOINTUPDATE, (4 + 4 + 4 + 4 + 4));
    data << float(loc.x);
    data << float(loc.y);
    data << float(loc.z);
    data << uint32(loc.mapId);
    data << uint32(areaId);
    player->SendDirectMessage(&data);

    sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "New Home Position X is %f", loc.x);
    sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "New Home Position Y is %f", loc.y);
    sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "New Home Position Z is %f", loc.z);
    sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "New Home MapId is %u", loc.mapId);
    sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "New Home AreaId is %u", areaId);

    // zone update
    data.Initialize(SMSG_PLAYERBOUND, 8 + 4);
    data << m_caster->GetObjectGuid();
    data << uint32(areaId);
    player->SendDirectMessage(&data);
}
```

**Key Behaviors**:
- Only works for player targets
- Gets current position and area ID
- Sets home bind location via `SetHomebindToLocation()`
- Sends `SMSG_BINDPOINTUPDATE` packet with new coordinates
- Sends `SMSG_PLAYERBOUND` packet with caster GUID and area ID
- Players can only have one home bind location at a time
- Hearthstone will teleport to this location
- Innkeeper NPCs typically cast this on players

---

### 43 - SPELL_EFFECT_TELEPORT_UNITS_FACE_CASTER
**Function**: `effect_teleport_units_face_caster()`

Teleports target to caster and makes them face the caster.

**Parameters**: None

**Usage**:
- Boss encounter mechanics
- Special teleport effects

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2897-2915`):
```cpp
void Spell::EffectTeleUnitsFaceCaster(SpellEffectIndex effIdx)
{
    if (!unitTarget)
        return;

    if (unitTarget->IsTaxiFlying())
        return;

    float fx, fy, fz;
    if (m_targets.m_targetMask & TARGET_FLAG_DEST_LOCATION)
        m_targets.getDestination(fx, fy, fz);
    else
    {
        float dis = GetSpellRadius(sSpellRadiusStore.LookupEntry(m_spellInfo->EffectRadiusIndex[effIdx]));
        m_caster->GetClosePoint(fx, fy, fz, unitTarget->GetObjectBoundingRadius(), dis);
    }

    unitTarget->NearTeleportTo(fx, fy, fz, -m_caster->GetOrientation(), TELE_TO_NOT_LEAVE_COMBAT | TELE_TO_NOT_UNSUMMON_PET | (unitTarget == m_caster ? TELE_TO_SPELL : 0));
}
```

**Key Behaviors**:
- Cannot teleport units that are taxi flying
- If destination location is specified, uses that
- Otherwise calculates position in front of caster using spell radius
- Teleports target to calculated position
- Sets target's orientation to face caster (negative caster orientation)
- Uses `NearTeleportTo()` for instant teleport
- Teleport flags preserve combat and pet
- Used for boss mechanics that require specific positioning

---

### 84 - SPELL_EFFECT_STUCK
**Function**: `effect_stuck()`

Emergency teleport for stuck players.

**Parameters**: None

**Usage**:
- Help menu "Stuck" option
- GM unstuck command

**Implementation Details** (from MaNGOS `SpellEffects.cpp:4701-4720`):
```cpp
void Spell::EffectStuck(SpellEffectIndex /*effIdx*/)
{
    if (!unitTarget || unitTarget->GetTypeId() != TYPEID_PLAYER)
        return;

    if (!sWorld.getConfig(CONFIG_BOOL_CAST_UNSTUCK))
        return;

    Player* pTarget = (Player*)unitTarget;

    sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "Spell Effect: Stuck");
    sLog.Out(LOG_BASIC, LOG_LVL_MINIMAL, "Player %s (guid %u) used auto-unstuck feature at map %u (%f, %f, %f).", pTarget->GetName(), pTarget->GetGUIDLow(), m_caster->GetMapId(), m_caster->GetPositionX(), pTarget->GetPositionY(), pTarget->GetPositionZ());

    if (pTarget->IsTaxiFlying())
        return;

    // TP to last overmap position
    if (fabs(pTarget->m_lastSafePosition.x) > 0.1f && fabs(pTarget->m_lastSafePosition.y) > 0.1f)
        pTarget->TeleportTo(pTarget->GetMapId(), pTarget->m_lastSafePosition.x, pTarget->m_lastSafePosition.y, pTarget->m_lastSafePosition.z - 2.0f + 0.7f, pTarget->m_lastSafePosition.o);
}
```

**Key Behaviors**:
- Only works for player targets
- Requires `CONFIG_BOOL_CAST_UNSTUCK` to be enabled
- Logs usage for GM tracking
- Cannot use while taxi flying
- Teleports to last safe position (not home bind)
- Last safe position tracked by server when player is on valid ground
- Adjusts Z coordinate by -2.0f + 0.7f to place player on ground
- Different from Hearthstone - uses last safe position instead of home bind
- Used by the automatic /stuck command

---

### 85 - SPELL_EFFECT_SUMMON_PLAYER
**Function**: `effect_summon_player()`

Summons a player to the caster's location.

**Parameters**: None

**Usage**:
- Meeting stones
- Warlock Ritual of Summoning
- GM summon

**Implementation Details** (from MaNGOS `SpellEffects.cpp:4722-4741`):
```cpp
void Spell::EffectSummonPlayer(SpellEffectIndex /*effIdx*/)
{
    Player* pPlayerTarget = ToPlayer(unitTarget);
    if (!pPlayerTarget)
        return;

    // Evil Twin (ignore player summon, but hide this for summoner)
    if (pPlayerTarget->HasAura(23445))
        return;

    float x, y, z;
    SpellCaster* landingObject = m_caster;
    // summon to the ritual go location if any
    if (GameObject* pGo = m_targets.getGOTarget())
        if (pGo->GetGoType() == GAMEOBJECT_TYPE_SUMMONING_RITUAL)
            landingObject = pGo;

    landingObject->GetClosePoint(x, y, z, unitTarget->GetObjectBoundingRadius());
    pPlayerTarget->SendSummonRequest(m_caster->GetObjectGuid(), m_caster->GetMapId(), m_caster->GetZoneId(), x, y, z);
}
```

**Key Behaviors**:
- Only works on player targets
- Blocked by "Evil Twin" aura (spell 23445) - prevents summoning
- If cast via GAMEOBJECT_TYPE_SUMMONING_RITUAL, uses gameobject position
- Otherwise calculates position near caster
- Sends summon request to player (confirmation dialog)
- Player must accept to be teleported
- Target restrictions checked when player accepts:
  - Cannot accept while in combat
  - Cannot accept while in battleground
  - Cannot accept if caster is in instance and target is not
- Used by Warlock Ritual of Summoning and Meeting Stones

---

### 123 - SPELL_EFFECT_SEND_TAXI
**Function**: `effect_send_taxi()`

Sends the player on a flight path.

**Parameters**:
- `misc_value`: Taxi path ID from TaxiPath.dbc

**Usage**:
- Flight masters
- Special flight spells

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5426-5432`):
```cpp
void Spell::EffectSendTaxi(SpellEffectIndex effIdx)
{
    if (!unitTarget || unitTarget->GetTypeId() != TYPEID_PLAYER)
        return;

    ((Player*)unitTarget)->ActivateTaxiPathTo(m_spellInfo->EffectMiscValue[effIdx], m_spellInfo->Id, true);
}
```

**Key Behaviors**:
- Only works for player targets
- Taxi path ID from `EffectMiscValue[effIdx]`
- Calls `ActivateTaxiPathTo()` which:
  - Mounts taxi mount (gryphon, wyvern, etc.)
  - Follows predefined path from TaxiPathNodes.dbc
  - Player is invulnerable during flight
  - Cannot be interrupted once started
  - Player cannot control character during flight
  - Arrives at destination after path completes
- Third parameter `true` indicates this is a spell-activated taxi
- Used by flight masters and special flight spells (e.g., free rides)

---

### 124 - SPELL_EFFECT_PLAYER_PULL
**Function**: `effect_player_pull()`

Pulls the player toward the caster.

**Parameters**:
- `base_value`: Pull distance

**Usage**:
- Boss mechanics (pull player to center)
- Special abilities

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5435-5445`):
```cpp
void Spell::EffectPlayerPull(SpellEffectIndex effIdx)
{
    if (!unitTarget)
        return;

    // Todo: this implementation seems very wrong. Gives terrible results for maexxna web-wrap and thaddius magnetic pull
    float dist = unitTarget->GetDistance2d(m_caster);
    if (damage && dist > damage)
        dist = damage;
    unitTarget->KnockBackFrom(m_caster, -dist, float(m_spellInfo->EffectMiscValue[effIdx]) / 10);
}
```

**Key Behaviors**:
- Calculates 2D distance between target and caster
- If `damage` is specified and distance > damage, caps at damage value
- Uses `KnockBackFrom()` with negative distance (pull instead of push)
- Vertical component from `EffectMiscValue[effIdx]` / 10
- Note: MaNGOS comment indicates this implementation may be incorrect
- Issues with Maexxna web-wrap and Thaddius magnetic pull
- Pulls target toward caster position
- Does not change target's facing

## Dependencies

Required systems:
- `PlayerSystem` - For teleportation and position management
- `TaxiSystem` - For flight paths
- `MapSystem` - For map transitions

## References

- MaNGOS: `SpellEffects.cpp` - `EffectTeleportUnits()`, `EffectBind()`, etc.
- MaNGOS: `Player.cpp` - Teleportation and home bind
- MaNGOS: `TaxiHandler.cpp` - Flight path handling
