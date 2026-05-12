--[[
    @script_type: creature_ai
    @entry: 10220
    @name: boss_halycon

    Boss Halycon - Lower Blackrock Spire
    Ported from MaNGOS boss_halycon.cpp
]]

local SPELL_CROWD_PUMMEL = 10887
local SPELL_MIGHTY_BLOW  = 14099

local NPC_GIZRUL = 10268

local TIMER_CROWD_PUMMEL = 1
local TIMER_MIGHTY_BLOW  = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_CROWD_PUMMEL, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_MIGHTY_BLOW, duration = 14000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_CROWD_PUMMEL, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_MIGHTY_BLOW, duration = 14000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    if input:IsTimerReady(TIMER_CROWD_PUMMEL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CROWD_PUMMEL, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CROWD_PUMMEL, duration = 14000 })
    end

    if input:IsTimerReady(TIMER_MIGHTY_BLOW) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MIGHTY_BLOW, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MIGHTY_BLOW, duration = 10000 })
    end

    return actions
end

function boss:OnDeath(input, killer_guid)
    return {
        { action = "TEXT_EMOTE", text = "Halycon lets loose a gutteral growl as her body collapses. A horrifying howl can be heard echoing through the halls of Blackrock Spire. Something is very, very angry." },
        { action = "SPAWN_CREATURE", entry = NPC_GIZRUL, x = -167.58, y = -382.41, z = 64.401, o = 1.563, summon_type = "DEAD_DESPAWN", duration = 0 },
    }
end

RegisterCreatureAI(10220, boss)
