# Spell::CalculatePowerCost

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Static Spell::CalculatePowerCost computes a spell's power cost from SpellEntry fields and caster state. It returns 0 for a null caster or item cast; drains all current health or power when SPELL_ATTR_EX_USE_ALL_MANA is set; otherwise builds an int32 cost from base/per-level mana, optional percentage-of-pool additions, school flat and percent aura modifiers (client-build gated), optional player SPELLMOD_COST mods, optional creature-level scaling, clamps negatives to zero, and returns the result as uint32.

## Source
```cpp

```
