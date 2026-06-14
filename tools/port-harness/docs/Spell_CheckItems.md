# Spell::CheckItems

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::CheckItems validates caster/target item, equipment, reagent, totem, spell-focus, and effect-specific inventory requirements for a non-passive spell cast. It returns SPELL_CAST_OK when all checks pass, or a specific SpellCastResult failure code; non-player casters skip most player-only checks after an optional creature weapon-disarm guard.

## Source
```cpp

```
