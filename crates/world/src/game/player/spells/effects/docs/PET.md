# Pet Management Effects Documentation

## File: `pet.rs`

## Overview

Handles pet-related spell effects including teaching pet spells, dismissing pets, and totem management.

## Effects (3 total)

### 57 - SPELL_EFFECT_LEARN_PET_SPELL
**Function**: `effect_learn_pet_spell()`

Teaches a spell to the caster's active pet.

**Parameters**:
- `misc_value`: Spell ID to teach

**Usage**:
- Hunter pet training (Growl, Bite, Claw, etc.)
- Warlock pet abilities
- Teaching pet spells from trainers

**Implementation Details** (from MaNGOS `SpellEffects.cpp:3265-3290`):
```cpp
void Spell::EffectLearnPetSpell(SpellEffectIndex effIdx)
{
    Player* player = m_caster->ToPlayer();
    if (!player)
        return;

    Pet* pet = player->GetPet();
    if (!pet)
        return;

    if (!pet->IsAlive())
        return;

    SpellEntry const* pLearnSpell = sSpellMgr.GetSpellEntry(m_spellInfo->EffectTriggerSpell[effIdx]);
    if (!pLearnSpell)
        return;

    if (!pet->CanLearnPetSpell(pLearnSpell->Id))
        return;

    pet->SetTP(pet->m_trainingPoints - pet->GetTPForSpell(pLearnSpell->Id));
    pet->LearnSpell(pLearnSpell->Id);

    pet->SavePetToDB(PET_SAVE_AS_CURRENT);
    player->PetSpellInitialize();
}
```

**Key Behaviors**:
- Only works for player casters with an active pet
- Pet must be alive
- Spell ID from `EffectTriggerSpell[effIdx]` (not misc_value)
- Validates pet can learn the spell via `CanLearnPetSpell()`
- Deducts training points: `SetTP(currentTP - cost)`
- Training point cost from `GetTPForSpell()`
- Teaches spell via `LearnSpell()`
- Saves pet to database
- Reinitializes pet spell bar for UI update
- Used for Hunter pet training and Warlock grimoires

---

### 102 - SPELL_EFFECT_DISMISS_PET
**Function**: `effect_dismiss_pet()`

Dismisses the caster's active pet.

**Parameters**: None

**Usage**:
- Dismiss Pet spell (Hunter)
- Temporary pet dismissal

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5072-5087`):
```cpp
void Spell::EffectDismissPet(SpellEffectIndex effIdx)
{
    Player* pPlayer = m_caster->ToPlayer();
    if (!pPlayer)
        return;

    Pet* pet = pPlayer->GetPet();

    // not let dismiss dead pet
    if (!pet || !pet->IsAlive())
        return;

    pet->Unsummon(PET_SAVE_NOT_IN_SLOT, pPlayer);

    AddExecuteLogInfo(effIdx, ExecuteLogInfo(pet->GetObjectGuid()));
}
```

**Key Behaviors**:
- Only works for player casters
- Cannot dismiss dead pets
- Calls `Unsummon()` with `PET_SAVE_NOT_IN_SLOT`
- Pet is saved to database but not in active slot
- Pet retains health/mana/position for next summon
- Different from pet death - pet can be resummoned immediately
- Used by Hunter Dismiss Pet ability
- Also used when stabling pets or switching pets

---

### 110 - SPELL_EFFECT_DESTROY_ALL_TOTEMS
**Function**: `effect_destroy_all_totems()`

Destroys all of the caster's active totems.

**Parameters**: None

**Usage**:
- Shaman totem clearing
- Emergency totem removal

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5502-5510`):
```cpp
void Spell::EffectDestroyAllTotems(SpellEffectIndex /*effIdx*/)
{
    if (!m_casterUnit)
        return;

    for (int slot = 0;  slot < MAX_TOTEM_SLOT; ++slot)
        if (Totem* totem = m_casterUnit->GetTotem(TotemSlot(slot)))
            totem->UnSummon();
}
```

**Key Behaviors**:
- Only works if caster is a unit
- Iterates through all 4 totem slots (Fire, Earth, Water, Air)
- Calls `UnSummon()` on each active totem
- Totems despawn immediately without delay
- No mana refund (unlike individual totem despawn)
- Used by Shaman totem clearing abilities
- Also triggered by some spells automatically
- Does not trigger any totem death effects

## Dependencies

Required systems:
- `PetSystem` - For pet management
- `TotemSystem` - For totem management

## References

- MaNGOS: `SpellEffects.cpp` - Pet-related effects
- MaNGOS: `Pet.cpp` - Pet implementation
- MaNGOS: `Totem.cpp` - Totem implementation
