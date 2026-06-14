# Spell::RemoveChanneledAuraHolder

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Removes a channeled SpellAuraHolder from the spell's m_channeledHolders list when an aura is deleted or expires externally, skipping removal for AURA_REMOVE_BY_CHANNEL, AURA_REMOVE_BY_GROUP, and AURA_REMOVE_BY_RANGE; clears the holder's in-use flag and carefully updates m_channeledUpdateIterator if the removed entry is currently being iterated in Spell::update.

## Source
```cpp

```
