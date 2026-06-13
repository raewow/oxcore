# Spell::cancel

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::cancel aborts an in-progress spell unless already finished: it clears auto-repeat, performs state-specific cleanup (GCD restore, spell mods, interrupt/channel packets, aura removal, summoning-ritual GO detachment), then always calls finish(false) and removes dynamic objects and spell-associated game objects from the caster.

## Source
```cpp

```
