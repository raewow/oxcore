# Profession and Skill Effects Documentation

## File: `profession.rs`

## Overview

Handles profession, skill, and crafting-related effects including skill increases, crafting, and skinning.

## Effects (6 total)

### 44 - SPELL_EFFECT_SKILL_STEP
**Function**: `effect_skill_step()`

Increases a skill maximum by a specified tier/step.

**Parameters**:
- `misc_value`: Skill ID
- `base_value`: Step amount (typically 1-4)

**Usage**:
- Skill books (First Aid, Fishing, Cooking books)
- Quest rewards that increase skills
- Special skill increases

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2917-2935`):
```cpp
void Spell::EffectLearnSkill(SpellEffectIndex effIdx)
{
    if (unitTarget->GetTypeId() != TYPEID_PLAYER)
        return;

    if (damage < 0)
        return;

    Player* target = static_cast<Player*>(unitTarget);

    uint16 skillid = uint16(m_spellInfo->EffectMiscValue[effIdx]);
    uint16 step = uint16(damage);
    uint16 current = std::max(uint16(1), target->GetSkillValuePure(skillid));
    uint16 max = (step * 75);
    target->SetSkill(skillid, current, max, step);

    if (SpellCaster const* caster = GetCastingObject())
        sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "Spell: %s has learned skill %u (to maxlevel %u) from %s", target->GetGuidStr().c_str(), skillid, max, caster->GetGuidStr().c_str());
}
```

**Key Behaviors**:
- Only works for player targets
- `damage` field determines the step (1-4)
- Maximum skill calculated as: `step * 75`
  - Step 1: 75 max (Apprentice)
  - Step 2: 150 max (Journeyman)
  - Step 3: 225 max (Expert)
  - Step 4: 300 max (Artisan)
- Current skill preserved (minimum 1)
- Sets both current value and maximum
- Used for profession tier unlocks via skill books
- Example: Expert First Aid book sets step=3 (225 max)

---

### 47 - SPELL_EFFECT_TRADE_SKILL
**Function**: `effect_trade_skill()`

Performs a trade skill crafting action.

**Parameters**:
- `misc_value`: Recipe ID

**Usage**:
- All crafting professions (Blacksmithing, Tailoring, etc.)
- Creating items from recipes

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2958-2966`):
```cpp
void Spell::EffectTradeSkill(SpellEffectIndex /*effIdx*/)
{
    if (unitTarget->GetTypeId() != TYPEID_PLAYER)
        return;
    // uint32 skillid =  m_spellInfo->EffectMiscValue[i];
    // uint16 skillmax = ((Player*)unitTarget)->(skillid);
    // ((Player*)unitTarget)->SetSkill(skillid,skillval?skillval:1,skillmax+75);
}
```

**Key Behaviors**:
- Only works for player targets
- **NOT IMPLEMENTED** in MaNGOS (empty function)
- Trade skills are handled client-side in vanilla WoW
- Server validates crafting via CMSG_CRAFT_ITEM opcode
- Client opens crafting window
- Server validates:
  - Player has required skill level
  - Player knows the recipe
  - Player has required reagents
  - Player has inventory space
- On success:
  - Consumes reagents
  - Creates item
  - May increase skill (orange/yellow recipes)
  - Applies cooldown if applicable
- Skill gain chance based on recipe color:
  - Orange: 100% chance
  - Yellow: ~50% chance
  - Green: ~25% chance
  - Gray: 0% chance

---

### 60 - SPELL_EFFECT_PROFICIENCY
**Function**: `effect_proficiency()`

Learns weapon or armor proficiency.

**Parameters**:
- `misc_value`: Proficiency mask (item subclass mask)

**Usage**:
- Weapon skills (Swords, Axes, etc.)
- Armor proficiencies (Mail, Plate)

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2263-2280`):
```cpp
void Spell::EffectProficiency(SpellEffectIndex /*effIdx*/)
{
    Player* pTarget = ToPlayer(unitTarget);
    if (!pTarget)
        return;

    uint32 subClassMask = m_spellInfo->EquippedItemSubClassMask;
    if (m_spellInfo->EquippedItemClass == ITEM_CLASS_WEAPON && !(pTarget->GetWeaponProficiency() & subClassMask))
    {
        pTarget->AddWeaponProficiency(subClassMask);
        pTarget->SendProficiency(ITEM_CLASS_WEAPON, pTarget->GetWeaponProficiency());
    }
    if (m_spellInfo->EquippedItemClass == ITEM_CLASS_ARMOR && !(pTarget->GetArmorProficiency() & subClassMask))
    {
        pTarget->AddArmorProficiency(subClassMask);
        pTarget->SendProficiency(ITEM_CLASS_ARMOR, pTarget->GetArmorProficiency());
    }
}
```

**Key Behaviors**:
- Only works for player targets
- Proficiency mask from `EquippedItemSubClassMask` in spell
- Checks if player already has proficiency (prevents duplicates)
- Weapon proficiencies:
  - Adds to `weaponProficiency` mask via `AddWeaponProficiency()`
  - Sends `SMSG_SET_PROFICIENCY` packet to client
- Armor proficiencies:
  - Adds to `armorProficiency` mask via `AddArmorProficiency()`
  - Sends `SMSG_SET_PROFICIENCY` packet to client
- Required to equip items of that type
- Usually learned from class trainers
- Passive spells that remain active
- Examples:
  - Plate Mail (Armor)
  - Two-Handed Swords (Weapon)

---

### 95 - SPELL_EFFECT_SKINNING
**Function**: `effect_skinning()`

Skins a creature corpse for leather/hides.

**Parameters**: None (target is creature corpse)

**Usage**:
- Skinning profession
- Gathering leather from beasts

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5311-5330`):
```cpp
void Spell::EffectSkinning(SpellEffectIndex /*effIdx*/)
{
    if (!unitTarget->IsInWorld() || unitTarget->GetTypeId() != TYPEID_UNIT)
        return;
    if (!m_caster || m_caster->GetTypeId() != TYPEID_PLAYER || !m_caster->IsInWorld())
        return;

    Creature* creature = (Creature*) unitTarget;
    int32 targetLevel = creature->GetLevel();

    ((Player*)m_caster)->SendLoot(creature->GetObjectGuid(), LOOT_SKINNING);
    creature->RemoveFlag(UNIT_FIELD_FLAGS, UNIT_FLAG_SKINNABLE);

    int32 reqValue = targetLevel < 10 ? 0 : targetLevel < 20 ? (targetLevel - 10) * 10 : targetLevel * 5;

    int32 skillValue = ((Player*)m_caster)->GetSkillValuePure(SKILL_SKINNING);

    // Double chances for elites
    ((Player*)m_caster)->UpdateGatherSkill(SKILL_SKINNING, skillValue, reqValue, creature->IsElite() ? 2 : 1);
}
```

