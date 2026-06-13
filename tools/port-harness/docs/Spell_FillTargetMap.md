# Spell::FillTargetMap

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::FillTargetMap builds per-effect unit target lists for a cast: it iterates MAX_EFFECT_INDEX effect slots, skips SPELL_EFFECT_NONE, optionally pre-registers the caster for area-aura and related effects, may copy targets from a prior effect when MaxAffectedTargets > 1 and implicit targets match, resolves EffectImplicitTargetA/B pairs through nested switches into a temporary UnitList via SetTargetMap (with special-case destination and magnet-target handling), filters candidates with CheckTarget, registers survivors through AddUnitTarget, then enforces SPELL_ATTR_EX2_FAIL_ON_ALL_TARGETS_IMMUNE (abort cast if every target is immune) and SPELL_ATTR_EX_REQUIRE_ALL_TARGETS (zero all effect masks and return if the explicit unit target missed).

## Source
```cpp

```
