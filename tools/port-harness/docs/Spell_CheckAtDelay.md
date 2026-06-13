# Spell::CheckAtDelay

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::CheckAtDelay re-validates a delayed unit TargetInfo entry immediately before effect application: it resolves the target Unit, strips per-effect bits from effectMask when the target is immune to those effects, sets missCondition to SPELL_MISS_IMMUNE when non-caster targets fail damage/spell/creature-type checks, and sets missCondition to SPELL_MISS_EVADE when the target is a creature in evade mode.

## Source
```cpp

```
