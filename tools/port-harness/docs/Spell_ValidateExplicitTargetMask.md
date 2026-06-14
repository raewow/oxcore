# Spell::ValidateExplicitTargetMask

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Const Spell validation for player client-started casts: compares m_targets.m_targetMask against spell AllowedTargetMask (disallowed extra flags) and Targets (missing required flags). Non-player casters skip validation (returns true). On mismatch logs PassiveAnticheat via the player session and returns false; otherwise returns true.

## Source
```cpp

```
