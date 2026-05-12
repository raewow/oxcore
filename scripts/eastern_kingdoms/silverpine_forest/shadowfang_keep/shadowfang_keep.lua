--[[
    Shadowfang Keep dungeon helper scripts.
    Ported from vmangos instance_shadowfang_keep.cpp

    Contents:
    NONE — vmangos shadowfang_keep.cpp is purely an instance state manager
    (door states, NPC visibility toggles, patrol faction management).
    All of this is already covered by instance_shadowfang_keep.lua.

    Additional vmangos content (not portable):
    - Haunting Spirits AuraScript (spell 16336): 5% periodic proc chance to
      summon Spiteful Phantom (16334) or Wrath Phantom (16335).
      Requires AuraScript/SpellScript API (blocked on Phase E).

    TODO(api): When AuraScript is available, port:
    - HauntingPhantomsScript: OnBeforeApply sets 5s periodic timer;
      OnPeriodicDummy rolls 5% chance to cast summon spell on target.
    Spell ID: 16336 (Haunting Phantoms)
]]

-- Intentionally empty: all content either in instance script or blocked on API.
