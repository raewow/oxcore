# Spell::prepareDataForTriggerSystem

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::prepareDataForTriggerSystem (lines 622–811) initializes proc/trigger metadata on the Spell instance: resets m_canTrigger, m_procAttacker, and m_procVictim; decides whether the cast may trigger procs from spell attributes, cast context (item, game-object caster, triggered/aura-triggered), and spell-family exceptions; assigns base attacker/victim proc flags from DmgClass (melee, ranged, default positive/heal/wand/negative branches); ORs hunter trap activation for trap-family spells in the default branch; then builds m_negativeEffectMask by marking non-positive effects and self-targeted school-damage effects.

## Source
```cpp

```
