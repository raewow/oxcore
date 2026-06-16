# Combat Mechanics Effects Documentation

## File: `combat.rs`

## Overview

Handles combat-related effects including threat management, combo points, interrupts, and defense abilities.

## Effects (12 total)

### 19 - SPELL_EFFECT_ADD_EXTRA_ATTACKS
**Function**: `effect_add_extra_attacks()`

Adds extra melee attacks to the caster.

**Parameters**:
- `base_value`: Number of extra attacks

**Usage**:
- Windfury Weapon (Shaman)
- Sword Specialization (Warrior)
- Various proc effects

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5193-5218`):
```cpp
void Spell::EffectAddExtraAttacks(SpellEffectIndex effIdx)
{
    if (!unitTarget)
        return;

    if (!unitTarget->IsAlive() || unitTarget->IsExtraAttacksLocked())
        return;

    if (unitTarget->GetExtraAttacks())
        return;

    // World of Warcraft Client Patch 1.3.0 (2005-03-22)
    // - Fixed a bug where abilities that give extra attacks, like the paladin
    //   Reckoning talent, could cause the following swing to take longer than
    //   it should.
#if SUPPORTED_CLIENT_BUILD <= CLIENT_BUILD_1_2_4
    unitTarget->ResetAttackTimer();
#endif

    unitTarget->AddExtraAttackOnUpdate();
    unitTarget->SetExtraAttaks(damage);

    ExecuteLogInfo info(unitTarget->GetObjectGuid());
    info.extraAttacks.count = damage;
    AddExecuteLogInfo(effIdx, info);
}
```

**Key Behaviors**:
- Target must be alive
- Cannot add extra attacks if `IsExtraAttacksLocked()` returns true
- Cannot add if target already has extra attacks queued
- Pre-1.3.0: Reset attack timer to prevent swing timer issues
- Calls `AddExtraAttackOnUpdate()` to flag for next update
- Sets extra attack count via `SetExtraAttaks(damage)`
- Attacks occur on next melee swing update
- Can trigger weapon procs and other on-hit effects
- Respects weapon damage and can crit independently

---

### 20, 22, 23 - Defense Abilities (Dodge, Parry, Block)
**Functions**: Simple flag setters for player abilities

These effects enable basic defense abilities for players.

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5220-5230`):
```cpp
void Spell::EffectParry(SpellEffectIndex /*effIdx*/)
{
    if (unitTarget && unitTarget->GetTypeId() == TYPEID_PLAYER)
        ((Player*)unitTarget)->SetCanParry(true);
}

void Spell::EffectBlock(SpellEffectIndex /*effIdx*/)
{
    if (unitTarget && unitTarget->GetTypeId() == TYPEID_PLAYER)
        ((Player*)unitTarget)->SetCanBlock(true);
}
```

**Effect IDs**:
| ID | Name | Function |
|----|------|----------|
| 20 | SPELL_EFFECT_DODGE | Enables dodge (handled by auras, this is a marker) |
| 22 | SPELL_EFFECT_PARRY | `SetCanParry(true)` for players |
| 23 | SPELL_EFFECT_BLOCK | `SetCanBlock(true)` for players |

**Key Behaviors**:
- Only works for player targets
- Sets internal flags enabling the abilities
- Actual dodge/parry/block chance calculated from:
  - Base class values
  - Stats (agility for dodge, defense skill)
  - Talents and gear bonuses
- Block requires shield equipped to function
- These effects are typically passive and learned once
- Most defense abilities are handled by auras rather than these effects

---

### 40 - SPELL_EFFECT_DUAL_WIELD
**Function**: `effect_dual_wield()`

Enables dual wielding.

**Parameters**: None

