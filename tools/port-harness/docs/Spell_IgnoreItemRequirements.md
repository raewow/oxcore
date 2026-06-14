# Spell::IgnoreItemRequirements

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::IgnoreItemRequirements is a const predicate that returns false for non-triggered spells. For triggered spells it returns false when the item target is owned by someone other than the caster (trade-slot case), otherwise returns true unless the triggering master spell has no first-slot reagent—meaning triggered spells skip item/reagent checks when the master spell already supplies reagents in slot 0, or when there is no triggering spell info.

## Source
```cpp

```
