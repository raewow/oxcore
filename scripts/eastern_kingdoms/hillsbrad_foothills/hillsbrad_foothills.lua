--[[
    Hillsbrad Foothills zone scripts
    Reference: eastern_kingdoms/hillsbrad_foothills/hillsbrad_foothills.cpp

    go_helcular_s_grave (entry 2083)
        OnQuestRewarded: quest 558 (Helcular's Rod Translated).
        Summons Helcular (entry 2433) at fixed coords if not already present.
        C++ checks AI state for existing summon; we always spawn (server
        dedup logic prevents exact-position duplicates).

    go_dusty_rug (entry 1966)
        OnQuestRewarded: kicks off a multi-step farmer wave event.
        The event requires GO UpdateAI loop (farmers walk to keg, kneel, die).
        Deferred — GO AI loop not yet supported in Lua.
]]

local GO_HELCULAR_S_GRAVE   = 2083
local NPC_HELCULAR           = 2433
local QUEST_HELCULAR_ROD     = 558

local go_helcular_s_grave = {}

function go_helcular_s_grave:OnQuestRewarded(player, go_guid, quest_id)
    if quest_id == QUEST_HELCULAR_ROD then
        return {
            { action = "SPAWN_CREATURE",
              entry       = NPC_HELCULAR,
              x           = -741.982,
              y           = -621.186,
              z           =   18.385,
              o           =    2.050,
              summon_type = "DEAD_DESPAWN",
              duration_ms = 0 },
        }
    end
    return {}
end

RegisterGossipScript(GO_HELCULAR_S_GRAVE, go_helcular_s_grave)

-- go_dusty_rug (entry 1966): OnQuestRewarded wave event deferred (requires GO UpdateAI)
