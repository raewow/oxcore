# Spell::TakeAmmo

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::TakeAmmo consumes ranged ammunition for player casters after a spell cast. It returns immediately for non-players and for a hardcoded set of spell IDs. When m_attackType is RANGED_ATTACK, it resolves the equipped ranged weapon: wands and missing weapons consume nothing; thrown weapons either lose durability (non-stackable) or one stack count (stackable); other ranged weapons destroy one unit of the ammo item ID stored in PLAYER_AMMO_ID if that field is non-zero.

## Source
```cpp

```
