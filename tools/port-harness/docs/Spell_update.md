# Spell::update

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::update(uint32 difftime) is the per-tick spell driver: it refreshes target/caster pointers, cancels the cast when targets, movement, ritual state, or incapacitating conditions fail various checks, then advances SPELL_STATE_PREPARING (countdown then cast()) or SPELL_STATE_CASTING (channel maintenance, aura-holder updates, timer expiry with quest rewards and finish(), or periodic channel visuals).

## Source
```cpp

```
