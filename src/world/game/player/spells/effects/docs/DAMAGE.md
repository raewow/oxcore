# Damage Effects Documentation

## File: `damage.rs`

## Overview

Handles all damage-dealing spell effects including direct spell damage, weapon damage, and environmental damage.

## Effects (8 total)

### 0 - SPELL_EFFECT_NONE
**Function**: Handled in dispatcher

No effect. Used as placeholder.

---

### 2 - SPELL_EFFECT_SCHOOL_DAMAGE
**Function**: `effect_school_damage()`

Direct spell damage (Fireball, Frostbolt, Shadow Bolt, etc.).

**Parameters**:
- `base_value`: Base damage from DBC
- Damage school from spell entry

**Usage**:
- All direct damage spells
- Nukes (Fireball, Frostbolt)
- DoT initial hits

**Implementation Details** (from MaNGOS `SpellEffects.cpp:300-307`):
```cpp
void Spell::EffectSchoolDMG(SpellEffectIndex effect_idx)
{
    if (unitTarget && unitTarget->IsAlive())
    {
        if (damage >= 0)
            m_damage += damage;
    }
}
```

**Key Behaviors**:
- Simple damage accumulation to `m_damage`
- Damage calculated before this call via `CalculateDamage()`
- Spell power bonuses applied in `SpellDamageBonusDone()` / `SpellDamageBonusTaken()`
- Can crit (150% damage)
- Affected by target resistance
- Generates threat
- Shows combat log entry
- Negative damage values are ignored

---

### 7 - SPELL_EFFECT_ENVIRONMENTAL_DAMAGE
**Function**: `effect_environmental_damage()`

Damage from environmental sources.

**Parameters**:
- `misc_value`: Damage type (Fire, Lava, Drowning, Falling)
- `base_value`: Damage amount

**Usage**:
- Lava damage
- Fire environmental damage
- Drowning
- Falling damage

**Implementation Details** (from MaNGOS `SpellEffects.cpp:283-298`):
```cpp
void Spell::EffectEnvironmentalDMG(SpellEffectIndex effIdx)
{
    if (!unitTarget || !unitTarget->IsAlive())
        return;

    if (unitTarget->GetTypeId() == TYPEID_PLAYER)
        ((Player*)unitTarget)->EnvironmentalDamage(DAMAGE_FIRE, dither(damage));
    else
    {
        uint32 absorb = 0;
        int32 resist = 0;
        unitTarget->CalculateDamageAbsorbAndResist(m_caster, m_spellInfo->GetSpellSchoolMask(), 
            SPELL_DIRECT_DAMAGE, dither(damage), &absorb, &resist, m_spellInfo);
        m_caster->SendSpellNonMeleeDamageLog(unitTarget, m_spellInfo->Id, dither(damage), 
            m_spellInfo->GetSpellSchoolMask(), absorb, resist, false, 0, false);
    }
}
```

**Key Behaviors**:
- Players: Calls `EnvironmentalDamage()` with DAMAGE_FIRE type
- Creatures: Calculates absorb/resist, sends damage log
- Bypasses armor
- Affected by resistance
- No threat generation
- Shows in combat log
- Uses `dither()` for damage variance

---

### 9 - SPELL_EFFECT_HEALTH_LEECH
**Function**: `effect_health_leech()`

Drains health from target and heals caster.

**Parameters**:
- `base_value`: Damage/heal amount

**Usage**:
- Drain Life (Warlock)
- Death Coil (Warlock)
- Life Drain effects

**Implementation Details** (from MaNGOS `SpellEffects.cpp:1812-1843`):
```cpp
void Spell::EffectHealthLeech(SpellEffectIndex effIndex)
{
    if (!unitTarget || !unitTarget->IsAlive() || damage < 0)
        return;

    float initialDamage = damage;
    damage = m_caster->SpellDamageBonusDone(unitTarget, m_spellInfo, effIndex, damage, SPELL_DIRECT_DAMAGE);
    damage = unitTarget->SpellDamageBonusTaken(m_caster, m_spellInfo, effIndex, damage, SPELL_DIRECT_DAMAGE);

    float healMultiplier = m_spellInfo->EffectMultipleValue[effIndex];

    if (m_casterUnit)
    {
        if (Player* modOwner = m_casterUnit->GetSpellModOwner())
            modOwner->ApplySpellMod(m_spellInfo->Id, SPELLMOD_MULTIPLE_VALUE, healMultiplier);
    }

    // get max possible damage, don't count overkill for heal
    if (damage > static_cast<int32>(unitTarget->GetHealth()))
        damage = unitTarget->GetHealth();

    if (m_casterUnit && m_casterUnit->IsAlive())
        m_casterUnit->DealHeal(m_casterUnit, ditheru(damage * healMultiplier), m_spellInfo);

    // Non delayed spells bonus damage is added later
    if (!m_delayed)
        damage = initialDamage;

    m_damage += damage;
}
```

