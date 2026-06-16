# Summoning Effects Documentation

## File: `summon.rs`

## Overview

Handles all creature and object summoning effects including pets, guardians, totems, demons, and temporary summons.

## Effects (11 total)

### 28 - SPELL_EFFECT_SUMMON
**Function**: `effect_summon()`

Summons a creature at the target location. This is the primary summoning effect used for:
- Warlock pets (Imp, Voidwalker, Succubus, Felhunter)
- Mage elementals (Water Elemental)
- Temporary summons with duration

**Parameters**:
- `misc_value`: Creature entry ID from creature_template
- `base_value`: Duration in seconds (0 = permanent)

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2293-2391`):
```cpp
void Spell::EffectSummon(SpellEffectIndex effIdx)
{
    if (!m_casterUnit)
        return;

    if (!m_casterUnit->GetPetGuid().IsEmpty())
        return;

    if (!unitTarget)
        return;

    uint32 petEntry = m_spellInfo->EffectMiscValue[effIdx];
    if (!petEntry)
        return;

    CreatureInfo const* cInfo = sObjectMgr.GetCreatureTemplate(petEntry);
    if (!cInfo)
    {
        sLog.Out(LOG_DBERROR, LOG_LVL_MINIMAL, "Spell::DoSummon: creature entry %u not found for spell %u.", petEntry, m_spellInfo->Id);
        return;
    }

    Pet* spawnCreature = new Pet(SUMMON_PET);

    if (m_casterUnit->GetTypeId() == TYPEID_PLAYER && spawnCreature->LoadPetFromDB((Player*)m_casterUnit, petEntry))
    {
        // Summon in dest location
        if (m_targets.m_targetMask & TARGET_FLAG_DEST_LOCATION)
            spawnCreature->Relocate(m_targets.m_destX, m_targets.m_destY, m_targets.m_destZ, -m_casterUnit->GetOrientation());

        // set timer for unsummon
        if (m_duration > 0)
            spawnCreature->SetDuration(m_duration);

        return;
    }

    // Summon in dest location
    CreatureCreatePos pos(m_casterUnit->GetMap(), m_targets.m_destX, m_targets.m_destY, m_targets.m_destZ, -m_casterUnit->GetOrientation());

    if (!(m_targets.m_targetMask & TARGET_FLAG_DEST_LOCATION))
        pos = CreatureCreatePos(m_casterUnit, -m_casterUnit->GetOrientation());

    Map* map = m_casterUnit->GetMap();
    uint32 petNumber = sObjectMgr.GeneratePetNumber();
    if (!spawnCreature->Create(map->GenerateLocalLowGuid(HIGHGUID_PET), pos, cInfo, petNumber))
    {
        sLog.Out(LOG_DBERROR, LOG_LVL_MINIMAL, "Spell::EffectSummon: can't create creature with entry %u for spell %u", cInfo->entry, m_spellInfo->Id);
        delete spawnCreature;
        return;
    }
    spawnCreature->SetSummonPoint(pos);

    // set timer for unsummon
    if (m_duration > 0)
        spawnCreature->SetDuration(m_duration);

    spawnCreature->SetOwnerGuid(m_casterUnit->GetObjectGuid());
    spawnCreature->SetCreatorGuid(m_casterUnit->GetObjectGuid());
    spawnCreature->SetFactionTemplateId(m_casterUnit->GetFactionTemplateId());
    spawnCreature->SetUInt32Value(UNIT_FIELD_PET_NAME_TIMESTAMP, 0);
    spawnCreature->SetUInt32Value(UNIT_FIELD_PETEXPERIENCE, 0);
    spawnCreature->SetUInt32Value(UNIT_FIELD_PETNEXTLEVELEXP, 1000);
    spawnCreature->SetUInt32Value(UNIT_CREATED_BY_SPELL, m_spellInfo->Id);
    spawnCreature->SetUInt32Value(UNIT_NPC_FLAGS, UNIT_NPC_FLAG_NONE);
    spawnCreature->InitStatsForLevel(m_casterUnit->GetLevel(), m_casterUnit);
    spawnCreature->GetCharmInfo()->SetPetNumber(petNumber, false);

    if (m_casterUnit->GetTypeId() == TYPEID_PLAYER)
        spawnCreature->SetReactState(REACT_DEFENSIVE);
    else
        spawnCreature->SetReactState(REACT_AGGRESSIVE);

    spawnCreature->InitializeDefaultName();
    spawnCreature->AIM_Initialize();
    spawnCreature->InitPetCreateSpells();
    spawnCreature->SetHealth(spawnCreature->GetMaxHealth());
    spawnCreature->SetPower(POWER_MANA, spawnCreature->GetMaxPower(POWER_MANA));

    if (m_casterUnit->IsPvP())
        spawnCreature->SetPvP(true);

    map->Add((Creature*)spawnCreature);
    m_casterUnit->SetPet(spawnCreature);

    if (m_casterUnit->GetTypeId() == TYPEID_PLAYER)
    {
        spawnCreature->SavePetToDB(PET_SAVE_AS_CURRENT);
        ((Player*)m_casterUnit)->PetSpellInitialize();
    }

    if (m_casterUnit->IsCreature() && ((Creature*)m_casterUnit)->AI())
        ((Creature*)m_casterUnit)->AI()->JustSummoned((Creature*)spawnCreature);

    AddExecuteLogInfo(effIdx, ExecuteLogInfo(spawnCreature->GetObjectGuid()));

    if (m_spellScript)
        m_spellScript->OnSummon(this, spawnCreature);
}
```

**Key Behaviors**:
- First attempts to load existing pet from database (for players)
- Creates new `SUMMON_PET` type creature if not in DB
- Spawns at destination location if specified, otherwise at caster
- Sets owner and creator GUIDs to caster
- Sets faction to match caster
- Initializes pet stats for caster's level
- Sets react state: DEFENSIVE for players, AGGRESSIVE for creatures
- Initializes AI and pet spells
- Restores full health and mana
- Saves to database and initializes pet spell bar for players
- Notifies caster's AI of summon event

---

### 41 - SPELL_EFFECT_SUMMON_WILD
**Function**: `effect_summon_wild()`

Summons a wild creature that is NOT controlled by the caster.

**Parameters**:
- `misc_value`: Creature entry ID
- `base_value`: Duration in seconds

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2643-2730`):
```cpp
void Spell::EffectSummonWild(SpellEffectIndex effIdx)
{
    uint32 creature_entry = m_spellInfo->EffectMiscValue[effIdx];
    if (!creature_entry)
        return;

    uint32 level = m_caster->GetLevel();

    // level of creature summoned using engineering item based at engineering skill level
    if (m_caster->GetTypeId() == TYPEID_PLAYER && m_CastItem)
    {
        ItemPrototype const* proto = m_CastItem->GetProto();
        if (proto && proto->RequiredSkill == SKILL_ENGINEERING)
        {
            uint16 skill202 = ((Player*)m_caster)->GetSkillValue(SKILL_ENGINEERING);
            if (skill202)
                level = skill202 / 5;
        }
    }

    // select center of summon position
    float centerX = m_targets.m_destX;
    float centerY = m_targets.m_destY;
    float centerZ = m_targets.m_destZ;

    float radius = GetSpellRadius(sSpellRadiusStore.LookupEntry(m_spellInfo->EffectRadiusIndex[effIdx]));
    int32 duration = m_spellInfo->GetDuration();
    TempSummonType summonType = (duration == 0) ? TEMPSUMMON_DEAD_DESPAWN : TEMPSUMMON_TIMED_DEATH_AND_DEAD_DESPAWN;

    int32 amount = damage > 0 ? damage : 1;

    for (int32 count = 0; count < amount; ++count)
    {
        float px, py, pz;
        // If dest location if present
        if (m_targets.m_targetMask & TARGET_FLAG_DEST_LOCATION)
        {
            // Summon 1 unit in dest location
            if (count == 0)
            {
                px = m_targets.m_destX;
                py = m_targets.m_destY;
                pz = m_targets.m_destZ;
            }
            // Summon in random point all other units if location present
            else
                m_caster->GetRandomPoint(centerX, centerY, centerZ, radius, px, py, pz);
        }
        // Summon if dest location not present near caster
        else
        {
            if (radius > 0.0f)
            {
                // not using bounding radius of caster here
                m_caster->GetClosePoint(px, py, pz, 0.0f, radius);
            }
            else
            {
                // EffectRadiusIndex 0 or 36
                px = m_caster->GetPositionX();
                py = m_caster->GetPositionY();
                pz = m_caster->GetPositionZ();
            }
        }

        if (Creature* summon = m_caster->SummonCreature(creature_entry, px, py, pz, m_caster->GetOrientation(), summonType, duration))
        {
            summon->SetUInt32Value(UNIT_CREATED_BY_SPELL, m_spellInfo->Id);

            if (m_casterUnit && summon->HasStaticFlag(CREATURE_STATIC_FLAG_CREATOR_LOOT))
            {
                summon->lootForCreator = true;
                summon->SetCreatorGuid(m_casterUnit->GetObjectGuid());
                summon->SetLootRecipient(m_casterUnit);
            }

            // UNIT_FIELD_CREATEDBY are not set for these kind of spells.
            // Does exceptions exist? If so, what are they?
            // summon->SetCreatorGuid(m_caster->GetObjectGuid());

            if (count == 0)
                AddExecuteLogInfo(effIdx, ExecuteLogInfo(summon->GetObjectGuid()));

            if (m_spellScript)
                m_spellScript->OnSummon(this, summon);
        }
    }
}
```

