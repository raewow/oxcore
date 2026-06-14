# Spell::SetClientStarted

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::SetClientStarted is a void mutator that assigns its bool parameter to the Spell instance member m_isClientStarted. It performs no validation, branching, or other side effects. The stored flag is later read during strict CheckCast to gate ValidateExplicitTargetMask (Spell.cpp:5391). Callers set it true for client opcode casts (SpellHandler.cpp:315) and to !IsBot() for item-use casts (Player.cpp:7537). Note: port-harness indexes this at src/game/Server/Packets/Spell.cpp:8220 due to Spell.cpp basename collision; the implementation lives in src/game/Spells/Spell.cpp:8220-8223.

## Source
```cpp

```