**Usage**:
- Dual Wield skill (Warrior/Rogue)
- Shaman Dual Wield (talent)

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2578-2582`):
```cpp
void Spell::EffectDualWield(SpellEffectIndex /*effIdx*/)
{
    if (unitTarget && unitTarget->GetTypeId() == TYPEID_PLAYER)
        ((Player*)unitTarget)->SetCanDualWield(true);
}
```

**Key Behaviors**:
- Only works for player targets
- Sets `SetCanDualWield(true)` flag
- Allows equipping off-hand weapon
- Off-hand damage penalty (50% base, reduced by talents)
- Learned once and permanent
- Required for:
  - Equipping weapons in off-hand slot
  - Dual wield specialization talents
  - Off-hand attacks and abilities

---

### 63 - SPELL_EFFECT_THREAT
**Function**: `effect_threat()`

Modifies threat on the target.

**Parameters**:
- `base_value`: Threat amount (positive or negative)

**Usage**:
- Threat-generating abilities
- Threat-reducing abilities (Fade, etc.)

**Implementation Details** (from MaNGOS `SpellEffects.cpp:3439-3450`):
```cpp
void Spell::EffectThreat(SpellEffectIndex effIdx)
{
    if (!unitTarget || !unitTarget->IsAlive() || !m_casterUnit || !m_casterUnit->IsAlive())
        return;

    if (!unitTarget->CanHaveThreatList())
        return;

    unitTarget->AddThreat(m_casterUnit, damage, false, m_spellInfo->GetSpellSchoolMask(), m_spellInfo);

    AddExecuteLogInfo(effIdx, ExecuteLogInfo(unitTarget->GetObjectGuid()));
}
```

**Key Behaviors**:
- Both caster and target must be alive
- Target must have a threat list (creatures/players with threat)
- Adds flat threat amount via `AddThreat()`
- `damage` field determines threat amount (can be negative)
- School mask passed for threat calculation modifiers
- Affects threat table immediately
- Used by tank threat abilities (Heroic Strike, etc.)
- Also used by threat reduction abilities (negative values)

---

### 68 - SPELL_EFFECT_INTERRUPT_CAST
**Function**: `effect_interrupt_cast()`

Interrupts the target's spell cast.

**Parameters**:
- `misc_value`: School to lock (0 = interrupted spell's school)
- `base_value`: Lockout duration in milliseconds

**Usage**:
- Kick (Rogue)
- Counterspell (Mage)
- Shield Bash (Warrior)

**Implementation Details** (from MaNGOS `SpellEffects.cpp:3496-3530`):
```cpp
void Spell::EffectInterruptCast(SpellEffectIndex effIdx)
{
    if (!unitTarget)
        return;
    if (!unitTarget->IsAlive())
        return;

    // TODO: not all spells that used this effect apply cooldown at school spells
    // also exist case: apply cooldown to interrupted cast only and to all spells
    for (uint32 i = CURRENT_FIRST_NON_MELEE_SPELL; i < CURRENT_MAX_SPELL; ++i)
    {
        if (Spell* spell = unitTarget->GetCurrentSpell(CurrentSpellTypes(i)))
        {
            // Nostalrius: fix CS of instant spells (with CC Delay)
            if (i != CURRENT_CHANNELED_SPELL && !spell->GetCastTime())
                continue;

            SpellEntry const* curSpellInfo = spell->m_spellInfo;
            // check if we can interrupt spell
            if ((spell->getState() == SPELL_STATE_CASTING
                || (spell->getState() == SPELL_STATE_PREPARING && spell->GetCastTime() > 0.0f))
                && curSpellInfo->PreventionType == SPELL_PREVENTION_TYPE_SILENCE
                && ((i == CURRENT_GENERIC_SPELL && curSpellInfo->HasSpellInterruptFlag(SPELL_INTERRUPT_FLAG_DAMAGE_PUSHBACK))
                || (i == CURRENT_CHANNELED_SPELL && curSpellInfo->HasChannelInterruptFlag(AURA_INTERRUPT_ACTION_CANCELS))))
            {
                unitTarget->LockOutSpells(curSpellInfo->GetSpellSchoolMask(), m_spellInfo->GetDuration());
                unitTarget->InterruptSpell(CurrentSpellTypes(i), false);

                ExecuteLogInfo info(unitTarget->GetObjectGuid());
                info.interruptCast.spellId = curSpellInfo->Id;
                AddExecuteLogInfo(effIdx, info);
            }
        }
    }
}
```

**Key Behaviors**:
- Checks all current spell types (generic, channeled, etc.)
- Skips instant spells (except channeled)
- Can interrupt if:
  - Spell is in CASTING state, OR
  - Spell is preparing with cast time > 0
  - Spell has `SPELL_PREVENTION_TYPE_SILENCE`
  - Spell has appropriate interrupt flags
- Applies school lockout via `LockOutSpells()` for spell duration
- Interrupts spell via `InterruptSpell()`
- Lockout duration from spell duration
- Generates threat
- Some interrupts have no lockout (duration = 0)

---

### 69 - SPELL_EFFECT_DISTRACT
**Function**: `effect_distract()`

Distracts NPCs at the target location.

**Parameters**:
- Target location

**Usage**:
- Rogue Distract

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2590-2607`):
```cpp
void Spell::EffectDistract(SpellEffectIndex effIdx)
{
    // Check for possible target
    if (!unitTarget || unitTarget->IsInCombat())
        return;

    // target must be OK to do this
    if (unitTarget->HasUnitState(UNIT_STATE_CAN_NOT_REACT))
        return;

    unitTarget->SetFacingTo(unitTarget->GetAngle(m_targets.m_destX, m_targets.m_destY));
    unitTarget->ClearUnitState(UNIT_STATE_MOVING);

    if (unitTarget->GetTypeId() == TYPEID_UNIT)
        unitTarget->GetMotionMaster()->MoveDistract(damage * (uint32)IN_MILLISECONDS);

    AddExecuteLogInfo(effIdx, ExecuteLogInfo(unitTarget->GetObjectGuid()));
}
```

