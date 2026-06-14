# Spell::DelayedChannel

**File:** src/game/Server/Packets/Spell.cpp

## Summary
On damage pushback during an active player channeled spell (SPELL_STATE_CASTING), optionally resisted via spell mods and SPELL_AURA_RESIST_PUSHBACK, shortens the channel timer by an escalating delay from GetNextDelayAtDamageMsTime, propagates that delay to non-deleted hit targets' aura holders and persistent dynamic objects, then either interrupts the channeled spell when the timer reaches zero or sends a channel-duration update to the client.

## Source
```cpp

```
