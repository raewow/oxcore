--[[
    Deadmines dungeon helper scripts.
    Ported from vmangos deadmines.cpp

    Contents:
    NONE — all vmangos deadmines.cpp content is GO-based:
    - go_door_lever_dm    (GO lever interaction)
    - go_defias_cannon    (GO cannon trigger → summons Defias Overseer entry 657)
    - go_defias_gunpowder (GO gunpowder → creature summon + movement)

    All mechanics require the GameObject interaction API which is not yet
    available (blocked on Phase E GO scripting).

    TODO(api): When GO scripting is implemented, port:
    - go_door_lever_dm:     GOHello → sets loot state to open relevant door
    - go_defias_cannon:     GOHello → set loot state GO_JUST_DEACTIVATED
    - go_defias_gunpowder:  GOHello → SummonCreature(657) + MovePoint to cannon

    Relevant NPC entry: Defias Overseer = 657
]]

-- Intentionally empty: all content is GO-scripted.
-- This file serves as a placeholder so the tracker knows Deadmines was reviewed.
