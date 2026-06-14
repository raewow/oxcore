# Spell::SendSpellStart

Spell::SendSpellStart sends `SMSG_SPELL_START` when the cast should be visible to clients.

It chooses the cast-item GUID when one exists, otherwise the caster GUID, writes the caster unit GUID, spell id, cast flags, cast delay, target payload, and projectile ammo data when applicable, then broadcasts the packet to nearby clients.

Implementation location: `reference/core/src/game/Spells/Spell.cpp:4457-4492`.
