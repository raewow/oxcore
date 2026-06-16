# Aura Effects Documentation

## File: `aura.rs`

## Overview

Handles aura (buff/debuff) application and area aura effects.

## Effects (7 total)

### 6 - SPELL_EFFECT_APPLY_AURA
**Function**: `effect_apply_aura()`

Applies a buff or debuff to the target.

**Parameters**:
- Aura type from spell entry
- Duration from DBC
- Periodic interval (for DoTs/HoTs)

**Usage**:
- All buffs and debuffs
- DoTs (Damage over Time)
- HoTs (Heal over Time)
- Stat modifiers
- Crowd control

**Implementation Details** (from MaNGOS `SpellEffects.cpp:1626-1657`):
```cpp
void Spell::EffectApplyAura(SpellEffectIndex effIdx)
{
    if (!unitTarget || !m_spellAuraHolder)
        return;

    if (!m_spellInfo->EffectApplyAuraName[effIdx])
        return;

    // ghost spell check, allow apply any auras at player loading in ghost mode
    if ((!unitTarget->IsAlive() && !(m_spellInfo->CanTargetDeadTarget() || m_spellInfo->IsDeathPersistentSpell())) &&
            (unitTarget->GetTypeId() != TYPEID_PLAYER || !((Player*)unitTarget)->GetSession()->PlayerLoading()))
        return;

    if (unitTarget->HasMorePowerfulSpellActive(m_spellInfo))
        return;

    Unit* caster = GetAffectiveCaster();
    if (!caster)
    {
        // FIXME: currently we can't have auras applied explicitly by gameobjects
        // so for auras from wild gameobjects (no owner) target used
        if (m_originalCasterGUID.IsGameObject())
            caster = unitTarget;
        else
            return;
    }

    Aura* aur = CreateAura(m_spellInfo, effIdx, &m_currentBasePoints[effIdx], 
                           m_spellAuraHolder, unitTarget, caster, m_CastItem);
    m_spellAuraHolder->AddAura(aur, effIdx);
}
```

**Key Behaviors**:
- Checks if aura name is set in spell entry
- Validates target is alive OR spell can target dead OR target is loading in ghost mode
- Checks for "more powerful spell" already active (prevents weaker version)
- Gets effective caster (handles gameobject case by using target as caster)
- Creates aura via `CreateAura()` with current base points
- Adds aura to spell aura holder
- Reads aura type from Spell.dbc
- Duration from Duration.dbc
- Can stack (based on rules)
- Can have charges
- Periodic effects tick at interval
- Removed by dispel

---

### 27 - SPELL_EFFECT_PERSISTENT_AREA_AURA
**Function**: `effect_persistent_area_aura()`

Creates a persistent ground effect.

**Parameters**:
- Target location
- Radius
- Duration

**Usage**:
- Consecration (Paladin)
- Blizzard (Mage)
- Rain of Fire (Warlock)
- Ground-targeted AoE

**Implementation Details** (from MaNGOS `SpellEffects.cpp:1949-1978`):
```cpp
void Spell::EffectPersistentAA(SpellEffectIndex effIdx)
{
    SpellCaster* pCaster = GetAffectiveCasterObject();

    if (GameObject* pGo = ToGameObject(pCaster))
        if (Unit* pOwner = pGo->GetOwner())
            pCaster = pOwner;
    
    if (!pCaster)
        pCaster = m_caster;

    float radius = GetSpellRadius(sSpellRadiusStore.LookupEntry(m_spellInfo->EffectRadiusIndex[effIdx]));

    if (Unit* pUnit = pCaster->ToUnit())
    {
        if (Player* modOwner = pUnit->GetSpellModOwner())
            modOwner->ApplySpellMod(m_spellInfo->Id, SPELLMOD_RADIUS, radius);
    }

    DynamicObject* dynObj = new DynamicObject;
    if (!dynObj->Create(pCaster->GetMap()->GenerateLocalLowGuid(HIGHGUID_DYNAMICOBJECT), pCaster, m_spellInfo->Id,
                        effIdx, m_targets.m_destX, m_targets.m_destY, m_targets.m_destZ, m_duration, radius, DYNAMIC_OBJECT_AREA_SPELL))
    {
        delete dynObj;
        return;
    }

    pCaster->AddDynObject(dynObj);
    pCaster->GetMap()->Add(dynObj);
}
```

**Key Behaviors**:
- Gets effective caster (handles gameobject owner)
- Calculates radius from spell radius entry
- Applies SPELLMOD_RADIUS modifier from spell mods
- Creates DynamicObject at destination location
- DynamicObject type: `DYNAMIC_OBJECT_AREA_SPELL`
- Adds to caster's dynamic object list
- Applies aura to targets in radius periodically
- Lasts for spell duration
- Visible effect on ground

---

### 35, 119, 128, 129, 132 - Area Aura Effects
**Function**: All handled by `EffectApplyAreaAura()` in MaNGOS

These effects create area auras that affect multiple targets based on type.

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2282-2291`):
```cpp
void Spell::EffectApplyAreaAura(SpellEffectIndex effIdx)
{
    if (!unitTarget)
        return;
    if (!unitTarget->IsAlive())
        return;

    AreaAura* Aur = new AreaAura(m_spellInfo, effIdx, &m_currentBasePoints[effIdx], 
                                 m_spellAuraHolder, unitTarget, 
                                 m_casterUnit ? m_casterUnit : unitTarget, m_CastItem);
    m_spellAuraHolder->AddAura(Aur, effIdx);
}
```

**Area Aura Types**:

| Effect ID | Name | Target Type |
|-----------|------|-------------|
| 35 | SPELL_EFFECT_APPLY_AREA_AURA_PARTY | Party members |
| 119 | SPELL_EFFECT_APPLY_AREA_AURA_PET | Caster's pet only |
| 128 | SPELL_EFFECT_APPLY_AREA_AURA_FRIEND | All friendly units |
| 129 | SPELL_EFFECT_APPLY_AREA_AURA_ENEMY | All enemy units |
| 132 | SPELL_EFFECT_APPLY_AREA_AURA_RAID | Raid members |

**Key Behaviors**:
- Creates `AreaAura` object (subclass of Aura)
- AreaAura handles target selection based on effect type
- Uses caster if available, otherwise target as caster
- Stays on caster/target unit
- Periodically checks for valid targets in range
- Applies aura to valid targets
- Removes aura from targets that move out of range
- Target selection logic:
  - **PARTY**: Party members in radius
  - **PET**: Only caster's active pet
  - **FRIEND**: All friendly units (not just party)
  - **ENEMY**: All enemy units
  - **RAID**: All raid members in radius

**Usage Examples**:
- **Party (35)**: Paladin auras, Totem auras, Party buffs
- **Pet (119)**: Pet buffs, Pet-specific auras
- **Friend (128)**: Friendly AoE buffs (BC+)
- **Enemy (129)**: Enemy AoE debuffs (BC+)
- **Raid (132)**: Raid-wide buffs (BC+)

## Dependencies

Required systems:
- `AuraSystem` - For aura management
- `GroupSystem` - For party/raid auras

## References

- MaNGOS: `SpellEffects.cpp` - `EffectApplyAura()`
- MaNGOS: `SpellAuras.cpp` - Aura implementation
