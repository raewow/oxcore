--[[
    Dun Morogh zone scripts
    Reference: eastern_kingdoms/dun_morogh/dun_morogh.cpp

    npc_narm_faulk (entry 6162)
        On spawn: set DYNFLAG_DEAD + stand state dead (playing dead).
        SpellHit 8593 (Healing Touch): stand up, remove dynflag, say heal line.
        UpdateAI: after standing, 120s timer then evade back.
        Quest: 1783 (In Defense of the King's Lands I)
]]

local NPC_NARM_FAULK        = 6162
local SPELL_HEALING_TOUCH   = 8593   -- "Healing Touch" used by quest player
local SAY_HEAL              = 2281   -- broadcast text ID
local UNIT_DYNFLAG_DEAD     = 0x20   -- matches server constant

local npc_narm_faulk = {}

local TIMER_LIFE  = 1
local DATA_HEALED = "healed"

function npc_narm_faulk:OnSpawn(input)
    return {
        { action = "SET_DYNFLAG",    flag  = UNIT_DYNFLAG_DEAD },
        { action = "SET_STAND_STATE", state = 7 }, -- UNIT_STAND_STATE_DEAD
        { action = "STOP_MOVEMENT" },
    }
end

function npc_narm_faulk:OnReset(input)
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_HEALED, value = 0 },
        { action = "SET_DYNFLAG",    flag  = UNIT_DYNFLAG_DEAD },
        { action = "SET_STAND_STATE", state = 7 },
        { action = "STOP_MOVEMENT" },
    }
end

function npc_narm_faulk:OnSpellHit(input, spell_id, caster_guid)
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

function npc_narm_faulk:OnUpdate(input)
    if input:GetCustomData(DATA_HEALED) == 1 and input:IsTimerReady(TIMER_LIFE) then
        return {
            { action = "SET_CUSTOM_DATA", key = DATA_HEALED, value = 0 },
            { action = "ENTER_EVADE_MODE" },
        }
    end
    return {}
end

RegisterCreatureAI(NPC_NARM_FAULK, npc_narm_faulk)
