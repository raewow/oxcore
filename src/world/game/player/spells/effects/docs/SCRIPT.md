# Script and Dummy Effects Documentation

## File: `script.rs`

## Overview

Handles script-based effects, dummy placeholders, triggers, and custom server effects.

## Effects (6 total)

### 3 - SPELL_EFFECT_DUMMY
**Function**: `effect_dummy()`

Dummy effect placeholder handled by script or hardcoded logic.

**Parameters**: Varies by spell

**Usage**:
- Execute (Warrior) - damage based on rage
- Combo point finishers (Eviscerate, etc.)
- Druid form changes
- Custom spell logic

**Implementation Details** (from MaNGOS `SpellEffects.cpp:309-3620`):
```cpp
void Spell::EffectDummy(SpellEffectIndex effIdx)
{
    if (!unitTarget && !gameObjTarget && !itemTarget && !corpseTarget)
        return;

    // selection by spell family
    switch (m_spellInfo->SpellFamilyName)
    {
        case SPELLFAMILY_GENERIC:
        {
            switch (m_spellInfo->Id)
            {
                case 8856:  // Bending Shinbone
                {
                    if (!itemTarget && m_caster->GetTypeId() != TYPEID_PLAYER)
                        return;
                    uint32 spellId = (urand(1, 5) == 1) ? 8854 : 8855;
                    m_casterUnit->CastSpell(m_casterUnit, spellId, true, nullptr);
                    return;
                }
                // ... hundreds of other spell-specific cases
            }
        }
        case SPELLFAMILY_WARRIOR:
            // Warrior-specific dummy effects
            break;
        case SPELLFAMILY_ROGUE:
            // Rogue-specific dummy effects
            break;
        // ... etc for each class
    }
}
```

**Key Behaviors**:
- Massive switch statement organized by `SpellFamilyName`
- Each spell family has its own section
- Within each family, spells handled by `spellId`
- Can target units, gameobjects, items, or corpses
- Common uses:
  - Random outcomes (Bending Shinbone: 1 in 5 chance)
  - Conditional spell casting based on state
  - Quest credit granting
  - Battleground flag interactions
  - Emote forcing
  - Power-dependent effects
- Allows complex logic without creating new effect types
- Most custom spell behavior implemented here
- Also routes to SpellScript system if configured

---

### 32 - SPELL_EFFECT_TRIGGER_MISSILE
**Function**: `effect_trigger_missile()`

Creates a projectile that travels to target location.

**Parameters**:
- `misc_value`: Spell ID to cast when missile lands

**Usage**:
- Blizzard (Mage)
- Rain of Fire (Warlock)
- Other ground-targeted AoE

**Implementation Details** (from MaNGOS `SpellEffects.cpp:1540-1558`):
```cpp
void Spell::EffectTriggerMissileSpell(SpellEffectIndex effect_idx)
{
    uint32 triggeredSpellId = m_spellInfo->EffectTriggerSpell[effect_idx];

    // normal case
    SpellEntry const* spellInfo = sSpellMgr.GetSpellEntry(triggeredSpellId);

    if (!spellInfo)
    {
        sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "EffectTriggerMissileSpell of spell %u (eff: %u): triggering unknown spell id %u",
                      m_spellInfo->Id, effect_idx, triggeredSpellId);
        return;
    }

    if (m_CastItem)
        DEBUG_FILTER_LOG(LOG_FILTER_SPELL_CAST, "WORLD: cast Item spellId - %i", spellInfo->Id);

    m_caster->CastSpell(m_targets.m_destX, m_targets.m_destY, m_targets.m_destZ, spellInfo, true, m_CastItem, nullptr, m_originalCasterGUID);
}
```

**Key Behaviors**:
- Triggered spell ID from `EffectTriggerSpell[effect_idx]`
- Validates spell exists in database
- Casts spell at destination coordinates (m_targets.m_destX/Y/Z)
- Cast as triggered spell (true parameter)
- Preserves cast item and original caster GUID
- Creates visible projectile arc to destination
- Spell effects apply on arrival at destination
- Used for:
  - Ground-targeted AoE spells
  - Delayed damage effects
  - Visual projectile spells
