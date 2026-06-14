# Spell::ShouldRemoveStealthAuras

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Instance method that returns whether stealth auras should be removed when this spell is cast. Returns false immediately if there is no caster unit. Otherwise returns true for most non-triggered casts that lack SPELL_ATTR_EX_ALLOW_WHILE_STEALTHED and are not Camouflage (250), Shadowmeld (103), or Vanish (252); Sap (249) on a player may retain stealth probabilistically based on Improved Sap aura ranks (14076/14094/14095). Returns false in all other cases.

## Source
```cpp

```
