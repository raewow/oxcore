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

function npc_henze_faulk:OnSpawn(self, npc_guid)
    return {
        { action = "SET_DYNFLAG",    flag  = UNIT_DYNFLAG_DEAD },
        { action = "SET_STAND_STATE", state = 7 },
        { action = "STOP_MOVEMENT" },
    }
end

function npc_henze_faulk:OnReset(self, npc_guid)
    return {
        { action = "SET_DYNFLAG",    flag  = UNIT_DYNFLAG_DEAD },
        { action = "SET_STAND_STATE", state = 7 },
        { action = "STOP_MOVEMENT" },
    }
end

function npc_henze_faulk:OnSpellHit(self, npc_guid, spell_id, caster_guid)
    if spell_id == SPELL_HEALING_TOUCH and not self.spell_hit then
        self.spell_hit  = true
        self.life_timer = 120000
        return {
            { action = "SET_STAND_STATE", state = 0 },
            { action = "REMOVE_DYNFLAG",  flag  = UNIT_DYNFLAG_DEAD },
            { action = "SAY", text_id = SAY_HEAL },
        }
    end
    return {}
end

function npc_henze_faulk:OnUpdate(self, npc_guid, diff_ms, input)
    if self.spell_hit then
        self.life_timer = (self.life_timer or 120000) - diff_ms
        if self.life_timer <= 0 then
            self.spell_hit = false
            return { { action = "ENTER_EVADE_MODE" } }
        end
    end
    return {}
end

RegisterCreatureAI(NPC_HENZE_FAULK, npc_henze_faulk)
