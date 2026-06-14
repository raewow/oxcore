# Spell::InitializeChanneledVisualTimer

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::InitializeChanneledVisualTimer conditionally configures periodic channeled spell visual playback by reading spell custom flags and SpellVisual DBC data; on success it stores the channel kit id in m_channeledVisualKit and sets m_channeledVisualTimer to SPELL_CHANNEL_VISUAL_TIMER (800), otherwise it returns without writing those members.

## Source
```cpp

```
