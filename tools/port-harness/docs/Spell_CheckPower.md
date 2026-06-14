# Spell::CheckPower

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::CheckPower is a const cast-validation helper that returns SPELL_CAST_OK when power checks are skipped (item cast, triggered spell, or missing caster unit), otherwise validates combo-point targeting for player combo-point spells, health sufficiency for POWER_HEALTH spells, power-type validity, creature power-type exemptions, and finally whether the caster's current power pool meets m_powerCost.

## Source
```cpp

```
