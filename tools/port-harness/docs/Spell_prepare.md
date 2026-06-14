# Spell::prepare

Spell::prepare has two overloads. The `SpellCastTargets` overload moves the provided targets into `m_targets` and delegates to the `Aura*` overload.

The main overload puts the spell into `SPELL_STATE_PREPARING`, decides whether the cast is delayed, refreshes the cast start position, records triggered-aura metadata when present, schedules the spell update event, and then performs early failures for overlapping casts or disabled spells before continuing into cost and validation setup.

Implementation location: `reference/core/src/game/Spells/Spell.cpp:3300-3346`.
