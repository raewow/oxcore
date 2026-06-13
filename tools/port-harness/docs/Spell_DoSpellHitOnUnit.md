# Spell::DoSpellHitOnUnit

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::DoSpellHitOnUnit(Unit* unit, uint32 effectMask) applies spell-hit processing for one unit: early exits for null unit, zero mask, resisted effects, or delayed-spell immunity/evade; on hostile hits removes stealth/invisibility and may enter combat or PvP state; snapshots diminishing-returns data, optionally creates a SpellAuraHolder, runs HandleEffects for each bit in effectMask (with damage multipliers), then applies the holder with DR-adjusted duration (deleting it and possibly interrupting a channeled cast if fully diminished) or tracks it for channeled spells.

## Source
```cpp

```