**Key Behaviors**:
- For engineering items: creature level scales with engineering skill (skill/5)
- Supports summoning multiple creatures (from `damage` field)
- First creature at destination, others at random points within radius
- If no destination, summons near caster
- Summon type: `TEMPSUMMON_DEAD_DESPAWN` (permanent) or `TEMPSUMMON_TIMED_DEATH_AND_DEAD_DESPAWN` (timed)
- Sets `UNIT_CREATED_BY_SPELL` field
- Special handling for CREATOR_LOOT flag (loot only for creator)
- No owner assigned - creature uses default AI
- Typically used for environmental spawns

---

### 42 - SPELL_EFFECT_SUMMON_GUARDIAN
**Function**: `effect_summon_guardian()`

Summons a guardian creature that protects the caster.

**Parameters**:
- `misc_value`: Creature entry ID
- `base_value`: Duration in seconds

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2733-2872`):
```cpp
void Spell::EffectSummonGuardian(SpellEffectIndex effIdx)
{
    if (!m_casterUnit)
        return;

    uint32 petEntry = m_spellInfo->EffectMiscValue[effIdx];
    if (!petEntry)
        return;

    CreatureInfo const* cInfo = sObjectMgr.GetCreatureTemplate(petEntry);
    if (!cInfo)
    {
        sLog.Out(LOG_DBERROR, LOG_LVL_MINIMAL, "Spell::DoSummonGuardian: creature entry %u not found for spell %u.", petEntry, m_spellInfo->Id);
        return;
    }

    // second direct cast unsummon guardian(s) (guardians without like functionality have cooldown > spawn time)
    if (!m_IsTriggeredSpell && m_casterUnit->GetTypeId() == TYPEID_PLAYER)
    {
        bool found = false;
        // including protector
        while (Pet* oldSummon = m_casterUnit->FindGuardianWithEntry(petEntry))
        {
            oldSummon->Unsummon(PET_SAVE_AS_DELETED, m_casterUnit);
            found = true;
        }

        if (found && !(m_spellInfo->DurationIndex && m_spellInfo->Category))
            return;
    }

    // Hard cap for NPC summoned guardians
    if (m_casterUnit->GetTypeId() != TYPEID_PLAYER && m_casterUnit->GetGuardianCountWithEntry(petEntry) > 15)
        return;

    // Guardian pets use their creature template level by default
    uint32 level = urand(cInfo->level_min, cInfo->level_max);
    if (m_casterUnit->GetTypeId() != TYPEID_PLAYER)
    {
        // If EffectMultipleValue <= 0, guardian pets use their caster level modified by EffectMultipleValue for their own level
        if (m_spellInfo->EffectMultipleValue[effIdx] <= 0)
        {
            uint32 resultLevel = std::max(m_casterUnit->GetLevel() + m_spellInfo->EffectMultipleValue[effIdx], 0.0f);

            // Result level should be a valid level for creatures
            if (resultLevel > 0 && resultLevel <= CREATURE_MAX_LEVEL)
                level = resultLevel;
        }
    }
    // level of pet summoned using engineering trinket scales with engineering skill level
    else if (m_CastItem)
    {
        ItemPrototype const* proto = m_CastItem->GetProto();
        if (proto && proto->RequiredSkill == SKILL_ENGINEERING && proto->InventoryType == INVTYPE_TRINKET)
        {
            uint16 engiLevel = ((Player*)m_casterUnit)->GetSkillValue(SKILL_ENGINEERING);
            if (engiLevel)
                level = engiLevel / 5;
        }
    }

    // select center of summon position
    float centerX = m_targets.m_destX;
    float centerY = m_targets.m_destY;
    float centerZ = m_targets.m_destZ;

    float radius = GetSpellRadius(sSpellRadiusStore.LookupEntry(m_spellInfo->EffectRadiusIndex[effIdx]));

    int32 amount = damage > 0 ? damage : 1;

    for (int32 count = 0; count < amount; ++count)
    {
        Pet* spawnCreature = new Pet(GUARDIAN_PET);

        // If dest location if present
        // Summon 1 unit in dest location
        CreatureCreatePos pos(m_casterUnit->GetMap(), m_targets.m_destX, m_targets.m_destY, m_targets.m_destZ, -m_casterUnit->GetOrientation());

        if (m_targets.m_targetMask & TARGET_FLAG_DEST_LOCATION)
        {
            // Summon in random point all other units if location present
            if (count > 0)
            {
                float x, y, z;
                m_casterUnit->GetRandomPoint(centerX, centerY, centerZ, radius, x, y, z);
                pos = CreatureCreatePos(m_casterUnit->GetMap(), x, y, z, m_casterUnit->GetOrientation());
            }
        }
        // Summon if dest location not present near caster
        else
            pos = CreatureCreatePos(m_casterUnit, m_casterUnit->GetOrientation());

        Map* map = m_casterUnit->GetMap();
        uint32 petNumber = sObjectMgr.GeneratePetNumber();
        if (!spawnCreature->Create(map->GenerateLocalLowGuid(HIGHGUID_PET), pos, cInfo, petNumber))
        {
            sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "Spell::DoSummonGuardian: can't create creature entry %u for spell %u.", petEntry, m_spellInfo->Id);
            delete spawnCreature;
            return;
        }
        spawnCreature->SetSummonPoint(pos);

        if (m_duration > 0)
            spawnCreature->SetDuration(m_duration);

        spawnCreature->SetOwnerGuid(m_casterUnit->GetObjectGuid());
        spawnCreature->SetCreatorGuid(m_casterUnit->GetObjectGuid());
        spawnCreature->SetFactionTemplateId(m_casterUnit->GetFactionTemplateId());
        spawnCreature->SetUInt32Value(UNIT_FIELD_PET_NAME_TIMESTAMP, 0);
        spawnCreature->SetUInt32Value(UNIT_CREATED_BY_SPELL, m_spellInfo->Id);
        spawnCreature->SetUInt32Value(UNIT_NPC_FLAGS, spawnCreature->GetCreatureInfo()->npc_flags);
        spawnCreature->InitStatsForLevel(level, m_casterUnit);
        spawnCreature->GetCharmInfo()->SetPetNumber(petNumber, false);

        if (uint32 totalGuardians = m_casterUnit->GetGuardiansCount() + (m_casterUnit->GetPetGuid().IsEmpty() ? 0 : 1))
        {
            float followAngle = PET_FOLLOW_ANGLE + (M_PI_F / 6) * totalGuardians;
            while (followAngle > M_PI_F * 2)
                followAngle -= M_PI_F * 2;
            spawnCreature->SetFollowAngle(followAngle);
        }

        spawnCreature->InitializeDefaultName();
        spawnCreature->AIM_Initialize();
        spawnCreature->LoadCreatureAddon();

        map->Add((Creature*)spawnCreature);
        m_casterUnit->AddGuardian(spawnCreature);

        // Notify Summoner
        if (m_casterUnit->IsCreature() && ((Creature*)m_casterUnit)->AI())
            ((Creature*)m_casterUnit)->AI()->JustSummoned(spawnCreature);

        if (count == 0)
            AddExecuteLogInfo(effIdx, ExecuteLogInfo(spawnCreature->GetObjectGuid()));

        if (m_spellScript)
            m_spellScript->OnSummon(this, spawnCreature);
    }
}
```

**Key Behaviors**:
- Second cast unsummons existing guardians of same entry (for players)
- Hard cap of 15 guardians for NPCs
- Level calculation:
  - Default: random between creature template min/max
  - NPC casters: caster level + EffectMultipleValue
  - Engineering trinkets: engineering skill / 5
- Supports multiple guardians (from `damage` field)
- Each guardian gets unique follow angle (PET_FOLLOW_ANGLE + increments)
- Creates `GUARDIAN_PET` type creature
- Adds to caster's guardian list (not pet slot)
- Loads creature addon data for visual customization

---

### 55 - SPELL_EFFECT_TAME_CREATURE
**Function**: `effect_tame_creature()`

Attempts to tame a beast creature (Hunter pet taming).

**Parameters**:
- Target must be a valid beast
- Target must not already be owned

**Implementation Details** (from MaNGOS `SpellEffects.cpp:3064-3124`):
```cpp
void Spell::EffectTameCreature(SpellEffectIndex /*effIdx*/)
{
    // Caster must be player, checked in Spell::CheckCast
    // Spell can be triggered, we need to check original caster prior to caster
    Player* plr = (Player*)GetAffectiveCaster();

    Creature* creatureTarget = (Creature*)unitTarget;

    // cast finish successfully
    //SendChannelUpdate(0);
    finish();

    Pet* pet = new Pet(HUNTER_PET);

    if (!pet->CreateBaseAtCreature(creatureTarget))
    {
        delete pet;
        return;
    }

    pet->SetOwnerGuid(plr->GetObjectGuid());
    pet->SetCreatorGuid(plr->GetObjectGuid());
    pet->SetFactionTemplateId(plr->GetFactionTemplateId());
    pet->SetUInt32Value(UNIT_CREATED_BY_SPELL, m_spellInfo->Id);

    if (!pet->InitStatsForLevel(creatureTarget->GetLevel()))
    {
        sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "Pet::InitStatsForLevel() failed for creature (Entry: %u)!", creatureTarget->GetEntry());
        delete pet;
        return;
    }

    pet->GetCharmInfo()->SetPetNumber(pet->GetObjectGuid().GetEntry(), true);
    pet->GetCharmInfo()->SetReactState(REACT_DEFENSIVE);
    pet->InitializeDefaultName();
    pet->AIM_Initialize();
    pet->InitPetCreateSpells();
    pet->SetHealth(pet->GetMaxHealth());

    // "kill" original creature
    creatureTarget->ForcedDespawn();

    // prepare visual effect for levelup
    pet->SetUInt32Value(UNIT_FIELD_LEVEL, creatureTarget->GetLevel() - 1);

    // Apply default loyalty at summon
    LoyaltyLevel defaultLoyalty = LoyaltyLevel(sWorld.getConfig(CONFIG_UINT32_PET_DEFAULT_LOYALTY));
    while (pet->GetLoyaltyLevel() != defaultLoyalty)
        pet->ModifyLoyalty(pet->GetStartLoyaltyPoints(defaultLoyalty));

    if (plr->IsPvP())
        pet->SetPvP(true);

    // add to world
    pet->GetMap()->Add((Creature*)pet);

    // visual effect for levelup
    pet->SetUInt32Value(UNIT_FIELD_LEVEL, creatureTarget->GetLevel());

    // caster have pet now
    plr->SetPet(pet);
}
```

**Key Behaviors**:
- Creates `HUNTER_PET` type creature
- Copies base stats from target creature
- Sets owner and creator to player
- Initializes stats for creature's level
- Sets pet number from creature entry
- "Kills" original creature via `ForcedDespawn()`
- Visual level-up effect (sets level to target-1, then to target level)
- Applies default loyalty level from config
- Copies PvP flag from player
- Original creature is removed, new pet is added to world

---

### 56 - SPELL_EFFECT_SUMMON_PET
**Function**: `effect_summon_pet()`

Summons the caster's currently active pet.

**Parameters**: None

**Implementation Details** (from MaNGOS `SpellEffects.cpp:3129-3140`):
```cpp
void Spell::EffectSummonPet(SpellEffectIndex effIdx)
{
    if (!m_casterUnit)
        return;

    uint32 petLevel = m_casterUnit->IsPlayer() ? m_casterUnit->GetLevel() : std::max(int32(m_casterUnit->GetLevel()) + int32(m_spellInfo->EffectMultipleValue[effIdx]), 1);

    ObjectGuid petGuid = m_casterUnit->EffectSummonPet(m_spellInfo->Id, m_spellInfo->EffectMiscValue[effIdx], petLevel);
    if (petGuid)
        AddExecuteLogInfo(effIdx, ExecuteLogInfo(petGuid));
}
```

**Key Behaviors**:
- Pet level calculation:
  - Players: caster's level
  - Creatures: caster level + EffectMultipleValue (minimum 1)
- Calls `EffectSummonPet()` which handles:
  - Unsummoning old pet if needed
  - Loading pet from database if exists
  - Creating new pet if entry specified
- Used by Call Pet spell and other pet summons
- Pet appears at caster location with full health/mana
- Removes Demonic Sacrifice auras if present

---

### 73 - SPELL_EFFECT_SUMMON_POSSESSED
**Function**: `effect_summon_possessed()`

Summons a possessed minion that the caster controls directly.

**Parameters**:
- `misc_value`: Creature entry ID
- `base_value`: Duration in seconds

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2874-2895`):
```cpp
void Spell::EffectSummonPossessed(SpellEffectIndex effIdx)
{
    Player* pCaster = m_caster->ToPlayer();
    if (!pCaster)
        return;

    uint32 creatureEntry = m_spellInfo->EffectMiscValue[effIdx];

    Creature* pMinion = pCaster->SummonPossessedMinion(creatureEntry, m_spellInfo->Id, m_targets.m_destX, m_targets.m_destY, m_targets.m_destZ, m_caster->GetOrientation(), m_spellInfo->GetDuration());
    if (!pMinion)
    {
        sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "Spell::EffectSummonPossessed: creature entry %u for spell %u could not be summoned.", creatureEntry, m_spellInfo->Id);
        return;
    }

    // Notify Summoner
    if (m_originalCaster && m_originalCaster != m_caster && m_originalCaster->AI())
        m_originalCaster->AI()->JustSummoned(pMinion);

    if (m_spellScript)
        m_spellScript->OnSummon(this, pMinion);
}
```

