# Spell::UpdatePointers

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Refreshes cached spell object pointers after a time delay by first updating the original-caster Unit pointer from its GUID, then re-resolving all stored spell-cast targets (unit, game object, item) via m_targets.Update(m_caster).

## Source
```cpp

```
