# Spell::TakePower

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::TakePower deducts the precomputed m_powerCost from the caster unit when a spell cast consumes resources. It no-ops for item casts, aura-triggered spells, channeling-visual spells, null caster units, or players with PLAYER_CHEAT_NO_POWER. For POWER_HEALTH it subtracts health; otherwise it subtracts the spell's power type via ModifyPower, logs and returns on invalid power types, and for positive mana costs sets the five-second mana-regen timer via SetLastManaUse unless SPELL_ATTR_EX2_DONT_BLOCK_MANA_REGEN is set.

## Source
```cpp

```
