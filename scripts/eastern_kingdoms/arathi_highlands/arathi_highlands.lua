--[[
    Arathi Highlands zone scripts
    Reference: eastern_kingdoms/arathi_highlands/arathi_highlands.cpp

    npc_professor_phizzlethorpe (entry 2768)
        Quest support: Sprinkle's Secret Ingredient (id 665)
        OnQuestAccept: set escort faction + say progress text

    npc_shakes_o_breen (entry 2610)
        Quest support: Shakes O'Breen (id 667)
        OnQuestAccept: yell text

    npc_kinelory (entry 2713)
        Quest support: Hints of a New Plague? (id 660)
        OnQuestAccept: say start text + set escort faction
        Note: escort AI logic handled by creature_ai script (not yet ported).
]]

-- npc_professor_phizzlethorpe (entry 2768)
local QUEST_SPRINKLES_SECRET        = 665
local FACTION_ESCORT_N_NEUTRAL_PASSIVE = 113  -- FACTION_ESCORT_N_NEUTRAL_PASSIVE
local SAY_PHIZZ_PROGRESS_1          = 845

local npc_phizzlethorpe = {}

function npc_phizzlethorpe:OnQuestAccept(player, npc_guid, quest_id)
    if quest_id == QUEST_SPRINKLES_SECRET then
        return {
            { action = "SET_FACTION", faction_id = FACTION_ESCORT_N_NEUTRAL_PASSIVE },
            { action = "SCRIPT_TEXT", text_id = SAY_PHIZZ_PROGRESS_1 },
        }
    end
    return {}
end

RegisterGossipScript(2768, npc_phizzlethorpe)

-- npc_shakes_o_breen (entry 2610)
local QUEST_SHAKES_OBREEN           = 667
local BREEN_YELL_1                  = 6372

local npc_shakes = {}

function npc_shakes:OnQuestAccept(player, npc_guid, quest_id)
    if quest_id == QUEST_SHAKES_OBREEN then
        return {
            { action = "SCRIPT_TEXT", text_id = BREEN_YELL_1 },
        }
    end
    return {}
end

RegisterGossipScript(2610, npc_shakes)

-- npc_kinelory (entry 2713)
local QUEST_HINTS_NEW_PLAGUE        = 660
local SAY_KINELORY_START            = 816

local npc_kinelory = {}

function npc_kinelory:OnQuestAccept(player, npc_guid, quest_id)
    if quest_id == QUEST_HINTS_NEW_PLAGUE then
        return {
            { action = "SCRIPT_TEXT", text_id = SAY_KINELORY_START },
            { action = "SET_FACTION", faction_id = FACTION_ESCORT_N_NEUTRAL_PASSIVE },
        }
    end
    return {}
end

RegisterGossipScript(2713, npc_kinelory)
