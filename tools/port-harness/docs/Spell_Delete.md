# Spell::Delete

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::Delete is a const instance method that conditionally self-destructs the Spell via delete this when IsDeletable() is true; otherwise it logs a minimal-level crash message with the spell ID and leaves the object alive.

## Source
```cpp

```
