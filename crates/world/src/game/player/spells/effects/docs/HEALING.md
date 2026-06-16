# Healing Effects Documentation

## File: `healing.rs`

## Overview

Handles all healing spell effects including direct heals and full heals.

## Effects (4 total)

### 10 - SPELL_EFFECT_HEAL
**Function**: `effect_heal()`

Direct heal (Flash Heal, Healing Touch, etc.).

**Parameters**:
- `base_value`: Base heal amount from DBC

**Usage**:
- Flash Heal (Priest)
- Healing Touch (Druid)
- Holy Light (Paladin)
- Healing Wave (Shaman)
- All direct healing spells

**Implementation Details** (from MaNGOS `SpellEffects.cpp:1771-1793`):
```cpp
void Spell::EffectHeal(SpellEffectIndex effIdx)
{
    if (unitTarget && unitTarget->IsAlive() && damage >= 0)
    {
        // Try to get original caster
        SpellCaster* caster = GetAffectiveCasterObject();
        if (!caster)
            return;

        float addhealth = damage;
        addhealth = caster->SpellHealingBonusDone(unitTarget, m_spellInfo, effIdx, addhealth, HEAL, 1, this);
        addhealth = unitTarget->SpellHealingBonusTaken(caster, m_spellInfo, effIdx, addhealth, HEAL, 1, this);

        m_healing += addhealth;
    }
}
```

**Key Behaviors**:
- Gets effective caster via `GetAffectiveCasterObject()` (handles original caster)
- Applies healing bonuses via `SpellHealingBonusDone()` / `SpellHealingBonusTaken()`
- Accumulates to `m_healing` (applied later in spell execution)
- Can crit (150% heal)
- Affected by healing reduction debuffs (Mortal Strike, etc.)
- Generates threat (0.5x effective heal)
- Tracks overhealing
- Shows combat log entry

**Calculation Pipeline**:
1. Base value from DBC
2. Add healing power bonus (HP * coefficient)
3. Apply healing modifiers (talents)
4. Roll for crit
5. Apply crit multiplier if crit
6. Apply target healing reduction
7. Calculate overheal
8. Apply heal and generate threat

---

### 67 - SPELL_EFFECT_HEAL_MAX_HEALTH
**Function**: `effect_heal_max_health()`

Heals target to full health.

**Parameters**: None

**Usage**:
- Lay on Hands (Paladin)
- Full heal effects

**Implementation Details** (from MaNGOS `SpellEffects.cpp:3452-3494`):
```cpp
void Spell::EffectHealMaxHealth(SpellEffectIndex effIdx)
{
    if (!unitTarget)
        return;
    if (!unitTarget->IsAlive())
        return;

    float heal = m_casterUnit ? m_casterUnit->GetMaxHealth() : unitTarget->GetMaxHealth();

    // Healing percent modifiers
    float DoneTotalMod = 1.0f;
    float TakenTotalMod = 1.0f;

    // Healing done percent
    if (m_casterUnit)
    {
        std::list<Aura*> const& mHealingDonePct = m_casterUnit->GetAurasByType(SPELL_AURA_MOD_HEALING_DONE_PERCENT);
        for (const auto i : mHealingDonePct)
            DoneTotalMod *= (100.0f + i->GetModifier()->m_amount) / 100.0f;
    }

    heal *= DoneTotalMod;

    // Healing taken percent
    float minval = float(unitTarget->GetMaxNegativeAuraModifier(SPELL_AURA_MOD_HEALING_PCT));
    if (minval)
        TakenTotalMod *= (100.0f + minval) / 100.0f;

    float maxval = float(unitTarget->GetMaxPositiveAuraModifier(SPELL_AURA_MOD_HEALING_PCT));
    if (maxval)
        TakenTotalMod *= (100.0f + maxval) / 100.0f;

    heal *= TakenTotalMod;

    m_healing += heal;
}
```

