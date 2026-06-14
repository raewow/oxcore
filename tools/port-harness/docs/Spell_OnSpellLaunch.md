# Spell::OnSpellLaunch

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::OnSpellLaunch is invoked during cast() after CheckCast succeeds (Spell.cpp:3407). It no-ops when the caster unit is missing or not in world. Otherwise it may aggro nearby creatures when opening a lock-target game object, assigns the spell's unitTarget from cast targets, and no-ops again if that unit is missing or not in world. For spells with SPELL_ATTR_EX2_ACTIVE_THREAT it may start PvP combat timing on the caster. Charge (SPELL_EFFECT_CHARGE) is handled here rather than in EffectCharge: player casters get delayed main/off-hand attack timers, then MoveCharge is issued toward the unit target with an optional spell-batching delay and conditional auto-attack trigger.

## Source
```cpp

```