- Client shows targeting indicator during cast
- Can be interrupted before missile lands

---

### 61 - SPELL_EFFECT_SEND_EVENT
**Function**: `effect_send_event()`

Triggers a game event.

**Parameters**:
- `misc_value`: Event ID

**Usage**:
- Dungeon/raid encounter triggers
- Quest events
- World events

**Implementation Details** (from MaNGOS `SpellEffects.cpp:1722-1736`):
```cpp
void Spell::EffectSendEvent(SpellEffectIndex effIdx)
{
    /*
    we do not handle a flag dropping or clicking on flag in battleground by sendevent system
    */
    DEBUG_FILTER_LOG(LOG_FILTER_SPELL_CAST, "Spell ScriptStart %u for spellid %u in EffectSendEvent ", m_spellInfo->EffectMiscValue[effIdx], m_spellInfo->Id);

    // In some cases, the spell does not require a focus but still uses a game object
    // eg. using an Altar or similar GO.
    // Therefore, pass the GO as the target if this is the case.
    GameObject* gObject = focusObject ? focusObject : m_targets.getGOTarget();

    if (!sScriptMgr.OnProcessEvent(m_spellInfo->EffectMiscValue[effIdx], m_caster, gObject, true))
        m_caster->GetMap()->ScriptsStart(sEventScripts, m_spellInfo->EffectMiscValue[effIdx], m_caster->GetObjectGuid(), gObject ? gObject->GetObjectGuid() : ObjectGuid());
}
```

**Key Behaviors**:
- Event ID from `EffectMiscValue[effIdx]`
- Gets gameobject target if no focus object
- First tries `ScriptMgr::OnProcessEvent()`:
  - Allows custom C++ script handling
  - Returns true if handled by script
- If not handled by script, starts DB scripts via `ScriptsStart()`:
  - Uses `sEventScripts` script map
  - Passes caster GUID and target GO GUID
- Used for:
  - Scripted dungeon/raid encounters
  - Quest events
  - World events
  - Gameobject interactions
- Coordinates multiple game objects/creatures
- Can trigger complex scripted sequences

---

### 64 - SPELL_EFFECT_TRIGGER_SPELL
**Function**: `effect_trigger_spell()`

Casts another spell.

**Parameters**:
- `misc_value`: Spell ID to trigger

**Usage**:
- Spell chains (one spell triggers another)
- Proc effects
- Hidden spell triggers

**Implementation Details** (from MaNGOS `SpellEffects.cpp:1476-1538`):
```cpp
void Spell::EffectTriggerSpell(SpellEffectIndex effIdx)
{
    // only unit case known
    if (!unitTarget)
    {
        if (gameObjTarget || itemTarget)
            sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "Spell::EffectTriggerSpell (Spell: %u): Unsupported non-unit case!", m_spellInfo->Id);
        return;
    }

    uint32 triggeredSpellId = m_spellInfo->EffectTriggerSpell[effIdx];

    // normal case
    SpellEntry const* spellInfo = sSpellMgr.GetSpellEntry(triggeredSpellId);
    if (!spellInfo)
    {
        sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "EffectTriggerSpell of spell %u: triggering unknown spell id %i", m_spellInfo->Id, triggeredSpellId);
        return;
    }

    // select formal caster for triggered spell
    SpellCaster* caster = m_caster;

    // some triggered spells require specific equipment
    if (spellInfo->EquippedItemClass >= 0 && m_caster->GetTypeId() == TYPEID_PLAYER)
    {
        // main hand weapon required
        if (spellInfo->HasAttribute(SPELL_ATTR_EX3_REQUIRES_MAIN_HAND_WEAPON))
        {
            Item* item = ((Player*)m_caster)->GetWeaponForAttack(BASE_ATTACK, true, false);
            if (!item)
                return;
            if (!item->IsFitToSpellRequirements(spellInfo))
                return;
        }

        // offhand hand weapon required
        if (spellInfo->AttributesEx3 & SPELL_ATTR_EX3_REQUIRES_OFFHAND_WEAPON)
        {
            Item* item = ((Player*)m_caster)->GetWeaponForAttack(OFF_ATTACK, true, false);
            if (!item)
                return;
            if (!item->IsFitToSpellRequirements(spellInfo))
                return;
        }
    }

    caster->CastSpell(unitTarget, spellInfo, true, m_CastItem, nullptr, m_originalCasterGUID);
}
```