**Key Behaviors**:
- Only works for player casters
- Creates a possessed minion at destination location
- Caster gains direct control of the summoned creature
- Creature uses caster's action bar
- Duration controlled by spell duration
- Notifies original caster's AI of summon event
- Used for temporary minion control (not Mind Control spell)
- Different from SPELL_EFFECT_SUMMON_GUARDIAN - this is direct possession

---

### 74, 87-90 - SPELL_EFFECT_SUMMON_TOTEM (and SLOT variants)
**Function**: `effect_summon_totem()` - All totem summons use same function

Summons a totem at the caster's location (Shaman).

**Parameters**:
- `misc_value`: Totem creature entry ID
- `base_value`: Duration in seconds

**Implementation Details** (from MaNGOS `SpellEffects.cpp:4861-4945`):
```cpp
void Spell::EffectSummonTotem(SpellEffectIndex effIdx)
{
    if (!m_casterUnit)
        return;

    int slot;
    switch (m_spellInfo->Effect[effIdx])
    {
        case SPELL_EFFECT_SUMMON_TOTEM:
            slot = TOTEM_SLOT_NONE;
            break;
        case SPELL_EFFECT_SUMMON_TOTEM_SLOT1:
            slot = TOTEM_SLOT_FIRE;
            break;
        case SPELL_EFFECT_SUMMON_TOTEM_SLOT2:
            slot = TOTEM_SLOT_EARTH;
            break;
        case SPELL_EFFECT_SUMMON_TOTEM_SLOT3:
            slot = TOTEM_SLOT_WATER;
            break;
        case SPELL_EFFECT_SUMMON_TOTEM_SLOT4:
            slot = TOTEM_SLOT_AIR;
            break;
        default:
            return;
    }

    // unsummon old totem
    if (slot < MAX_TOTEM_SLOT)
        if (Totem *OldTotem = m_casterUnit->GetTotem(TotemSlot(slot)))
            OldTotem->UnSummon();

    // FIXME: Setup near to finish point because GetObjectBoundingRadius set in Create but some Create calls can be dependent from proper position
    // if totem have creature_template_addon.auras with persistent point for example or script call
    float angle = slot < MAX_TOTEM_SLOT ? M_PI_F / MAX_TOTEM_SLOT - (slot * 2 * M_PI_F / MAX_TOTEM_SLOT) : 0;

    CreatureCreatePos pos(m_casterUnit, m_casterUnit->GetOrientation(), 2.0f, angle);

    CreatureInfo const* cinfo = sObjectMgr.GetCreatureTemplate(m_spellInfo->EffectMiscValue[effIdx]);
    if (!cinfo)
    {
        sLog.Out(LOG_DBERROR, LOG_LVL_MINIMAL, "Creature entry %u does not exist but used in spell %u totem summon.", m_spellInfo->EffectMiscValue[effIdx], m_spellInfo->Id);
        return;
    }

    Totem* pTotem = new Totem;

    if (!pTotem->Create(m_casterUnit->GetMap()->GenerateLocalLowGuid(HIGHGUID_UNIT), pos, cinfo, m_casterUnit))
    {
        delete pTotem;
        return;
    }

    pTotem->SetSummonPoint(pos);

    if (slot < MAX_TOTEM_SLOT)
        m_casterUnit->_AddTotem(TotemSlot(slot), pTotem);

    //pTotem->SetName("");                                  // generated by client
    pTotem->SetOwner(m_casterUnit);
    pTotem->SetTypeBySummonSpell(m_spellInfo);              // must be after Create call where m_spells initialized

    pTotem->SetDuration(m_duration);

    if (damage)                                             // if not spell info, DB values used
    {
        pTotem->SetMaxHealth(damage);
        pTotem->SetHealth(damage);
    }

    pTotem->SetUInt32Value(UNIT_CREATED_BY_SPELL, m_spellInfo->Id);

    if (m_casterUnit->IsPlayer())
        pTotem->SetFlag(UNIT_FIELD_FLAGS, UNIT_FLAG_PLAYER_CONTROLLED);

    if (m_casterUnit->IsPvP())
        pTotem->SetPvP(true);

    pTotem->Summon(m_casterUnit);

    AddExecuteLogInfo(effIdx, ExecuteLogInfo(pTotem->GetObjectGuid()));

    if (m_spellScript)
        m_spellScript->OnSummon(this, pTotem);
}
```

