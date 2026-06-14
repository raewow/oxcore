# Spell::CheckCasterAuras

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Const Spell validation that checks whether the caster's unit flags and active auras prevent casting the current spell, with exemptions when the spell ignores restrictions or grants school/mechanic/dispel immunity via SPELL_ATTR_EX_IMMUNITY_PURGES_EFFECT; returns SPELL_CAST_OK or a specific SPELL_FAILED_* code.

## Source
```cpp

```
