# Spell::_handle_immediate_phase

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::_handle_immediate_phase (3889-3927) runs immediate pre-target-effect setup: calls HandleThreatSpells, sets m_needSpellLog from IsNeedSendToClient then may clear it for school-damage effects, applies SEND_EVENT to ground when an effect has no targets, resets diminishing-returns fields, processes all m_UniqueItemInfo entries via DoAllEffectOnTarget, and applies PERSISTENT_AREA_AURA effects to ground via HandleEffects with null unit/item/corpse pointers.

## Source
```cpp

```
