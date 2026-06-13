# Spell::CleanupTargetList

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::CleanupTargetList resets spell targeting state: it soft-invalidates all unit and game-object targets by setting deleted=true on each list entry, hard-clears the item-target vector, and resets m_delayMoment to zero. Unit/GO entries remain in their vectors; only item targets are physically removed.

## Source
```cpp

```
