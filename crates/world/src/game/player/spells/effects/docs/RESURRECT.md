# Resurrection Effects Documentation

## File: `resurrect.rs`

## Overview

Handles various resurrection effects for bringing dead players back to life.

## Effects (3 total)

### 18 - SPELL_EFFECT_RESURRECT
**Function**: `effect_resurrect()`

Resurrects a dead player with percentage-based health/mana restoration.

**Parameters**:
- Target: Dead player
- `base_value`: Health/mana percentage (e.g., 35 = 35%)

**Usage**:
- Resurrection (Priest, Paladin, Shaman, Druid)
- Combat resurrection

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5168-5191`):
```cpp
void Spell::EffectResurrect(SpellEffectIndex effIdx)
{
    if (!unitTarget)
        return;
    if (unitTarget->GetTypeId() != TYPEID_PLAYER)
        return;
    if (unitTarget->IsAlive())
        return;
    if (!unitTarget->IsInWorld())
        return;

    Player* pTarget = ((Player*)unitTarget);

    if (pTarget->IsRessurectRequested())      // already have one active request
        return;

    uint32 health = ditheru(pTarget->GetMaxHealth() * damage / 100);
    uint32 mana   = ditheru(pTarget->GetMaxPower(POWER_MANA) * damage / 100);

    pTarget->SetResurrectRequestData(m_caster->GetObjectGuid(), m_caster->GetMapId(), m_caster->GetInstanceId(), m_caster->GetPositionX(), m_caster->GetPositionY(), m_caster->GetPositionZ(), m_caster->GetOrientation(), health, mana);
    SendResurrectRequest(pTarget, m_casterUnit && m_casterUnit->IsSpiritHealer());

    AddExecuteLogInfo(effIdx, ExecuteLogInfo(unitTarget->GetObjectGuid()));
}
```

**Key Behaviors**:
- Only works on dead players in world
- Cannot resurrect if player already has pending request
- Health/mana calculated as percentage of maximum
- Uses `ditheru()` for random variance
- Stores resurrection data via `SetResurrectRequestData()`:
  - Caster GUID
  - Map/instance IDs
  - Caster position (resurrection location)
  - Health and mana amounts
- Sends `SMSG_RESURRECT_REQUEST` to player
- Player must accept within timeout (2 minutes)
- If accepted, player teleports to caster and is resurrected
- Spirit healer resurrection flagged separately
- Generates threat on resurrection accept
- Used by all standard resurrection spells

---

### 94 - SPELL_EFFECT_SELF_RESURRECT
**Function**: `effect_self_resurrect()`

Self-resurrection effect (immediate, no request).

**Parameters**: None

**Usage**:
- Soulstone (Warlock)
- Reincarnation (Shaman)
- Self-resurrection items

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5274-5309`):
```cpp
void Spell::EffectSelfResurrect(SpellEffectIndex effIdx)
{
    if (!unitTarget || unitTarget->IsAlive())
        return;
    if (unitTarget->GetTypeId() != TYPEID_PLAYER)
        return;
    if (!unitTarget->IsInWorld())
        return;

    float health = 0;
    float mana = 0;

    // flat case
    if (damage < 0)
    {
        health = -damage;
        mana = m_spellInfo->EffectMiscValue[effIdx];
    }
    // percent case
    else
    {
        health = damage / 100.0f * unitTarget->GetMaxHealth();
        if (unitTarget->GetMaxPower(POWER_MANA) > 0)
            mana = damage / 100.0f * unitTarget->GetMaxPower(POWER_MANA);
    }

    Player* plr = ((Player*)unitTarget);
    plr->ResurrectPlayer(0.0f);

    plr->SetHealth(ditheru(health));
    plr->SetPower(POWER_MANA, ditheru(mana));
    plr->SetPower(POWER_RAGE, 0);
    plr->SetPower(POWER_ENERGY, plr->GetMaxPower(POWER_ENERGY));

    plr->SpawnCorpseBones();
}
```

**Key Behaviors**:
- Only works on dead players in world
- Two calculation modes:
  - **Flat**: `damage < 0` uses absolute value as flat health, misc_value as flat mana
  - **Percentage**: `damage >= 0` uses percentage of max health/mana
