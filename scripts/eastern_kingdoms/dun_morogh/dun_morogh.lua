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

function npc_narm_faulk:OnSpawn(self, npc_guid)
    return {
        { action = "SET_DYNFLAG",    flag  = UNIT_DYNFLAG_DEAD },
        { action = "SET_STAND_STATE", state = 7 }, -- UNIT_STAND_STATE_DEAD
        { action = "STOP_MOVEMENT" },
    }
end

function npc_narm_faulk:OnReset(self, npc_guid)
    return {
        { action = "SET_DYNFLAG",    flag  = UNIT_DYNFLAG_DEAD },
        { action = "SET_STAND_STATE", state = 7 },
        { action = "STOP_MOVEMENT" },
    }
end

function npc_narm_faulk:OnSpellHit(self, npc_guid, spell_id, caster_guid)
    if spell_id == SPELL_HEALING_TOUCH and not self.spell_hit then
        self.spell_hit  = true
        self.life_timer = 120000
        return {
            { action = "SET_STAND_STATE", state = 0 }, -- STAND
            { action = "REMOVE_DYNFLAG",  flag  = UNIT_DYNFLAG_DEAD },
            { action = "SAY", text_id = SAY_HEAL },
        }
    end
    return {}
end

function npc_narm_faulk:OnUpdate(self, npc_guid, diff_ms, input)
    if self.spell_hit then
        self.life_timer = (self.life_timer or 120000) - diff_ms
        if self.life_timer <= 0 then
            self.spell_hit = false
            return { { action = "ENTER_EVADE_MODE" } }
        end
    end
    return {}
end

RegisterCreatureAI(NPC_NARM_FAULK, npc_narm_faulk)
