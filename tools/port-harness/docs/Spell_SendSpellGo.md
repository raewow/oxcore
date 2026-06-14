# Spell::SendSpellGo

Spell::SendSpellGo sends `SMSG_SPELL_GO` for the execution phase of a visible cast.

If the cast is not client-visible it falls back to `SendAllTargetsMiss()` and returns. Otherwise it writes the cast-item or caster GUID, caster unit GUID, spell id, cast flags, serialized hit/miss targets, target payload, and ammo data before broadcasting the packet.

Implementation location: `reference/core/src/game/Spells/Spell.cpp:4494-4527`.
