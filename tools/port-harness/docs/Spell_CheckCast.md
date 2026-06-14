# Spell::CheckCast

**File:** src/game/AI/PlayerAI.cpp

## Summary
Spell::CheckCast(bool strict) is the central pre-cast validator on a Spell instance. It returns SpellCastResult, failing fast on cheat bypass, caster/target state, cooldown/GCD, environment, target suitability, items, range, power, caster auras, effect-specific rules, dispel eligibility, trade-slot deferral, and optional script overrides; it returns SPELL_CAST_OK when all checks pass. PlayerAI::CanCastSpell notes that power is checked here too, but its local pre-check uses SpellEntry::manaCost while CheckCast delegates to CheckPower(), which compares m_powerCost.

## Source
```cpp
        // Check for power (also done by Spell::CheckCast())
```
