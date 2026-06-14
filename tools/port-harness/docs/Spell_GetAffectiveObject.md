# Spell::GetAffectiveObject

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::GetAffectiveObject has no function definition in the codebase. Line 8135 is a Doxygen @param comment on Spell::FillAreaTargets stating that when originalCaster is nullptr, the return value of Spell::GetAffectiveObject() should be used. The only nearby executable substitute is Spell::GetAffectiveCasterObject() (lines 8188–8196), which SpellNotifierCreatureAndPlayer actually calls at line 7949 when originalCaster is nullptr.

## Source
```cpp

```
