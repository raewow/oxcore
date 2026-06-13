# Spell::AddItemTarget

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Registers an Item pointer as a spell target for a given effect index: no-ops when the spell effect slot is zero; otherwise deduplicates by raw pointer in m_UniqueItemInfo (OR-ing the effect bit) or appends a new ItemTargetInfo entry.

## Source
```cpp

```
