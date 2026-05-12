# Quest and Reputation Effects Documentation

## File: `quest.rs`

## Overview

Handles quest completion, honor gains, and reputation changes.

## Effects (3 total)

### 16 - SPELL_EFFECT_QUEST_COMPLETE
**Function**: `effect_quest_complete()`

Completes a quest for the target.

**Parameters**:
- `misc_value`: Quest ID

**Usage**:
- Quest reward spells
- Automatic quest completion
- Event-based quest completion

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5265-5272`):
```cpp
void Spell::EffectQuestComplete(SpellEffectIndex effIdx)
{
    if (!unitTarget || unitTarget->GetTypeId() != TYPEID_PLAYER)
        return;

    uint32 questId = m_spellInfo->EffectMiscValue[effIdx];
    ((Player*)unitTarget)->AreaExploredOrEventHappens(questId);
}
```

**Key Behaviors**:
- Only works for player targets
- Quest ID from `EffectMiscValue[effIdx]`
- Calls `AreaExploredOrEventHappens()` which:
  - Checks if player has quest in log
  - Marks quest objective as complete
  - If all objectives complete, marks quest ready for turn-in
  - Does NOT auto-complete the quest (player must still turn in)
- Used for:
  - Area exploration objectives
  - Event-triggered quest progress
  - Scripted quest completion
- Quest rewards given when player turns in at quest giver
- Cannot complete if quest not in player's log

---

### 45 - SPELL_EFFECT_ADD_HONOR
**Function**: `effect_add_honor()`

Adds honor points to the target.

**Parameters**:
- `base_value`: Honor points to add

**Usage**:
- Battleground completion rewards
- PvP kill honor
- Honor bonus spells

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2937-2946`):
```cpp
void Spell::EffectAddHonor(SpellEffectIndex /*effIdx*/)
{
    if (unitTarget->GetTypeId() != TYPEID_PLAYER)
        return;

    // honor-spells don't scale with level and won't be casted by an item
    // also we must use damage (spelldescription says +25 honor but damage is only 24)
    ((Player*)unitTarget)->GetHonorMgr().Add(damage, QUEST);
    DEBUG_FILTER_LOG(LOG_FILTER_SPELL_CAST, "SpellEffect::AddHonor (spellId %u) rewards %u honor points (non scale) for player: %u", m_spellInfo->Id, damage, ((Player*)unitTarget)->GetGUIDLow());
}
```

**Key Behaviors**:
- Only works for player targets
- Honor amount from `damage` field
- Note: Spell description may say +25 but damage field is 24 (as comment notes)
- Calls `HonorMgr::Add()` with source type QUEST
- Honor is not scaled by level
- Cannot be cast by items
- Shows honor gain in chat
- Contributes to rank progression
- Subject to weekly honor decay
- Used for battleground rewards and special honor bonuses

---

### 103 - SPELL_EFFECT_REPUTATION
**Function**: `effect_reputation()`

Modifies reputation with a faction.

**Parameters**:
- `misc_value`: Faction ID
- `base_value`: Reputation amount (positive or negative)

**Usage**:
- Quest reputation rewards
- Kill reputation gains
- Special reputation spells

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5247-5263`):
```cpp
void Spell::EffectReputation(SpellEffectIndex effIdx)
{
    Player* player = ToPlayer(unitTarget);
    if (!player)
        return;

    int32  repChange = m_currentBasePoints[effIdx];
    uint32 factionId = m_spellInfo->EffectMiscValue[effIdx];

    FactionEntry const* factionEntry = sObjectMgr.GetFactionEntry(factionId);
    if (!factionEntry)
        return;

    repChange = player->CalculateReputationGain(REPUTATION_SOURCE_SPELL, repChange, factionId);

    player->GetReputationMgr().ModifyReputation(factionEntry, repChange);
}
```

**Key Behaviors**:
- Only works for player targets
- Reputation change from `m_currentBasePoints[effIdx]`
- Faction ID from `EffectMiscValue[effIdx]`
- Validates faction exists in database
- Calculates reputation gain via `CalculateReputationGain()`:
  - Applies human racial bonus (+10%)
  - Applies other reputation modifiers
- Modifies reputation via `ReputationMgr::ModifyReputation()`
- Shows reputation change in chat
- Can be positive or negative
- Subject to reputation caps per standing
- May trigger reputation-based events
- Used for quest rewards and special reputation spells

## Dependencies

Required systems:
- `QuestSystem` - For quest completion
- `PvPSystem` - For honor management
- `ReputationSystem` - For faction reputation

## References

- MaNGOS: `SpellEffects.cpp` - Quest and reputation effects
- MaNGOS: `QuestHandler.cpp` - Quest completion
- MaNGOS: `HonorMgr.cpp` - Honor management