**Key Behaviors**:
- Slot determination:
  - SLOT1 = Fire totem
  - SLOT2 = Earth totem
  - SLOT3 = Water totem
  - SLOT4 = Air totem
  - No slot = generic totem
- Despawns existing totem in the same slot before summoning
- Totem spawns 2 yards from caster at specific angle based on slot
- Health can be set from `damage` field (otherwise uses DB values)
- Sets owner and type based on summon spell
- Totem automatically casts its summon spell effects
- Sets `UNIT_FLAG_PLAYER_CONTROLLED` if caster is player
- Copies PvP flag from caster
- Max 1 totem per element type (4 total for Shaman)

---

### 97 - SPELL_EFFECT_SUMMON_CRITTER
**Function**: `effect_summon_critter()`

Summons a vanity/non-combat pet.

**Parameters**:
- `misc_value`: Critter creature entry ID

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5340-5400`):
```cpp
void Spell::EffectSummonCritter(SpellEffectIndex effIdx)
{
    Player* player = m_caster->ToPlayer();
    if (!player)
        return;

    uint32 petEntry = m_spellInfo->EffectMiscValue[effIdx];
    if (!petEntry)
        return;

    CreatureInfo const* cInfo = sObjectMgr.GetCreatureTemplate(petEntry);
    if (!cInfo)
    {
        sLog.Out(LOG_DBERROR, LOG_LVL_MINIMAL, "Spell::DoSummonCritter: creature entry %u not found for spell %u.", petEntry, m_spellInfo->Id);
        return;
    }

    Pet* oldCritter = player->GetMiniPet();

    // for same pet just despawn
    if (oldCritter && oldCritter->GetEntry() == petEntry)
    {
        player->RemoveMiniPet();
        return;
    }

    // despawn old pet before summon new
    if (oldCritter)
        player->RemoveMiniPet();

    // summon new pet
    Pet* critter = new Pet(MINI_PET);

    CreatureCreatePos pos(m_caster, m_caster->GetOrientation(), PET_FOLLOW_DIST, MINI_PET_SUMMON_ANGLE);
    if (!(m_targets.m_targetMask & TARGET_FLAG_DEST_LOCATION))
        pos = CreatureCreatePos(m_caster, m_caster->GetOrientation());

    Map* map = m_caster->GetMap();
    uint32 petNumber = sObjectMgr.GeneratePetNumber();
    if (!critter->Create(map->GenerateLocalLowGuid(HIGHGUID_PET), pos, cInfo, petNumber))
    {
        sLog.Out(LOG_BASIC, LOG_LVL_ERROR, "Spell::EffectSummonCritter, spellid %u: no such creature entry %u", m_spellInfo->Id, petEntry);
        delete critter;
        return;
    }
    critter->SetSummonPoint(pos);

    if (m_duration > 0)
        critter->SetDuration(m_duration);

    critter->SetOwnerGuid(m_caster->GetObjectGuid());
    critter->SetCreatorGuid(m_caster->GetObjectGuid());
    critter->SetFactionTemplateId(m_caster->GetFactionTemplateId());
    critter->SetUInt32Value(UNIT_CREATED_BY_SPELL, m_spellInfo->Id);
    critter->SetUInt32Value(UNIT_NPC_FLAGS, critter->GetCreatureInfo()->npc_flags); // some mini-pets have quests
    critter->InitializeDefaultName();
    critter->AIM_Initialize();
    critter->InitPetCreateSpells();                         // e.g. disgusting oozeling has a create spell as critter...
    critter->SelectLevel();                                 // some summoned creatures have different from 1 DB data for level/hp

    map->Add((Creature*)critter);
```

**Key Behaviors**:
- Creates `MINI_PET` type creature (cosmetic only)
- Only works for player casters
- Clicking same pet entry again despawns it (toggle behavior)
- Despawns any existing mini-pet before summoning new one
- Spawns at `PET_FOLLOW_DIST` with `MINI_PET_SUMMON_ANGLE`
- Can have duration if specified in spell
- Some mini-pets have NPC flags (can have quests)
- Initializes pet create spells (some critters have special abilities)
- Uses `SelectLevel()` for creatures with non-standard level data
- Purely cosmetic - doesn't fight or assist in combat

---

### 109 - SPELL_EFFECT_SUMMON_DEAD_PET
**Function**: `effect_summon_dead_pet()`

Resurrects and summons the caster's dead pet.

**Parameters**: None

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5473-5500`):
```cpp
void Spell::EffectSummonDeadPet(SpellEffectIndex /*effIdx*/)
{
    Player* player = m_caster->ToPlayer();
    if (!player)
        return;

    Pet* pet = player->GetPet();
    if (!pet)
        return;
    if (pet->IsAlive())
        return;

    if (damage < 0)
        return;

    // Chakor : Teleport the pet to the player's location
    pet->NearTeleportTo(player->GetPosition(), false);
    pet->SetUInt32Value(UNIT_DYNAMIC_FLAGS, UNIT_DYNFLAG_NONE);
    pet->RemoveFlag(UNIT_FIELD_FLAGS, UNIT_FLAG_SKINNABLE);
    pet->SetDeathState(ALIVE);
    pet->ClearUnitState(UNIT_STATE_ALL_DYN_STATES);
    pet->SetHealth(uint32(pet->GetMaxHealth() * (damage / 100)));

    pet->AIM_Initialize();

    // player->PetSpellInitialize(); -- action bar not removed at death and not required send at revive
    pet->SavePetToDB(PET_SAVE_AS_CURRENT);
}
```

