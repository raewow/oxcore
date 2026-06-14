# Spell::SendChannelStart

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::SendChannelStart begins a channeled spell: it picks a channel target object (persistent-area dynobject, first valid unit target, or game object), sends MSG_CHANNEL_START to a player caster, sets m_timer, optionally broadcasts SMSG_SPELL_UPDATE_CHAIN_TARGETS for channeled spells on clients after 1.11.2, and updates the caster unit's channel spell id and channel object guid.

## Source
```cpp

```
