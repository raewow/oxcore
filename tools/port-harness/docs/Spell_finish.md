# Spell::finish

Spell::finish(bool ok) marks the cast result, returns immediately if the caster is missing or the spell already finished, and then transitions the spell into `SPELL_STATE_FINISHED`.

It clears the creature casting target when needed, applies player-specific cooldown fixes and cheat-option cooldown cleanup, restores pet movement flags after the cast, and runs failure cleanup such as restoring spell mods.

Implementation location: `reference/core/src/game/Spells/Spell.cpp:4269-4314`.
