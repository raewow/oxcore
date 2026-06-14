# Spell::HandleThreatSpells

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::HandleThreatSpells applies flat bonus threat from the spell_threat database during the immediate spell phase. It no-ops without a caster unit, without unit targets, or without a non-zero threat entry; classifies the spell as all-positive or all-negative via m_negativeEffectMask (skipping mixed spells); then for each non-missed target whose effect mask passes CanCauseThreatOnMask, either distributes assist threat to mobs fighting a healed ally (positive) or adds threat on a hostile target toward the caster (negative).

## Source
```cpp

```
