--[[
    Ashenvale zone scripts
    Reference: kalimdor/ashenvale/ashenvale.cpp

    event_king_of_the_foulweald (event_id from DB)
        OnProcessEventId: when the totem mound event fires, starts the
        enraged foulweald event sequence. The full AI-driven event requires
        game object AI (go_foulweald_totem_moundAI) which is not yet available.
        This stub fires and logs; full implementation requires GO AI support.

    npc_ruul_snowhoof (entry not deterministic from script alone — uses GetAI)
        Quest support: Freedom to Ruul (6482) — escort AI, not yet ported.

    npc_torek (escort AI) — not yet ported.
    npc_feero_ironhand (escort AI) — not yet ported.
]]

-- event_king_of_the_foulweald
-- The event_id is set by the DB (eventai or waypoint event); the exact ID
-- must match what is configured in the event_scripts or eventai table.
-- From vmangos reference: event registered as "event_king_of_the_foulweald".
-- Placeholder event_id: 14861 (to be confirmed against DB).
local EVENT_KING_OF_THE_FOULWEALD = 14861

local event_king = {}

function event_king:OnProcessEventId(player, event_id, source_guid, is_start)
    -- The C++ version accesses the totem mound GO AI and calls EventStart.
    -- That requires game object AI support which is not yet available.
    -- Log the event firing; full GO AI behavior is Phase E+.
    return {}
end

RegisterProcessEventScript(EVENT_KING_OF_THE_FOULWEALD, event_king)