**Key Behaviors**:
- Uses caster's max health if caster is a unit, otherwise target's max health
- Applies SPELL_AURA_MOD_HEALING_DONE_PERCENT (done percent modifiers)
- Applies SPELL_AURA_MOD_HEALING_PCT (taken percent modifiers)
- No overheal (by definition)
- Long cooldown typically
- Generates threat based on amount healed
- May also restore mana (Lay on Hands)

---

### 75 - SPELL_EFFECT_HEAL_MECHANICAL
**Function**: `effect_heal_mechanical()`

Heal mechanical units.

**Parameters**:
- `base_value`: Heal amount

**Usage**:
- Mechanical Patch Kit
- Repair abilities

**Implementation Details** (from MaNGOS `SpellEffects.cpp:1795-1810`):
```cpp
void Spell::EffectHealMechanical(SpellEffectIndex effIdx)
{
    // Mechanic creature type should be correctly checked by targetCreatureType field
    if (unitTarget && unitTarget->IsAlive() && damage >= 0)
    {
        // Try to get original caster
        SpellCaster* caster = GetAffectiveCasterObject();
        if (!caster)
            return;

        float addhealth = caster->SpellHealingBonusDone(unitTarget, m_spellInfo, effIdx, damage, HEAL);
        addhealth = unitTarget->SpellHealingBonusTaken(caster, m_spellInfo, effIdx, addhealth, HEAL);

        caster->DealHeal(unitTarget, addhealth, m_spellInfo);
    }
}
```

**Key Behaviors**:
- Target creature type checked by `targetCreatureType` field (mechanical)
- Gets effective caster via `GetAffectiveCasterObject()`
- Applies healing bonuses via `SpellHealingBonusDone()` / `SpellHealingBonusTaken()`
- Immediately applies heal via `DealHeal()` (not accumulated to `m_healing`)
- Only works on mechanical targets
- Used for repairing mechanical units

---

### 117 - SPELL_EFFECT_SPIRIT_HEAL
**Function**: `effect_spirit_heal()`

Spirit healer resurrection heal.

**Parameters**: None

**Usage**:
- Spirit Healer resurrection
- Graveyard resurrection

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5757-5782`):
```cpp
void Spell::EffectSpiritHeal(SpellEffectIndex /*effIdx*/)
{
    // TODO player can't see the heal-animation - he should respawn some ticks later
    if (!unitTarget || unitTarget->IsAlive())
        return;
    if (unitTarget->GetTypeId() != TYPEID_PLAYER)
        return;
    if (!unitTarget->IsInWorld())
        return;
    if (!unitTarget->HasAura(2584))  // Waiting to Resurrect aura
        return;

    auto player = unitTarget->ToPlayer();

    if (!player)
        return;

    // no resurrection on a GY other than homie if BG is not in progress
    if (player->GetBattleGround()->GetStatus() != STATUS_IN_PROGRESS && !player->IsGameMaster())
        player->RepopAtGraveyard();

    player->RemoveAurasDueToSpell(2584);
    player->ResurrectPlayer(1.0f);
    player->SpawnCorpseBones();
    player->AutoReSummonPet();
}
```

**Key Behaviors**:
- Only works on dead players
- Requires "Waiting to Resurrect" aura (spell 2584)
- In battlegrounds: Only resurrects if BG is in progress or player is GM
- Removes resurrection aura
- Resurrects player at 100% health (1.0f = 100%)
- Spawns corpse bones
- Auto-resummons pet
- Applies resurrection sickness (handled by `ResurrectPlayer`)
- No equipment durability loss

## Dependencies

Required systems:
- `CombatSystem` - For heal application
- `StatSystem` - For healing power
- `ThreatSystem` - For threat generation

## References

- MaNGOS: `SpellEffects.cpp` - `EffectHeal()`
- MaNGOS: `SpellAuras.cpp` - Periodic heal handling
