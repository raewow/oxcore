# Spell::CheckScriptTargeting

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Resolves spell script targets from the spell_script_target database entries for a given effect: searches nearby gameobjects/units (or uses explicit/focus targets), updates m_targets and tempUnitList according to targetMode, logs DB configuration errors when no script rows exist, and returns SPELL_CAST_OK or a failure code. Implementation is at src/game/Spells/Spell.cpp:445-620 (indexed path src/game/Server/Packets/Spell.cpp is a basename collision).

## Source
```cpp

```
