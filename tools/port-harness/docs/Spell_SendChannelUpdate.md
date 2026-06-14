# Spell::SendChannelUpdate

Spell::SendChannelUpdate updates the client-facing state for a channeled spell.

It no-ops for non-channeled or finished spells. When the update time reaches zero it performs the special cleanup path for farsight/possession-style channeling; otherwise it keeps the channel state in sync with the remaining time and interruption flag.

Implementation location: `reference/core/src/game/Spells/Spell.cpp:4810-4856`.
