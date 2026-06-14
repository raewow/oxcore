# Spell::UpdateOriginalCasterPointer

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Refreshes the cached Unit pointer m_originalCaster from m_originalCasterGUID: if the GUID matches the spell caster, uses m_casterUnit; if it is a game-object GUID, resolves the GO on the caster's map (when in world) and stores its owner; otherwise looks up a Unit via ObjectAccessor and stores it only when found and in world, else nullptr.

## Source
```cpp

```
