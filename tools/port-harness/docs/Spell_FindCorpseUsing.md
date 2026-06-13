# Spell::FindCorpseUsing

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Template member Spell::FindCorpseUsing<T>() returns nullptr when m_casterUnit is absent; otherwise it derives the spell's max range from DBC, searches nearby grid objects with MaNGOS::WorldObjectSearcher<T>, falls back to VisitWorldObjects if nothing matched, and returns the found WorldObject* or nullptr.

## Source
```cpp

```
