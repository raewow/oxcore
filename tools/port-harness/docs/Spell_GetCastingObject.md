# Spell::GetCastingObject

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::GetCastingObject is a const accessor that returns the spell's casting/visual object: when m_originalCasterGUID identifies a game object it looks up that GameObject on m_caster's map (only if m_caster is in-world, otherwise nullptr); in all other cases it returns m_caster. The result is a raw SpellCaster* with no null-check on the non-GO path.

## Source
```cpp

```
