# Spell::IsTriggeredByProc

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::IsTriggeredByProc is a const, read-only predicate that returns true only when the spell was triggered by an aura (m_triggeredByAuraSpell is non-null) and that triggering spell entry's procFlags is non-zero; it returns false immediately if the spell has SPELL_ATTR_EX3_NOT_A_PROC, and false otherwise.

## Source
```cpp

```
