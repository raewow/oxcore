# Spell::CheckPetCast

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Validates pet/charmed-creature spell casts: rejects dead or busy casters, combat-restricted spells, dead owners, missing or invalid unit targets, hostile/friendly mismatches, and cooldown violations; may normalize m_targets unit target, then delegates to CheckCast(true) for remaining checks.

## Source
```cpp

```
