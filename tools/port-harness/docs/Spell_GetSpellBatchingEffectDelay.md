# Spell::GetSpellBatchingEffectDelay

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Const Spell method that returns a uint32 delay (ms) for batched spell effect execution: 0 when Spell.EffectDelay config is off, when the target is the caster unit without a chain target on that effect, or when a peaceful-target spell hits a creature; otherwise returns milliseconds until the next world spell-batching interval.

## Source
```cpp

```