**Key Behaviors**:
- Only works on targets not in combat
- Target must not have `UNIT_STATE_CAN_NOT_REACT`
- Sets target facing toward distract location
- Clears movement state
- For creatures: initiates distract movement via `MoveDistract()`
- Duration from `damage` field (converted to milliseconds)
- Interrupts current actions
- Does NOT break stealth (stealth break is separate mechanic)

---

### 79 - SPELL_EFFECT_SANCTUARY
**Function**: `effect_sanctuary()`

Applies sanctuary effect (PvP protection).

**Parameters**: None

**Usage**:
- Sanctuary zones
- PvP protection buffs

**Implementation Details** (from MaNGOS `SpellEffects.cpp:4512-4554`):
```cpp
void Spell::EffectSanctuary(SpellEffectIndex effIdx)
{
    if (!unitTarget)
        return;

    // World of Warcraft Client Patch 1.12.0 (2006-08-22)
    // - Neutral guards are now able to see through the rogue Vanish ability.
    bool guardCheck = m_spellInfo->IsFitToFamily<SPELLFAMILY_ROGUE, CF_ROGUE_VANISH>() && (sWorld.GetWowPatch() >= WOW_PATCH_112);
    bool noGuards = true;

    unitTarget->InterruptSpellsCastedOnMe(true);
    unitTarget->InterruptAttacksOnMe(0.0f, guardCheck);
    unitTarget->m_lastSanctuaryTime = WorldTimer::getMSTime();

    // Flask of Petrification does not cause mobs to stop attacking.
    if (m_spellInfo->IsFitToFamily<SPELLFAMILY_ROGUE, CF_ROGUE_VANISH>())
    {
        // Vanish allows to remove all threat and cast regular stealth so other spells can be used
        unitTarget->CombatStop();

        HostileReference* pReference = unitTarget->GetHostileRefManager().getFirst();
        while (pReference)
        {
            HostileReference* pNextRef = pReference->next();
            if (!guardCheck || !pReference->getSource()->getOwner()->IsContestedGuard())
            {
                pReference->removeReference();
                delete pReference;
            }
            else
                noGuards = false;

            pReference = pNextRef;
        }

        if (noGuards && unitTarget->IsPlayer())
            static_cast<Player*>(unitTarget)->SetCannotBeDetectedTimer(1000);
    }
    else
        unitTarget->DoResetThreat();

    AddExecuteLogInfo(effIdx, ExecuteLogInfo(unitTarget->GetObjectGuid()))
}
```

**Key Behaviors**:
- Interrupts spells being cast on target
- Interrupts attacks on target
- Records sanctuary time for threat management
- **For Vanish (Rogue)**:
  - Stops combat entirely
  - Removes all threat from hostile references
  - Post-1.12: Guards can see through Vanish (special handling)
  - Sets 1-second "cannot be detected" timer if no guards present
- **For other sanctuary effects**:
  - Resets threat on target
- Prevents PvP combat while active
- Used by Vanish, Flask of Petrification, and sanctuary zones

---

### 80 - SPELL_EFFECT_ADD_COMBO_POINTS
**Function**: `effect_add_combo_points()`

Adds combo points to the target.

**Parameters**:
- `base_value`: Number of combo points

**Usage**:
- Rogue combo point generators
- Finisher setup

**Implementation Details** (from MaNGOS `SpellEffects.cpp:4556-4569`):
```cpp
void Spell::EffectAddComboPoints(SpellEffectIndex /*effIdx*/)
{
    if (!unitTarget)
        return;

    if (m_caster->GetTypeId() != TYPEID_PLAYER)
        return;

    if (damage <= 0)
        return;

    ((Player*)m_caster)->AddComboPoints(unitTarget, damage);
    ((Player*)m_caster)->SetUInt64Value(PLAYER_FIELD_COMBO_TARGET, unitTarget->GetGUID());
}
```