**Key Behaviors**:
- Only works for player casters
- Requires pet to exist and be dead
- Teleports pet to player's location
- Clears death-related flags and states
- Sets death state to ALIVE
- Restores health based on `damage` percentage (damage/100 * max health)
- Reinitializes AI
- Saves pet to database as current pet
- Does NOT reinitialize spell bar (preserved from before death)
- Used by Revive Pet and similar resurrection spells

---

### 112 - SPELL_EFFECT_SUMMON_DEMON
**Function**: `effect_summon_demon()`

Summons a warlock demon pet.

**Parameters**:
- `misc_value`: Demon creature entry ID

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5732-5755`):
```cpp
void Spell::EffectSummonDemon(SpellEffectIndex effIdx)
{
    float px = m_targets.m_destX;
    float py = m_targets.m_destY;
    float pz = m_targets.m_destZ;

    // summon to the ritual object location if any
    if (GameObject* pGo = m_targets.getGOTarget())
        if (pGo->GetGoType() == GAMEOBJECT_TYPE_SUMMONING_RITUAL)
            pGo->GetPosition(px, py, pz);

    uint32 const summonDuration = m_duration > 0 ? m_duration : 3600000;
    Creature* pSummon = m_caster->SummonCreature(m_spellInfo->EffectMiscValue[effIdx], px, py, pz, m_caster->GetOrientation(), TEMPSUMMON_TIMED_COMBAT_OR_DEAD_DESPAWN, summonDuration);
    if (!pSummon)
        return;

    // might not always work correctly, maybe the creature that dies from CoD casts the effect on itself and is therefore the caster?
    pSummon->SetLevel(m_caster->GetLevel());

    AddExecuteLogInfo(effIdx, ExecuteLogInfo(pSummon->Summon->GetObjectGuid()));

    if (m_spellScript)
        m_spellScript->OnSummon(this, pSummon);
}
```

**Key Behaviors**:
- Summons at destination location or ritual gameobject location
- If summoned via GAMEOBJECT_TYPE_SUMMONING_RITUAL, uses object position
- Default duration: 1 hour (3600000ms) if no spell duration specified
- Summon type: `TEMPSUMMON_TIMED_COMBAT_OR_DEAD_DESPAWN`
- Sets demon level to match caster's level
- Used for warlock demon summons (Imp, Voidwalker, Succubus, Felhunter, Doomguard)
- Soul shard consumption handled by spell reagents, not this effect
- Supports ritual summoning (meeting stones, warlock rituals)

## Dependencies

Required systems:
- `CreatureSystem` - For spawning creatures
- `PetSystem` - For pet management
- `TotemSystem` - For totem management

## References

- MaNGOS: `SpellEffects.cpp` - `EffectSummon()`, `EffectSummonWild()`, etc.
- MaNGOS: `Pet.cpp` - Pet management
- MaNGOS: `Totem.cpp` - Totem implementation
