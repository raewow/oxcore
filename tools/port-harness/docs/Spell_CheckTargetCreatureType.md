# Spell::CheckTargetCreatureType

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Const member that returns whether a Unit target satisfies the spell's TargetCreatureType bitmask. It short-circuits true for Grounding Totem aura 8179, applies spell-specific overrides for Curse of Doom (603), Dismiss Pet (2641), and Taming Lesson (23356), and otherwise requires a zero target creature-type mask or a bitwise overlap between the spell mask and target->GetCreatureTypeMask().

## Source
```cpp

```
