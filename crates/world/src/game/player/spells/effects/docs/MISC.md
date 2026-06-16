# Miscellaneous Effects Documentation

## File: `misc.rs`

## Overview

Handles miscellaneous effects that don't fit into other categories.

## Effects (5 total)

### 1 - SPELL_EFFECT_INSTAKILL
**Function**: `effect_insta_kill()`

Instantly kills the target.

**Parameters**: None

**Usage**:
- GM kill command
- Special mechanics
- Suicide spells

**Implementation Details** (from MaNGOS `SpellEffects.cpp:265-281`):
```cpp
void Spell::EffectInstaKill(SpellEffectIndex /*effIdx*/)
{
    if (!unitTarget || !unitTarget->IsAlive())
        return;

    if (m_caster == unitTarget)                             // prevent interrupt message
        finish();

    m_caster->DealDamage(unitTarget, unitTarget->GetHealth(), nullptr, DIRECT_DAMAGE, SPELL_SCHOOL_MASK_NORMAL, m_spellInfo, false, this);
}
```

**Key Behaviors**:
- Deals damage equal to target's current health
- Uses `DealDamage()` with `DIRECT_DAMAGE` type
- Damage school: Normal (physical)
- If caster targets self, finishes spell early (prevents interrupt message)
- Ignores all damage reduction and immunity
- Can be resisted by bosses with specific immunity flags
- Generates no threat (damage dealt this way doesn't add threat)
- Used for:
  - GM kill commands
  - Suicide spells
  - Special encounter mechanics
  - Quest kill credits

---

### 36 - SPELL_EFFECT_LEARN_SPELL
**Function**: `effect_learn_spell()`

Teaches the caster (or target) a new spell.

**Parameters**:
- `misc_value`: Spell ID to learn (from `EffectTriggerSpell`)

**Usage**:
- Class trainers
- Quest spell rewards
- Item spell teaching
- Pet spell training

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2393-2412`):
```cpp
void Spell::EffectLearnSpell(SpellEffectIndex effIdx)
{
    if (!unitTarget)
        return;

    if (unitTarget->GetTypeId() != TYPEID_PLAYER)
    {
        if (m_caster->GetTypeId() == TYPEID_PLAYER)
            EffectLearnPetSpell(effIdx);

        return;
    }

    Player* player = (Player*)unitTarget;

    uint32 spellToLearn = m_spellInfo->EffectTriggerSpell[effIdx];
    player->LearnSpell(spellToLearn, false);

    sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "Spell: Player %u has learned spell %u from NpcGUID=%u", player->GetGUIDLow(), spellToLearn, m_caster->GetGUIDLow());
}
```

**Key Behaviors**:
- If target is not a player but caster is, teaches spell to caster's pet instead
- Spell ID from `EffectTriggerSpell[effIdx]` (not misc_value)
- Calls `LearnSpell()` which:
  - Validates spell exists
  - Checks class/race requirements
  - Checks level requirements
  - Adds to spellbook permanently
  - Sends packet to client
- Does not consume training points (unlike pet spells)
- Permanent addition to spellbook
- Used by:
  - Class trainers
  - Quest rewards
  - Spell learning items/books
  - Pet training (redirected to EffectLearnPetSpell)

---

### 39 - SPELL_EFFECT_LANGUAGE
**Function**: `effect_language()`

Teaches the caster a new language.

**Parameters**:
- `misc_value`: Language ID

**Usage**:
- Language learning books
- Racial languages

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2569-2576`):
```cpp
void Spell::EffectLanguage(SpellEffectIndex effIdx)
{
    Player* pPlayer = ToPlayer(unitTarget);
    if (!pPlayer)
        return;

    pPlayer->LearnLanguage(m_spellInfo->EffectMiscValue[effIdx]);
}
```

**Key Behaviors**:
- Only works for player targets
- Language ID from `EffectMiscValue[effIdx]`
- Calls `LearnLanguage()` which:
  - Adds language to known languages list
  - Allows speaking in that language
  - Allows understanding that language in chat
- Languages in WoW:
  - Common/Orcish (default)
  - Dwarven, Gnomish, Thalassian, etc. (racial)
  - Draconic, Titan, etc. (lore)
- Used for:
  - Language learning books
  - Special quest rewards
  - RP and lore purposes

---

### 46 - SPELL_EFFECT_SPAWN
**Function**: `effect_spawn()`

Spawn/login animation effect.

**Parameters**: None

**Usage**:
- Character spawn animation
- Login effects
- Visibility restoration

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2948-2956`):
```cpp
void Spell::EffectSpawn(SpellEffectIndex /*effIdx*/)
{
    if (!unitTarget || (unitTarget->GetTypeId() != TYPEID_UNIT))
        return;

    if (unitTarget->GetVisibility() != VISIBILITY_ON)
        unitTarget->SetVisibility(VISIBILITY_ON);
    unitTarget->RemoveFlag(UNIT_FIELD_FLAGS, UNIT_FLAG_SPAWNING);
}
```

**Key Behaviors**:
- Only works on creatures (not players)
- Sets visibility to ON if not already visible
- Removes `UNIT_FLAG_SPAWNING` flag
- Used for:
  - Creature spawn-in animations
  - Login effects
  - Visibility restoration after being hidden
  - Spawn animation completion
- Typically triggered by creature scripts or AI

---

### 81 - SPELL_EFFECT_CREATE_HOUSE
**Function**: `effect_create_house()`

Creates a house gameobject (TEST spell).

**Parameters**:
- `misc_value`: Game object entry ID (house model)
- Target location

**Usage**:
- Test spell only
- "Create House (TEST)"
- Development/testing

**Implementation Details** (from MaNGOS `SpellEffects.cpp:4571-4586`):
```cpp
void Spell::EffectCreateHouse(SpellEffectIndex effIdx)
{
    Player* pPlayer = m_caster->ToPlayer();
    if (!pPlayer)
        return;

    uint32 gameobjectId = m_spellInfo->EffectMiscValue[effIdx];
    if (!gameobjectId)
        return;

    // Remove old house.
    pPlayer->RemoveGameObject(m_spellInfo->Id, true);

    if (GameObject* pHouse = m_caster->SummonGameObject(gameobjectId, m_targets.m_destX, m_targets.m_destY, m_targets.m_destZ, 0))
        pHouse->SetSpellId(m_spellInfo->Id);
}
```

**Key Behaviors**:
- Only works for player casters
- Game object ID from `EffectMiscValue[effIdx]`
- Removes any existing house from same spell
- Spawns house at destination location
- Sets spell ID on house for tracking
- **NOT USED IN PRODUCTION**
- Was likely for testing guild housing or player housing concepts
- Never implemented in retail WoW

## Dependencies

Required systems:
- `PlayerSystem` - For spell learning
- `LanguageSystem` - For language learning

## References

- MaNGOS: `SpellEffects.cpp` - Miscellaneous effects
