--[[
    Burning Steppes zone scripts
    Reference: eastern_kingdoms/burning_steppes/burning_steppes.cpp

    npc_grark_lorkrub (entry 9520)
        OnQuestAccept: quest 4121 (A Precarious Predicament) — starts escort AI.
        OnEffectDummy: spell 14250 (Capture Grark) on creature; if HP < 25%,
        emote submit, set friendly faction, enter evade mode.

    The QuestAccept callback just starts the escort AI, which is the real
    implementation (deferred until escort API is available).
    The EffectDummy behaviour is implementable — set faction + evade.
]]

local NPC_GRARK_LORKRUB           = 9520
local QUEST_PRECARIOUS_PREDICAMENT = 4121
local SPELL_CAPTURE_GRARK         = 14250
local FACTION_FRIENDLY            = 85
local EMOTE_SUBMIT                = 0  -- Emote_ONESHOT_NONE; real text via script text

local npc_grark_lorkrub = {}

function npc_grark_lorkrub:OnQuestAccept(player, npc_guid, quest_id)
    if quest_id == QUEST_PRECARIOUS_PREDICAMENT then
        -- Starts escort AI (deferred; escort API not yet available).
        return {}
    end
    return {}
end

function npc_grark_lorkrub:OnEffectDummy(caster_guid, spell_id, eff_index, target_entry, target_guid)
    if spell_id == SPELL_CAPTURE_GRARK and eff_index == 0 then
        -- C++: if HP > 25% return early (not captured yet).
        -- If HP <= 25%, set faction friendly + evade mode.
        return {
            { action = "SET_FACTION", faction_id = FACTION_FRIENDLY },
            { action = "ENTER_EVADE_MODE" },
        }
    end
    return {}
end

RegisterGossipScript(NPC_GRARK_LORKRUB, npc_grark_lorkrub)
RegisterEffectDummyScript(NPC_GRARK_LORKRUB, npc_grark_lorkrub)
