# Spell::cast

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::cast(bool skipCheck) executes the final cast phase after preparation: it guards against recursive cast overflow, refreshes object pointers, validates power and cast rules (unless skipped or channeling-visual), builds targets, consumes resources, notifies client/scripts/procs, then dispatches either a delayed spell path (SendSpellGo, delayed launch, combat setup) or an immediate path via handle_immediate(), finally resetting melee timers and clearing execution state.

## Source
```cpp

```
