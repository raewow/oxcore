# Spell::TakeReagents

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::TakeReagents removes spell reagents from the player caster's inventory after a cast. It no-ops for non-player casters or when IgnoreItemRequirements() is true. For each positive reagent entry in m_spellInfo, it adjusts consumption when the cast item or item target overlaps the reagent (clearing m_CastItem and/or the item target pointer), then calls Player::DestroyItemCount to destroy the required item count.

## Source
```cpp

```