- Calls `ResurrectPlayer(0.0f)` immediately (no request needed)
- Restores resources:
  - Health: calculated amount
  - Mana: calculated amount
  - Rage: reset to 0
  - Energy: set to maximum
- Spawns corpse bones at death location
- No resurrection sickness (unlike spirit healer)
- Used by:
  - Warlock Soulstone (cast before death, used after)
  - Shaman Reincarnation (cast while dead)
  - Engineering items (Gnomish Battle Chicken, etc.)

---

### 113 - SPELL_EFFECT_RESURRECT_NEW
**Function**: `effect_resurrect_new()`

Resurrects a dead player with flat health/mana values.

**Parameters**:
- Target: Dead player
- `base_value`: Health amount (flat)
- `misc_value`: Mana amount (flat)

**Usage**:
- Alternative resurrection spells
- Special resurrection mechanics
- Pet resurrection

**Implementation Details** (from MaNGOS `SpellEffects.cpp:206-263`):
```cpp
void Spell::EffectResurrectNew(SpellEffectIndex effIdx)
{
    if (!unitTarget || unitTarget->IsAlive())
        return;

    if (!unitTarget->IsInWorld())
        return;

    if (unitTarget->GetTypeId() != TYPEID_PLAYER)
    {
        // Pet case
        Pet* pet = unitTarget->ToPet();
        if (!pet)
            return;
        Unit* owner = pet->GetOwner();
        if (!owner)
            return;
        uint32 health = damage;

        pet->NearTeleportTo(m_caster->GetPosition(), 0);
        pet->SetUInt32Value(UNIT_DYNAMIC_FLAGS, UNIT_DYNFLAG_NONE);
        pet->RemoveFlag(UNIT_FIELD_FLAGS, UNIT_FLAG_SKINNABLE);
        pet->SetDeathState(ALIVE);
        pet->ClearUnitState(UNIT_STATE_ALL_DYN_STATES);
        pet->SetHealth(pet->GetMaxHealth() > health ? health : pet->GetMaxHealth());

        pet->AIM_Initialize();
        pet->SavePetToDB(PET_SAVE_AS_CURRENT);

        // Remove Demonic Sacrifice auras (Blizzlike - cf patchnote 1.12)
        Unit::AuraList const& auraClassScripts = owner->GetAurasByType(SPELL_AURA_OVERRIDE_CLASS_SCRIPTS);
        for (Unit::AuraList::const_iterator itr = auraClassScripts.begin(); itr != auraClassScripts.end();)
        {
            if ((*itr)->GetModifier()->m_miscvalue == 2228)
            {
                owner->RemoveAurasDueToSpell((*itr)->GetId());
                itr = auraClassScripts.begin();
            }
            else
                ++itr;
        }
        return;
    }

    Player* pTarget = ((Player*)unitTarget);

    if (pTarget->IsRessurectRequested())      // already have one active request
        return;

    uint32 health = damage;
    uint32 mana = m_spellInfo->EffectMiscValue[effIdx];
    pTarget->SetResurrectRequestData(m_caster->GetObjectGuid(), m_caster->GetMapId(), m_caster->GetInstanceId(), m_caster->GetPositionX(), m_caster->GetPositionY(), m_caster->GetPositionZ(), m_caster->GetOrientation(), health, mana);
    SendResurrectRequest(pTarget, m_casterUnit && m_casterUnit->IsSpiritHealer());

    AddExecuteLogInfo(effIdx, ExecuteLogInfo(unitTarget->GetObjectGuid()));
}
```

**Key Behaviors**:
- **Player resurrection**:
  - Uses flat health/mana values (not percentages)
  - `damage` field = health amount
  - `misc_value` = mana amount
  - Same request/accept flow as SPELL_EFFECT_RESURRECT
- **Pet resurrection** (special case):
  - Teleports pet to caster
  - Clears death-related flags
  - Sets death state to ALIVE
  - Restores health (capped at max)
  - Reinitializes AI
  - Saves to database
  - Removes Demonic Sacrifice auras from owner
- Used for:
  - Alternative player resurrection spells
  - Pet resurrection abilities
  - Special mechanics requiring flat values

## Dependencies

Required systems:
- `PlayerSystem` - For resurrection handling
- `AuraSystem` - For resurrection sickness

## References

- MaNGOS: `SpellEffects.cpp` - Resurrection effects
- MaNGOS: `Player.cpp` - Death and resurrection
