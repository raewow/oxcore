# Dispel Effects Documentation

## File: `dispel.rs`

## Overview

Handles dispel mechanics for removing buffs/debuffs from targets.

## Effects (2 total)

### 38 - SPELL_EFFECT_DISPEL
**Function**: `effect_dispel()`

Dispels magic effects from the target.

**Parameters**:
- `misc_value`: Dispel type (0=Magic, 1=Curse, 2=Disease, 3=Poison)
- `base_value`: Number of effects to dispel

**Usage**:
- Dispel Magic (Priest)
- Purge (Shaman)
- Cleanse (Paladin)
- Remove Curse (Mage/Druid)
- Abolish Disease (Priest/Shaman)

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2414-2568`):
```cpp
void Spell::EffectDispel(SpellEffectIndex effIdx)
{
    if (!unitTarget)
        return;

    // Shield Slam 50% chance dispel
    if (m_spellInfo->IsFitToFamily<SPELLFAMILY_WARRIOR, CF_WARRIOR_SHIELD_SLAM>() && !roll_chance_i(50))
        return;

    // Fill possible dispel list
    int32 priorityDispel = -1;
    std::list <std::pair<SpellAuraHolder*, uint32>> dispelList;

    bool checkFaction = true;
    // Pierre de sort dissipe sorts negatifs et positifs.
    if (m_spellInfo->IsFitToFamily<SPELLFAMILY_WARLOCK, CF_WARLOCK_SPELLSTONE>())
        checkFaction = false;
    bool friendly = checkFaction && !isReflected ? unitTarget->IsFriendlyTo(m_caster) : false;
    // Create dispel mask by dispel type
    int32 dispelType = m_spellInfo->EffectMiscValue[effIdx];
    uint32 dispelMask  = GetDispellMask(dispelType < 0 ? DISPEL_ALL : DispelType(dispelType));
    Unit::SpellAuraHolderMap const& auras = unitTarget->GetSpellAuraHolderMap();
    for (const auto& aura : auras)
    {
        SpellAuraHolder* holder = aura.second;
        if ((1 << holder->GetSpellProto()->Dispel) & dispelMask)
        {
            if (holder->GetSpellProto()->Dispel == DISPEL_MAGIC ||
                holder->GetSpellProto()->Dispel == DISPEL_POISON)
            {
                if (checkFaction)
                {
                    bool positive = holder->IsPositive();
                    // do not remove positive auras if friendly target
                    // do not remove negative auras if non-friendly target
                    // when removing charm auras ignore hostile reaction from the charm
                    if (!friendly && holder->GetSpellProto()->IsCharmSpell())
                    {
                        if (CharmInfo *charm = unitTarget->GetCharmInfo())
                            if (FactionTemplateEntry const* ft = charm->GetOriginalFactionTemplate())
                                if (FactionTemplateEntry const* ft2 = m_caster->GetFactionTemplateEntry())
                                    if (ft->IsFriendlyTo(*ft2))
                                        priorityDispel = dispelList.size();
                    }
                    else if (positive == friendly)
                        continue;
                }
            }
            dispelList.push_back(std::pair<SpellAuraHolder*, uint32>(holder, holder->GetStackAmount()));
        }
    }
    // Ok if exist some buffs for dispel try dispel it
    if (!dispelList.empty())
    {
        std::vector<std::pair<SpellAuraHolder*, uint32> > successList; // (spellId,casterGuid)
        std::vector < uint32 > failList; // spellId

        // some spells have effect value = 0 and all from its by meaning expect 1
        if (!damage)
            damage = 1;

        // Dispel N = damage buffs (or while exist buffs for dispel)
        for (int32 count = 0; count < damage && !dispelList.empty(); ++count)
        {
            // Random select buff for dispel
            std::list<std::pair<SpellAuraHolder*, uint32> >::iterator dispelItr = dispelList.begin();
            if (priorityDispel >= 0)
            {
                std::advance(dispelItr, priorityDispel);
                priorityDispel = -1;
            }
            else
            {
                std::advance(dispelItr, urand(0, dispelList.size() - 1));
            }

            SpellAuraHolder* holder = dispelItr->first;

            dispelItr->second -= 1;

            // remove entry from dispelList if nothing left in stack
            if (dispelItr->second == 0)
                dispelList.erase(dispelItr);

            SpellEntry const* spellInfo = holder->GetSpellProto();
            // Base dispel chance
            // TODO: possible chance depend from spell level??
            int32 missChance = 0;
            // Apply dispel mod from aura caster
            if (Unit* caster = holder->GetCaster())
            {
                if (Player* modOwner = caster->GetSpellModOwner())
                    modOwner->ApplySpellMod(spellInfo->Id, SPELLMOD_RESIST_DISPEL_CHANCE, missChance, this);
            }
            // Try dispel
            if (roll_chance_i(missChance))
                failList.push_back(spellInfo->Id);
            else
            {
                bool foundDispelled = false;
                for (auto& successIter : successList)
                {
                    if (successIter.first->GetId() == holder->GetId())
                    {
                        successIter.second += 1;
                        foundDispelled = true;
                        break;
                    }
                }
                if (!foundDispelled)
                    successList.push_back(std::pair<SpellAuraHolder*, uint32>(holder, 1));
            }
        }

        if (!successList.empty())
        {
            for (auto& successIter : successList)
            {
                unitTarget->RemoveAurasDueToSpell(successIter.first->GetId());
            }
        }
    }
}
```

**Key Behaviors**:
- Shield Slam has 50% dispel chance check
- Warlock Spellstone bypasses faction checks
- Friendly targets: can only dispel negative effects
- Enemy targets: can only dispel positive effects
- Charm spells have special priority handling
- Creates dispel list based on dispel mask
- Randomly selects effects to dispel (unless priority)
- Dispel resistance from SPELLMOD_RESIST_DISPEL_CHANCE
- Can fail based on miss chance
- Supports stacked aura dispelling
- Generates threat

---

### 108 - SPELL_EFFECT_DISPEL_MECHANIC
**Function**: `effect_dispel_mechanic()`

Dispels effects by mechanic type.

**Parameters**:
- `misc_value`: Mechanic to dispel (Stun, Root, Fear, etc.)

**Usage**:
- Remove effects by type (all stuns, all roots, etc.)

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5447-5471`):
```cpp
void Spell::EffectDispelMechanic(SpellEffectIndex effIdx)
{
    if (!unitTarget)
        return;

    uint32 mechanic = m_spellInfo->EffectMiscValue[effIdx];

    Unit::SpellAuraHolderMap& Auras = unitTarget->GetSpellAuraHolderMap();
    for (Unit::SpellAuraHolderMap::iterator iter = Auras.begin(), next; iter != Auras.end(); iter = next)
    {
        next = iter;
        ++next;
        SpellEntry const* spell = iter->second->GetSpellProto();
        if (iter->second->HasMechanic(mechanic))
        {
            unitTarget->RemoveAurasDueToSpell(spell->Id);
            if (Auras.empty())
                break;
            else
                next = Auras.begin();
        }
    }

    AddExecuteLogInfo(effIdx, ExecuteLogInfo(unitTarget->GetObjectGuid()));
}
```

**Key Behaviors**:
- Mechanic ID from `EffectMiscValue[effIdx]`
- Iterates through all auras on target
- Checks if aura has the specified mechanic via `HasMechanic()`
- Removes ALL matching auras immediately
- Does NOT check resistance or failure chance
- Safe iteration (handles iterator invalidation)
- Used by:
  - PvP trinkets (removes all movement impairing effects)
  - Boss mechanics
  - Special class abilities
- Common mechanics dispelled:
  - STUN
  - ROOT
  - FEAR
  - CHARM
  - POLYMORPH
  - SLOW

## Dependencies

Required systems:
- `AuraSystem` - For aura removal
- `CombatSystem` - For threat generation

## References

- MaNGOS: `SpellEffects.cpp` - Dispel effects
- MaNGOS: `SpellAuras.cpp` - Aura dispel mechanics
