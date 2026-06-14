# Spell::IsTriggeredSpellWithRedundentData

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Const member predicate on Spell that returns true when the cast is considered a triggered spell carrying redundant spell-entry data (aura-triggered, has a triggering spell entry pointer, or is flagged triggered with non-zero mana cost fields), so callers can ignore fields such as cast time that exist only for client display.

## Source
```cpp

```
