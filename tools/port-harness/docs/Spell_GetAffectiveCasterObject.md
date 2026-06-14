# Spell::GetAffectiveCasterObject

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::GetAffectiveCasterObject is a const accessor that resolves the effective SpellCaster for spell effects: it returns m_caster when m_originalCasterGUID is empty, looks up the original GameObject on the caster's map when the GUID is a game object and m_caster is in-world, otherwise returns the cached m_originalCaster unit pointer. The result is a raw SpellCaster* that may be nullptr.

## Source
```cpp

```
