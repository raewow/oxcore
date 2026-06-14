# Spell::SendAllTargetsMiss

Spell::SendAllTargetsMiss emits `SMSG_SPELLLOGMISS` when every unique target on the spell missed.

It returns early if the caster is not in world, if there are no unique targets, or if any target actually landed. When all targets missed, it serializes the spell id, caster guid, target count, and each target GUID plus miss condition.

Implementation location: `reference/core/src/game/Spells/Spell.cpp:4780-4807`.
