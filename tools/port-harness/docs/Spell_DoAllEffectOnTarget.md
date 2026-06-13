# Spell::DoAllEffectOnTarget

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::DoAllEffectOnTarget(TargetInfo*) applies one unit target's spell effects: marks the target processed, resolves the Unit and casters, adjusts effect/proc masks (including stripping persistent area aura bits and delayed-launch bits), runs DoSpellHitOnUnit on hit or reflect, handles miss combat/threat and power refunds, then applies accumulated healing or damage (with procs, threat, and combat logs) or a proc-only path when there is no immediate damage/heal, optionally triggers weapon/shield procs, and on a non-miss hit runs after-hit script hooks, creature quest/AI callbacks, zone scripts, and pet attack-stop for breakable CC.

## Source
```cpp

```
