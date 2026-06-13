# Spell::Spell

**File:** Spell.cpp

## Summary
Spell::Spell (Unit* overload) constructs a Spell from a Unit caster and SpellEntry: initializes member fields via the initializer list, asserts non-null caster/info and that info is the canonical sSpellMgr entry, derives attack type and school mask (with wand override for ranged wand users), sets original caster GUID/pointer, precomputes effect base points, configures auto-repeat/channel/reflect flags, resets channel iterator and target list, and optionally loads and invokes a spell script OnInit hook.

## Source
```cpp
}
```
