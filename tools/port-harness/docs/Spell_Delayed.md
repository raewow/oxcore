# Spell::Delayed

**File:** src/game/Server/Packets/Spell.cpp

## Summary
On damage pushback for a player spell cast, extends the remaining cast timer by an escalating delay (via GetNextDelayAtDamageMsTime), optionally resisted by spell mods and SPELL_AURA_RESIST_PUSHBACK, clamps to total cast time, logs the interruption, and sends SMSG_SPELL_DELAYED to the caster client. No-ops for non-players, spells already in SPELL_STATE_DELAYED, spells without SPELL_INTERRUPT_FLAG_DAMAGE_PUSHBACK, or successful pushback resistance rolls.

## Source
```cpp

```