**Key Behaviors**:
- Applies spell damage bonuses via `SpellDamageBonusDone()` / `SpellDamageBonusTaken()`
- Heal multiplier from `EffectMultipleValue` (modified by SPELLMOD_MULTIPLE_VALUE)
- Overkill damage is NOT counted for healing (capped at target's current health)
- Heal is applied via `DealHeal()` with dithered amount
- For non-delayed spells, uses initial damage (before bonuses)
- Shadow school typically
- Generates threat from damage portion

---

### 17, 31, 58, 121 - Weapon Damage Effects
**Functions**: All handled by `EffectWeaponDmg()` in MaNGOS

These effects are combined and handled together by a single function.

**Implementation Details** (from MaNGOS `SpellEffects.cpp:3333-3437`):
```cpp
void Spell::EffectWeaponDmg(SpellEffectIndex effIdx)
{
    if (!m_casterUnit || !unitTarget || !unitTarget->IsAlive())
        return;

    // Multiple weapon damage effects are combined
    // Only execute at the last weapon effect
    for (uint8 j = 0; j < MAX_EFFECT_INDEX; ++j)
    {
        switch (m_spellInfo->Effect[j])
        {
            case SPELL_EFFECT_WEAPON_DAMAGE:
            case SPELL_EFFECT_WEAPON_DAMAGE_NOSCHOOL:
            case SPELL_EFFECT_NORMALIZED_WEAPON_DMG:
            case SPELL_EFFECT_WEAPON_PERCENT_DAMAGE:
                if (j < uint8(effIdx))
                    return;  // Wait for last effect
                break;
        }
    }
    
    float weaponDamagePercentMod = 1.0f;
    bool normalized = false;
    float bonus = 0.f;
    
    // Process all weapon damage effects
    for (uint8 j = 0; j < MAX_EFFECT_INDEX; ++j)
    {
        switch (m_spellInfo->Effect[j])
        {
            case SPELL_EFFECT_WEAPON_DAMAGE:
            case SPELL_EFFECT_WEAPON_DAMAGE_NOSCHOOL:
                bonus += CalculateDamage(SpellEffectIndex(j), unitTarget);
                break;
            case SPELL_EFFECT_NORMALIZED_WEAPON_DMG:
                bonus += CalculateDamage(SpellEffectIndex(j), unitTarget);
                normalized = true;
                break;
            case SPELL_EFFECT_WEAPON_PERCENT_DAMAGE:
                if (m_casterUnit->IsCreature() && !((Creature*)m_casterUnit)->HasWeapon())
                {
                    weaponDamagePercentMod = 0.0f;
                    bonus += CalculateDamage(SpellEffectIndex(j), unitTarget);
                }
                else
                    weaponDamagePercentMod *= CalculateDamage(SpellEffectIndex(j), unitTarget) / 100.0f;
                break;
        }
    }

    // Apply bonus damage modifiers
    if (bonus)
    {
        if (weaponDamagePercentMod > 0.0f)
            bonus = bonus * weaponDamagePercentMod;

        // Get weapon damage modifier
        UnitMods unitMod;
        switch (m_attackType)
        {
            case BASE_ATTACK:  unitMod = UNIT_MOD_DAMAGE_MAINHAND; break;
            case OFF_ATTACK:   unitMod = UNIT_MOD_DAMAGE_OFFHAND;  break;
            case RANGED_ATTACK: unitMod = UNIT_MOD_DAMAGE_RANGED;  break;
        }

        float weapon_total_pct = m_casterUnit->GetModifierValue(unitMod, TOTAL_PCT);
        bonus = bonus * weapon_total_pct;
    }

    // Apply weapon percent damage to base weapon swing
    for (uint8 i = 0; i < m_casterUnit->GetWeaponDamageCount(m_attackType); i++)
    {
        if (unitTarget->IsImmuneToDamage(GetSchoolMask(m_casterUnit->GetWeaponDamageSchool(m_attackType, i))))
            continue;

        bonus += m_casterUnit->CalculateDamage(m_attackType, normalized, i) * weaponDamagePercentMod;
    }

    // Seal of Command special handling
    if (m_spellInfo->School == SPELL_SCHOOL_HOLY)
    {
        bonus = m_casterUnit->SpellDamageBonusDone(unitTarget, m_spellInfo, effIdx, bonus, SPELL_DIRECT_DAMAGE);
        bonus = unitTarget->SpellDamageBonusTaken(m_casterUnit, m_spellInfo, effIdx, bonus, SPELL_DIRECT_DAMAGE);
    }

    m_damage += bonus > 0.f ? bonus : 0.f;
}
```

**Key Behaviors**:
- All four weapon damage effects are processed together in one call
- Multiple effects on same spell are accumulated
- **SPELL_EFFECT_WEAPON_PERCENT_DAMAGE**: Multiplies total damage by percentage
- **SPELL_EFFECT_NORMALIZED_WEAPON_DMG**: Sets `normalized = true` for AP calculation
- **SPELL_EFFECT_WEAPON_DAMAGE_NOSCHOOL**: Physical damage, no school
- **SPELL_EFFECT_WEAPON_DAMAGE**: Adds spell school damage
- Normalized speeds: Dagger 1.7s, 1H 2.4s, 2H 3.3s
- Creatures without weapons do static damage
- Seal of Command (Holy school) gets spell damage bonuses
- Can crit (200% for melee)
- Affected by armor

## Dependencies

Required systems:
- `CombatSystem` - For damage application
- `StatSystem` - For spell power and AP
- `ThreatSystem` - For threat generation

## References

- MaNGOS: `SpellEffects.cpp` - `EffectSchoolDMG()`, `EffectWeaponDamage()`
- MaNGOS: `Spell.cpp` - Damage calculation
