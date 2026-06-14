# Spell::ResetEffectDamageAndHeal

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::ResetEffectDamageAndHeal is a parameterless void member that zeroes three per-spell accumulators—m_damage, m_healing, and m_absorbed—before (or after aborting) per-target effect processing so damage/heal/absorb totals from a prior target do not carry over.

## Source
```cpp

```
