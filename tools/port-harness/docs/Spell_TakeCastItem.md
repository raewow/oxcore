# Spell::TakeCastItem

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::TakeCastItem consumes spell charges on the player’s cast item after a successful cast: it skips non-players and most triggered spells, decrements per-slot item spell charges (destroying expendable items when charges reach zero), marks the item changed, and for expendable depleted items clears m_CastItem then destroys the item.

## Source
```cpp

```
