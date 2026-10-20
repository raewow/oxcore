--[[
    Elwynn Forest zone scripts
    Reference: eastern_kingdoms/elwynn_forest/elwynn_forest.cpp

    npc_henze_faulk (entry 6165)
        Identical behaviour to npc_narm_faulk in Dun Morogh:
        spawns playing dead, healed by spell 8593, stands up, says line,
        then after 120s evades back.
        Quest: 1786 (In Defense of the King's Lands I - Elwynn branch)
]]

local NPC_HENZE_FAULK       = 6165
local SPELL_HEALING_TOUCH   = 8593
local SAY_HEAL              = 2283   -- broadcast text ID (different from Narm)
local UNIT_DYNFLAG_DEAD     = 0x20

local npc_henze_faulk = {}

local TIMER_LIFE  = 1
local DATA_HEALED = "healed"

function npc_henze_faulk:OnSpawn(input)
    return {
        { action = "SET_DYNFLAG",    flag  = UNIT_DYNFLAG_DEAD },
        { action = "SET_STAND_STATE", state = 7 }, -- UNIT_STAND_STATE_DEAD
        { action = "STOP_MOVEMENT" },
    }
end

function npc_henze_faulk:OnReset(input)
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_HEALED, value = 0 },
        { action = "SET_DYNFLAG",    flag  = UNIT_DYNFLAG_DEAD },
        { action = "SET_STAND_STATE", state = 7 },
        { action = "STOP_MOVEMENT" },
    }
end

function npc_henze_faulk:OnSpellHit(input, spell_id, caster_guid)
    if spell_id ~= SPELL_HEALING_TOUCH or input:GetCustomData(DATA_HEALED) == 1 then
        return {}
    end
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_HEALED, value = 1 },
        { action = "SET_TIMER", timer_id = TIMER_LIFE, duration = 120000 },
        { action = "SET_STAND_STATE", state = 0 }, -- STAND
        { action = "REMOVE_DYNFLAG",  flag  = UNIT_DYNFLAG_DEAD },
        { action = "SCRIPT_TEXT", text_id = SAY_HEAL },
    }
end

function npc_henze_faulk:OnUpdate(input)
    if input:GetCustomData(DATA_HEALED) == 1 and input:IsTimerReady(TIMER_LIFE) then
        return {
            { action = "SET_CUSTOM_DATA", key = DATA_HEALED, value = 0 },
            { action = "ENTER_EVADE_MODE" },
        }
    end
    return {}
end

RegisterCreatureAI(NPC_HENZE_FAULK, npc_henze_faulk)
