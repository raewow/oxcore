# Spell::SendSpellCooldown

Spell::SendSpellCooldown adds the spell cooldown to the caster unless the spell is passive or the player has the no-cooldown cheat enabled.

For normal casts it calls `m_caster->AddCooldown(...)`, passing the spell prototype, the cast item prototype when one exists, and the event-cooldown flag when the spell has `SPELL_ATTR_COOLDOWN_ON_EVENT`.

Implementation location: `reference/core/src/game/Spells/Spell.cpp:3936-3946`.
