# Spell::AddGOTarget

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::AddGOTarget(GameObject*, SpellEffectIndex) registers a GameObject for one spell effect slot in m_UniqueGOTargetInfo: it no-ops when the effect is empty, MaxAffectedTargets is reached, or spell script rejects the target; if the GO is already listed it ORs the effect bit and returns; otherwise it builds a GOTargetInfo with travel-time or batching delay and appends it, updating m_delayMoment.

## Source
```cpp

```
