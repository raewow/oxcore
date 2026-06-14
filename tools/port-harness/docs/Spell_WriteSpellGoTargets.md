# Spell::WriteSpellGoTargets

Spell::WriteSpellGoTargets writes the target summary used by `SMSG_SPELL_GO`.

It first marks fully immuned unit targets as `SPELL_MISS_IMMUNE2`, then serializes the hit target list, the GO target list, and the miss target list. For hit targets it updates `m_needAliveTargetMask`, which later helps channeled spells stop when a needed target dies.

Implementation location: `reference/core/src/game/Spells/Spell.cpp:4597-4692`.
