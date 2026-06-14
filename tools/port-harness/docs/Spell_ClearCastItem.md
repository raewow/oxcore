# Spell::ClearCastItem

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::ClearCastItem clears the spell's cast-item pointer (m_CastItem). When the cast item is the same object as the spell's item target, it also calls m_targets.setItemTarget(nullptr) before nulling m_CastItem.

## Source
```cpp

```
