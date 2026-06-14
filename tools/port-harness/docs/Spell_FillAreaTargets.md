# Spell::FillAreaTargets

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::FillAreaTargets is a void member that fills an area-effect unit target list by constructing a SpellNotifierCreatureAndPlayer visitor with the spell, output list, radius, push-type, target-filter flags, and optional original caster, then scanning the caster's map around the notifier's center coordinates via Cell::VisitAllObjects.

## Source
```cpp

```
