# Spell::CanOpenLock

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Validates whether the current spell cast can open a target lock: zero lockId succeeds immediately; missing Lock DBC entry fails; iterates up to eight LockEntry requirements—matching cast item entry succeeds for LOCK_KEY_ITEM, matching spell EffectMiscValue for LOCK_KEY_SKILL sets out skill parameters and may require player skill plus spell bonus (skipped for cast items, non-players, or non-skill lock types except LOCKTYPE_BLASTING); if no requirement is satisfied returns bad targets.

## Source
```cpp

```
