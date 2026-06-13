# Spell::SendSpellCooldown

**File:** src/game/Server/Packets/Spell.cpp

## Summary
Spell::SendSpellCooldown is a parameterless void method that early-returns for passive spells or player casters with PLAYER_CHEAT_NO_COOLDOWN, otherwise invokes m_caster->AddCooldown with m_spellInfo, an optional cast-item prototype from m_CastItem, and permanent=true when the spell has SPELL_ATTR_COOLDOWN_ON_EVENT.

## Source
```cpp

```
