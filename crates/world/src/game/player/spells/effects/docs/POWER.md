# Power Effects Documentation

## File: `power.rs`

## Overview

Handles power (mana/energy/rage) manipulation effects including drain, restore, and burn.

## Effects (3 total)

### 8 - SPELL_EFFECT_POWER_DRAIN
**Function**: `effect_power_drain()`

Drains power from target and gives to caster.

**Parameters**:
- `base_value`: Amount to drain
- `misc_value`: Power type (0=Mana, 1=Rage, 3=Energy)

**Usage**:
- Mana Drain (Warlock)
- Drain Mana (Priest)

**Implementation Details** (from MaNGOS `SpellEffects.cpp:1659-1720`):
```cpp
void Spell::EffectPowerDrain(SpellEffectIndex effIdx)
{
    if (m_spellInfo->EffectMiscValue[effIdx] < 0 || m_spellInfo->EffectMiscValue[effIdx] >= MAX_POWERS)
        return;

    if (!unitTarget || !unitTarget->IsAlive() || damage < 0)
        return;

    Powers drainPower = Powers(m_spellInfo->EffectMiscValue[effIdx]);

    // happiness is never a creature's main power so it has special handling
    if (drainPower == POWER_HAPPINESS)
    {
        if (!unitTarget->IsPet())
            return;
    }
    else
    {
        if (unitTarget->GetPowerType() != drainPower)
            return;
    }

    int32 curPower = unitTarget->GetPower(drainPower);

    //add spell damage bonus
    damage = m_caster->SpellDamageBonusDone(unitTarget, m_spellInfo, effIdx, damage, SPELL_DIRECT_DAMAGE, 1, this);
    damage = unitTarget->SpellDamageBonusTaken(m_caster, m_spellInfo, effIdx, damage, SPELL_DIRECT_DAMAGE, 1, this);

    float new_damage;
    if (curPower < damage)
        new_damage = curPower;
    else
        new_damage = damage;

    unitTarget->ModifyPower(drainPower, -new_damage);

    // Don`t restore from self drain
    if (drainPower == POWER_MANA && m_caster != unitTarget)
    {
        float manaMultiplier = m_spellInfo->EffectMultipleValue[effIdx];
        if (manaMultiplier == 0)
            manaMultiplier = 1;

        if (m_casterUnit)
        {
            if (Player* modOwner = m_casterUnit->GetSpellModOwner())
                modOwner->ApplySpellMod(m_spellInfo->Id, SPELLMOD_MULTIPLE_VALUE, manaMultiplier);
        }

        float gain = new_damage * manaMultiplier;

        if (m_casterUnit)
            m_casterUnit->ModifyPower(POWER_MANA, dither(gain));
    }
}
```

**Key Behaviors**:
- Validates power type from `EffectMiscValue`
- Special handling for POWER_HAPPINESS (only works on pets)
- Checks target's power type matches drain power type
- Applies spell damage bonuses via `SpellDamageBonusDone()` / `SpellDamageBonusTaken()`
- Caps drain at target's current power level
- For mana drain: Restores mana to caster with multiplier from `EffectMultipleValue`
- Multiplier modified by SPELLMOD_MULTIPLE_VALUE
- Self-drain does NOT restore mana to caster
- Generates threat
- Can be resisted

---

### 30 - SPELL_EFFECT_ENERGIZE
**Function**: `effect_energize()`

Restores power to target.

**Parameters**:
- `base_value`: Amount to restore
- `misc_value`: Power type

**Usage**:
- Mana potions
- Innervate (Druid)
- Mana Tide Totem (Shaman)
- Energy regeneration

**Implementation Details** (from MaNGOS `SpellEffects.cpp:1980-2006`):
```cpp
void Spell::EffectEnergize(SpellEffectIndex effIdx)
{
    if (!unitTarget)
        return;
    if (!unitTarget->IsAlive())
        return;

    if (m_spellInfo->EffectMiscValue[effIdx] < 0 || m_spellInfo->EffectMiscValue[effIdx] >= MAX_POWERS)
        return;

    Powers power = Powers(m_spellInfo->EffectMiscValue[effIdx]);

    if (damage < 0)
        return;

    if (unitTarget->GetMaxPower(power) == 0)
        return;

    m_caster->EnergizeBySpell(unitTarget, m_spellInfo->Id, damage, power);
}
```

**Key Behaviors**:
- Validates power type from `EffectMiscValue`
- Validates target has max power > 0 for that power type
- Applies energize via `EnergizeBySpell()`
- Cannot exceed maximum power
- No threat generation
- Used for power restoration

---

### 62 - SPELL_EFFECT_POWER_BURN
**Function**: `effect_power_burn()`

Burns target's power and deals damage.

**Parameters**:
- `base_value`: Damage per power burned
- `misc_value`: Power type to burn

**Usage**:
- Mana Burn (Priest)
- Power burn effects

**Implementation Details** (from MaNGOS `SpellEffects.cpp:1738-1769`):
```cpp
void Spell::EffectPowerBurn(SpellEffectIndex effIdx)
{
    if (m_spellInfo->EffectMiscValue[effIdx] < 0 || m_spellInfo->EffectMiscValue[effIdx] >= MAX_POWERS)
        return;

    Powers powertype = Powers(m_spellInfo->EffectMiscValue[effIdx]);

    if (!unitTarget)
        return;
    if (!unitTarget->IsAlive())
        return;
    if (unitTarget->GetPowerType() != powertype)
        return;
    if (damage < 0)
        return;

    int32 curPower = int32(unitTarget->GetPower(powertype));

    float newDamage = (curPower < damage) ? curPower : damage;

    unitTarget->ModifyPower(powertype, -newDamage);
    float multiplier = m_spellInfo->EffectMultipleValue[effIdx];

    if (m_casterUnit)
    {
        if (Player* modOwner = m_casterUnit->GetSpellModOwner())
            modOwner->ApplySpellMod(m_spellInfo->Id, SPELLMOD_MULTIPLE_VALUE, multiplier);
    }
    
    newDamage = newDamage * multiplier;
    m_damage += newDamage;
}
```

**Key Behaviors**:
- Validates power type from `EffectMiscValue`
- Checks target's power type matches burn power type
- Caps burn at target's current power level
- Burns power via `ModifyPower()`
- Damage = power_burned * multiplier from `EffectMultipleValue`
- Multiplier modified by SPELLMOD_MULTIPLE_VALUE
- Shadow damage typically
- Generates threat
- Damage added to `m_damage` for later application

## Dependencies

Required systems:
- `PowerSystem` - For power management
- `CombatSystem` - For damage from power burn

## References

- MaNGOS: `SpellEffects.cpp` - `EffectPowerDrain()`, `EffectEnergize()`, `EffectPowerBurn()`
