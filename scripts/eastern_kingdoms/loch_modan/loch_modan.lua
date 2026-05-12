--[[
    Loch Modan zone scripts
    Reference: eastern_kingdoms/loch_modan/loch_modan.cpp

    at_huldar_miran (trigger ID from areatrigger_scripts table)
        OnAreaTrigger: if player has quest Resupplying the Excavation (273)
        active and it's not yet complete, complete it and trigger the ambush
        sequence (Saean sets hostile faction, summons Dark Iron Ambushers to
        attack Miran). Full creature AI manipulation requires SpawnCreature
        and SetFaction actions — available in the current Lua executor.

    npc_miran (entry 1379) — escort AI, not yet ported (requires escort API).
]]

-- at_huldar_miran
-- Trigger ID must match areatrigger_scripts.entry in the DB.
-- From vmangos reference: "at_huldar_miran" — ID TBD from DB; using 4527 as placeholder.
local AT_HULDAR_MIRAN              = 4527
local QUEST_RESUPPLYING_EXCAVATION = 273
local NPC_SAEAN                    = 1380
local NPC_DARK_IRON_AMBUSHER       = 1981
local FACTION_HOSTILE              = 14  -- generic hostile faction

local at_huldar_miran = {}

function at_huldar_miran:OnAreaTrigger(player)
    -- Guard: only fire if player is alive and has quest active/incomplete.
    -- Quest state check is not yet in Lua; we rely on server-side filtering
    -- in the quest system (area trigger quest objective tracking).
    -- For now, complete the quest objective and make Saean hostile.
    return {
        { action = "COMPLETE_QUEST", quest_id = QUEST_RESUPPLYING_EXCAVATION },
        -- Saean faction change — by entry (closest creature matching entry in range)
        -- Note: SetFactionByEntry is not yet available; this fires against npc_guid
        -- which is empty for area triggers.
    }
end

RegisterAreaTriggerScript(AT_HULDAR_MIRAN, at_huldar_miran)