**Key Behaviors**:
- Target must be a creature in world
- Only player casters can skin
- Sends skinning loot window to player
- Removes `UNIT_FLAG_SKINNABLE` flag (prevents multiple skins)
- Required skill calculation:
  - Level 1-9: 0 skill required
  - Level 10-19: `(level - 10) * 10`
  - Level 20+: `level * 5`
- Skill gain chance:
  - Normal mobs: standard chance
  - Elite mobs: double chance
- Skill gain via `UpdateGatherSkill()`
- Loot table determined by creature entry
- Corpse disappears after looting
- Other players cannot skin same corpse

---

### 116 - SPELL_EFFECT_SKIN_PLAYER_CORPSE
**Function**: `effect_skin_player_corpse()`

Removes insignia from player corpse (Battlegrounds).

**Parameters**: None (target is player corpse)

**Usage**:
- Battleground corpse looting
- "Remove Insignia" ability

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5785-5810`):
```cpp
void Spell::EffectSkinPlayerCorpse(SpellEffectIndex effIdx)
{
    sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "Effect: SkinPlayerCorpse");
    Player* playerCaster = m_caster->ToPlayer();
    if (!playerCaster)
        return;

    Unit* target = unitTarget;
    if (!target && corpseTarget)
        target = ObjectAccessor::FindPlayer(corpseTarget->GetOwnerGuid());
    if (!target)
    {
        ASSERT(corpseTarget);
        sObjectAccessor.ConvertCorpseForPlayer(corpseTarget->GetOwnerGuid(), playerCaster);
        playerCaster->SendLoot(corpseTarget->GetObjectGuid(), LOOT_INSIGNIA);
        sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "Effect SkinPlayerCorpse: corpse owner was not found");
        return;
    }

    if (target->GetTypeId() != TYPEID_PLAYER || target->IsAlive())
        return;

    ((Player*)target)->RemovedInsignia(playerCaster, corpseTarget);

    AddExecuteLogInfo(effIdx, ExecuteLogInfo(target->GetObjectGuid()));
}
```

**Key Behaviors**:
- Only works for player casters
- Can target unit or corpse directly
- If corpse has no owner (player offline):
  - Converts corpse for looting
  - Sends insignia loot window
- If player is online:
  - Calls `RemovedInsignia()` on target player
- Only works on dead players
- Used in battlegrounds for corpse looting
- Can loot money and items from corpse
- No honor gain from this action
- Target player gets "Your insignia has been removed" message

---

### 118 - SPELL_EFFECT_SKILL
**Function**: `effect_skill()`

Placeholder for skill-related effects.

**Parameters**: None

**Usage**:
- Placeholder effect

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5727-5730`):
```cpp
void Spell::EffectSkill(SpellEffectIndex /*effIdx*/)
{
    sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "WORLD: SkillEFFECT");
}
```

**Key Behaviors**:
- **NOT IMPLEMENTED** in MaNGOS (empty function)
- Logs debug message only
- Skill learning is handled through other systems:
  - Trainers use direct skill learning
  - Skill books use SPELL_EFFECT_SKILL_STEP
  - Quest rewards modify skills directly
- This effect type appears to be a placeholder or unused
- Real skill functionality handled by:
  - `Player::SetSkill()`
  - `Player::LearnSkill()`
  - Trainer gossip handlers

## Dependencies

Required systems:
- `SkillSystem` - For skill management
- `ProfessionSystem` - For crafting
- `LootSystem` - For skinning loot

## References

- MaNGOS: `SpellEffects.cpp` - Skill-related effects
- MaNGOS: `SkillHandler.cpp` - Skill management
- MaNGOS: `LootMgr.cpp` - Skinning loot