**Key Behaviors**:
- Only works for player casters
- `damage` field determines combo points to add
- Calls `AddComboPoints()` to add points to target
- Sets combo target GUID for UI display
- Max 5 combo points per target
- Points are per-target (lost when switching targets)
- Required for finishing moves
- Combo points persist until used or target dies
- UI shows combo points on target frame

---

### 114 - SPELL_EFFECT_ATTACK_ME
**Function**: `effect_attack_me()`

Forces the target to attack the caster (taunt).

**Parameters**: None

**Usage**:
- Taunt (Warrior)
- Growl (Druid)
- Righteous Defense (Paladin)

**Implementation Details** (from MaNGOS `SpellEffects.cpp:3292-3331`):
```cpp
void Spell::EffectTaunt(SpellEffectIndex effIdx)
{
    if (!unitTarget || !m_casterUnit)
        return;

    // this effect use before aura Taunt apply for prevent taunt already attacking target
    // for spell as marked "non effective at already attacking target"
    if (unitTarget->GetTypeId() != TYPEID_PLAYER)
    {
        if (unitTarget->GetVictim() == m_caster)
        {
            SendCastResult(SPELL_FAILED_DONT_REPORT);
            return;
        }
    }

    // Also use this effect to set the taunter's threat to the taunted creature's highest value
    if (unitTarget->CanHaveThreatList() && unitTarget->GetThreatManager().getCurrentVictim())
    {
        // Add the difference of the current highest value threat and the taunter's threat
        unitTarget->GetThreatManager().addThreat(m_casterUnit, unitTarget->GetThreatManager().getCurrentVictim()->getThreat()-unitTarget->GetThreatManager().getThreat(m_casterUnit, false));

        // Patch 1.11 notes
        // https://web.archive.org/web/20061109034626/http://evilempireguild.org:80/guides/kenco2.php
        // Recently(1.11.x), the behaviour of Taunt has been buffed slightly. It now does three things :

        // Taunt debuff.The mob is forced to attack you for 3 seconds.Later taunts by other players override this.
        // You are given threat equal to the mob's previous aggro target, permanently. Importantly, you won't necessarily get as much threat
        // as the highest person on the mob's list, only as much as whoever is currently tanking it.
        // You gain complete aggro on the mob at the instant you taunt.Usually you would need 10 % more threat to gain aggro(see section 3),
        // but a taunt now gives you instant aggro on the mob.
        // Of course if other people are generating significant threat on the mob, they could exceed your threat by more than 10 % before the taunt debuff wears off,
        // and will gain aggro as soon as it does.There is no limit to the amount of threat you can gain from Taunt.
#if SUPPORTED_CLIENT_BUILD > CLIENT_BUILD_1_10_2
        unitTarget->GetThreatManager().setCurrentVictimIfCan(m_casterUnit);
#endif
    }

    AddExecuteLogInfo(effIdx, ExecuteLogInfo(unitTarget->GetObjectGuid()));
}
```

**Key Behaviors**:
- Fails silently if target is already attacking caster (non-player targets)
- **Three main effects** (Patch 1.11+):
  1. **Taunt debuff**: Forces target to attack caster for 3 seconds
  2. **Threat equalization**: Sets caster's threat to match current target's threat
  3. **Instant aggro**: Bypasses 10% threat threshold normally required
- Calculates threat difference: `currentVictimThreat - casterThreat`
- Adds difference to caster's threat
- Post-1.10: Sets caster as current victim immediately
- Target will attack caster for 3 seconds minimum
- Other players can override with their own taunt
- If others generate >10% more threat during debuff, they gain aggro when it expires
- No threat cap - can gain unlimited threat from taunt

---

### 125 - SPELL_EFFECT_MODIFY_THREAT_PERCENT
**Function**: `effect_modify_threat_percent()`

Modifies threat by percentage.

**Parameters**:
- `base_value`: Percentage modifier

**Usage**:
- Threat reduction abilities
- Threat amplification

**Implementation Notes**:
- Multiplies current threat
- Negative values reduce threat
- Used by Fade, Feign Death, etc.

## Dependencies

Required systems:
- `CombatSystem` - For extra attacks and threat
- `PlayerSystem` - For combo points and dual wield
- `CreatureSystem` - For distract and taunt

## References

- MaNGOS: `SpellEffects.cpp` - `EffectAddExtraAttacks()`, `EffectThreat()`, etc.
- MaNGOS: `ThreatManager.cpp` - Threat handling
- MaNGOS: `Unit.cpp` - Combat mechanics
