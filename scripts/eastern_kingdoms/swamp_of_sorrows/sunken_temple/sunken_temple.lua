--[[
    @script_type: creature_ai
    @entry: 15192
    @name: sunken_temple

    Sunken Temple dungeon helper scripts.
    Ported from vmangos sunken_temple.cpp

    Contents:
    - npc_malfurion_stormrage (entry 15192): timed speech/emote sequence on spawn

    NOTE: at_shade_of_eranikus (area trigger that spawns Malfurion) is blocked
    on Phase E area-trigger API. The NPC AI itself is fully ported here.
]]

-- Spell IDs
local SPELL_RESURRECTION_VISUAL = 20761  -- visual cast on become visible

-- Text IDs (vmangos DoScriptText IDs)
-- These map to broadcast_text DB entries
local EMOTE_MALFURION1 = 11191  -- walls tremble emote (on spawn, while invisible)
local SAY_MALFURION1   = 11193
local SAY_MALFURION2   = 11194
local SAY_MALFURION3   = 11195
local SAY_MALFURION4   = 11196

-- Timer IDs
local TIMER_SPEECH = 1

-- Speech phases (mirrors vmangos m_uiSpeech counter 0-6)
-- Phase 0: become visible, roar emote, resurrection visual (after 3s)
-- Phase 1: bow emote                (after 1.5s)
-- Phase 2: SAY_MALFURION1           (after 2s)
-- Phase 3: SAY_MALFURION2           (after 10s)
-- Phase 4: SAY_MALFURION3           (after 10s)
-- Phase 5: SAY_MALFURION4           (after 8s)
-- Phase 6: set questgiver/gossip flags, done (after 5s)

local PHASE_SPAWN_HIDDEN = 0
local PHASE_APPEAR       = 1
local PHASE_BOW          = 2
local PHASE_SAY1         = 3
local PHASE_SAY2         = 4
local PHASE_SAY3         = 5
local PHASE_SAY4         = 6
local PHASE_DONE         = 7

local malfurion = {}

function malfurion:OnSpawn(input)
    -- Start invisible; fire initial 3s timer to begin speech sequence
    return {
        { action = "SET_REACT_STATE", state = "PASSIVE" },
        { action = "SET_PHASE", phase = PHASE_SPAWN_HIDDEN },
        { action = "SET_TIMER", timer_id = TIMER_SPEECH, duration = 3000 },
    }
end

function malfurion:JustRespawned(input)
    return self:OnSpawn(input)
end

function malfurion:OnUpdate(input)
    if input.phase == PHASE_DONE then return {} end
    if not input:IsTimerReady(TIMER_SPEECH) then return {} end

    local actions = {}
    local phase = input.phase

    if phase == PHASE_SPAWN_HIDDEN then
        -- Become visible, roar, cast resurrection visual
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RESURRECTION_VISUAL, target = "self", triggered = true })
        table.insert(actions, { action = "EMOTE", emote_id = 11 })  -- EMOTE_ONESHOT_ROAR
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_APPEAR })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SPEECH, duration = 1500 })

    elseif phase == PHASE_APPEAR then
        -- Bow emote
        table.insert(actions, { action = "EMOTE", emote_id = 66 })  -- EMOTE_ONESHOT_BOW
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_BOW })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SPEECH, duration = 2000 })

    elseif phase == PHASE_BOW then
        table.insert(actions, { action = "YELL", text_id = SAY_MALFURION1 })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_SAY1 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SPEECH, duration = 10000 })

    elseif phase == PHASE_SAY1 then
        table.insert(actions, { action = "YELL", text_id = SAY_MALFURION2 })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_SAY2 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SPEECH, duration = 10000 })

    elseif phase == PHASE_SAY2 then
        table.insert(actions, { action = "YELL", text_id = SAY_MALFURION3 })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_SAY3 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SPEECH, duration = 8000 })

    elseif phase == PHASE_SAY3 then
        table.insert(actions, { action = "YELL", text_id = SAY_MALFURION4 })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_SAY4 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SPEECH, duration = 5000 })

    elseif phase == PHASE_SAY4 then
        -- Set questgiver/gossip flags (NPC_FLAG_QUESTGIVER | NPC_FLAG_GOSSIP = 0x3)
        table.insert(actions, { action = "SET_UNIT_FLAG", flag = 0x3 })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_DONE })
    end

    return actions
end

RegisterCreatureAI(15192, malfurion)
