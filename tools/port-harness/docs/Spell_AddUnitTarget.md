# Spell::AddUnitTarget

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::AddUnitTarget registers a Unit for one spell effect index: it returns early when the effect slot is empty or the original caster is excluded; if the unit already exists in m_UniqueTargetInfo (non-deleted), it ORs the effect bit when not immune and returns; otherwise it builds a TargetInfo with immunity mask, crit, hit/miss, travel or batching delay, reflect handling, and appends it to m_UniqueTargetInfo while updating m_delayMoment.

## Source
```cpp

```
