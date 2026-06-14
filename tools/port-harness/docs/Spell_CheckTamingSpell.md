# Spell::CheckTamingSpell

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Validates whether the spell's current unit target can be tamed by the given player, returning a PetTameFailureReason code. Checks hunter eligibility (unless gm is true), absence of active pet/charm/saved pet, target existence as a creature, ownership state, level (unless gm is true), and creature tameability; returns PETTAME_NONE when all checks pass.

## Source
```cpp

```
