# Spell::handle_delayed

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::handle_delayed(uint64 t_offset) drives incremental execution of a delayed spell: on the first invocation it runs _handle_immediate_phase once, then each call resets m_targetNum, applies CheckAtDelay and DoAllEffectOnTarget to unit targets whose timeDelay has elapsed, applies DoAllEffectOnTarget to due gameobject targets, tracks the earliest remaining timeDelay among unprocessed targets, and either completes the spell via _handle_finish_phase/finish(true) returning 0, or returns the next delay timestamp for rescheduling.

## Source
```cpp

```
