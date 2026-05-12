--[[
    Mulgore zone scripts
    Reference: kalimdor/mulgore/mulgore.cpp

    npc_plains_vision (entry 3489)
        The C++ uses npc_escortAI: on first UpdateAI tick it calls Start()
        to begin the escort walk. MoveInLineOfSight is empty (no aggro).
        If it has a target it does melee.

        In Lua we suppress aggro via OnMoveInLineOfSight and let the server's
        DB waypoint motion type handle the path automatically (set in DB).
        OnUpdate handles melee if combat is somehow triggered.
]]

local NPC_PLAINS_VISION = 3489

local npc_plains_vision = {}

function npc_plains_vision:OnMoveInLineOfSight(self, npc_guid, unit_guid, is_hostile)
    -- C++ override is empty; suppress default aggro
    return {}
end

function npc_plains_vision:OnUpdate(self, npc_guid, diff_ms, input)
    if input.is_in_combat and input.target_guid then
        return { { action = "MELEE_ATTACK", target_guid = input.target_guid } }
    end
    return {}
end

RegisterCreatureAI(NPC_PLAINS_VISION, npc_plains_vision)
