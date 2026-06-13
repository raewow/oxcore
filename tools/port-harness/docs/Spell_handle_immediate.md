# Spell::handle_immediate

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::handle_immediate (lines 3778–3844) runs the non-delayed spell execution path: it optionally sends SMSG_SPELL_GO before or after effects (deferred for INSTAKILL/SPAWN), calls _handle_immediate_phase for immediate setup, starts channeling when m_channeled and m_duration are set, applies effects to each entry in m_UniqueTargetInfo and m_UniqueGOTargetInfo (with early return if a channeled spell is interrupted mid-target-loop), applies up to three HandleEffects on a located corpse target, runs _handle_finish_phase, sends a delayed SendSpellGo when needed, consumes the cast item via TakeCastItem, and calls finish(true) unless the spell entered SPELL_STATE_CASTING.

## Source
```cpp

```