**Key Behaviors**:
- Triggered spell ID from `EffectTriggerSpell[effIdx]`
- Only works with unit targets (logs error for GO/item)
- Validates triggered spell exists
- Checks equipment requirements:
  - Main hand weapon required
  - Off-hand weapon required
  - Validates weapon fits spell requirements
- Casts as triggered spell (true parameter)
- Preserves cast item and original caster GUID
- New spell inherits caster and target
- Does not consume reagents or trigger cooldown
- Used for:
  - Spell chains and combos
  - Proc-triggered spells
  - Hidden spell triggers
  - Spell variations and ranks

---

### 77 - SPELL_EFFECT_SCRIPT_EFFECT
**Function**: `effect_script_effect()`

General script effect handler.

**Parameters**: Varies by spell

**Usage**:
- Complex scripted spells
- Boss abilities
- Special mechanics

**Implementation Details** (from MaNGOS `SpellEffects.cpp:3621-4510`):
```cpp
void Spell::EffectScriptEffect(SpellEffectIndex effIdx)
{
    switch (m_spellInfo->SpellFamilyName)
    {
        case SPELLFAMILY_GENERIC:
        {
            switch (m_spellInfo->Id)
            {
                case 8856:                                  // Bending Shinbone
                {
                    if (!itemTarget && m_caster->GetTypeId() != TYPEID_PLAYER)
                        return;

                    uint32 spellId = (urand(1, 5) == 1) ? 8854 : 8855;
                    m_casterUnit->CastSpell(m_casterUnit, spellId, true, nullptr);
                    return;
                }
                case 10101:                                 // Knock Away
                {
                    if (!unitTarget || !m_casterUnit)
                        return;

                    m_casterUnit->GetThreatManager().modifyThreatPercent(unitTarget, -100);

                    return;
                }
                case 17512:                                 // Piccolo of the Flaming Fire
                {
                    if (!unitTarget || unitTarget->GetTypeId() != TYPEID_PLAYER)
                        return;

                    unitTarget->HandleEmoteCommand(EMOTE_STATE_DANCE);

                    return;
                }
                // ... hundreds more spell-specific cases
            }
        }
        // ... class-specific sections
    }
}
```

**Key Behaviors**:
- Similar structure to SPELL_EFFECT_DUMMY
- Massive switch statement by SpellFamilyName and spellId
- Used for more complex scripted behaviors than DUMMY
- Common uses:
  - Boss encounter mechanics
  - Quest items with complex behavior
  - Special item effects
  - Event triggers
  - Threat modifications
  - Emote forcing
  - Cooldown management
  - Race-specific spell variations
- Each spell handled individually
- Can interact with items, units, and gameobjects
- More specialized than DUMMY for complex cases

---

### 131 - SPELL_EFFECT_NOSTALRIUS
**Function**: `effect_nostalrius()`

Custom server-specific effect.

**Parameters**: Server-defined

**Usage**:
- Server custom mechanics
- Special events
- GM commands

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5855-5858`):
```cpp
void Spell::EffectNostalrius(SpellEffectIndex effIdx)
{
    sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "SPELL_EFFECT_NOSTALRIUS");
}
```

**Key Behaviors**:
- **NOT IMPLEMENTED** in MaNGOS (empty function)
- Logs debug message only
- Reserved for server-specific custom features
- Can be used for:
  - Custom server events
  - GM command implementations
  - Server-specific mechanics
  - Development/testing tools
- Implementation would be added by server developers
- Not part of standard MaNGOS/TrinityCore
- Used by Nostalrius and similar private servers

## Dependencies

Required systems:
- `SpellScriptSystem` - For custom spell logic
- `EventSystem` - For game events
- `MissileSystem` - For projectiles

## References

- MaNGOS: `SpellEffects.cpp` - Script effects
- MaNGOS: `ScriptMgr.cpp` - Script handling
- MaNGOS: `GameEventMgr.cpp` - Event management
