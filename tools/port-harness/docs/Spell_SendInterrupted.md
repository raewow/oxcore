# Spell::SendInterrupted

Spell::SendInterrupted sends the surrounding-player interrupt notification for a spell cast.

It builds `SMSG_SPELL_FAILED_OTHER` with the caster GUID and spell id, then broadcasts it to nearby objects so the interruption animation and feedback propagate correctly.

Implementation location: `reference/core/src/game/Spells/Spell.cpp:4769-4778`.
