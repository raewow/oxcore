# Spell::IsNeedSendToClient

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::IsNeedSendToClient is a const predicate that returns true when a spell cast should be visible to the client: the caster must be in-world, the cast must not be a channeling-visual-only spell, and at least one client-visible indicator must apply (non-zero SpellVisual, channeled flag, positive projectile speed, or a non-triggered/non-aura-triggered cast).

## Source
```cpp

```
