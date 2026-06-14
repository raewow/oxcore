# Spell::CheckTarget

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::CheckTarget is a per-effect Unit filter used during target-map filling: it returns false to reject a candidate target when positive-spell level-rank, duel, creature-type, selectability, player GM/visibility, possess/charm, AoE-immune, or player-only rules fail, otherwise delegates to m_spellScript->OnCheckTarget or returns true.

## Source
```cpp

```
