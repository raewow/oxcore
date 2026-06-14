# Spell::CanAutoCast

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Returns whether this spell may be auto-cast on the given Unit (pet AI): rejects invalid caster, combat-cancel spells while fighting, warlock imp Fire Shield without a melee attacker on the target, redundant buffs/stuns/speed auras, full-health heal targets, failed CheckPetCast (except UNIT_NOT_INFRONT), or when FillTargetMap does not include the requested target; otherwise returns true.

## Source
```cpp

```
